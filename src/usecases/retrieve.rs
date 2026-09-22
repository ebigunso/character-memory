mod activity;
mod scene;
mod state;

// Continuity retrieval pipeline used by the public facade and internal tests.
// Some helper APIs are intentionally retained for retrieval policy validation.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::api::types::{
    ContextPackSection, ContinuityContextPack, CueFloorAdmission, CueFloorStage, CueKind,
    FanoutUtilizationTrace, GraphExpansionOutcome, GraphExpansionTelemetry, GraphExpansionTrace,
    GraphRootSource, IncludedDerivedMemory, LifecycleFilterAction, LifecycleFilterDecision,
    LifecycleFilterReason, LifecycleOmissionSummary, RetrievalContext, RetrievalCueFloors,
    RetrievalRationale, RetrievalTelemetry, RetrievalTrace, RetrieveOutcome, SectionAssignment,
    SectionAssignmentReason, SectionPressureSummary, SectionScoreComponents, SelectivityTelemetry,
    StaleCandidateOmission, StaleCandidateOmissionSummary, StaleCandidateReason,
    VectorCandidateTrace,
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
                date_match_floor_eligible: false,
                object_id: *id,
                object_type: ObjectType::Entity,
                score: 1.0,
                source: GraphRootSource::Participant,
                vector_score: None,
                cue_kinds: BTreeSet::from([CueKind::Participant]),
                full_standing_score: Some(1.0),
                full_standing_kinds: BTreeSet::from([CueKind::Participant]),
            })
            .collect::<Vec<_>>();
        let mut assembly = RetrieveAssembly::new(trace_mode);
        let mut state_scopes = state::StateScopes::new();
        let mut root_order = HashMap::new();
        let keys = context.scene.scope_keys();
        let scope_kinds = std::iter::repeat_n(CueKind::Participant, cues.participants.len())
            .chain(std::iter::repeat_n(CueKind::Place, keys.len()))
            .chain([CueKind::Activity])
            .collect::<Vec<_>>();
        if context.graph_limits.allowed_object_types.is_empty()
            || context
                .graph_limits
                .allowed_object_types
                .contains(&ObjectType::DerivedMemory)
        {
            for (offset, key) in keys.iter().enumerate() {
                let (ids, filtered) = self
                    .graph_store
                    .query_scope_state(
                        key,
                        GraphExpansionLifecyclePolicy {
                            include_suppressed: context.lifecycle_policy.include_suppressed,
                            include_superseded: context.lifecycle_policy.include_superseded,
                        },
                        context.candidate_limits.max_graph_roots,
                    )
                    .await?;
                assembly
                    .lifecycle_decisions
                    .extend(filtered.into_iter().map(|entry| {
                        filtered_lifecycle_decision(
                            entry.object_ref,
                            entry.reason,
                            &entry.superseded_by,
                        )
                    }));
                for (rank, id) in ids.into_iter().enumerate() {
                    root_order.insert(
                        (
                            cues.participants.len() + offset,
                            MemoryObjectRef::new(ObjectType::DerivedMemory, id),
                        ),
                        rank,
                    );
                    state_scopes
                        .entry(MemoryObjectRef::new(ObjectType::DerivedMemory, id))
                        .or_default()
                        .push(cues.participants.len() + offset);
                    explicit_roots.push(CandidateRoot {
                        date_match_floor_eligible: false,
                        object_id: id,
                        object_type: ObjectType::DerivedMemory,
                        score: 1.0,
                        source: GraphRootSource::Place,
                        vector_score: None,
                        cue_kinds: BTreeSet::from([CueKind::Place]),
                        full_standing_score: Some(1.0),
                        full_standing_kinds: BTreeSet::from([CueKind::Place]),
                    });
                }
            }
        }
        let (activity, activity_roots, filtered) = self.activity_roots(&context).await?;
        assembly
            .lifecycle_decisions
            .extend(filtered.into_iter().map(|entry| {
                filtered_lifecycle_decision(entry.object_ref, entry.reason, &entry.superseded_by)
            }));
        for (rank, root) in activity_roots.iter().enumerate() {
            if root.object_type == ObjectType::DerivedMemory {
                root_order.insert(
                    (
                        cues.participants.len() + keys.len(),
                        MemoryObjectRef::new(root.object_type, root.object_id),
                    ),
                    rank,
                );
                state_scopes
                    .entry(MemoryObjectRef::new(root.object_type, root.object_id))
                    .or_default()
                    .push(cues.participants.len() + keys.len());
            }
        }
        explicit_roots.extend(activity_roots);
        let time_limit = prompt_ready_sections()
            .into_iter()
            .map(|section| section_limit(section, context.section_limits))
            .max()
            .unwrap_or(0);
        let time_range_has_more = if let Some(range) = context.time_range {
            let matches = self
                .graph_store
                .query_episodes_by_time(
                    Some(range.start),
                    range.end,
                    time_limit.saturating_add(1),
                    GraphExpansionLifecyclePolicy {
                        include_suppressed: context.lifecycle_policy.include_suppressed,
                        include_superseded: context.lifecycle_policy.include_superseded,
                    },
                )
                .await?;
            let has_more = matches.len() > time_limit;
            explicit_roots.extend(matches.into_iter().take(time_limit).map(|object_id| {
                CandidateRoot {
                    date_match_floor_eligible: true,
                    object_id,
                    object_type: ObjectType::Episode,
                    score: 0.0,
                    source: GraphRootSource::DateMatch,
                    vector_score: None,
                    cue_kinds: BTreeSet::from([CueKind::DateMatch]),
                    full_standing_score: None,
                    full_standing_kinds: BTreeSet::new(),
                }
            }));
            Some(has_more)
        } else {
            None
        };
        let resolved = cues
            .references
            .iter()
            .filter_map(|reference| match reference.resolution {
                crate::api::types::SceneReferenceResolution::Resolved { notion_id } => {
                    Some(notion_id)
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let anniversary_limit = context.cue_floors.date_match.max(1);
        let anniversaries = self
            .graph_store
            .query_anniversaries(
                context.scene.time.date_naive(),
                &resolved,
                anniversary_limit.saturating_add(1),
                GraphExpansionLifecyclePolicy {
                    include_suppressed: context.lifecycle_policy.include_suppressed,
                    include_superseded: context.lifecycle_policy.include_superseded,
                },
            )
            .await?;
        let anniversary_has_more = anniversaries.len() > anniversary_limit;
        explicit_roots.extend(anniversaries.into_iter().take(anniversary_limit).map(
            |(object_id, shared)| CandidateRoot {
                object_id,
                object_type: ObjectType::Episode,
                score: 0.0,
                source: GraphRootSource::DateMatch,
                vector_score: None,
                cue_kinds: BTreeSet::from([CueKind::DateMatch]),
                full_standing_score: None,
                full_standing_kinds: BTreeSet::new(),
                date_match_floor_eligible: shared,
            },
        ));
        let recent = self
            .graph_store
            .query_episodes_by_time(
                None,
                context.scene.time.to_utc(),
                time_limit,
                GraphExpansionLifecyclePolicy {
                    include_suppressed: context.lifecycle_policy.include_suppressed,
                    include_superseded: context.lifecycle_policy.include_superseded,
                },
            )
            .await?;
        explicit_roots.extend(recent.into_iter().map(|object_id| CandidateRoot {
            date_match_floor_eligible: false,
            object_id,
            object_type: ObjectType::Episode,
            score: 0.0,
            source: GraphRootSource::Recency,
            vector_score: None,
            cue_kinds: BTreeSet::from([CueKind::Recency]),
            full_standing_score: None,
            full_standing_kinds: BTreeSet::new(),
        }));
        let root_selection = select_candidate_roots(
            cues.roots,
            &explicit_roots,
            &cues.orders,
            context.candidate_limits.max_graph_roots,
            context.cue_floors,
            (&state_scopes, &root_order, &scope_kinds),
        );
        let candidate_roots = root_selection.roots;
        // Explicit roots and what they bring precede word matches at sections.
        let mut section_orders: BTreeMap<CueKind, Vec<MemoryObjectRef>> = BTreeMap::new();
        for root in &explicit_roots {
            for kind in &root.cue_kinds {
                section_orders
                    .entry(*kind)
                    .or_default()
                    .push(MemoryObjectRef::new(root.object_type, root.object_id));
            }
        }
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
                    candidate.source == GraphRootSource::Participant,
                )
                .await?
            } else {
                SelectivityPlan::default()
            };
            absorb_selectivity_telemetry(&mut selectivity_telemetry, &selectivity_plan.telemetry);
            if let Some(traces) = &mut selectivity_traces {
                traces.extend(selectivity_plan.traces);
            }
            let mut query =
                graph_query_for_candidate(candidate, &context, selectivity_plan.fanout_overrides)
                    .with_fanout_utilization_recording(trace_mode);
            if context.activity == Some(crate::api::types::ActivityRef::Thread(candidate.object_id))
                && candidate.object_type == ObjectType::MemoryThread
            {
                // Filter traversal itself: the bounded audit cannot list every resolved member.
                query.current_thread_state = true;
            }
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
                    if candidate.source == GraphRootSource::Participant {
                        let scope = cues
                            .participants
                            .iter()
                            .position(|id| *id == candidate.object_id)
                            .expect("participant roots follow resolved scene notions");
                        state::record_subject_state(
                            &mut state_scopes,
                            scope,
                            candidate.object_id,
                            &expansion,
                        );
                    }
                    for kind in explicit_roots
                        .iter()
                        .filter(|root| {
                            root.object_type == candidate.object_type
                                && root.object_id == candidate.object_id
                        })
                        .flat_map(|root| &root.cue_kinds)
                    {
                        section_orders
                            .entry(*kind)
                            .or_default()
                            .extend(&expansion.selection_order);
                    }
                    assembly.absorb_expansion(candidate, &query, expansion)?;
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
        for (kind, order) in root_selection.orders {
            section_orders.entry(kind).or_default().extend(order);
        }
        let mut details = RetrievalDetails {
            lifecycle_filter_decisions: assembly.lifecycle_decisions,
            stale_candidate_omissions: assembly.stale_omissions,
            section_assignments: Vec::new(),
            floor_admissions: cues.floor_admissions,
        };
        details
            .floor_admissions
            .extend(root_selection.floor_admissions);

        let mut section_pressure = initial_section_pressure(context.section_limits);
        let pack = build_pack(
            ranked_objects,
            (&state_scopes, &scope_kinds),
            &section_orders,
            context.section_limits,
            context.cue_floors,
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
            scene_cue_searches: cues.scene_cue_searches,
            time_range_has_more,
            anniversary_has_more,
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
            floor_admissions: details.floor_admissions,
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
            time_range: context.time_range,
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
    floor_admissions: Vec<CueFloorAdmission>,
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

    fn absorb_expansion(
        &mut self,
        candidate: &CandidateRoot,
        query: &GraphExpansionQuery,
        expansion: GraphExpansion,
    ) -> Result<(), CustomError> {
        let bounded_failure = expansion.bounded_failure;
        let candidate_ref =
            MemoryObjectRef::from_id_type(candidate.object_id, candidate.object_type);

        // Reuse the traversal rule on the admitted graph. This reads no store and
        // cannot widen the root's bounds; it identifies where the reminder road ends.
        let reminder_reached = if candidate.full_standing_score.is_some()
            && candidate.cue_kinds != candidate.full_standing_kinds
            && expansion
                .objects
                .iter()
                .any(|object| object.object_ref() == candidate_ref)
        {
            let mut reminder_query = query.clone();
            reminder_query.reminder_only = true;
            crate::policy::graph_expansion::bounded_expansion(
                &reminder_query,
                expansion.objects.clone(),
                expansion.links.clone(),
                &crate::policy::graph_expansion::ParticipantOccasions::new(),
            )?
            .objects
            .into_iter()
            .map(|object| object.object_ref())
            .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };

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
            let (score, kinds) = match candidate.full_standing_score {
                Some(score)
                    if object_ref != candidate_ref && !reminder_reached.contains(&object_ref) =>
                {
                    (score, &candidate.full_standing_kinds)
                }
                _ => (candidate.score, &candidate.cue_kinds),
            };
            let inherited_cue = if object_ref == candidate_ref {
                score
            } else {
                score * 0.75
            };
            let kinds = kinds.clone();
            let reminder_only = candidate.reminder_only();
            let candidate_score = candidate
                .vector_score
                .filter(|_| object_ref == candidate_ref);
            let ranked = self
                .objects
                .entry(object_ref)
                .and_modify(|ranked| {
                    ranked.cue_component = ranked.cue_component.max(inherited_cue);
                    ranked.cue_kinds.extend(&kinds);
                    ranked.date_match_floor_eligible |=
                        candidate.date_match_floor_eligible && kinds.contains(&CueKind::DateMatch);
                    ranked.is_root |= object_ref == candidate_ref;
                    if ranked.is_root || ranked.reminder_only == reminder_only {
                        ranked.graph_component = ranked.graph_component.max(graph_component);
                    } else if ranked.reminder_only {
                        ranked.graph_component = graph_component;
                    }
                    ranked.reminder_only &= reminder_only;
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
                    let mut ranked = RankedObject::new(
                        object,
                        inherited_cue,
                        kinds.clone(),
                        graph_component,
                        candidate_score,
                    );
                    ranked.reminder_only = reminder_only;
                    ranked.date_match_floor_eligible =
                        candidate.date_match_floor_eligible && kinds.contains(&CueKind::DateMatch);
                    ranked.is_root = object_ref == candidate_ref;
                    ranked
                });
            if object_ref.object_type == ObjectType::DerivedMemory {
                if let Some(resolvers) = expansion.resolved_by.get(&object_ref.id) {
                    ranked.resolved_by.extend(resolvers);
                    ranked.resolved_by.sort_unstable();
                    ranked.resolved_by.dedup();
                }
            }
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
        Ok(())
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
        // Reorder only contributed-occasion slots within an equal-score/type
        // tie: floor-eligible date matches precede recency and unshared anniversaries.
        // Other objects retain the general ID order and their positions.
        for tied in ranked_objects.chunk_by_mut(|left, right| {
            left.rank_key().score == right.rank_key().score
                && left.object.object_type() == right.object.object_type()
        }) {
            let positions = tied
                .iter()
                .enumerate()
                .filter_map(|(index, ranked)| {
                    (matches!(ranked.object, MemoryObject::Episode(_))
                        && (ranked.cue_kinds.contains(&CueKind::DateMatch)
                            || ranked.cue_kinds.contains(&CueKind::Recency)))
                    .then_some(index)
                })
                .collect::<Vec<_>>();
            if positions.len() < 2 {
                continue;
            }
            let mut recent = positions
                .iter()
                .map(|&index| tied[index].clone())
                .collect::<Vec<_>>();
            recent.sort_by_key(|ranked| match &ranked.object {
                MemoryObject::Episode(episode) => (
                    !ranked.date_match_floor_eligible,
                    std::cmp::Reverse(episode.scene.time),
                    episode.id,
                ),
                _ => unreachable!("only contributed episodes occupy these slots"),
            });
            for (position, ranked) in positions.into_iter().zip(recent) {
                tied[position] = ranked;
            }
        }
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
    reminder_only: bool,
    date_match_floor_eligible: bool,
    is_root: bool,
    vector_candidate_score: Option<f32>,
    graph_component: f32,
    salience_component: f32,
    resolved_by: Vec<MemoryId>,
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
            reminder_only: false,
            date_match_floor_eligible: false,
            is_root: false,
            vector_candidate_score,
            graph_component,
            salience_component,
            resolved_by: Vec::new(),
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

// Only a kind's own queue head credits its turn, even if already selected.
// Direct selector/search order precedes inherited-only members in stage order.
// Given-kind floors take full-standing members first. An explicit time floor
// reserves its latest occasions regardless of which other roads reached them.
// Only roots share spare turns; other stages fill from their ranked prefix.
// An index outside the old prefix records the floor that changed its admission.
fn floor_kinds(
    kinds: &BTreeSet<CueKind>,
    date_match_eligible: bool,
) -> std::borrow::Cow<'_, BTreeSet<CueKind>> {
    if !date_match_eligible && kinds.contains(&CueKind::DateMatch) {
        let mut reserved = kinds.clone();
        reserved.remove(&CueKind::DateMatch);
        std::borrow::Cow::Owned(reserved)
    } else {
        std::borrow::Cow::Borrowed(kinds)
    }
}

fn select_with_cue_floors<'a>(
    candidates: impl IntoIterator<Item = (MemoryObjectRef, &'a BTreeSet<CueKind>, bool)>,
    orders: &BTreeMap<CueKind, Vec<MemoryObjectRef>>,
    limit: usize,
    floors: RetrievalCueFloors,
    stage: CueFloorStage,
) -> Vec<(usize, Option<CueKind>)> {
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    let kinds = candidates
        .iter()
        .flat_map(|(_, kinds, _)| kinds.iter().copied())
        .collect::<BTreeSet<_>>();
    if candidates.len() <= limit || kinds.is_empty() {
        return (0..candidates.len().min(limit))
            .map(|index| (index, None))
            .collect();
    }
    let mut queues = [
        (CueKind::Participant, floors.participant),
        (CueKind::Place, floors.place),
        (CueKind::Activity, floors.activity),
        (CueKind::DateMatch, floors.date_match),
        (CueKind::Topic, floors.topic),
        (CueKind::Recency, floors.recency),
    ]
    .map(|(kind, floor)| {
        let mut ranks = HashMap::new();
        for (rank, object) in orders.get(&kind).into_iter().flatten().enumerate() {
            ranks.entry(*object).or_insert(rank);
        }
        let mut queue = candidates
            .iter()
            .enumerate()
            .filter_map(|(index, (_, kinds, _))| kinds.contains(&kind).then_some(index))
            .collect::<Vec<_>>();
        queue.sort_by_key(|&index| {
            (
                !matches!(kind, CueKind::Recency | CueKind::DateMatch) && candidates[index].2,
                ranks
                    .get(&candidates[index].0)
                    .copied()
                    .unwrap_or(usize::MAX),
            )
        });
        (kind, floor, queue.into_iter())
    });
    let mut selected = BTreeMap::new();
    'selection: for reserve_floors in [true, false] {
        if !reserve_floors
            && (!matches!(stage, CueFloorStage::GraphRoots)
                || kinds
                    .iter()
                    .filter(|&&kind| !matches!(kind, CueKind::Recency | CueKind::DateMatch))
                    .count()
                    <= 1)
        {
            break;
        }
        for round in 0..limit {
            for (kind, floor, queue) in &mut queues {
                if selected.len() == limit {
                    break 'selection;
                }
                if (reserve_floors && round >= *floor)
                    || (!reserve_floors && matches!(kind, CueKind::Recency | CueKind::DateMatch))
                {
                    continue;
                }
                let Some(index) = queue.next() else { continue };
                selected
                    .entry(index)
                    .or_insert((index >= limit).then_some(*kind));
            }
        }
    }
    for index in 0..candidates.len() {
        if selected.len() == limit {
            break;
        }
        selected.entry(index).or_insert(None);
    }
    selected.into_iter().collect()
}

fn build_pack(
    mut ranked_objects: Vec<RankedObject>,
    (state_scopes, scope_kinds): (&state::StateScopes, &[CueKind]),
    orders: &BTreeMap<CueKind, Vec<MemoryObjectRef>>,
    limits: crate::api::types::ContinuitySectionLimits,
    floors: RetrievalCueFloors,
    details: &mut RetrievalDetails,
    section_pressure: &mut [SectionPressureSummary],
) -> ContinuityContextPack {
    let mut pack = ContinuityContextPack::empty();
    let mut section_counts = SectionCounts::default();
    let mut selected = HashSet::new();
    for section in prompt_ready_sections() {
        state::order_section_state(&mut ranked_objects, section, state_scopes);
        let candidates = ranked_objects
            .iter()
            .filter(|ranked| section_for_object(ranked) == Some(section))
            .collect::<Vec<_>>();
        // Section state already has its scope/score order; root recency must not
        // become the section floor order. Reuse this prefix without re-sorting it.
        let mut orders = orders.clone();
        for kind in [CueKind::Participant, CueKind::Place, CueKind::Activity] {
            let state_order = candidates.iter().filter_map(|ranked| {
                let object = ranked.object.object_ref();
                state_scopes.get(&object).and_then(|scopes| {
                    scopes
                        .iter()
                        .any(|&scope| scope_kinds[scope] == kind)
                        .then_some(object)
                })
            });
            orders.entry(kind).or_default().splice(..0, state_order);
        }
        let reserved_kinds = candidates
            .iter()
            .map(|ranked| floor_kinds(&ranked.cue_kinds, ranked.date_match_floor_eligible))
            .collect::<Vec<_>>();
        for (index, cause) in select_with_cue_floors(
            candidates
                .iter()
                .zip(&reserved_kinds)
                .map(|(ranked, kinds)| {
                    (
                        ranked.object.object_ref(),
                        kinds.as_ref(),
                        ranked.reminder_only,
                    )
                }),
            &orders,
            section_limit(section, limits),
            floors,
            CueFloorStage::Section { section },
        ) {
            let object = candidates[index].object.object_ref();
            selected.insert(object);
            if let Some(cue_kind) = cause {
                details.floor_admissions.push(CueFloorAdmission {
                    object,
                    stage: CueFloorStage::Section { section },
                    cue_kind,
                });
            }
        }
    }

    // A state-route omission no longer describes a memory admitted by another route.
    details.lifecycle_filter_decisions.retain(|entry| {
        entry.reason != LifecycleFilterReason::ResolvedOmitted || !selected.contains(&entry.object)
    });

    for ranked in ranked_objects {
        let Some(section) = section_for_object(&ranked) else {
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
        if !selected.contains(&ranked.object.object_ref()) {
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
            MemoryObject::DerivedMemory(object) => {
                push_derived(&mut pack, section, object, ranked.resolved_by)
            }
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

fn push_derived(
    pack: &mut ContinuityContextPack,
    section: ContextPackSection,
    object: DerivedMemory,
    resolved_by: Vec<MemoryId>,
) {
    let mut included = IncludedDerivedMemory::from(object);
    included.resolved_by = resolved_by;
    match section {
        ContextPackSection::Preferences => pack.preferences.push(included),
        ContextPackSection::RelationshipNotes => pack.relationship_notes.push(included),
        ContextPackSection::OpenLoops => pack.open_loops.push(included),
        ContextPackSection::Commitments => pack.commitments.push(included),
        ContextPackSection::CharacterSignals => pack.character_signals.push(included),
        ContextPackSection::DerivedMemories => pack.derived_memories.push(included),
        _ => unreachable!("derived memories require a derived-memory pack section"),
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
    full_standing_score: Option<f32>,
    full_standing_kinds: BTreeSet<CueKind>,
    date_match_floor_eligible: bool,
}

impl CandidateRoot {
    fn from_vector(
        candidate: &VectorCandidateMatch,
        cue_kinds: BTreeSet<CueKind>,
        topic_score: Option<f32>,
    ) -> Self {
        Self {
            date_match_floor_eligible: false,
            object_id: candidate.object_id,
            object_type: candidate.object_type,
            score: candidate.score,
            source: GraphRootSource::Vector,
            vector_score: Some(candidate.score),
            full_standing_score: topic_score,
            full_standing_kinds: cue_kinds
                .iter()
                .filter(|&&kind| kind == CueKind::Topic)
                .copied()
                .collect(),
            cue_kinds,
        }
    }

    fn reminder_only(&self) -> bool {
        self.full_standing_score.is_none()
    }
}

#[derive(Debug)]
struct CandidateRootSelection {
    roots: Vec<CandidateRoot>,
    unique_count: usize,
    omitted: Vec<CandidateRoot>,
    floor_admissions: Vec<CueFloorAdmission>,
    orders: BTreeMap<CueKind, Vec<MemoryObjectRef>>,
}

fn select_candidate_roots(
    candidates: Vec<CandidateRoot>,
    explicit_roots: &[CandidateRoot],
    content_orders: &BTreeMap<CueKind, Vec<MemoryObjectRef>>,
    max_graph_roots: usize,
    floors: RetrievalCueFloors,
    (scopes, root_order, scope_kinds): (
        &state::StateScopes,
        &HashMap<(usize, MemoryObjectRef), usize>,
        &[CueKind],
    ),
) -> CandidateRootSelection {
    let mut orders: BTreeMap<CueKind, Vec<MemoryObjectRef>> = BTreeMap::new();
    for root in explicit_roots {
        for kind in &root.cue_kinds {
            orders
                .entry(*kind)
                .or_default()
                .push(MemoryObjectRef::new(root.object_type, root.object_id));
        }
    }
    let mut by_ref: HashMap<MemoryObjectRef, CandidateRoot> = HashMap::new();
    for candidate in candidates {
        let object_ref = MemoryObjectRef::from_id_type(candidate.object_id, candidate.object_type);
        match by_ref.entry(object_ref) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if candidate.score.total_cmp(&entry.get().score).is_gt() {
                    entry.insert(candidate);
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(candidate);
            }
        }
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
    let mut time_roots = explicit_roots
        .iter()
        .filter(|root| {
            matches!(
                root.source,
                GraphRootSource::Recency | GraphRootSource::DateMatch
            )
        })
        .collect::<Vec<_>>();
    // Only eligible date matches lead. Recency is the newest bounded prefix;
    // additional unshared anniversaries are older, and overlaps merge below.
    time_roots.sort_by_key(|root| {
        (
            !root.date_match_floor_eligible,
            root.source == GraphRootSource::DateMatch,
        )
    });
    let roots = explicit_roots
        .iter()
        .filter(|root| {
            !matches!(
                root.source,
                GraphRootSource::Recency | GraphRootSource::DateMatch
            )
        })
        .cloned()
        .chain(content)
        // Time sources use only their reservations and remaining root room.
        .chain(time_roots.into_iter().cloned())
        .collect::<Vec<_>>();
    let mut indices = HashMap::new();
    let mut merged: Vec<CandidateRoot> = Vec::new();
    for root in roots {
        let object = MemoryObjectRef::new(root.object_type, root.object_id);
        if let Some(index) = indices.get(&object).copied() {
            let existing: &mut CandidateRoot = &mut merged[index];
            existing.date_match_floor_eligible |= root.date_match_floor_eligible;
            existing.cue_kinds.extend(root.cue_kinds);
            existing.score = existing.score.max(root.score);
            existing
                .full_standing_kinds
                .extend(root.full_standing_kinds);
            if let Some(score) = root.full_standing_score {
                existing.full_standing_score = Some(
                    existing
                        .full_standing_score
                        .map_or(score, |previous| previous.max(score)),
                );
            }
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
    // A shared root may first appear in another scope. Keep each selector's own
    // order here; pack queues deliberately use only their final ranked indices.
    state::order_state(
        &mut merged,
        scopes,
        |root| Some(MemoryObjectRef::new(root.object_type, root.object_id)),
        |scope, root| {
            root_order[&(
                scope,
                MemoryObjectRef::new(root.object_type, root.object_id),
            )]
        },
    );
    let root_positions = merged
        .iter()
        .enumerate()
        .map(|(index, root)| {
            (
                MemoryObjectRef::new(root.object_type, root.object_id),
                index,
            )
        })
        .collect::<HashMap<_, _>>();
    for (kind, order) in &mut orders {
        if !matches!(kind, CueKind::Recency | CueKind::DateMatch) {
            order.sort_by_key(|object| root_positions[object]);
        }
        let mut seen = HashSet::new();
        order.retain(|object| seen.insert(*object));
        let own_scopes = state::scopes_for_kind(scopes, scope_kinds, *kind);
        state::order_state(
            order,
            &own_scopes,
            |object| Some(*object),
            |scope, object| root_order[&(scope, *object)],
        );
    }
    for (kind, order) in content_orders {
        orders.entry(*kind).or_default().extend(order);
    }
    let unique_count = merged.len();
    let reserved_kinds = merged
        .iter()
        .map(|root| floor_kinds(&root.cue_kinds, root.date_match_floor_eligible))
        .collect::<Vec<_>>();
    let selection = select_with_cue_floors(
        merged.iter().zip(&reserved_kinds).map(|(root, kinds)| {
            (
                MemoryObjectRef::new(root.object_type, root.object_id),
                kinds.as_ref(),
                root.reminder_only(),
            )
        }),
        &orders,
        max_graph_roots,
        floors,
        CueFloorStage::GraphRoots,
    );
    let mut selection = selection.into_iter().peekable();
    let mut roots = Vec::new();
    let mut omitted = Vec::new();
    let mut floor_admissions = Vec::new();
    for (index, root) in merged.into_iter().enumerate() {
        if selection.peek().is_some_and(|&(chosen, _)| chosen == index) {
            if let Some((_, Some(cue_kind))) = selection.next() {
                floor_admissions.push(CueFloorAdmission {
                    object: MemoryObjectRef::new(root.object_type, root.object_id),
                    stage: CueFloorStage::GraphRoots,
                    cue_kind,
                });
            }
            roots.push(root);
        } else {
            omitted.push(root);
        }
    }
    CandidateRootSelection {
        roots,
        unique_count,
        omitted,
        floor_admissions,
        orders,
    }
}

fn graph_query_for_candidate(
    candidate: &CandidateRoot,
    context: &RetrievalContext,
    fanout_overrides: Vec<crate::ports::graph_authority::GraphExpansionFanoutOverride>,
) -> GraphExpansionQuery {
    let mut query = GraphExpansionQuery::new(
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
    });
    query.current_subject_state = candidate.object_type == ObjectType::Entity
        && candidate.source == GraphRootSource::Participant;
    query.reminder_only = candidate.reminder_only();
    query.participant_reference_time = context.scene.time.to_utc();
    query
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
            GraphExpansionFilteredReason::Resolved => LifecycleFilterReason::ResolvedOmitted,
        },
    }
}

fn stale_reason_from_filtered(reason: GraphExpansionFilteredReason) -> StaleCandidateReason {
    match reason {
        GraphExpansionFilteredReason::Suppressed => StaleCandidateReason::LifecycleMismatch,
        GraphExpansionFilteredReason::Resolved => {
            unreachable!("resolution filters state neighbors, never recall roots")
        }
        GraphExpansionFilteredReason::Superseded => StaleCandidateReason::Superseded,
    }
}

fn section_for_object(ranked: &RankedObject) -> Option<ContextPackSection> {
    match &ranked.object {
        MemoryObject::Episode(_) => Some(ContextPackSection::RelevantEpisodes),
        MemoryObject::Observation(_) => Some(ContextPackSection::SalientObservations),
        MemoryObject::MemoryThread(thread) if thread.status == ThreadStatus::Active => {
            Some(ContextPackSection::ActiveThreads)
        }
        MemoryObject::MemoryThread(_) => None,
        MemoryObject::DerivedMemory(memory) => match memory.derived_type {
            DerivedType::OpenLoop | DerivedType::Commitment if !ranked.resolved_by.is_empty() => {
                Some(ContextPackSection::DerivedMemories)
            }
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
        LifecycleFilterReason::ResolvedOmitted => 13,
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
    use crate::domain::ScopeKey;
    use crate::ports::graph_authority::GraphExpansionFilteredNode;

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

    #[test]
    fn resolution_metadata_stays_on_derived_memory_when_ids_collide() {
        let fixtures = representative_fixtures();
        let entity = fixtures.hub_entity;
        let mut memory = fixtures.open_loop;
        memory.id = entity.id;
        let resolver = fixtures.correction.id;
        let candidate = CandidateRoot {
            object_id: entity.id,
            object_type: ObjectType::Entity,
            score: 1.0,
            source: GraphRootSource::Participant,
            vector_score: None,
            cue_kinds: BTreeSet::from([CueKind::Participant]),
            full_standing_score: Some(1.0),
            full_standing_kinds: BTreeSet::from([CueKind::Participant]),
            date_match_floor_eligible: false,
        };
        let query = GraphExpansionQuery::new(entity.id, ObjectType::Entity, 1, 2);
        let mut expansion = GraphExpansion::new(
            vec![
                MemoryObject::Entity(entity.clone()),
                MemoryObject::DerivedMemory(memory),
            ],
            Vec::new(),
        );
        expansion.resolved_by.insert(entity.id, vec![resolver]);
        let mut assembly = RetrieveAssembly::new(TraceMode::Disabled);
        assembly
            .absorb_expansion(&candidate, &query, expansion)
            .unwrap();
        assert!(
            assembly.objects[&MemoryObjectRef::new(ObjectType::Entity, entity.id)]
                .resolved_by
                .is_empty()
        );
        assert_eq!(
            assembly.objects[&MemoryObjectRef::new(ObjectType::DerivedMemory, entity.id)]
                .resolved_by,
            vec![resolver]
        );
    }

    #[test]
    fn single_kind_reserves_own_head_then_fills_final_ranked_room() {
        let objects =
            [1, 2, 3].map(|id| MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(id)));
        let kinds = BTreeSet::from([CueKind::Participant]);
        let orders = BTreeMap::from([(
            CueKind::Participant,
            vec![objects[2], objects[1], objects[0]],
        )]);
        let selected = select_with_cue_floors(
            objects.into_iter().map(|object| (object, &kinds, false)),
            &orders,
            2,
            RetrievalCueFloors {
                participant: 1,
                ..Default::default()
            },
            CueFloorStage::GraphRoots,
        );
        assert_eq!(selected, vec![(0, None), (2, Some(CueKind::Participant))]);
    }

    #[test]
    fn recent_ties_keep_uncontributed_positions_and_general_id_order() {
        let fixtures = representative_fixtures();
        let mut assembly = RetrieveAssembly::new(TraceMode::Disabled);
        for n in 1..=3 {
            let mut episode = fixtures.episode.clone();
            episode.id = MemoryId::from_u128(n);
            episode.salience_score = 0.0;
            episode.scene.time += chrono::Duration::days(n as i64);
            let kinds = BTreeSet::from([if n == 2 {
                CueKind::Topic
            } else {
                CueKind::Recency
            }]);
            assembly.objects.insert(
                MemoryObjectRef::new(ObjectType::Episode, episode.id),
                RankedObject::new(MemoryObject::Episode(episode), 0.0, kinds, 1.0, None),
            );
        }
        assert_eq!(
            assembly
                .ranked_objects()
                .iter()
                .map(|ranked| ranked.object.id().as_u128())
                .collect::<Vec<_>>(),
            [3, 2, 1]
        );
    }

    #[test]
    fn date_ties_keep_other_positions_and_distinct_scores() {
        let fixtures = representative_fixtures();
        let mut assembly = RetrieveAssembly::new(TraceMode::Disabled);
        for n in 1..=6 {
            let mut episode = fixtures.episode.clone();
            episode.id = MemoryId::from_u128(n);
            episode.salience_score = 0.0;
            episode.scene.time += chrono::Duration::days(n as i64);
            let kind = match n {
                1 | 6 => CueKind::Recency,
                3 | 5 => CueKind::DateMatch,
                _ => CueKind::Topic,
            };
            let mut ranked = RankedObject::new(
                MemoryObject::Episode(episode),
                if n == 6 { 1.0 } else { 0.0 },
                BTreeSet::from([kind]),
                1.0,
                None,
            );
            ranked.date_match_floor_eligible = kind == CueKind::DateMatch;
            assembly.objects.insert(ranked.object.object_ref(), ranked);
        }
        assert_eq!(
            assembly
                .ranked_objects()
                .iter()
                .map(|ranked| ranked.object.id().as_u128())
                .collect::<Vec<_>>(),
            [6, 5, 2, 3, 4, 1]
        );
    }

    #[test]
    fn expanded_root_keeps_best_proximity_in_either_expansion_order() {
        let fixtures = representative_fixtures();
        let mut episode = fixtures.episode.clone();
        episode.salience_score = 0.0;
        let person = fixtures.hub_entity.clone();
        let link = crate::api::types::MemoryLinkDraft::new(
            ObjectType::Entity,
            person.id,
            RelationType::Involves,
            ObjectType::Episode,
            episode.id,
        )
        .into_domain()
        .unwrap();
        let full = CandidateRoot {
            date_match_floor_eligible: false,
            object_id: person.id,
            object_type: ObjectType::Entity,
            score: 1.0,
            source: GraphRootSource::Participant,
            vector_score: None,
            cue_kinds: BTreeSet::from([CueKind::Participant]),
            full_standing_score: Some(1.0),
            full_standing_kinds: BTreeSet::from([CueKind::Participant]),
        };
        let recent = CandidateRoot {
            date_match_floor_eligible: false,
            object_id: episode.id,
            object_type: ObjectType::Episode,
            score: 0.0,
            source: GraphRootSource::Recency,
            vector_score: None,
            cue_kinds: BTreeSet::from([CueKind::Recency]),
            full_standing_score: None,
            full_standing_kinds: BTreeSet::new(),
        };
        for roots in [[&full, &recent], [&recent, &full]] {
            let mut assembly = RetrieveAssembly::new(TraceMode::Disabled);
            for root in roots {
                let query = graph_query_for_candidate(
                    root,
                    &RetrievalContext::default().with_scene(episode.scene.clone()),
                    Vec::new(),
                );
                let expansion = crate::policy::graph_expansion::bounded_expansion(
                    &query,
                    [
                        MemoryObject::Episode(episode.clone()),
                        MemoryObject::Entity(person.clone()),
                    ],
                    [link.clone()],
                    &crate::policy::graph_expansion::ParticipantOccasions::new(),
                )
                .unwrap();
                assembly.absorb_expansion(root, &query, expansion).unwrap();
            }
            let ranked = &assembly.objects[&MemoryObjectRef::new(ObjectType::Episode, episode.id)];
            assert_eq!(ranked.graph_component, 1.0);
            assert_eq!(ranked.final_score(), 0.737_499_95);
            assert!(!ranked.reminder_only);
            assert_eq!(
                ranked.cue_kinds,
                BTreeSet::from([CueKind::Participant, CueKind::Recency])
            );
        }
    }

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
            [middle.clone(), low, high.clone()]
                .iter()
                .map(|candidate| CandidateRoot::from_vector(candidate, BTreeSet::new(), None))
                .collect(),
            &[],
            &BTreeMap::new(),
            2,
            RetrievalCueFloors::default(),
            (&state::StateScopes::new(), &HashMap::new(), &[]),
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
                && assignment.cue_kinds == BTreeSet::from([CueKind::Topic, CueKind::Recency])
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
            false,
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
            false,
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
            2
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
            source_observation: None,
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

        async fn delete_candidates(&self, objects: &[MemoryObjectRef]) -> Result<(), CustomError> {
            self.inner.delete_candidates(objects).await
        }
    }

    #[derive(Debug)]
    struct ErrorGraphStore;

    #[async_trait]
    impl GraphAuthorityStore for ErrorGraphStore {
        async fn query_anniversaries(
            &self,
            date: chrono::NaiveDate,
            participants: &[crate::domain::MemoryId],
            limit: usize,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Vec<(crate::domain::MemoryId, bool)>, CustomError> {
            let _ = (date, participants, limit, policy);
            Ok(Vec::new())
        }

        async fn query_episodes_by_time(
            &self,
            start: Option<chrono::DateTime<chrono::Utc>>,
            end: chrono::DateTime<chrono::Utc>,
            limit: usize,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Vec<crate::domain::MemoryId>, CustomError> {
            let _ = (start, end, limit, policy);
            Ok(Vec::new())
        }

        async fn query_episode_occasions(
            &self,
            episodes: &[crate::domain::MemoryObjectRef],
        ) -> Result<crate::policy::graph_expansion::ParticipantOccasions, CustomError> {
            let _ = episodes;
            unreachable!("this fixture never queries episode occasions")
        }

        async fn query_last_interaction(
            &self,
            participant: MemoryId,
            reference_time: chrono::DateTime<chrono::Utc>,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Option<(MemoryId, chrono::DateTime<chrono::Utc>)>, CustomError> {
            let _ = (participant, reference_time, policy);
            unreachable!("this fixture never queries participant interactions")
        }

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
        ) -> Result<
            (
                Vec<crate::domain::DerivedMemory>,
                Vec<crate::ports::graph_authority::GraphExpansionFilteredNode>,
            ),
            CustomError,
        > {
            Ok((Vec::new(), Vec::new()))
        }

        async fn query_scope_state(
            &self,
            key: &ScopeKey,
            policy: GraphExpansionLifecyclePolicy,
            limit: usize,
        ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
            let _ = (key, policy, limit);
            unreachable!("scope selector is not used by this failure fixture")
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
