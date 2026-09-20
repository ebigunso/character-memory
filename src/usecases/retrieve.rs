mod activity;
mod scene;

// Continuity retrieval pipeline used by the public facade and internal tests.
// Some helper APIs are intentionally retained for retrieval policy validation.
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::api::types::{
    ContextPackSection, ContinuityContextPack, CueKind, FanoutUtilizationTrace,
    GraphExpansionOutcome, GraphExpansionTelemetry, GraphExpansionTrace, GraphRootSource,
    IncludedDerivedMemory, LifecycleFilterAction, LifecycleFilterDecision, LifecycleFilterReason,
    LifecycleOmissionSummary, RetrievalContext, RetrievalRationale, RetrievalTelemetry,
    RetrievalTrace, RetrieveOutcome, SectionAssignment, SectionAssignmentReason,
    SectionPressureSummary, SectionScoreComponents, SelectivityTelemetry, StaleCandidateOmission,
    StaleCandidateOmissionSummary, StaleCandidateReason, VectorCandidateTrace,
};
use crate::domain::{
    DerivedMemory, DerivedType, GraphExpansionBoundedReason, GraphFailureMode, MemoryId,
    MemoryObject, MemoryObjectRef, ObjectType, RelationType, ThreadStatus, VectorSurface,
};
use crate::errors::CustomError;
use crate::models::vector::{EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch};
use crate::policy::graph_expansion::graph_expansion_bounded_failure_trace;
use crate::policy::{
    selectivity_plan_for_entity, RetrievalSelectivityPolicy, SelectivityPlan,
    SelectivityStatsContext,
};
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphExpansion, GraphExpansionBoundedFailure,
    GraphExpansionBoundedFailureReason, GraphExpansionFailurePolicy, GraphExpansionFilteredReason,
    GraphExpansionLifecyclePolicy, GraphExpansionQuery, TraceMode,
};
use crate::ports::retrieval_stats::RetrievalStatsStore;
#[cfg(test)]
use crate::ports::vector_candidate::VectorCandidateRecall;
use crate::ports::vector_candidate::VectorCandidateStore;

pub(crate) struct RetrievePipeline<'a, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    graph_store: &'a G,
    vector_store: &'a V,
    embedder: &'a E,
    stats_store: &'a dyn RetrievalStatsStore,
    selectivity_policy: RetrievalSelectivityPolicy,
}

impl<'a, G, V, E> RetrievePipeline<'a, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    #[cfg(test)]
    pub(crate) fn new(graph_store: &'a G, vector_store: &'a V, embedder: &'a E) -> Self {
        Self {
            graph_store,
            vector_store,
            embedder,
            stats_store: crate::adapters::stats::noop_retrieval_stats_store(),
            selectivity_policy: RetrievalSelectivityPolicy::default(),
        }
    }

    pub(crate) fn new_with_stats(
        graph_store: &'a G,
        vector_store: &'a V,
        embedder: &'a E,
        stats_store: &'a dyn RetrievalStatsStore,
        selectivity_policy: RetrievalSelectivityPolicy,
    ) -> Self {
        Self {
            graph_store,
            vector_store,
            embedder,
            stats_store,
            selectivity_policy,
        }
    }

    pub(crate) async fn retrieve(
        &self,
        mut context: RetrievalContext,
    ) -> Result<RetrieveOutcome, CustomError> {
        context.scene = context.scene.without_blank_participants();
        context.validate()?;
        let cues = self.recall_cues(&context).await?;
        let vector_candidates = cues.candidates;
        let query_embedding_dimension = cues.dimension;
        let vector_recall_completeness = cues.completeness;
        let trace_mode = TraceMode::from_enabled(context.include_trace);
        let mut explicit_roots = cues
            .participants
            .iter()
            .map(|id| CandidateRoot {
                object_id: *id,
                object_type: ObjectType::Entity,
                score: 1.0,
                source: GraphRootSource::Participant,
                vector_score: None,
                cue_kinds: BTreeSet::from([CueKind::Participant]),
            })
            .collect::<Vec<_>>();
        let (activity, activity_roots) = self.activity_roots(&context).await?;
        explicit_roots.extend(activity_roots);
        let root_selection = select_candidate_roots(
            &vector_candidates,
            &cues.kinds,
            &explicit_roots,
            context.candidate_limits.max_graph_roots,
        );
        let candidate_roots = root_selection.roots;
        let mut assembly = RetrieveAssembly::new(trace_mode);
        let mut graph_expansion_telemetry = GraphExpansionTelemetry::default();
        let mut selectivity_telemetry = SelectivityTelemetry::default();
        let mut graph_expansion_traces = trace_mode.is_enabled().then(Vec::new);
        let mut fanout_utilization_traces = trace_mode.is_enabled().then(Vec::new);
        let mut selectivity_traces = trace_mode.is_enabled().then(Vec::new);
        let selectivity_stats_context = if candidate_roots
            .iter()
            .any(|candidate| candidate.object_type == ObjectType::Entity)
        {
            Some(
                SelectivityStatsContext::load_with_scope(
                    self.stats_store,
                    &context.graph_limits.allowed_object_types,
                    &context.graph_limits.allowed_relation_types,
                )
                .await?,
            )
        } else {
            None
        };

        for candidate in &candidate_roots {
            let selectivity_plan = if let Some(stats_context) = selectivity_stats_context
                .as_ref()
                .filter(|_| candidate.object_type == ObjectType::Entity)
            {
                selectivity_plan_for_entity(
                    candidate.object_id,
                    candidate.score,
                    context.graph_limits.max_fanout_per_node,
                    self.stats_store,
                    self.selectivity_policy,
                    stats_context,
                    context.lifecycle_policy,
                    trace_mode,
                )
                .await?
            } else {
                SelectivityPlan::default()
            };
            absorb_selectivity_telemetry(&mut selectivity_telemetry, &selectivity_plan.telemetry);
            if let Some(traces) = &mut selectivity_traces {
                traces.extend(selectivity_plan.traces);
            }
            let query =
                graph_query_for_candidate(candidate, &context, selectivity_plan.fanout_overrides)
                    .with_fanout_utilization_recording(trace_mode);
            graph_expansion_telemetry.attempted_root_count += 1;
            match self.graph_store.expand_bounded(&query).await {
                Ok(expansion) => {
                    graph_expansion_telemetry.expanded_root_count += 1;
                    record_expansion_telemetry(&mut graph_expansion_telemetry, &expansion);
                    if let Some(traces) = &mut graph_expansion_traces {
                        traces.push(graph_expansion_trace(candidate, &expansion));
                    }
                    if let Some(traces) = &mut fanout_utilization_traces {
                        traces.extend(fanout_utilization_traces_for_expansion(&expansion));
                    }
                    if let Some(failure) = expansion.bounded_failure {
                        if context.graph_limits.failure_mode == GraphFailureMode::FailClosed {
                            return Err(bounded_failure_error(failure));
                        }
                    }
                    assembly.absorb_expansion(candidate, expansion);
                }
                Err(CustomError::GraphExpansionRootNotFound { .. }) => {
                    graph_expansion_telemetry.missing_root_count += 1;
                    if let Some(traces) = &mut graph_expansion_traces {
                        traces.push(missing_root_expansion_trace(candidate));
                    }
                    assembly.omit_missing_candidate(candidate)
                }
                Err(error) => return Err(error),
            }
        }

        if let Some(traces) = &mut graph_expansion_traces {
            traces.extend(
                root_selection
                    .omitted
                    .iter()
                    .map(|root| {
                        let mut trace = missing_root_expansion_trace(root);
                        trace.outcome = GraphExpansionOutcome::RootLimit;
                        trace
                    })
                    .collect::<Vec<_>>(),
            );
        }

        let ranked_objects = assembly.ranked_objects();
        let mut details = RetrievalDetails {
            lifecycle_filter_decisions: assembly.lifecycle_decisions,
            stale_candidate_omissions: assembly.stale_omissions,
            section_assignments: Vec::new(),
        };

        let mut section_pressure = initial_section_pressure(context.section_limits);
        let pack = build_pack(
            ranked_objects,
            context.section_limits,
            &mut details,
            &mut section_pressure,
        );
        let graph_verified_count = included_section_assignment_count(&details.section_assignments);
        let stale_candidate_omission_reasons =
            summarize_stale_candidate_omissions(&details.stale_candidate_omissions);
        let lifecycle_omission_reasons =
            summarize_lifecycle_omissions(&details.lifecycle_filter_decisions);
        let stale_candidate_omission_count = stale_candidate_omission_reasons
            .iter()
            .map(|summary| summary.count)
            .sum();
        let lifecycle_omission_count = lifecycle_omission_reasons
            .iter()
            .map(|summary| summary.count)
            .sum();
        let mut rationale = RetrievalRationale::new(rationale_summary(
            vector_candidates.len(),
            graph_verified_count,
            stale_candidate_omission_count,
            lifecycle_omission_count,
        ));
        rationale.vector_candidate_count = vector_candidates.len();
        rationale.graph_verified_count = graph_verified_count;
        rationale.stale_candidate_omission_count = stale_candidate_omission_count;
        rationale.stale_candidate_omission_reasons = stale_candidate_omission_reasons;
        rationale.lifecycle_omission_count = lifecycle_omission_count;
        rationale.lifecycle_omission_reasons = lifecycle_omission_reasons;
        rationale.telemetry = RetrievalTelemetry {
            configured_candidate_limits: context.candidate_limits,
            configured_graph_limits: context.graph_limits.clone(),
            configured_section_limits: context.section_limits,
            configured_object_types: context.object_type_defaults.clone(),
            configured_lifecycle_policy: context.lifecycle_policy,
            query_embedding_dimension,
            returned_vector_candidate_count: vector_candidates.len(),
            vector_recall_completeness,
            unique_graph_root_candidate_count: root_selection.unique_count,
            selected_graph_root_count: candidate_roots.len(),
            graph_root_omission_count: root_selection.omitted.len(),
            graph_expansion: graph_expansion_telemetry,
            selectivity: selectivity_telemetry,
            section_pressure,
        };
        let trace = trace_mode.is_enabled().then(|| RetrievalTrace {
            vector_candidates: vector_candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| VectorCandidateTrace {
                    object: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
                    surface: candidate.surface,
                    score: candidate.score,
                    rank: index + 1,
                })
                .collect(),
            graph_relations: assembly.graph_relations.unwrap_or_default(),
            graph_expansions: graph_expansion_traces.unwrap_or_default(),
            fanout_utilization: fanout_utilization_traces.unwrap_or_default(),
            selectivity_decisions: selectivity_traces.unwrap_or_default(),
            lifecycle_filter_decisions: details.lifecycle_filter_decisions,
            stale_candidate_omissions: details.stale_candidate_omissions,
            section_assignments: details.section_assignments,
        });

        let memory_scenes = self
            .memory_scenes(&pack, context.lifecycle_policy.include_suppressed)
            .await?;
        Ok(RetrieveOutcome {
            scene: context.scene,
            activity,
            scene_references: cues.references,
            memory_scenes,
            pack,
            rationale,
            trace,
        })
    }
}

#[derive(Debug, Default)]
struct RetrievalDetails {
    lifecycle_filter_decisions: Vec<LifecycleFilterDecision>,
    stale_candidate_omissions: Vec<StaleCandidateOmission>,
    section_assignments: Vec<SectionAssignment>,
}

#[derive(Debug, Default)]
struct RetrieveAssembly {
    objects: HashMap<MemoryObjectRef, RankedObject>,
    superseded_by: HashMap<MemoryId, Vec<MemoryId>>,
    lifecycle_decisions: Vec<LifecycleFilterDecision>,
    stale_omissions: Vec<StaleCandidateOmission>,
    graph_relations: Option<Vec<crate::api::types::GraphRelationTrace>>,
}

impl RetrieveAssembly {
    fn new(trace_mode: TraceMode) -> Self {
        Self {
            graph_relations: trace_mode.is_enabled().then(Vec::new),
            ..Self::default()
        }
    }

    fn absorb_expansion(&mut self, candidate: &CandidateRoot, expansion: GraphExpansion) {
        let bounded_failure = expansion.bounded_failure;
        let candidate_ref =
            MemoryObjectRef::from_id_type(candidate.object_id, candidate.object_type);

        for relation in &expansion.relations {
            if relation.relation == RelationType::Supersedes
                && relation.from.object_type == ObjectType::DerivedMemory
                && relation.to.object_type == ObjectType::DerivedMemory
            {
                self.superseded_by
                    .entry(relation.to.id)
                    .or_default()
                    .push(relation.from.id);
            }
        }

        if let Some(graph_relations) = &mut self.graph_relations {
            for relation in &expansion.relations {
                graph_relations.push(crate::api::types::GraphRelationTrace {
                    link_id: relation.link_id,
                    from: MemoryObjectRef::new(relation.from.object_type, relation.from.id),
                    to: MemoryObjectRef::new(relation.to.object_type, relation.to.id),
                    relation: relation.relation,
                    proximity: relation.proximity,
                });
            }
        }

        let mut proximity_by_ref = HashMap::new();
        proximity_by_ref.insert(candidate_ref, 0_u8);
        for relation in &expansion.relations {
            proximity_by_ref
                .entry(relation.from)
                .and_modify(|proximity| *proximity = (*proximity).min(relation.proximity))
                .or_insert(relation.proximity);
            proximity_by_ref
                .entry(relation.to)
                .and_modify(|proximity| *proximity = (*proximity).min(relation.proximity))
                .or_insert(relation.proximity);
        }

        let mut root_verified = false;
        for object in expansion.objects {
            let object_ref = object.object_ref();
            if object_ref == candidate_ref {
                root_verified = true;
            }
            let graph_component = proximity_by_ref
                .get(&object_ref)
                .copied()
                .map(graph_component)
                .unwrap_or(0.0);
            let inherited_cue = if object_ref == candidate_ref {
                candidate.score
            } else {
                candidate.score * 0.75
            };
            let candidate_score = candidate
                .vector_score
                .filter(|_| object_ref == candidate_ref);
            self.objects
                .entry(object_ref)
                .and_modify(|ranked| {
                    ranked.cue_component = ranked.cue_component.max(inherited_cue);
                    ranked.cue_kinds.extend(&candidate.cue_kinds);
                    ranked.graph_component = ranked.graph_component.max(graph_component);
                    if let Some(candidate_score) = candidate_score {
                        ranked.vector_candidate_score = Some(
                            ranked
                                .vector_candidate_score
                                .map(|score| score.max(candidate_score))
                                .unwrap_or(candidate_score),
                        );
                    }
                })
                .or_insert_with(|| {
                    RankedObject::new(
                        object,
                        inherited_cue,
                        candidate.cue_kinds.clone(),
                        graph_component,
                        candidate_score,
                    )
                });
        }

        let mut root_filtered = false;
        for filtered in expansion.filtered_nodes {
            let decision = filtered_lifecycle_decision(
                filtered.object_ref,
                filtered.reason,
                &filtered.superseded_by,
            );
            if filtered.object_ref == candidate_ref {
                root_filtered = true;
                let stale_reason = stale_reason_from_filtered(filtered.reason);
                self.stale_omissions.push(StaleCandidateOmission {
                    candidate: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
                    vector_score: candidate.vector_score,
                    reason: stale_reason,
                });
            }
            self.lifecycle_decisions.push(decision);
        }

        if !root_filtered && !root_verified && !self.objects.contains_key(&candidate_ref) {
            if bounded_failure.is_some() {
                self.omit_bounded_candidate(candidate);
            } else {
                self.omit_missing_candidate(candidate);
            }
        }
    }

    fn omit_bounded_candidate(&mut self, candidate: &CandidateRoot) {
        self.stale_omissions.push(StaleCandidateOmission {
            candidate: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
            vector_score: candidate.vector_score,
            reason: StaleCandidateReason::GraphExpansionBounded,
        });
        self.lifecycle_decisions.push(LifecycleFilterDecision {
            object: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
            retention_state: None,
            superseded_by: Vec::new(),
            action: LifecycleFilterAction::Omitted,
            reason: LifecycleFilterReason::GraphExpansionBounded,
        });
    }

    fn omit_missing_candidate(&mut self, candidate: &CandidateRoot) {
        self.stale_omissions.push(StaleCandidateOmission {
            candidate: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
            vector_score: candidate.vector_score,
            reason: StaleCandidateReason::GraphObjectMissing,
        });
        self.lifecycle_decisions.push(LifecycleFilterDecision {
            object: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
            retention_state: None,
            superseded_by: Vec::new(),
            action: LifecycleFilterAction::Omitted,
            reason: LifecycleFilterReason::GraphObjectMissing,
        });
    }

    fn ranked_objects(&mut self) -> Vec<RankedObject> {
        for superseded in self.superseded_by.values_mut() {
            superseded.sort();
            superseded.dedup();
        }

        let mut ranked_objects = Vec::new();
        for (_, ranked) in std::mem::take(&mut self.objects) {
            ranked_objects.push(ranked);
        }

        ranked_objects.sort_by_key(|ranked| ranked.rank_key());
        self.stale_omissions.sort_by_key(|omission| {
            (
                omission.candidate.id,
                omission.candidate.object_type.stable_rank(),
                stale_reason_rank(omission.reason),
            )
        });
        self.stale_omissions.dedup_by_key(|omission| {
            (
                omission.candidate.object_type,
                omission.candidate.id,
                omission.reason,
            )
        });

        ranked_objects
    }
}

#[derive(Debug, Clone)]
struct RankedObject {
    object: MemoryObject,
    cue_component: f32,
    cue_kinds: BTreeSet<CueKind>,
    vector_candidate_score: Option<f32>,
    graph_component: f32,
    salience_component: f32,
}

impl RankedObject {
    fn new(
        object: MemoryObject,
        cue_component: f32,
        cue_kinds: BTreeSet<CueKind>,
        graph_component: f32,
        vector_candidate_score: Option<f32>,
    ) -> Self {
        let salience_component = salience_component(&object);
        Self {
            object,
            cue_component,
            cue_kinds,
            vector_candidate_score,
            graph_component,
            salience_component,
        }
    }

    fn final_score(&self) -> f32 {
        (self.cue_component * 0.65)
            + (self.graph_component * 0.25)
            + (self.salience_component * 0.10)
    }

    fn rank_key(&self) -> RankKey {
        let object_id = self.object.id();
        let object_type = self.object.object_type();
        RankKey {
            score: SortableScore(self.final_score()),
            object_type_rank: object_type.stable_rank(),
            object_id,
        }
    }

    fn section_score_components(&self) -> SectionScoreComponents {
        SectionScoreComponents {
            final_score: self.final_score(),
            cue_score: Some(self.cue_component),
            graph_score: (self.graph_component > 0.0).then_some(self.graph_component),
            salience_score: (self.salience_component > 0.0).then_some(self.salience_component),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SortableScore(f32);

impl PartialEq for SortableScore {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for SortableScore {}

impl PartialOrd for SortableScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SortableScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.total_cmp(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RankKey {
    score: SortableScore,
    object_type_rank: u8,
    object_id: MemoryId,
}

fn build_pack(
    ranked_objects: Vec<RankedObject>,
    limits: crate::api::types::ContinuitySectionLimits,
    details: &mut RetrievalDetails,
    section_pressure: &mut [SectionPressureSummary],
) -> ContinuityContextPack {
    let mut pack = ContinuityContextPack::empty();
    let mut section_counts = SectionCounts::default();

    for ranked in ranked_objects {
        let Some(section) = section_for_object(&ranked.object) else {
            details.section_assignments.push(SectionAssignment {
                object: ranked.object.object_ref(),
                section: ContextPackSection::Omitted,
                rank: None,
                reason: section_omission_reason(&ranked.object),
                cue_kinds: ranked.cue_kinds.clone(),
            });
            continue;
        };

        let count = section_counts.count_mut(section);
        if *count >= section_limit(section, limits) {
            increment_section_omitted_by_limit(section_pressure, section);
            details
                .stale_candidate_omissions
                .push(StaleCandidateOmission {
                    candidate: ranked.object.object_ref(),
                    vector_score: ranked.vector_candidate_score,
                    reason: StaleCandidateReason::SectionLimit,
                });
            details.section_assignments.push(SectionAssignment {
                object: ranked.object.object_ref(),
                section: ContextPackSection::Omitted,
                rank: None,
                reason: SectionAssignmentReason::OmittedByLimit {
                    intended_section: section,
                    scores: ranked.section_score_components(),
                },
                cue_kinds: ranked.cue_kinds.clone(),
            });
            continue;
        }

        *count += 1;
        increment_section_included(section_pressure, section);
        let rank = *count;
        details.section_assignments.push(SectionAssignment {
            object: ranked.object.object_ref(),
            section,
            rank: Some(rank),
            reason: SectionAssignmentReason::Selected {
                scores: ranked.section_score_components(),
            },
            cue_kinds: ranked.cue_kinds.clone(),
        });

        match ranked.object {
            MemoryObject::Episode(object) => pack.relevant_episodes.push(object),
            MemoryObject::Observation(object) => pack.salient_observations.push(object),
            MemoryObject::MemoryThread(object) => pack.active_threads.push(object),
            MemoryObject::DerivedMemory(object) => push_derived(&mut pack, object),
            MemoryObject::Entity(_) | MemoryObject::MemoryLink(_) => {}
        }
    }

    pack
}

fn included_section_assignment_count(section_assignments: &[SectionAssignment]) -> usize {
    section_assignments
        .iter()
        .filter(|assignment| assignment.section != ContextPackSection::Omitted)
        .count()
}

fn initial_section_pressure(
    limits: crate::api::types::ContinuitySectionLimits,
) -> Vec<SectionPressureSummary> {
    prompt_ready_sections()
        .into_iter()
        .map(|section| SectionPressureSummary {
            section,
            limit: section_limit(section, limits),
            included_count: 0,
            omitted_by_limit_count: 0,
        })
        .collect()
}

fn prompt_ready_sections() -> Vec<ContextPackSection> {
    vec![
        ContextPackSection::ActiveThreads,
        ContextPackSection::RelevantEpisodes,
        ContextPackSection::SalientObservations,
        ContextPackSection::DerivedMemories,
        ContextPackSection::Preferences,
        ContextPackSection::RelationshipNotes,
        ContextPackSection::OpenLoops,
        ContextPackSection::Commitments,
        ContextPackSection::CharacterSignals,
    ]
}

fn increment_section_included(
    section_pressure: &mut [SectionPressureSummary],
    section: ContextPackSection,
) {
    if let Some(summary) = section_pressure
        .iter_mut()
        .find(|summary| summary.section == section)
    {
        summary.included_count += 1;
    }
}

fn increment_section_omitted_by_limit(
    section_pressure: &mut [SectionPressureSummary],
    section: ContextPackSection,
) {
    if let Some(summary) = section_pressure
        .iter_mut()
        .find(|summary| summary.section == section)
    {
        summary.omitted_by_limit_count += 1;
    }
}

fn summarize_stale_candidate_omissions(
    omissions: &[StaleCandidateOmission],
) -> Vec<StaleCandidateOmissionSummary> {
    let mut summaries = Vec::<StaleCandidateOmissionSummary>::new();
    for omission in omissions {
        if let Some(summary) = summaries
            .iter_mut()
            .find(|summary| summary.reason == omission.reason)
        {
            summary.count += 1;
        } else {
            summaries.push(StaleCandidateOmissionSummary {
                reason: omission.reason,
                count: 1,
            });
        }
    }
    summaries.sort_by_key(|summary| stale_reason_rank(summary.reason));
    summaries
}

fn summarize_lifecycle_omissions(
    decisions: &[LifecycleFilterDecision],
) -> Vec<LifecycleOmissionSummary> {
    let mut summaries = Vec::<LifecycleOmissionSummary>::new();
    for decision in decisions
        .iter()
        .filter(|decision| decision.action == LifecycleFilterAction::Omitted)
    {
        if let Some(summary) = summaries
            .iter_mut()
            .find(|summary| summary.reason == decision.reason)
        {
            summary.count += 1;
        } else {
            summaries.push(LifecycleOmissionSummary {
                reason: decision.reason,
                count: 1,
            });
        }
    }
    summaries.sort_by_key(|summary| lifecycle_reason_rank(summary.reason));
    summaries
}

fn push_derived(pack: &mut ContinuityContextPack, object: DerivedMemory) {
    let section = object.derived_type;
    let included = IncludedDerivedMemory::from(object);
    match section {
        DerivedType::UserPreference | DerivedType::AssistantPreference => {
            pack.preferences.push(included)
        }
        DerivedType::RelationshipNote => pack.relationship_notes.push(included),
        DerivedType::OpenLoop => pack.open_loops.push(included),
        DerivedType::Commitment => pack.commitments.push(included),
        DerivedType::CharacterSignal => pack.character_signals.push(included),
        DerivedType::Reflection
        | DerivedType::ProjectNote
        | DerivedType::Claim
        | DerivedType::Correction => pack.derived_memories.push(included),
    }
}

#[derive(Debug, Default)]
struct SectionCounts {
    active_threads: usize,
    relevant_episodes: usize,
    salient_observations: usize,
    derived_memories: usize,
    preferences: usize,
    relationship_notes: usize,
    open_loops: usize,
    commitments: usize,
    character_signals: usize,
}

impl SectionCounts {
    fn count_mut(&mut self, section: ContextPackSection) -> &mut usize {
        match section {
            ContextPackSection::ActiveThreads => &mut self.active_threads,
            ContextPackSection::RelevantEpisodes => &mut self.relevant_episodes,
            ContextPackSection::SalientObservations => &mut self.salient_observations,
            ContextPackSection::DerivedMemories => &mut self.derived_memories,
            ContextPackSection::Preferences => &mut self.preferences,
            ContextPackSection::RelationshipNotes => &mut self.relationship_notes,
            ContextPackSection::OpenLoops => &mut self.open_loops,
            ContextPackSection::Commitments => &mut self.commitments,
            ContextPackSection::CharacterSignals => &mut self.character_signals,
            ContextPackSection::Omitted => unreachable!("omitted is not a pack section counter"),
        }
    }
}

#[derive(Debug, Clone)]
struct CandidateRoot {
    object_id: MemoryId,
    object_type: ObjectType,
    score: f32,
    source: GraphRootSource,
    vector_score: Option<f32>,
    cue_kinds: BTreeSet<CueKind>,
}

#[derive(Debug)]
struct CandidateRootSelection {
    roots: Vec<CandidateRoot>,
    unique_count: usize,
    omitted: Vec<CandidateRoot>,
}

fn select_candidate_roots(
    candidates: &[VectorCandidateMatch],
    kinds: &HashMap<MemoryObjectRef, BTreeSet<CueKind>>,
    explicit_roots: &[CandidateRoot],
    max_graph_roots: usize,
) -> CandidateRootSelection {
    let mut by_ref: HashMap<MemoryObjectRef, &VectorCandidateMatch> = HashMap::new();
    for candidate in candidates {
        let object_ref = MemoryObjectRef::from_id_type(candidate.object_id, candidate.object_type);
        by_ref
            .entry(object_ref)
            .and_modify(|existing| {
                if candidate.score.total_cmp(&existing.score).is_gt() {
                    *existing = candidate;
                }
            })
            .or_insert(candidate);
    }
    let mut content = by_ref.into_values().collect::<Vec<_>>();
    content.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| {
                left.object_type
                    .stable_rank()
                    .cmp(&right.object_type.stable_rank())
            })
            .then_with(|| left.object_id.cmp(&right.object_id))
    });
    let roots = explicit_roots
        .iter()
        .cloned()
        .chain(content.into_iter().map(|candidate| {
            CandidateRoot {
                object_id: candidate.object_id,
                object_type: candidate.object_type,
                score: candidate.score,
                source: GraphRootSource::Vector,
                vector_score: Some(candidate.score),
                cue_kinds: kinds
                    .get(&MemoryObjectRef::new(
                        candidate.object_type,
                        candidate.object_id,
                    ))
                    .cloned()
                    .unwrap_or_default(),
            }
        }))
        .collect::<Vec<_>>();
    let mut indices = HashMap::new();
    let mut merged: Vec<CandidateRoot> = Vec::new();
    for root in roots {
        let object = MemoryObjectRef::new(root.object_type, root.object_id);
        if let Some(index) = indices.get(&object).copied() {
            let existing: &mut CandidateRoot = &mut merged[index];
            existing.cue_kinds.extend(root.cue_kinds);
            if let Some(score) = root.vector_score {
                existing.vector_score = Some(
                    existing
                        .vector_score
                        .map_or(score, |previous| previous.max(score)),
                );
            }
        } else {
            indices.insert(object, merged.len());
            merged.push(root);
        }
    }
    let mut roots = merged;
    let unique_count = roots.len();
    let omitted = roots.split_off(max_graph_roots.min(roots.len()));
    CandidateRootSelection {
        roots,
        unique_count,
        omitted,
    }
}

fn graph_query_for_candidate(
    candidate: &CandidateRoot,
    context: &RetrievalContext,
    fanout_overrides: Vec<crate::ports::graph_authority::GraphExpansionFanoutOverride>,
) -> GraphExpansionQuery {
    GraphExpansionQuery::new(
        candidate.object_id,
        candidate.object_type,
        context.graph_limits.max_depth,
        context.graph_limits.max_nodes,
    )
    .with_allowed_object_types(context.graph_limits.allowed_object_types.clone())
    .with_allowed_relation_types(context.graph_limits.allowed_relation_types.clone())
    .with_fanout_overrides(fanout_overrides)
    .with_max_fanout_per_node(context.graph_limits.max_fanout_per_node)
    .with_max_hub_edges(context.graph_limits.max_hub_edges)
    .with_lifecycle_policy(GraphExpansionLifecyclePolicy {
        include_suppressed: context.lifecycle_policy.include_suppressed,
        include_superseded: context.lifecycle_policy.include_superseded,
    })
    .with_failure_policy(GraphExpansionFailurePolicy {
        timeout_ms: context.graph_limits.timeout_ms,
        mode: context.graph_limits.failure_mode,
    })
}

fn absorb_selectivity_telemetry(total: &mut SelectivityTelemetry, next: &SelectivityTelemetry) {
    total.decision_count += next.decision_count;
    total.high_selectivity_count += next.high_selectivity_count;
    total.low_selectivity_supported_count += next.low_selectivity_supported_count;
    total.low_selectivity_rejected_count += next.low_selectivity_rejected_count;
    total.fallback_count += next.fallback_count;
}

fn bounded_failure_error(failure: GraphExpansionBoundedFailure) -> CustomError {
    CustomError::GraphExpansionBounded(graph_expansion_bounded_failure_trace(failure))
}

fn record_expansion_telemetry(telemetry: &mut GraphExpansionTelemetry, expansion: &GraphExpansion) {
    telemetry.expanded_object_count += expansion.objects.len();
    telemetry.expanded_relation_count += expansion.relations.len();
    telemetry.filtered_node_count += expansion.filtered_nodes.len();
    if let Some(failure) = expansion.bounded_failure {
        telemetry.bounded_failure_count += 1;
        increment_bounded_failure_reason(&mut telemetry.bounded_failure_reasons, failure.reason);
    }
}

fn increment_bounded_failure_reason(
    summaries: &mut Vec<crate::api::types::GraphExpansionBoundedFailureSummary>,
    reason: GraphExpansionBoundedFailureReason,
) {
    let reason = public_bounded_failure_reason(reason);
    if let Some(summary) = summaries
        .iter_mut()
        .find(|summary| summary.reason == reason)
    {
        summary.count += 1;
    } else {
        summaries.push(crate::api::types::GraphExpansionBoundedFailureSummary { reason, count: 1 });
    }
}

fn graph_expansion_trace(
    candidate: &CandidateRoot,
    expansion: &GraphExpansion,
) -> GraphExpansionTrace {
    GraphExpansionTrace {
        source: candidate.source,
        root: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
        object_count: expansion.objects.len(),
        relation_count: expansion.relations.len(),
        filtered_node_count: expansion.filtered_nodes.len(),
        bounded_failure: expansion
            .bounded_failure
            .map(graph_expansion_bounded_failure_trace),
        outcome: if expansion.bounded_failure.is_some() {
            GraphExpansionOutcome::Bounded
        } else {
            GraphExpansionOutcome::Expanded
        },
    }
}

fn fanout_utilization_traces_for_expansion(
    expansion: &GraphExpansion,
) -> Vec<FanoutUtilizationTrace> {
    expansion
        .fanout_utilization
        .iter()
        .map(|entry| FanoutUtilizationTrace {
            root: MemoryObjectRef::new(entry.root.object_type, entry.root.id),
            relation: entry.relation,
            object_type: entry.object_type,
            configured_cap: entry.configured_cap,
            selected_cap: entry.selected_cap,
            retained_count: entry.retained_count,
            omitted_by_fanout_count: entry.omitted_by_fanout_count,
        })
        .collect()
}

fn missing_root_expansion_trace(candidate: &CandidateRoot) -> GraphExpansionTrace {
    GraphExpansionTrace {
        source: candidate.source,
        root: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
        object_count: 0,
        relation_count: 0,
        filtered_node_count: 0,
        bounded_failure: None,
        outcome: GraphExpansionOutcome::MissingRoot,
    }
}

fn public_bounded_failure_reason(
    reason: GraphExpansionBoundedFailureReason,
) -> GraphExpansionBoundedReason {
    match reason {
        GraphExpansionBoundedFailureReason::NodeLimit => GraphExpansionBoundedReason::NodeLimit,
        GraphExpansionBoundedFailureReason::Timeout => GraphExpansionBoundedReason::Timeout,
        GraphExpansionBoundedFailureReason::HubLimit => GraphExpansionBoundedReason::HubLimit,
    }
}

fn filtered_lifecycle_decision(
    object_ref: MemoryObjectRef,
    reason: GraphExpansionFilteredReason,
    superseded_by: &[MemoryId],
) -> LifecycleFilterDecision {
    LifecycleFilterDecision {
        object: MemoryObjectRef::new(object_ref.object_type, object_ref.id),
        retention_state: None,
        superseded_by: superseded_by.to_vec(),
        action: LifecycleFilterAction::Omitted,
        reason: match reason {
            GraphExpansionFilteredReason::Suppressed => LifecycleFilterReason::SuppressedOmitted,
            GraphExpansionFilteredReason::Superseded => LifecycleFilterReason::SupersededOmitted,
        },
    }
}

fn stale_reason_from_filtered(reason: GraphExpansionFilteredReason) -> StaleCandidateReason {
    match reason {
        GraphExpansionFilteredReason::Suppressed => StaleCandidateReason::LifecycleMismatch,
        GraphExpansionFilteredReason::Superseded => StaleCandidateReason::Superseded,
    }
}

fn section_for_object(object: &MemoryObject) -> Option<ContextPackSection> {
    match object {
        MemoryObject::Episode(_) => Some(ContextPackSection::RelevantEpisodes),
        MemoryObject::Observation(_) => Some(ContextPackSection::SalientObservations),
        MemoryObject::MemoryThread(thread) if thread.status == ThreadStatus::Active => {
            Some(ContextPackSection::ActiveThreads)
        }
        MemoryObject::MemoryThread(_) => None,
        MemoryObject::DerivedMemory(memory) => match memory.derived_type {
            DerivedType::UserPreference | DerivedType::AssistantPreference => {
                Some(ContextPackSection::Preferences)
            }
            DerivedType::RelationshipNote => Some(ContextPackSection::RelationshipNotes),
            DerivedType::OpenLoop => Some(ContextPackSection::OpenLoops),
            DerivedType::Commitment => Some(ContextPackSection::Commitments),
            DerivedType::CharacterSignal => Some(ContextPackSection::CharacterSignals),
            DerivedType::Reflection
            | DerivedType::ProjectNote
            | DerivedType::Claim
            | DerivedType::Correction => Some(ContextPackSection::DerivedMemories),
        },
        MemoryObject::Entity(_) | MemoryObject::MemoryLink(_) => None,
    }
}

fn section_omission_reason(object: &MemoryObject) -> SectionAssignmentReason {
    match object {
        MemoryObject::MemoryThread(thread) => SectionAssignmentReason::OmittedNonActiveThread {
            thread_status: thread.status,
        },
        MemoryObject::Entity(_) | MemoryObject::MemoryLink(_) => {
            SectionAssignmentReason::OmittedNoPromptSection {
                object_type: object.object_type(),
            }
        }
        MemoryObject::Episode(_)
        | MemoryObject::Observation(_)
        | MemoryObject::DerivedMemory(_) => {
            unreachable!("prompt-ready object must have a context-pack section")
        }
    }
}

fn section_limit(
    section: ContextPackSection,
    limits: crate::api::types::ContinuitySectionLimits,
) -> usize {
    match section {
        ContextPackSection::ActiveThreads => limits.active_threads,
        ContextPackSection::RelevantEpisodes => limits.relevant_episodes,
        ContextPackSection::SalientObservations => limits.salient_observations,
        ContextPackSection::DerivedMemories => limits.derived_memories,
        ContextPackSection::Preferences => limits.preferences,
        ContextPackSection::RelationshipNotes => limits.relationship_notes,
        ContextPackSection::OpenLoops => limits.open_loops,
        ContextPackSection::Commitments => limits.commitments,
        ContextPackSection::CharacterSignals => limits.character_signals,
        ContextPackSection::Omitted => 0,
    }
}

fn graph_component(proximity: u8) -> f32 {
    1.0 / (f32::from(proximity) + 1.0)
}

fn salience_component(object: &MemoryObject) -> f32 {
    match object {
        MemoryObject::Episode(object) => object.salience_score,
        MemoryObject::Observation(object) => object.salience_score,
        MemoryObject::MemoryThread(object) => object.salience_score,
        MemoryObject::DerivedMemory(object) => object.salience_score,
        MemoryObject::Entity(_) | MemoryObject::MemoryLink(_) => 0.0,
    }
}

fn lifecycle_reason_rank(reason: LifecycleFilterReason) -> u8 {
    match reason {
        LifecycleFilterReason::Active => 0,
        LifecycleFilterReason::SuppressedIncludedByPolicy => 2,
        LifecycleFilterReason::SupersededIncludedByPolicy => 5,
        LifecycleFilterReason::SuppressedOmitted => 7,
        LifecycleFilterReason::SupersededOmitted => 10,
        LifecycleFilterReason::GraphObjectMissing => 11,
        LifecycleFilterReason::GraphExpansionBounded => 12,
    }
}

fn stale_reason_rank(reason: StaleCandidateReason) -> u8 {
    match reason {
        StaleCandidateReason::GraphObjectMissing => 0,
        StaleCandidateReason::LifecycleMismatch => 1,
        StaleCandidateReason::Superseded => 3,
        StaleCandidateReason::SectionLimit => 4,
        StaleCandidateReason::GraphExpansionBounded => 5,
    }
}

fn rationale_summary(
    vector_candidate_count: usize,
    graph_verified_count: usize,
    stale_candidate_omission_count: usize,
    lifecycle_omission_count: usize,
) -> String {
    format!(
        "Evaluated {vector_candidate_count} vector candidates, included {graph_verified_count} final context-pack objects, omitted {stale_candidate_omission_count} stale or unresolved candidates, and recorded {lifecycle_omission_count} lifecycle omission decisions with deterministic cue, graph proximity, and salience scoring."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Mutex, MutexGuard};

    use async_trait::async_trait;
    use uuid::Uuid;

    use chrono::{DateTime, Utc};

    use crate::adapters::stats::InMemoryRetrievalStatsStore;
    use crate::api::types::retrieval::VectorRecallCompleteness;
    use crate::api::types::ContinuitySectionLimits;
    use crate::domain::RetentionState;
    use crate::models::vector::{CanonicalCandidates, VectorRecordEmbedding};
    use crate::policy::RetrievalSelectivityPolicy;
    use crate::ports::retrieval_stats::RetrievalStatsEdge;
    use crate::test_support::{
        high_fanout_graph_fixture, in_memory_graph_store, representative_fixtures,
        TemporaryVectorCandidateStore,
    };

    #[tokio::test]
    async fn section_rows_with_distinct_final_scores_are_descending() {
        let mut low = representative_fixtures().user_preference;
        low.id = MemoryId::from_u128(1);
        low.salience_score = 0.0;
        let mut middle = low.clone();
        middle.id = MemoryId::from_u128(2);
        let mut high = low.clone();
        high.id = MemoryId::from_u128(3);
        high.salience_score = 1.0;
        let graph = graph_with(
            &[
                MemoryObject::DerivedMemory(low.clone()),
                MemoryObject::DerivedMemory(middle.clone()),
                MemoryObject::DerivedMemory(high.clone()),
            ],
            &[],
        )
        .await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        for (memory, tilt) in [
            (low.clone(), 1.0),
            (middle.clone(), 0.0),
            (high.clone(), 0.4),
        ] {
            seed(&vector, MemoryObject::DerivedMemory(memory), tilt).await;
        }
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let outcome = RetrievePipeline::new(&graph, &vector, &embedder)
            .retrieve(RetrievalContext::new("rank by final score").with_trace())
            .await
            .unwrap();

        let ids = outcome
            .pack
            .preferences
            .iter()
            .map(|row| row.memory.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![high.id, middle.id, low.id]);
        let trace = outcome.trace.unwrap();
        let scores = ids
            .iter()
            .map(|id| {
                let assignment = trace
                    .section_assignments
                    .iter()
                    .find(|assignment| assignment.object.id == *id)
                    .unwrap();
                match assignment.reason {
                    SectionAssignmentReason::Selected { scores } => scores.final_score,
                    _ => panic!("pack row must have a selected section assignment"),
                }
            })
            .collect::<Vec<_>>();
        assert!(scores.windows(2).all(|pair| pair[0] > pair[1]));
    }

    #[test]
    fn graph_root_truncation_keeps_highest_scoring_roots() {
        let low = vector_candidate(MemoryId::from_u128(1), ObjectType::Episode, 0.1);
        let middle = vector_candidate(MemoryId::from_u128(2), ObjectType::Episode, 0.6);
        let high = vector_candidate(MemoryId::from_u128(3), ObjectType::Episode, 0.9);

        let selected = select_candidate_roots(
            &[middle.clone(), low, high.clone()],
            &HashMap::new(),
            &[],
            2,
        );

        assert_eq!(
            selected
                .roots
                .iter()
                .map(|root| (root.object_id, root.score))
                .collect::<Vec<_>>(),
            vec![
                (high.object_id, high.score),
                (middle.object_id, middle.score)
            ]
        );
    }

    #[tokio::test]
    async fn vector_to_graph_flow_groups_sections_and_records_trace() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::Episode(fixtures.episode.clone()),
            0.0,
        )
        .await;
        seed(
            &vector,
            MemoryObject::Observation(fixtures.salient_observation.clone()),
            0.1,
        )
        .await;
        seed(
            &vector,
            MemoryObject::MemoryThread(fixtures.soft_thread.clone()),
            0.2,
        )
        .await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            0.3,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("deterministic store contracts").with_trace())
            .await
            .unwrap();

        assert_eq!(embedder.inputs()[0].surface, VectorSurface::Query);
        assert_eq!(outcome.pack.relevant_episodes[0].id, fixtures.episode.id);
        assert_eq!(
            outcome.pack.salient_observations[0].id,
            fixtures.salient_observation.id
        );
        assert_eq!(outcome.pack.active_threads[0].id, fixtures.soft_thread.id);
        assert_eq!(
            outcome.pack.preferences[0].memory.id,
            fixtures.user_preference.id
        );
        assert_eq!(
            outcome.pack.preferences[0].source_episode_ids,
            fixtures.user_preference.derived_from_episode_ids
        );
        assert_eq!(
            outcome.pack.relevant_episodes[0].raw_ref,
            fixtures.episode.raw_ref
        );
        assert_eq!(outcome.rationale.vector_candidate_count, 4);
        assert_eq!(outcome.rationale.telemetry.query_embedding_dimension, 2);
        assert_eq!(
            outcome.rationale.telemetry.returned_vector_candidate_count,
            4
        );
        assert_eq!(
            outcome
                .rationale
                .telemetry
                .unique_graph_root_candidate_count,
            4
        );
        assert_eq!(outcome.rationale.telemetry.selected_graph_root_count, 4);
        assert_eq!(outcome.rationale.telemetry.graph_root_omission_count, 0);
        assert_eq!(
            outcome
                .rationale
                .telemetry
                .graph_expansion
                .attempted_root_count,
            4
        );
        let trace = outcome.trace.as_ref().unwrap();
        assert!(trace.section_assignments.iter().any(|assignment| {
            matches!(assignment.reason, SectionAssignmentReason::Selected { .. })
        }));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.episode.id
                && assignment.cue_kinds == BTreeSet::from([CueKind::Topic])
        }));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.user_preference.id
                && assignment.cue_kinds == BTreeSet::from([CueKind::Topic])
        }));
        assert_eq!(trace.vector_candidates.len(), 4);
        let seeded_surfaces = [
            MemoryObject::Episode(fixtures.episode.clone()),
            MemoryObject::Observation(fixtures.salient_observation.clone()),
            MemoryObject::MemoryThread(fixtures.soft_thread.clone()),
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
        ]
        .iter()
        .map(|object| {
            let record = crate::policy::memory_object_vector_record(object).unwrap();
            (record.object_id, record.surface)
        })
        .collect::<HashMap<_, _>>();
        assert!(trace.vector_candidates.iter().all(|candidate| {
            seeded_surfaces.get(&candidate.object.id) == Some(&candidate.surface)
        }));
        assert!(!trace.graph_relations.is_empty());
        assert!(trace.graph_relations.iter().all(|relation| fixtures
            .links()
            .iter()
            .any(|link| link.id == relation.link_id)));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.object_type == ObjectType::Entity
                && assignment.reason
                    == SectionAssignmentReason::OmittedNoPromptSection {
                        object_type: ObjectType::Entity,
                    }
        }));
        assert_eq!(trace.graph_expansions.len(), 4);
    }

    #[tokio::test]
    async fn section_scores_explain_direct_and_derived_rankings() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::Episode(fixtures.episode.clone()),
            0.0,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("score provenance").with_trace())
            .await
            .unwrap();
        let trace = outcome.trace.unwrap();
        let root = &trace.vector_candidates[0];
        assert!(trace.graph_expansions.iter().any(|expansion| {
            expansion.root == root.object
                && expansion.source == GraphRootSource::Vector
                && expansion.outcome == GraphExpansionOutcome::Expanded
        }));
        let components = trace
            .section_assignments
            .iter()
            .filter_map(|assignment| match assignment.reason {
                SectionAssignmentReason::Selected { scores }
                | SectionAssignmentReason::OmittedByLimit { scores, .. } => {
                    Some((assignment.object, scores))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(components
            .windows(2)
            .all(|pair| pair[0].1.final_score >= pair[1].1.final_score));
        let direct = components
            .iter()
            .find(|(object, _)| *object == root.object)
            .unwrap()
            .1;
        let weaker = components
            .iter()
            .map(|(_, scores)| scores)
            .find(|scores| {
                scores.cue_score < direct.cue_score
                    && scores.graph_score <= direct.graph_score
                    && scores.salience_score <= direct.salience_score
            })
            .expect("fixture includes a row with weaker score components");
        assert!(weaker.final_score < direct.final_score);
        assert_eq!(direct.cue_score, Some(root.score));
        let mut saw_expansion = false;
        for (object, scores) in components {
            if object != root.object {
                saw_expansion = true;
                assert!(trace
                    .graph_relations
                    .iter()
                    .any(|relation| relation.from == object || relation.to == object));
                assert_eq!(scores.cue_score, Some(root.score * 0.75));
            }
        }
        assert!(saw_expansion);
    }

    #[tokio::test]
    async fn trace_collection_does_not_change_retrieval_results() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
            0.0,
        )
        .await;
        seed(
            &vector,
            MemoryObject::Episode(fixtures.episode.clone()),
            0.1,
        )
        .await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            0.2,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let without_trace = pipeline
            .retrieve(RetrievalContext::new("trace parity"))
            .await
            .unwrap();
        let with_trace = pipeline
            .retrieve(RetrievalContext::new("trace parity").with_trace())
            .await
            .unwrap();

        assert_eq!(without_trace.pack, with_trace.pack);
        assert_eq!(without_trace.rationale, with_trace.rationale);
        assert!(without_trace.trace.is_none());
        assert!(with_trace.trace.is_some());
    }

    #[tokio::test]
    async fn entity_neutral_selectivity_rejects_low_selectivity_concept_entity_about_expansion() {
        let fixture = high_fanout_graph_fixture();
        let graph = graph_with(&fixture.objects(), &fixture.links).await;
        let stats = InMemoryRetrievalStatsStore::new();
        record_about_edges(
            &stats,
            fixture.hub_entity.id,
            &fixture
                .derived_memories
                .iter()
                .map(|memory| memory.id)
                .collect::<Vec<_>>(),
        )
        .await;

        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let plan = selectivity_plan_for_entity(
            fixture.hub_entity.id,
            1.0,
            16,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            crate::api::types::RetrievalLifecyclePolicy::default(),
            TraceMode::Enabled,
        )
        .await
        .unwrap();
        let expansion = graph
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 2, 96)
                    .with_fanout_overrides(plan.fanout_overrides)
                    .with_fanout_utilization_recording(TraceMode::Enabled),
            )
            .await
            .unwrap();
        assert!(plan
            .traces
            .iter()
            .any(|decision| decision.relation == RelationType::About
                && decision.object_type == ObjectType::DerivedMemory
                && decision.chosen_fanout == 0
                && decision.decision
                    == crate::api::types::SelectivityDecision::LowSelectivityRejected));
        assert!(!expansion
            .objects
            .iter()
            .any(|object| matches!(object, MemoryObject::DerivedMemory(_))));
        assert!(expansion
            .fanout_utilization
            .iter()
            .any(|entry| entry.root.id == fixture.hub_entity.id
                && entry.relation == RelationType::About
                && entry.object_type == ObjectType::DerivedMemory
                && entry.selected_cap == 0
                && entry.retained_count == 0
                && entry.omitted_by_fanout_count > 0));
    }

    #[tokio::test]
    async fn retrieval_telemetry_preserves_vector_recall_completeness() {
        let cases = [
            VectorRecallCompleteness::NotRequested,
            VectorRecallCompleteness::BoundaryTieOpen {
                fetched: 16,
                fetch_bound: 16,
            },
        ];

        for completeness in cases {
            let graph = in_memory_graph_store();
            let vector = VectorRecallOverride {
                inner: TemporaryVectorCandidateStore::open(2).await,
                completeness: Some(completeness),
                candidate: None,
            };
            let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
            let outcome = RetrievePipeline::new(&graph, &vector, &embedder)
                .retrieve(RetrievalContext::new("completeness telemetry"))
                .await
                .unwrap();

            assert_eq!(
                outcome.rationale.telemetry.vector_recall_completeness,
                completeness
            );
            assert_eq!(
                outcome.rationale.telemetry.returned_vector_candidate_count,
                0
            );
        }
    }

    #[tokio::test]
    async fn selectivity_allows_high_selectivity_entity_about_expansion() {
        let fixture = high_fanout_graph_fixture();
        let graph = graph_with(&fixture.objects(), &fixture.links).await;
        let stats = InMemoryRetrievalStatsStore::new();
        record_about_edges(
            &stats,
            fixture.hub_entity.id,
            &[fixture.derived_memories[0].id],
        )
        .await;
        record_other_about_edges(&stats, 80).await;
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let plan = selectivity_plan_for_entity(
            fixture.hub_entity.id,
            1.0,
            16,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            crate::api::types::RetrievalLifecyclePolicy::default(),
            TraceMode::Enabled,
        )
        .await
        .unwrap();
        let expansion = graph
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 2, 96)
                    .with_fanout_overrides(plan.fanout_overrides)
                    .with_fanout_utilization_recording(TraceMode::Enabled),
            )
            .await
            .unwrap();
        assert!(plan.telemetry.high_selectivity_count > 0);
        assert!(expansion
            .objects
            .iter()
            .any(|object| matches!(object, MemoryObject::DerivedMemory(_))));
        assert!(expansion
            .fanout_utilization
            .iter()
            .any(|entry| entry.root.id == fixture.hub_entity.id
                && entry.relation == RelationType::About
                && entry.object_type == ObjectType::DerivedMemory
                && entry.selected_cap <= entry.configured_cap
                && entry.retained_count > 0));
    }

    #[tokio::test]
    async fn omits_unresolved_and_lifecycle_stale_candidates() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let missing_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9999);
        let mut vector_only = fixtures.user_preference.clone();
        vector_only.id = missing_id;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(vector_only.clone()),
            0.0,
        )
        .await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
            0.1,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("omit stale").with_trace())
            .await
            .unwrap();
        let trace = outcome.trace.as_ref().unwrap();

        assert!(outcome.pack.preferences.is_empty());
        assert!(trace
            .stale_candidate_omissions
            .iter()
            .any(|omission| omission.candidate.id == missing_id
                && omission.reason == StaleCandidateReason::GraphObjectMissing));
        assert!(!trace
            .stale_candidate_omissions
            .iter()
            .any(
                |omission| omission.candidate.id == fixtures.suppressed_seed.id
                    && omission.reason == StaleCandidateReason::GraphObjectMissing
            ));
        assert!(trace
            .stale_candidate_omissions
            .iter()
            .any(
                |omission| omission.candidate.id == fixtures.suppressed_seed.id
                    && omission.reason == StaleCandidateReason::LifecycleMismatch
            ));
        assert!(trace
            .lifecycle_filter_decisions
            .iter()
            .any(|decision| decision.object.id == fixtures.suppressed_seed.id
                && decision.action == LifecycleFilterAction::Omitted));
    }

    #[tokio::test]
    async fn rationale_reports_compact_omissions_without_trace() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let missing_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9998);
        let mut vector_only = fixtures.user_preference.clone();
        vector_only.id = missing_id;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(vector_only.clone()),
            0.0,
        )
        .await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
            0.1,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("compact rationale omissions"))
            .await
            .unwrap();

        assert!(outcome.trace.is_none());
        assert_eq!(outcome.rationale.stale_candidate_omission_count, 2);
        assert!(outcome
            .rationale
            .stale_candidate_omission_reasons
            .iter()
            .any(
                |summary| summary.reason == StaleCandidateReason::GraphObjectMissing
                    && summary.count == 1
            ));
        assert!(outcome
            .rationale
            .stale_candidate_omission_reasons
            .iter()
            .any(
                |summary| summary.reason == StaleCandidateReason::LifecycleMismatch
                    && summary.count == 1
            ));
        assert_eq!(outcome.rationale.lifecycle_omission_count, 2);
        assert!(outcome
            .rationale
            .lifecycle_omission_reasons
            .iter()
            .any(
                |summary| summary.reason == LifecycleFilterReason::GraphObjectMissing
                    && summary.count == 1
            ));
        assert!(outcome
            .rationale
            .lifecycle_omission_reasons
            .iter()
            .any(
                |summary| summary.reason == LifecycleFilterReason::SuppressedOmitted
                    && summary.count == 1
            ));
    }

    #[tokio::test]
    async fn bounded_expansion_failure_errors_when_degraded_results_are_disabled() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            0.0,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("fail closed");
        context.graph_limits.timeout_ms = Some(0);
        context.graph_limits.failure_mode = GraphFailureMode::FailClosed;

        let error = pipeline.retrieve(context).await.unwrap_err();

        assert!(matches!(
            error,
            CustomError::GraphExpansionBounded(trace)
                if trace.reason == GraphExpansionBoundedReason::Timeout
                    && trace.at == Some(MemoryObjectRef::new(
                        ObjectType::DerivedMemory,
                        fixtures.user_preference.id,
                    ))
        ));
    }

    #[tokio::test]
    async fn bounded_empty_expansion_omits_without_reporting_graph_missing() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            0.0,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("bounded graph limits");
        context.graph_limits.max_nodes = 0;
        context.graph_limits.failure_mode = GraphFailureMode::AllowPartialResults;
        context.include_trace = true;

        let outcome = pipeline.retrieve(context).await.unwrap();
        let trace = outcome.trace.as_ref().unwrap();

        assert!(outcome.pack.preferences.is_empty());
        assert!(trace
            .stale_candidate_omissions
            .iter()
            .any(
                |omission| omission.candidate.id == fixtures.user_preference.id
                    && omission.reason == StaleCandidateReason::GraphExpansionBounded
            ));
        assert!(!trace
            .stale_candidate_omissions
            .iter()
            .any(
                |omission| omission.candidate.id == fixtures.user_preference.id
                    && omission.reason == StaleCandidateReason::GraphObjectMissing
            ));
        assert!(trace
            .lifecycle_filter_decisions
            .iter()
            .any(|decision| decision.object.id == fixtures.user_preference.id
                && decision.reason == LifecycleFilterReason::GraphExpansionBounded));
        assert_eq!(
            outcome
                .rationale
                .telemetry
                .graph_expansion
                .bounded_failure_count,
            1
        );
        assert_eq!(
            outcome
                .rationale
                .telemetry
                .graph_expansion
                .bounded_failure_reasons[0]
                .reason,
            GraphExpansionBoundedReason::NodeLimit
        );
        assert_eq!(
            trace.graph_expansions[0].outcome,
            GraphExpansionOutcome::Bounded
        );
    }

    #[tokio::test]
    async fn non_missing_graph_expansion_errors_are_propagated() {
        let object_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0050);
        let graph = ErrorGraphStore;
        let vector = VectorRecallOverride {
            inner: TemporaryVectorCandidateStore::open(2).await,
            completeness: None,
            candidate: Some(vector_candidate(object_id, ObjectType::MemoryLink, 0.99)),
        };
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("propagate graph errors");
        context.object_type_defaults.push(ObjectType::MemoryLink);

        let error = pipeline.retrieve(context).await.unwrap_err();

        assert!(matches!(
            error,
            CustomError::GraphQuery(crate::errors::GraphQueryError::Selection { .. })
        ));
    }

    #[tokio::test]
    async fn graph_authority_filters_lifecycle_state_after_vector_recall() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        let record = crate::policy::memory_object_vector_record(&MemoryObject::DerivedMemory(
            fixtures.user_preference.clone(),
        ))
        .unwrap();
        vector
            .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &[1.0, 0.0])])
            .await
            .unwrap();
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("preference"))
            .await
            .unwrap();

        assert_eq!(
            outcome.pack.preferences[0].memory.id,
            fixtures.user_preference.id
        );
        assert!(outcome.trace.is_none());
    }

    #[tokio::test]
    async fn telemetry_reports_graph_root_truncation_without_changing_defaults() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::Episode(fixtures.episode.clone()),
            0.0,
        )
        .await;
        seed(
            &vector,
            MemoryObject::Observation(fixtures.salient_observation.clone()),
            0.1,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("root truncation");
        context.candidate_limits.max_graph_roots = 1;

        let outcome = pipeline.retrieve(context).await.unwrap();
        let telemetry = &outcome.rationale.telemetry;

        assert_eq!(telemetry.returned_vector_candidate_count, 2);
        assert_eq!(telemetry.unique_graph_root_candidate_count, 2);
        assert_eq!(telemetry.selected_graph_root_count, 1);
        assert_eq!(telemetry.graph_root_omission_count, 1);
        assert_eq!(telemetry.graph_expansion.attempted_root_count, 1);
        assert!(outcome.trace.is_none());
    }

    #[tokio::test]
    async fn reranking_is_stable_and_section_limits_omit_overflow() {
        let fixtures = representative_fixtures();
        let mut second_preference = fixtures.user_preference.clone();
        second_preference.id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0036);
        second_preference.text = "Prefer small deterministic sections.".to_owned();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(second_preference.clone()));
        let graph = graph_with(&objects, &fixtures.links()).await;
        let first_vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &first_vector,
            MemoryObject::DerivedMemory(second_preference.clone()),
            0.0,
        )
        .await;
        seed(
            &first_vector,
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            0.0,
        )
        .await;
        let second_vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &second_vector,
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            0.0,
        )
        .await;
        seed(
            &second_vector,
            MemoryObject::DerivedMemory(second_preference.clone()),
            0.0,
        )
        .await;
        let first_embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let second_embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let first_pipeline = RetrievePipeline::new(&graph, &first_vector, &first_embedder);
        let second_pipeline = RetrievePipeline::new(&graph, &second_vector, &second_embedder);
        let mut context = RetrievalContext::new("preferences");
        context.section_limits = ContinuitySectionLimits {
            preferences: 1,
            ..ContinuitySectionLimits::default()
        };
        context.include_trace = true;

        let first = first_pipeline.retrieve(context.clone()).await.unwrap();
        let second = second_pipeline.retrieve(context).await.unwrap();
        let trace = first.trace.as_ref().unwrap();
        let omitted_vector_score = trace
            .vector_candidates
            .iter()
            .find(|candidate| candidate.object.id == second_preference.id)
            .unwrap()
            .score;

        assert_eq!(first.pack, second.pack);
        assert_eq!(first.trace, second.trace);
        assert_eq!(
            trace
                .vector_candidates
                .iter()
                .map(|candidate| candidate.object.id)
                .collect::<Vec<_>>(),
            vec![fixtures.user_preference.id, second_preference.id]
        );
        assert_eq!(
            first.pack.preferences[0].memory.id,
            fixtures.user_preference.id
        );
        assert!(trace
            .stale_candidate_omissions
            .iter()
            .any(|omission| omission.candidate.id == second_preference.id
                && omission.vector_score == Some(omitted_vector_score)
                && omission.reason == StaleCandidateReason::SectionLimit));
        assert!(trace
            .section_assignments
            .iter()
            .any(|assignment| assignment.object.id == second_preference.id
                && assignment.section == ContextPackSection::Omitted
                && matches!(
                    assignment.reason,
                    SectionAssignmentReason::OmittedByLimit {
                        intended_section: ContextPackSection::Preferences,
                        ..
                    }
                )
                && assignment.cue_kinds == BTreeSet::from([CueKind::Topic])));
        let preference_pressure = first
            .rationale
            .telemetry
            .section_pressure
            .iter()
            .find(|summary| summary.section == ContextPackSection::Preferences)
            .unwrap();
        assert_eq!(preference_pressure.limit, 1);
        assert_eq!(preference_pressure.included_count, 1);
        assert_eq!(preference_pressure.omitted_by_limit_count, 1);
    }

    #[tokio::test]
    async fn section_limit_omissions_only_report_actual_vector_candidate_scores() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
            0.0,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("section limits");
        context.section_limits = ContinuitySectionLimits {
            relevant_episodes: 0,
            ..ContinuitySectionLimits::default()
        };
        context.include_trace = true;

        let outcome = pipeline.retrieve(context).await.unwrap();
        let trace = outcome.trace.as_ref().unwrap();

        assert!(trace
            .stale_candidate_omissions
            .iter()
            .any(|omission| omission.candidate.id == fixtures.episode.id
                && omission.vector_score.is_none()
                && omission.reason == StaleCandidateReason::SectionLimit));
        assert!(trace
            .section_assignments
            .iter()
            .any(|assignment| assignment.object.id == fixtures.episode.id
                && assignment.section == ContextPackSection::Omitted
                && matches!(
                    assignment.reason,
                    SectionAssignmentReason::OmittedByLimit {
                        intended_section: ContextPackSection::RelevantEpisodes,
                        ..
                    }
                )));
    }

    #[tokio::test]
    async fn graph_verified_count_tracks_final_pack_inclusions() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
            0.0,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("pack counts").with_trace();
        context.section_limits = ContinuitySectionLimits {
            relevant_episodes: 0,
            ..ContinuitySectionLimits::default()
        };

        let outcome = pipeline.retrieve(context).await.unwrap();
        let trace = outcome.trace.as_ref().unwrap();
        let included_assignments = trace
            .section_assignments
            .iter()
            .filter(|assignment| assignment.section != ContextPackSection::Omitted)
            .count();

        assert_eq!(outcome.rationale.graph_verified_count, included_assignments);
        assert_eq!(outcome.rationale.graph_verified_count, 1);
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.episode.id
                && assignment.section == ContextPackSection::Omitted
        }));
    }

    #[tokio::test]
    async fn relationship_notes_are_assigned_to_their_section() {
        let fixtures = representative_fixtures();
        let mut relationship_note = fixtures.derived_reflection.clone();
        relationship_note.id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0040);
        relationship_note.derived_type = DerivedType::RelationshipNote;
        relationship_note.text = "User and assistant prefer calm direct collaboration.".to_owned();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(relationship_note.clone()));
        let graph = graph_with(&objects, &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(relationship_note.clone()),
            0.0,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("relationship note").with_trace())
            .await
            .unwrap();
        let trace = outcome.trace.as_ref().unwrap();

        assert_eq!(
            outcome.pack.relationship_notes[0].memory.id,
            relationship_note.id
        );
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == relationship_note.id
                && assignment.section == ContextPackSection::RelationshipNotes
        }));
    }

    #[tokio::test]
    async fn non_active_thread_omission_reason_names_thread_status() {
        let fixtures = representative_fixtures();
        let mut dormant_thread = fixtures.soft_thread.clone();
        dormant_thread.status = ThreadStatus::Dormant;
        let mut objects = fixtures.objects();
        objects.retain(|object| match object {
            MemoryObject::MemoryThread(thread) => thread.id != dormant_thread.id,
            _ => true,
        });
        objects.push(MemoryObject::MemoryThread(dormant_thread.clone()));
        let graph = graph_with(&objects, &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::MemoryThread(dormant_thread.clone()),
            0.0,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let context = RetrievalContext::new("dormant thread").with_trace();

        let outcome = pipeline.retrieve(context).await.unwrap();
        let trace = outcome.trace.as_ref().unwrap();

        assert!(outcome.pack.active_threads.is_empty());
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == dormant_thread.id
                && assignment.section == ContextPackSection::Omitted
                && assignment.reason
                    == SectionAssignmentReason::OmittedNonActiveThread {
                        thread_status: ThreadStatus::Dormant,
                    }
        }));
    }

    #[tokio::test]
    async fn retrieval_after_lifecycle_mutation_excludes_stale_records_by_default() {
        let graph = in_memory_graph_store();
        let fixtures = representative_fixtures();
        let mut superseded_memory = fixtures.user_preference.clone();
        let mut suppressed_memory = fixtures.suppressed_seed.clone();
        let mut replacement = fixtures.correction.clone();
        let mut dormant_thread = fixtures.soft_thread.clone();
        superseded_memory.retention_state = RetentionState::Active;
        suppressed_memory.retention_state = RetentionState::Suppressed;
        replacement.supersedes = vec![superseded_memory.id];
        dormant_thread.status = ThreadStatus::Dormant;
        let supersedes_link = crate::domain::MemoryLink {
            id: MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5600_0002),
            object_type: ObjectType::MemoryLink,
            from_id: replacement.id,
            from_type: ObjectType::DerivedMemory,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::Supersedes,
            rationale: Some("Replacement supersedes stale retrieval candidate.".to_owned()),
            created_at: replacement.created_at,
            schema_version: replacement.schema_version.clone(),
        };
        graph
            .upsert_objects(&[
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Observation(fixtures.salient_observation.clone()),
                MemoryObject::MemoryThread(dormant_thread.clone()),
                MemoryObject::DerivedMemory(superseded_memory.clone()),
                MemoryObject::DerivedMemory(suppressed_memory.clone()),
                MemoryObject::DerivedMemory(replacement.clone()),
            ])
            .await
            .unwrap();
        graph
            .upsert_links(&[fixtures.soft_thread_link.clone(), supersedes_link])
            .await
            .unwrap();

        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(replacement.clone()),
            0.0,
        )
        .await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(superseded_memory.clone()),
            0.1,
        )
        .await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(suppressed_memory.clone()),
            0.2,
        )
        .await;
        seed(
            &vector,
            MemoryObject::MemoryThread(dormant_thread.clone()),
            0.4,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut root_only = RetrievalContext::new("lifecycle graph truth").with_trace();
        root_only.graph_limits.max_depth = 0;
        let root_only = pipeline.retrieve(root_only).await.unwrap();
        let decision = root_only
            .trace
            .as_ref()
            .unwrap()
            .lifecycle_filter_decisions
            .iter()
            .find(|decision| decision.object.id == superseded_memory.id)
            .unwrap();
        assert_eq!(decision.reason, LifecycleFilterReason::SupersededOmitted);
        assert_eq!(decision.superseded_by, vec![replacement.id]);
        assert!(root_only
            .trace
            .as_ref()
            .unwrap()
            .stale_candidate_omissions
            .iter()
            .any(|omission| omission.candidate.id == superseded_memory.id
                && omission.reason == StaleCandidateReason::Superseded));
        let outcome = pipeline
            .retrieve(RetrievalContext::new("lifecycle graph truth").with_trace())
            .await
            .unwrap();
        let trace = outcome.trace.as_ref().unwrap();

        assert!(outcome
            .pack
            .derived_memories
            .iter()
            .any(|included| included.memory.id == replacement.id));
        assert!(!outcome
            .pack
            .preferences
            .iter()
            .any(|included| included.memory.id == superseded_memory.id));
        assert!(!outcome
            .pack
            .preferences
            .iter()
            .any(|included| included.memory.id == suppressed_memory.id));

        assert!(outcome.pack.active_threads.is_empty());
        assert!(trace.lifecycle_filter_decisions.iter().any(|decision| {
            decision.object.id == superseded_memory.id
                && decision.reason == LifecycleFilterReason::SupersededOmitted
        }));
        assert!(trace.lifecycle_filter_decisions.iter().any(|decision| {
            decision.object.id == suppressed_memory.id
                && decision.reason == LifecycleFilterReason::SuppressedOmitted
        }));
        let mut history = RetrievalContext::new("lifecycle graph truth");
        history.lifecycle_policy.include_superseded = true;
        let historical = pipeline.retrieve(history).await.unwrap();
        assert!(historical
            .pack
            .preferences
            .iter()
            .any(|included| included.memory.id == superseded_memory.id));
        assert!(!historical
            .pack
            .preferences
            .iter()
            .any(|included| included.memory.id == suppressed_memory.id));
        let mut suppressed = RetrievalContext::new("lifecycle graph truth");
        suppressed.lifecycle_policy.include_suppressed = true;
        let suppressed = pipeline.retrieve(suppressed).await.unwrap();
        assert!(suppressed
            .pack
            .preferences
            .iter()
            .any(|included| included.memory.id == suppressed_memory.id));
        assert!(!suppressed
            .pack
            .preferences
            .iter()
            .any(|included| included.memory.id == superseded_memory.id));
    }

    #[tokio::test]
    async fn retrieve_pipeline_expands_embedded_vector_candidate_with_embedded_oxigraph() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = TemporaryVectorCandidateStore::open(2).await;
        seed(
            &vector,
            MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
            0.0,
        )
        .await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("store contract continuity").with_trace())
            .await
            .unwrap();
        let repeated = pipeline
            .retrieve(RetrievalContext::new("store contract continuity").with_trace())
            .await
            .unwrap();
        let trace = outcome.trace.as_ref().unwrap();
        let repeated_trace = repeated.trace.as_ref().unwrap();
        let included_assignments = trace
            .section_assignments
            .iter()
            .filter(|assignment| assignment.section != ContextPackSection::Omitted)
            .count();

        assert_eq!(outcome.pack.relevant_episodes[0].id, fixtures.episode.id);
        assert_eq!(
            outcome.pack.derived_memories[0].memory.id,
            fixtures.derived_reflection.id
        );
        assert_eq!(outcome.rationale.vector_candidate_count, 1);
        assert_eq!(outcome.rationale.graph_verified_count, included_assignments);
        assert!(trace.graph_expansions.iter().any(|expansion| {
            expansion.root.id == fixtures.derived_reflection.id && expansion.object_count > 0
        }));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.episode.id
                && assignment.section == ContextPackSection::RelevantEpisodes
                && matches!(assignment.reason, SectionAssignmentReason::Selected { .. })
        }));
        assert_eq!(trace.vector_candidates.len(), 1);
        assert_eq!(
            trace.vector_candidates[0].object.id,
            fixtures.derived_reflection.id
        );
        assert_eq!(
            trace
                .section_assignments
                .iter()
                .map(|assignment| (assignment.object.id, assignment.section, assignment.rank))
                .collect::<Vec<_>>(),
            repeated_trace
                .section_assignments
                .iter()
                .map(|assignment| (assignment.object.id, assignment.section, assignment.rank))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            trace
                .graph_relations
                .iter()
                .map(|relation| (relation.from.id, relation.to.id, relation.relation))
                .collect::<Vec<_>>(),
            repeated_trace
                .graph_relations
                .iter()
                .map(|relation| (relation.from.id, relation.to.id, relation.relation))
                .collect::<Vec<_>>()
        );
        assert!(trace.graph_relations.iter().any(|relation| {
            relation.from.id == fixtures.hub_entity.id
                && relation.to.id == fixtures.episode.id
                && relation.relation == RelationType::Involves
        }));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.derived_reflection.id
                && assignment.section == ContextPackSection::DerivedMemories
        }));
    }

    #[tokio::test]
    async fn retrieve_pipeline_after_persistent_reopen_uses_graph_authority_filters() {
        let graph_dir = tempfile::TempDir::new().unwrap();
        let graph_path = graph_dir.path().join("graph");
        let fixtures = representative_fixtures();
        let missing_vector_only_id = MemoryId::new_v4();

        {
            let graph =
                crate::adapters::oxigraph::OxigraphGraphAuthorityStore::new_persistent(&graph_path)
                    .unwrap();
            graph.upsert_objects(&fixtures.objects()).await.unwrap();
            graph.upsert_links(&fixtures.links()).await.unwrap();
        }

        {
            let reopened =
                crate::adapters::oxigraph::OxigraphGraphAuthorityStore::new_persistent(&graph_path)
                    .unwrap();
            let vector = TemporaryVectorCandidateStore::open(2).await;
            seed(
                &vector,
                MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
                0.0,
            )
            .await;
            seed(
                &vector,
                MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
                0.1,
            )
            .await;
            let vector_only = crate::models::vector::VectorRecord::new(
                missing_vector_only_id,
                ObjectType::DerivedMemory,
                VectorSurface::Summary,
                crate::domain::DEFAULT_SCHEMA_VERSION,
                "Derived memory present in the vector index only",
            );
            vector
                .upsert_vector_records(&[VectorRecordEmbedding::new(&vector_only, &[1.0, 0.2])])
                .await
                .unwrap();
            let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
            let pipeline = RetrievePipeline::new(&reopened, &vector, &embedder);

            let outcome = pipeline
                .retrieve(RetrievalContext::new("restart graph authority").with_trace())
                .await
                .unwrap();
            let retrieved_ids = outcome
                .pack
                .derived_memories
                .iter()
                .chain(outcome.pack.preferences.iter())
                .map(|included| included.memory.id)
                .collect::<HashSet<_>>();

            assert!(retrieved_ids.contains(&fixtures.derived_reflection.id));
            assert!(!retrieved_ids.contains(&fixtures.suppressed_seed.id));
            assert!(!retrieved_ids.contains(&missing_vector_only_id));
            let trace = outcome.trace.as_ref().unwrap();
            assert!(trace.vector_candidates.iter().any(|candidate| {
                candidate.object.id == missing_vector_only_id
                    && candidate.object.object_type == ObjectType::DerivedMemory
            }));
            assert!(trace.lifecycle_filter_decisions.iter().any(|decision| {
                decision.object.id == fixtures.suppressed_seed.id
                    && decision.action == LifecycleFilterAction::Omitted
            }));
        }
    }

    /// Upserts the object's real vector record; `tilt` orders candidates by
    /// cosine distance from the `[1.0, 0.0]` query (0.0 ranks first).
    async fn seed(vector: &TemporaryVectorCandidateStore, object: MemoryObject, tilt: f32) {
        let record = crate::policy::memory_object_vector_record(&object).unwrap();
        vector
            .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &[1.0, tilt])])
            .await
            .unwrap();
    }

    async fn graph_with(
        objects: &[MemoryObject],
        links: &[crate::domain::MemoryLink],
    ) -> crate::adapters::oxigraph::OxigraphGraphAuthorityStore {
        let graph = in_memory_graph_store();
        graph.upsert_objects(objects).await.unwrap();
        graph.upsert_links(links).await.unwrap();
        graph
    }

    async fn record_about_edges(
        stats: &InMemoryRetrievalStatsStore,
        entity_id: MemoryId,
        derived_memory_ids: &[MemoryId],
    ) {
        let edges = derived_memory_ids
            .iter()
            .map(|object_id| stats_edge(entity_id, *object_id))
            .collect::<Vec<_>>();
        stats.record_edges(&edges).await.unwrap();
    }

    async fn record_other_about_edges(stats: &InMemoryRetrievalStatsStore, count: u128) {
        let edges = (0..count)
            .map(|offset| {
                stats_edge(
                    Uuid::from_u128(0x650e_8400_e29b_41d4_a716_4466_5544_0000 + offset),
                    Uuid::from_u128(0x750e_8400_e29b_41d4_a716_4466_5544_0000 + offset),
                )
            })
            .collect::<Vec<_>>();
        stats.record_edges(&edges).await.unwrap();
    }

    fn stats_edge(entity_id: MemoryId, object_id: MemoryId) -> RetrievalStatsEdge {
        let observed_at = timestamp();
        RetrievalStatsEdge {
            edge_key: format!("{}:about:derived_memory:{}", entity_id, object_id),
            entity_id,
            relation_kind: RelationType::About,
            object_id,
            object_type: ObjectType::DerivedMemory,
            retention_state: RetentionState::Active,
            is_current: true,
            first_seen_at: observed_at,
            last_seen_at: observed_at,
        }
    }

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-04-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn vector_candidate(
        object_id: MemoryId,
        object_type: ObjectType,
        score: f32,
    ) -> VectorCandidateMatch {
        VectorCandidateMatch::new(object_id, object_type, VectorSurface::Summary, score)
    }

    #[derive(Debug)]
    struct RecordingEmbedder {
        embedding: Vec<f32>,
        inputs: Arc<Mutex<Vec<EmbeddingInput>>>,
    }

    impl RecordingEmbedder {
        fn new(embedding: Vec<f32>) -> Self {
            Self {
                embedding,
                inputs: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn inputs(&self) -> Vec<EmbeddingInput> {
            lock(&self.inputs).unwrap().clone()
        }
    }

    #[async_trait]
    impl MemoryEmbedder for RecordingEmbedder {
        async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
            lock(&self.inputs)?.push(input.clone());
            Ok(self.embedding.clone())
        }

        async fn embed_batch(
            &self,
            inputs: &[EmbeddingInput],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            lock(&self.inputs)?.extend(inputs.iter().cloned());
            Ok(vec![self.embedding.clone(); inputs.len()])
        }
    }

    #[derive(Debug)]
    struct VectorRecallOverride {
        inner: TemporaryVectorCandidateStore,
        completeness: Option<VectorRecallCompleteness>,
        candidate: Option<VectorCandidateMatch>,
    }

    #[async_trait]
    impl VectorCandidateStore for VectorRecallOverride {
        async fn close(&self) -> Result<(), CustomError> {
            self.inner.close().await
        }

        async fn upsert_vector_records(
            &self,
            records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            self.inner.upsert_vector_records(records).await
        }

        async fn search_candidates(
            &self,
            query: &VectorCandidateSearch,
        ) -> Result<VectorCandidateRecall, CustomError> {
            let mut recall = self.inner.search_candidates(query).await?;
            if let Some(completeness) = self.completeness {
                recall.completeness = completeness;
            }
            if let Some(candidate) = &self.candidate {
                recall.candidates = CanonicalCandidates::new([candidate.clone()]);
            }
            Ok(recall)
        }

        async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
            self.inner.delete_candidates(object_ids).await
        }
    }

    #[derive(Debug)]
    struct ErrorGraphStore;

    #[async_trait]
    impl GraphAuthorityStore for ErrorGraphStore {
        async fn query_notions_known_as(
            &self,
            _name: &str,
        ) -> Result<Vec<crate::domain::MemoryId>, crate::errors::GraphQueryError> {
            unreachable!("this test never queries names")
        }

        async fn upsert_objects(&self, _objects: &[MemoryObject]) -> Result<(), CustomError> {
            Ok(())
        }

        async fn upsert_links(
            &self,
            _links: &[crate::domain::MemoryLink],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn upsert_objects_and_links(
            &self,
            _objects: &[MemoryObject],
            _links: &[crate::domain::MemoryLink],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn query_objects(
            &self,
            _query: &crate::ports::graph_authority::GraphObjectQuery,
        ) -> Result<Vec<MemoryObject>, crate::errors::GraphQueryError> {
            Ok(Vec::new())
        }

        async fn query_superseded_derived_memory_ids(
            &self,
            _memory_ids: &[crate::domain::MemoryId],
        ) -> Result<Vec<crate::domain::MemoryId>, crate::errors::GraphQueryError> {
            Ok(Vec::new())
        }

        async fn query_links_by_ids(
            &self,
            _link_ids: &[crate::domain::MemoryId],
        ) -> Result<Vec<crate::domain::MemoryLink>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_provenance(
            &self,
            _query: &crate::ports::graph_authority::GraphDerivedMemoryProvenanceQuery,
        ) -> Result<Vec<crate::domain::DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_thread(
            &self,
            _query: &crate::ports::graph_authority::GraphDerivedMemoryThreadQuery,
        ) -> Result<Vec<crate::domain::DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn expand_bounded(
            &self,
            _query: &GraphExpansionQuery,
        ) -> Result<GraphExpansion, CustomError> {
            Err(CustomError::GraphQuery(
                crate::errors::GraphQueryError::Selection {
                    detail: "injected graph selection failure".to_owned(),
                },
            ))
        }
    }

    fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, CustomError> {
        mutex
            .lock()
            .map_err(|error| CustomError::DatabaseError(format!("test lock poisoned: {error}")))
    }
}
