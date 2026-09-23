mod activity;
mod scene;
mod state;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::api::types::{
    AdmissionRoad, ContextPackSection, ContinuityContextPack, CueFloorAdmission, CueFloorStage,
    CueKind, FanoutUtilizationTrace, GraphExpansionOutcome, GraphExpansionTelemetry,
    GraphExpansionTrace, GraphRootSource, IncludedDerivedMemory, LifecycleFilterDecision,
    LifecycleFilterReason, LifecycleOmissionSummary, RetrievalContext, RetrievalCueFloors,
    RetrievalRationale, RetrievalTelemetry, RetrievalTrace, RetrieveOutcome, SectionAssignment,
    SectionAssignmentReason, SectionPressureSummary, SectionScoreComponents, SelectivityTelemetry,
    StaleCandidateOmission, StaleCandidateOmissionSummary, StaleCandidateReason,
    VectorCandidateTrace,
};
use crate::domain::{
    DerivedMemory, DerivedType, GraphExpansionBoundedReason, MemoryId, MemoryObject,
    MemoryObjectRef, ObjectType, RelationType, ThreadStatus, VectorSurface,
};
use crate::errors::CustomError;
use crate::models::vector::{EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch};
use crate::policy::graph_expansion::{fail_if_closed, graph_expansion_bounded_failure_trace};
use crate::policy::{
    selectivity_plan_for_entity, RetrievalSelectivityPolicy, SelectivityPlan,
    SelectivityStatsContext,
};
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphExpansion, GraphExpansionBoundedFailureReason,
    GraphExpansionFailurePolicy, GraphExpansionFilteredReason, GraphExpansionLifecyclePolicy,
    GraphExpansionQuery, TraceMode,
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
        context.scene.validate_time()?;
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
            .enumerate()
            .map(|(position, id)| {
                CandidateRoot::new(
                    MemoryObjectRef::new(ObjectType::Entity, *id),
                    RecallRoad::Participant,
                    1.0,
                    position,
                )
            })
            .collect::<Vec<_>>();
        let mut assembly = RetrieveAssembly::new(trace_mode);
        let mut state_scopes = state::StateScopes::new();
        let mut root_order = HashMap::new();
        let keys = context.scene.scope_keys();
        let scope_kinds = std::iter::repeat_n(CueKind::Participant, cues.participants.len())
            .chain([CueKind::Activity])
            .collect::<Vec<_>>();
        if context.graph_limits.allowed_object_types.is_empty()
            || context
                .graph_limits
                .allowed_object_types
                .contains(&ObjectType::DerivedMemory)
        {
            let contribution = RecallRoad::Place.contribution(&context);
            let mut place_rows = HashMap::new();
            for key in &keys {
                let (ids, filtered) = self
                    .graph_store
                    .query_scope_state(
                        key,
                        GraphExpansionLifecyclePolicy::from(context.lifecycle_policy),
                        contribution,
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
                place_rows.extend(ids.into_iter().map(|row| (row.id, row)));
            }
            let mut place_rows = place_rows.into_values().collect::<Vec<_>>();
            place_rows.sort_unstable_by_key(|row| (std::cmp::Reverse(row.time), row.id));
            place_rows.truncate(contribution);
            for (rank, row) in place_rows.into_iter().enumerate() {
                explicit_roots.push(CandidateRoot::from_rank(
                    row,
                    ObjectType::DerivedMemory,
                    RecallRoad::Place,
                    0.0,
                    rank,
                ));
            }
        }
        let (activity, activity_roots, filtered) = self.activity_roots(&context).await?;
        assembly
            .lifecycle_decisions
            .extend(filtered.into_iter().map(|entry| {
                filtered_lifecycle_decision(entry.object_ref, entry.reason, &entry.superseded_by)
            }));
        for (rank, root) in activity_roots.iter().enumerate() {
            if root.object.object_type == ObjectType::DerivedMemory {
                root_order.insert((cues.participants.len(), root.object), rank);
                state_scopes
                    .entry(root.object)
                    .or_default()
                    .push(cues.participants.len());
            }
        }
        explicit_roots.extend(activity_roots);
        let time_road = if context.time_range.is_some() {
            RecallRoad::Range
        } else {
            RecallRoad::Recency
        };
        let contribution = time_road.contribution(&context);
        // Only a caller range needs an extra row to report that more exists.
        let read_limit = contribution.saturating_add(usize::from(context.time_range.is_some()));
        let window = self
            .graph_store
            .query_episodes_by_time(
                context.time_range.map(|range| range.start),
                context
                    .time_range
                    .map_or(context.scene.time.to_utc(), |range| range.end),
                read_limit,
                GraphExpansionLifecyclePolicy::from(context.lifecycle_policy),
            )
            .await?;
        let time_range_has_more = context.time_range.map(|_| window.len() > contribution);
        explicit_roots.extend(window.into_iter().take(contribution).enumerate().map(
            |(position, row)| {
                CandidateRoot::from_rank(row, ObjectType::Episode, time_road, 0.0, position)
            },
        ));
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
        let anniversary_roads = [
            RecallRoad::SharedAnniversary,
            RecallRoad::UnsharedAnniversary,
        ];
        let anniversaries = self
            .graph_store
            .query_anniversaries(
                context.scene.time.date_naive(),
                &resolved,
                anniversary_roads
                    .iter()
                    .map(|road| road.contribution(&context))
                    .max()
                    .unwrap_or(0),
                GraphExpansionLifecyclePolicy::from(context.lifecycle_policy),
            )
            .await?;
        for road in anniversary_roads {
            explicit_roots.extend(
                anniversaries
                    .iter()
                    .copied()
                    .filter(|(_, shared)| *shared == (road == RecallRoad::SharedAnniversary))
                    .take(road.contribution(&context))
                    .enumerate()
                    .map(|(position, (row, _))| {
                        CandidateRoot::from_rank(row, ObjectType::Episode, road, 0.0, position)
                    }),
            );
        }
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
        let mut section_orders: BTreeMap<RecallRoad, Vec<MemoryObjectRef>> = BTreeMap::new();
        for root in &explicit_roots {
            for kind in root.roads.keys() {
                section_orders.entry(*kind).or_default().push(root.object);
            }
        }
        let mut graph_expansion_telemetry = GraphExpansionTelemetry::default();
        let mut selectivity_telemetry = SelectivityTelemetry::default();
        let mut graph_expansion_traces = trace_mode.is_enabled().then(Vec::new);
        let mut fanout_utilization_traces = trace_mode.is_enabled().then(Vec::new);
        let mut selectivity_traces = trace_mode.is_enabled().then(Vec::new);
        let selectivity_stats_context = if candidate_roots
            .iter()
            .any(|candidate| candidate.object.object_type == ObjectType::Entity)
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
                .filter(|_| candidate.object.object_type == ObjectType::Entity)
            {
                selectivity_plan_for_entity(
                    candidate.object.id,
                    candidate.score(),
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
            let mut query =
                graph_query_for_candidate(candidate, &context, selectivity_plan.fanout_overrides)
                    .with_fanout_utilization_recording(trace_mode);
            if context.activity == Some(crate::api::types::ActivityRef::Thread(candidate.object.id))
                && candidate.object.object_type == ObjectType::MemoryThread
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
                        fail_if_closed(context.graph_limits.failure_mode, failure)?;
                    }
                    if candidate.source() == GraphRootSource::Participant {
                        let scope = cues
                            .participants
                            .iter()
                            .position(|id| *id == candidate.object.id)
                            .expect("participant roots follow resolved scene notions");
                        state::record_subject_state(
                            &mut state_scopes,
                            scope,
                            candidate.object.id,
                            &expansion,
                        );
                    }
                    for kind in explicit_roots
                        .iter()
                        .filter(|root| {
                            root.object.object_type == candidate.object.object_type
                                && root.object.id == candidate.object.id
                        })
                        .flat_map(|root| root.roads.keys())
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
                    assembly.omit_unverified_candidate(
                        candidate,
                        StaleCandidateReason::GraphObjectMissing,
                    )
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

        let missing_times = assembly
            .objects
            .values()
            .filter_map(|ranked| match &ranked.object {
                MemoryObject::Observation(observation) if observation.observed_at.is_none() => {
                    Some(ranked.object.object_ref())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if !missing_times.is_empty() {
            for (object, occasion) in self
                .graph_store
                .query_episode_occasions(&missing_times)
                .await?
            {
                if let Some(ranked) = assembly.objects.get_mut(&object) {
                    ranked.memory_time = Some(occasion.time);
                }
            }
        }
        let ranked_objects = assembly.ranked_objects();
        for (kind, order) in root_selection.orders {
            section_orders.entry(kind).or_default().extend(order);
        }
        let mut details = RetrievalDetails {
            lifecycle_filter_decisions: assembly.lifecycle_decisions,
            stale_candidate_omissions: assembly.stale_omissions,
            section_assignments: Vec::new(),
            admitted_by: HashMap::new(),
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
        let memory_scenes = self
            .memory_scenes(
                &pack,
                details.admitted_by,
                context.lifecycle_policy.include_suppressed,
                context.scene.time.to_utc(),
            )
            .await?;
        let trace = trace_mode.is_enabled().then(|| RetrievalTrace {
            scene_cue_searches: cues.scene_cue_searches,
            time_range_has_more,
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
    admitted_by: HashMap<MemoryObjectRef, BTreeSet<AdmissionRoad>>,
    floor_admissions: Vec<CueFloorAdmission>,
}

#[derive(Debug, Default)]
struct RetrieveAssembly {
    objects: HashMap<MemoryObjectRef, RankedObject>,
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
        let candidate_ref = candidate.object;

        // Reuse the traversal rule on the admitted graph. This reads no store and
        // cannot widen the root's bounds; it identifies where the reminder road ends.
        let reminder_reached = if candidate.expanding_score().is_some()
            && candidate.roads.len() != candidate.expanding_roads().len()
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
            let (score, kinds) = match candidate.expanding_score() {
                Some(score)
                    if object_ref != candidate_ref && !reminder_reached.contains(&object_ref) =>
                {
                    (score, candidate.expanding_roads())
                }
                _ => (candidate.score(), candidate.road_set()),
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
                    ranked.roads.extend(&kinds);
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
                    ranked.is_root = object_ref == candidate_ref;
                    ranked
                });
            if let Some(resolvers) = expansion.resolved_by.get(&object_ref) {
                ranked.resolved_by.extend(resolvers);
                ranked.resolved_by.sort_unstable();
                ranked.resolved_by.dedup();
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
                    candidate: candidate.object,
                    vector_score: candidate.vector_score,
                    reason: stale_reason,
                });
            }
            self.lifecycle_decisions.push(decision);
        }

        if !root_filtered && !root_verified && !self.objects.contains_key(&candidate_ref) {
            let reason = if bounded_failure.is_some() {
                StaleCandidateReason::GraphExpansionBounded
            } else {
                StaleCandidateReason::GraphObjectMissing
            };
            self.omit_unverified_candidate(candidate, reason);
        }
        Ok(())
    }

    fn omit_unverified_candidate(
        &mut self,
        candidate: &CandidateRoot,
        reason: StaleCandidateReason,
    ) {
        self.stale_omissions.push(StaleCandidateOmission {
            candidate: candidate.object,
            vector_score: candidate.vector_score,
            reason,
        });
        self.lifecycle_decisions.push(LifecycleFilterDecision {
            object: candidate.object,
            superseded_by: Vec::new(),
            reason: match reason {
                StaleCandidateReason::GraphExpansionBounded => {
                    LifecycleFilterReason::GraphExpansionBounded
                }
                StaleCandidateReason::GraphObjectMissing => {
                    LifecycleFilterReason::GraphObjectMissing
                }
                _ => unreachable!("unverified candidates are missing or bounded"),
            },
        });
    }

    fn ranked_objects(&mut self) -> Vec<RankedObject> {
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
    roads: BTreeSet<RecallRoad>,
    reminder_only: bool,
    memory_time: Option<chrono::DateTime<chrono::Utc>>,
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
        roads: BTreeSet<RecallRoad>,
        graph_component: f32,
        vector_candidate_score: Option<f32>,
    ) -> Self {
        let salience_component = salience_component(&object);
        let memory_time = match &object {
            MemoryObject::Episode(object) => Some(object.scene.time.to_utc()),
            MemoryObject::Observation(object) => object.observed_at,
            MemoryObject::DerivedMemory(object) => Some(object.created_at),
            MemoryObject::MemoryThread(object) => Some(object.created_at),
            MemoryObject::Entity(object) => Some(object.created_at),
            MemoryObject::MemoryLink(object) => Some(object.created_at),
        };
        Self {
            object,
            cue_component,
            roads,
            reminder_only: false,
            memory_time,
            is_root: false,
            vector_candidate_score,
            graph_component,
            salience_component,
            resolved_by: Vec::new(),
        }
    }

    fn kinds(&self) -> BTreeSet<CueKind> {
        road_kinds(&self.roads)
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
            time: std::cmp::Reverse(self.memory_time),
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
    time: std::cmp::Reverse<Option<chrono::DateTime<chrono::Utc>>>,
    object_type_rank: u8,
    object_id: MemoryId,
}

// A reservation credits its road's own head, even if already selected. Only
// expanding roads can take a spare turn at roots; other stages fill ranked room.
fn select_with_cue_floors<I: IntoIterator<Item = MemoryObjectRef>>(
    candidates: impl IntoIterator<Item = (MemoryObjectRef, BTreeSet<RecallRoad>)>,
    orders: impl Fn(RecallRoad) -> I,
    limit: usize,
    floors: RetrievalCueFloors,
    stage: CueFloorStage,
) -> Vec<(usize, Option<CueKind>)> {
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    if candidates.len() <= limit {
        return (0..candidates.len()).map(|index| (index, None)).collect();
    }
    let mut roads = candidates
        .iter()
        .flat_map(|(_, roads)| roads.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    roads.sort_by_key(|road| (!road.rule().expands, *road));
    let expanding_kinds = roads
        .iter()
        .filter(|road| road.rule().expands)
        .map(|road| road.rule().kind)
        .collect::<BTreeSet<_>>();
    let mut queues = [
        CueKind::Participant,
        CueKind::Place,
        CueKind::Activity,
        CueKind::DateMatch,
        CueKind::Topic,
        CueKind::Recency,
    ]
    .map(|kind| {
        let mut queue = Vec::new();
        let mut seen = HashSet::new();
        for road in roads
            .iter()
            .filter(|road| road.rule().kind == kind && road.rule().reserves)
        {
            let mut ranks = HashMap::new();
            for (rank, object) in orders(*road).into_iter().enumerate() {
                ranks.entry(object).or_insert(rank);
            }
            let mut members = candidates
                .iter()
                .enumerate()
                .filter_map(|(index, (_, roads))| roads.contains(road).then_some(index))
                .collect::<Vec<_>>();
            members.sort_by_key(|&index| {
                ranks
                    .get(&candidates[index].0)
                    .copied()
                    .unwrap_or(usize::MAX)
            });
            queue.extend(members.into_iter().filter(|index| seen.insert(*index)));
        }
        (kind, cue_floor(floors, kind), queue.into_iter())
    });
    let mut selected = BTreeMap::new();
    'selection: for reserve in [true, false] {
        if !reserve && (stage != CueFloorStage::GraphRoots || expanding_kinds.len() < 2) {
            break;
        }
        for round in 0..limit {
            for (kind, floor, queue) in &mut queues {
                if selected.len() == limit {
                    break 'selection;
                }
                if reserve && round >= *floor {
                    continue;
                }
                let next = if reserve {
                    queue.next()
                } else {
                    queue.find(|&index| {
                        candidates[index]
                            .1
                            .iter()
                            .any(|road| road.rule().kind == *kind && road.rule().expands)
                    })
                };
                if let Some(index) = next {
                    selected
                        .entry(index)
                        .or_insert((index >= limit).then_some(*kind));
                }
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
    orders: &BTreeMap<RecallRoad, Vec<MemoryObjectRef>>,
    limits: crate::api::types::ContinuitySectionLimits,
    floors: RetrievalCueFloors,
    details: &mut RetrievalDetails,
    section_pressure: &mut [SectionPressureSummary],
) -> ContinuityContextPack {
    let mut pack = ContinuityContextPack::empty();
    let mut selected = HashSet::new();
    for section in prompt_ready_sections() {
        state::order_state_per_kind(
            &mut ranked_objects,
            state_scopes,
            scope_kinds,
            |object| {
                (section_for_object(object) == Some(section)).then(|| object.object.object_ref())
            },
            |_, _| 0,
        );
        let candidates = ranked_objects
            .iter()
            .filter(|ranked| section_for_object(ranked) == Some(section))
            .collect::<Vec<_>>();
        // Section state already has its scope/score order; root recency must not
        // become the section floor order. Reuse this prefix without re-sorting it.
        let mut state_orders = BTreeMap::new();
        for kind in scope_kinds.iter().copied().collect::<BTreeSet<_>>() {
            let state_order = candidates
                .iter()
                .filter_map(|ranked| {
                    let object = ranked.object.object_ref();
                    state_scopes.get(&object).and_then(|scopes| {
                        scopes
                            .iter()
                            .any(|&scope| scope_kinds[scope] == kind)
                            .then_some(object)
                    })
                })
                .collect::<Vec<_>>();
            state_orders.insert(kind, state_order);
        }
        for (index, cause) in select_with_cue_floors(
            candidates
                .iter()
                .map(|ranked| (ranked.object.object_ref(), ranked.roads.clone())),
            |road| {
                state_orders
                    .get(&road.rule().kind)
                    .filter(|_| road.rule().expands && orders.contains_key(&road))
                    .into_iter()
                    .flatten()
                    .chain(orders.get(&road).into_iter().flatten())
                    .copied()
            },
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
                cue_kinds: ranked.kinds(),
            });
            continue;
        };

        let pressure = section_pressure_for(section_pressure, section);
        if !selected.contains(&ranked.object.object_ref()) {
            pressure.omitted_by_limit_count += 1;
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
                cue_kinds: ranked.kinds(),
            });
            continue;
        }

        pressure.included_count += 1;
        let rank = pressure.included_count;
        details.admitted_by.insert(
            ranked.object.object_ref(),
            ranked
                .roads
                .iter()
                .map(|road| road.rule().admitted_by)
                .collect(),
        );
        details.section_assignments.push(SectionAssignment {
            object: ranked.object.object_ref(),
            section,
            rank: Some(rank),
            reason: SectionAssignmentReason::Selected {
                scores: ranked.section_score_components(),
            },
            cue_kinds: ranked.kinds(),
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

fn section_pressure_for(
    section_pressure: &mut [SectionPressureSummary],
    section: ContextPackSection,
) -> &mut SectionPressureSummary {
    section_pressure
        .iter_mut()
        .find(|summary| summary.section == section)
        .expect("every pack section has a pressure summary")
}

fn count_reasons<R: Copy + Eq>(
    reasons: impl Iterator<Item = R>,
    rank: impl Fn(R) -> u8,
) -> impl Iterator<Item = (R, usize)> {
    let mut counts = Vec::<(R, usize)>::new();
    for reason in reasons {
        if let Some((_, count)) = counts.iter_mut().find(|(value, _)| *value == reason) {
            *count += 1;
        } else {
            counts.push((reason, 1));
        }
    }
    counts.sort_by_key(|(reason, _)| rank(*reason));
    counts.into_iter()
}

fn summarize_stale_candidate_omissions(
    omissions: &[StaleCandidateOmission],
) -> Vec<StaleCandidateOmissionSummary> {
    count_reasons(
        omissions.iter().map(|entry| entry.reason),
        stale_reason_rank,
    )
    .map(|(reason, count)| StaleCandidateOmissionSummary { reason, count })
    .collect()
}

fn summarize_lifecycle_omissions(
    decisions: &[LifecycleFilterDecision],
) -> Vec<LifecycleOmissionSummary> {
    count_reasons(
        decisions.iter().map(|entry| entry.reason),
        lifecycle_reason_rank,
    )
    .map(|(reason, count)| LifecycleOmissionSummary { reason, count })
    .collect()
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

// Declaration order is the road order in the root key and in the README.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RecallRoad {
    Participant,
    Place,
    Activity,
    Topic,
    ParticipantDescription,
    SettingWords,
    Range,
    SharedAnniversary,
    UnsharedAnniversary,
    Recency,
}

#[derive(Clone, Copy)]
enum Contribution {
    RootCap,
    CandidateCap,
    Description,
    Room,
}

struct RoadRule {
    kind: CueKind,
    admitted_by: AdmissionRoad,
    source: GraphRootSource,
    expands: bool,
    reserves: bool,
    contribution: Contribution,
    time_rank: bool,
}

impl RecallRoad {
    fn rule(self) -> RoadRule {
        use Contribution::*;
        let (kind, admitted_by, source, expands, reserves, contribution, time_rank) = match self {
            Self::Participant => (
                CueKind::Participant,
                AdmissionRoad::Participant,
                GraphRootSource::Participant,
                true,
                true,
                RootCap,
                false,
            ),
            Self::Place => (
                CueKind::Place,
                AdmissionRoad::Place,
                GraphRootSource::Place,
                false,
                true,
                RootCap,
                true,
            ),
            Self::Activity => (
                CueKind::Activity,
                AdmissionRoad::Activity,
                GraphRootSource::Activity,
                true,
                true,
                RootCap,
                false,
            ),
            Self::Topic => (
                CueKind::Topic,
                AdmissionRoad::Topic,
                GraphRootSource::Vector,
                true,
                true,
                CandidateCap,
                false,
            ),
            Self::ParticipantDescription => (
                CueKind::Participant,
                AdmissionRoad::PersonDescription,
                GraphRootSource::Vector,
                false,
                true,
                Description,
                false,
            ),
            Self::SettingWords => (
                CueKind::Place,
                AdmissionRoad::SettingWords,
                GraphRootSource::Vector,
                false,
                true,
                Description,
                false,
            ),
            Self::Range => (
                CueKind::DateMatch,
                AdmissionRoad::Range,
                GraphRootSource::DateMatch,
                false,
                true,
                Room,
                true,
            ),
            Self::SharedAnniversary => (
                CueKind::DateMatch,
                AdmissionRoad::Anniversary,
                GraphRootSource::DateMatch,
                false,
                true,
                Room,
                true,
            ),
            Self::UnsharedAnniversary => (
                CueKind::DateMatch,
                AdmissionRoad::Anniversary,
                GraphRootSource::DateMatch,
                false,
                false,
                Room,
                true,
            ),
            Self::Recency => (
                CueKind::Recency,
                AdmissionRoad::Recency,
                GraphRootSource::Recency,
                false,
                true,
                Room,
                true,
            ),
        };
        RoadRule {
            kind,
            admitted_by,
            source,
            expands,
            reserves,
            contribution,
            time_rank,
        }
    }

    fn contribution(self, context: &RetrievalContext) -> usize {
        let room = || {
            prompt_ready_sections()
                .into_iter()
                .map(|section| section_limit(section, context.section_limits))
                .max()
                .unwrap_or(0)
        };
        match self.rule().contribution {
            Contribution::RootCap => context.candidate_limits.max_graph_roots,
            Contribution::CandidateCap => context.candidate_limits.max_vector_candidates,
            Contribution::Description => cue_floor(context.cue_floors, self.rule().kind).max(1),
            Contribution::Room => room(),
        }
    }
}

fn cue_floor(floors: RetrievalCueFloors, kind: CueKind) -> usize {
    match kind {
        CueKind::Participant => floors.participant,
        CueKind::Place => floors.place,
        CueKind::Activity => floors.activity,
        CueKind::Topic => floors.topic,
        CueKind::DateMatch => floors.date_match,
        CueKind::Recency => floors.recency,
    }
}

fn road_kinds(roads: &BTreeSet<RecallRoad>) -> BTreeSet<CueKind> {
    roads.iter().map(|road| road.rule().kind).collect()
}

#[derive(Debug, Clone, Copy)]
struct RoadReach {
    score: f32,
    position: usize,
}

#[derive(Debug, Clone)]
struct CandidateRoot {
    object: MemoryObjectRef,
    roads: BTreeMap<RecallRoad, RoadReach>,
    vector_score: Option<f32>,
    memory_rank: Option<crate::ports::graph_authority::GraphMemoryRank>,
}

impl CandidateRoot {
    fn new(object: MemoryObjectRef, road: RecallRoad, score: f32, position: usize) -> Self {
        Self {
            object,
            roads: BTreeMap::from([(
                road,
                RoadReach {
                    score: if score > 0.0 { score } else { 0.0 },
                    position,
                },
            )]),
            vector_score: None,
            memory_rank: None,
        }
    }

    fn from_rank(
        row: crate::ports::graph_authority::GraphMemoryRank,
        object_type: ObjectType,
        road: RecallRoad,
        score: f32,
        position: usize,
    ) -> Self {
        let mut root = Self::new(
            MemoryObjectRef::new(object_type, row.id),
            road,
            score,
            position,
        );
        if root.score() == 0.0 && road.rule().time_rank {
            root.memory_rank = Some(row);
        }
        root
    }

    fn score(&self) -> f32 {
        self.roads
            .values()
            .map(|reach| reach.score)
            .max_by(f32::total_cmp)
            .unwrap_or(0.0)
    }

    fn road_set(&self) -> BTreeSet<RecallRoad> {
        self.roads.keys().copied().collect()
    }

    fn expanding_roads(&self) -> BTreeSet<RecallRoad> {
        self.roads
            .keys()
            .filter(|road| road.rule().expands)
            .copied()
            .collect()
    }

    fn expanding_score(&self) -> Option<f32> {
        self.roads
            .iter()
            .filter(|(road, _)| road.rule().expands)
            .map(|(_, reach)| reach.score)
            .max_by(f32::total_cmp)
    }

    fn reminder_only(&self) -> bool {
        !self.roads.keys().any(|road| road.rule().expands)
    }

    fn source(&self) -> GraphRootSource {
        self.roads
            .first_key_value()
            .expect("a root has a road")
            .0
            .rule()
            .source
    }

    fn merge(&mut self, other: Self) {
        for (road, reach) in other.roads {
            self.roads
                .entry(road)
                .and_modify(|prior| {
                    prior.score = prior.score.max(reach.score);
                    prior.position = prior.position.min(reach.position);
                })
                .or_insert(reach);
        }
        if let Some(score) = other.vector_score {
            self.vector_score = Some(self.vector_score.map_or(score, |prior| prior.max(score)));
        }
        if other.memory_rank.is_some() {
            self.memory_rank = other.memory_rank;
        }
        if self.score() > 0.0 {
            self.memory_rank = None;
        }
    }

    fn rank_key(
        &self,
    ) -> (
        SortableScore,
        SortableScore,
        std::cmp::Reverse<Option<chrono::DateTime<chrono::Utc>>>,
        RecallRoad,
        usize,
        MemoryId,
    ) {
        let rank = self
            .memory_rank
            .filter(|_| self.score() == 0.0 && self.roads.keys().any(|road| road.rule().time_rank));
        let (road, reach) = self.roads.first_key_value().expect("a root has a road");
        (
            SortableScore(self.score()),
            SortableScore(rank.map_or(0.0, |rank| rank.salience)),
            std::cmp::Reverse(rank.map(|rank| rank.time)),
            *road,
            reach.position,
            self.object.id,
        )
    }
}

#[derive(Debug)]
struct CandidateRootSelection {
    roots: Vec<CandidateRoot>,
    unique_count: usize,
    omitted: Vec<CandidateRoot>,
    floor_admissions: Vec<CueFloorAdmission>,
    orders: BTreeMap<RecallRoad, Vec<MemoryObjectRef>>,
}

fn select_candidate_roots(
    candidates: Vec<CandidateRoot>,
    explicit_roots: &[CandidateRoot],
    content_orders: &BTreeMap<RecallRoad, Vec<MemoryObjectRef>>,
    max_graph_roots: usize,
    floors: RetrievalCueFloors,
    (scopes, root_order, scope_kinds): (
        &state::StateScopes,
        &HashMap<(usize, MemoryObjectRef), usize>,
        &[CueKind],
    ),
) -> CandidateRootSelection {
    let mut orders: BTreeMap<RecallRoad, Vec<MemoryObjectRef>> = BTreeMap::new();
    let mut by_ref: HashMap<MemoryObjectRef, CandidateRoot> = HashMap::new();
    for root in explicit_roots.iter().cloned().chain(candidates) {
        for road in root.roads.keys() {
            orders.entry(*road).or_default().push(root.object);
        }
        match by_ref.entry(root.object) {
            std::collections::hash_map::Entry::Occupied(mut entry) => entry.get_mut().merge(root),
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(root);
            }
        }
    }
    for (road, order) in content_orders {
        orders.insert(*road, order.clone());
    }
    for (road, order) in &mut orders {
        let mut seen = HashSet::new();
        order.retain(|object| seen.insert(*object));
        let own_scopes = state::scopes_for_kind(scopes, scope_kinds, road.rule().kind);
        state::order_state(
            order,
            &own_scopes,
            |object| Some(*object),
            |scope, object| root_order[&(scope, *object)],
        );
        for (position, object) in order.iter().enumerate() {
            if let Some(root) = by_ref.get_mut(object) {
                if let Some(reach) = root.roads.get_mut(road) {
                    reach.position = position;
                }
            }
        }
    }
    let mut merged = by_ref.into_values().collect::<Vec<_>>();
    merged.sort_by_key(CandidateRoot::rank_key);
    state::order_state_per_kind(
        &mut merged,
        scopes,
        scope_kinds,
        |root| Some(root.object),
        |scope, root| root_order[&(scope, root.object)],
    );
    let unique_count = merged.len();
    let selection = select_with_cue_floors(
        merged.iter().map(|root| (root.object, root.road_set())),
        |road| orders.get(&road).into_iter().flatten().copied(),
        max_graph_roots,
        floors,
        CueFloorStage::GraphRoots,
    );
    let mut selection = selection.into_iter().peekable();
    let mut roots = Vec::new();
    let mut omitted = Vec::new();
    let mut floor_admissions = Vec::new();
    for (index, root) in merged.into_iter().enumerate() {
        if let Some((_, cause)) = selection.next_if(|(chosen, _)| *chosen == index) {
            if let Some(cue_kind) = cause {
                floor_admissions.push(CueFloorAdmission {
                    object: root.object,
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
        candidate.object.id,
        candidate.object.object_type,
        context.graph_limits.max_depth,
        context.graph_limits.max_nodes,
    )
    .with_allowed_object_types(context.graph_limits.allowed_object_types.clone())
    .with_allowed_relation_types(context.graph_limits.allowed_relation_types.clone())
    .with_fanout_overrides(fanout_overrides)
    .with_max_fanout_per_node(context.graph_limits.max_fanout_per_node)
    .with_max_hub_edges(context.graph_limits.max_hub_edges)
    .with_lifecycle_policy(GraphExpansionLifecyclePolicy::from(
        context.lifecycle_policy,
    ))
    .with_failure_policy(GraphExpansionFailurePolicy {
        timeout_ms: context.graph_limits.timeout_ms,
        mode: context.graph_limits.failure_mode,
    });
    query.current_subject_state = candidate.object.object_type == ObjectType::Entity
        && candidate.source() == GraphRootSource::Participant;
    query.reminder_only = candidate.reminder_only();
    query.participant_reference_time = context.scene.time.to_utc();
    query.allow_future_root = candidate.object.object_type == ObjectType::Episode
        && candidate.roads.contains_key(&RecallRoad::Range);
    query
}

fn absorb_selectivity_telemetry(total: &mut SelectivityTelemetry, next: &SelectivityTelemetry) {
    total.decision_count += next.decision_count;
    total.high_selectivity_count += next.high_selectivity_count;
    total.low_selectivity_supported_count += next.low_selectivity_supported_count;
    total.low_selectivity_rejected_count += next.low_selectivity_rejected_count;
    total.fallback_count += next.fallback_count;
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
        source: candidate.source(),
        root: candidate.object,
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
        source: candidate.source(),
        root: candidate.object,
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
        superseded_by: superseded_by.to_vec(),
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
    use crate::domain::GraphFailureMode;
    use crate::domain::ScopeKey;
    use crate::ports::graph_authority::GraphExpansionFilteredNode;

    use std::sync::{Arc, Mutex, MutexGuard};

    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::api::types::retrieval::VectorRecallCompleteness;
    use crate::api::types::ContinuitySectionLimits;
    use crate::domain::RetentionState;
    use crate::models::vector::{CanonicalCandidates, VectorRecordEmbedding};
    use crate::test_support::{
        in_memory_graph_store, representative_fixtures, TemporaryVectorCandidateStore,
    };

    #[tokio::test]
    async fn search_and_explicit_roots_canonicalize_nonpositive_scores() {
        let graph = graph_with(&[], &[]).await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        for score in [-0.0_f32, 0.0, -0.5, 0.75] {
            let expected = if score > 0.0 { score } else { 0.0 };
            let object = MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(1));
            let vector = VectorRecallOverride {
                inner: TemporaryVectorCandidateStore::open(2).await,
                completeness: None,
                candidate: Some(vector_candidate(object.id, object.object_type, score)),
            };
            let cues = RetrievePipeline::new(&graph, &vector, &embedder)
                .recall_cues(&RetrievalContext::new("score boundary"))
                .await
                .unwrap();
            assert_eq!(
                cues.candidates.iter().next().unwrap().score.to_bits(),
                expected.to_bits()
            );
            let root = CandidateRoot::new(object, RecallRoad::Topic, score, 0);
            assert_eq!(root.score().to_bits(), expected.to_bits());
            vector.close().await.unwrap();
        }
    }

    #[tokio::test]
    async fn clamped_topic_roots_keep_the_raw_search_head() {
        let graph = graph_with(&[], &[]).await;
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        for (near, far) in [(2, 1), (1, 2)] {
            let vector = TemporaryVectorCandidateStore::open(2).await;
            for (id, score) in [(near, -0.25_f32), (far, -0.75)] {
                let mut memory = representative_fixtures().user_preference;
                memory.id = MemoryId::from_u128(id);
                let object = MemoryObject::DerivedMemory(memory);
                let record = crate::policy::memory_object_vector_record(&object).unwrap();
                vector
                    .upsert_vector_records(&[VectorRecordEmbedding::new(
                        &record,
                        &[score, (1.0 - score * score).sqrt()],
                    )])
                    .await
                    .unwrap();
            }
            let raw = vector
                .search_candidates(&VectorCandidateSearch::new(
                    vec![1.0, 0.0],
                    2,
                    vec![ObjectType::DerivedMemory],
                ))
                .await
                .unwrap();
            assert_eq!(
                raw.candidates.iter().next().unwrap().object_id,
                MemoryId::from_u128(near)
            );
            let cues = RetrievePipeline::new(&graph, &vector, &embedder)
                .recall_cues(&RetrievalContext::new("unmatched topic"))
                .await
                .unwrap();
            assert!(cues
                .candidates
                .iter()
                .all(|candidate| candidate.score.to_bits() == 0));
            let selected = select_candidate_roots(
                cues.roots,
                &[],
                &cues.orders,
                1,
                RetrievalCueFloors {
                    topic: 1,
                    ..Default::default()
                },
                (&HashMap::new(), &HashMap::new(), &[]),
            );
            assert_eq!(selected.roots[0].object.id, MemoryId::from_u128(near));
            vector.close().await.unwrap();
        }
    }

    #[test]
    fn resolution_metadata_stays_on_derived_memory_when_ids_collide() {
        let fixtures = representative_fixtures();
        let entity = fixtures.hub_entity;
        let mut memory = fixtures.open_loop;
        memory.id = entity.id;
        let resolver = fixtures.correction.id;
        let candidate = CandidateRoot::new(
            MemoryObjectRef::new(ObjectType::Entity, entity.id),
            RecallRoad::Participant,
            1.0,
            0,
        );
        let query = GraphExpansionQuery::new(entity.id, ObjectType::Entity, 1, 2);
        let mut expansion = GraphExpansion::new(
            vec![
                MemoryObject::Entity(entity.clone()),
                MemoryObject::DerivedMemory(memory),
            ],
            Vec::new(),
        );
        expansion.resolved_by.insert(
            MemoryObjectRef::new(ObjectType::DerivedMemory, entity.id),
            vec![resolver],
        );
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
        let kinds = BTreeSet::from([RecallRoad::Participant]);
        let orders = BTreeMap::from([(
            RecallRoad::Participant,
            vec![objects[2], objects[1], objects[0]],
        )]);
        let selected = select_with_cue_floors(
            objects.into_iter().map(|object| (object, kinds.clone())),
            |road| orders.get(&road).into_iter().flatten().copied(),
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
    fn equal_section_scores_use_memory_time_for_every_road() {
        let fixtures = representative_fixtures();
        let mut assembly = RetrieveAssembly::new(TraceMode::Disabled);
        for n in 1..=3 {
            let mut episode = fixtures.episode.clone();
            episode.id = MemoryId::from_u128(n);
            episode.salience_score = 0.0;
            episode.scene.time += chrono::Duration::days(n as i64);
            let kinds = BTreeSet::from([if n == 2 {
                RecallRoad::Topic
            } else {
                RecallRoad::Recency
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
    fn section_time_breaks_ties_without_overriding_distinct_scores() {
        let fixtures = representative_fixtures();
        let mut assembly = RetrieveAssembly::new(TraceMode::Disabled);
        for n in 1..=6 {
            let mut episode = fixtures.episode.clone();
            episode.id = MemoryId::from_u128(n);
            episode.salience_score = 0.0;
            episode.scene.time += chrono::Duration::days(n as i64);
            let kind = match n {
                1 | 6 => RecallRoad::Recency,
                3 | 5 => RecallRoad::Range,
                _ => RecallRoad::Topic,
            };
            let ranked = RankedObject::new(
                MemoryObject::Episode(episode),
                if n == 6 { 1.0 } else { 0.0 },
                BTreeSet::from([kind]),
                1.0,
                None,
            );
            assembly.objects.insert(ranked.object.object_ref(), ranked);
        }
        assert_eq!(
            assembly
                .ranked_objects()
                .iter()
                .map(|ranked| ranked.object.id().as_u128())
                .collect::<Vec<_>>(),
            [6, 5, 4, 3, 2, 1]
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
        let full = CandidateRoot::new(
            MemoryObjectRef::new(ObjectType::Entity, person.id),
            RecallRoad::Participant,
            1.0,
            0,
        );
        let recent = CandidateRoot::new(
            MemoryObjectRef::new(ObjectType::Episode, episode.id),
            RecallRoad::Recency,
            0.0,
            0,
        );
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
                ranked.kinds(),
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
                .map(|candidate| {
                    CandidateRoot::new(
                        MemoryObjectRef::new(candidate.object_type, candidate.object_id),
                        RecallRoad::Topic,
                        candidate.score,
                        0,
                    )
                })
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
                .map(|root| (root.object.id, root.score()))
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
            .any(|decision| decision.object.id == fixtures.suppressed_seed.id));
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
            assert!(trace
                .lifecycle_filter_decisions
                .iter()
                .any(|decision| { decision.object.id == fixtures.suppressed_seed.id }));
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
        ) -> Result<Vec<(crate::ports::graph_authority::GraphMemoryRank, bool)>, CustomError>
        {
            let _ = (date, participants, limit, policy);
            Ok(Vec::new())
        }

        async fn query_episodes_by_time(
            &self,
            start: Option<chrono::DateTime<chrono::Utc>>,
            end: chrono::DateTime<chrono::Utc>,
            limit: usize,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Vec<crate::ports::graph_authority::GraphMemoryRank>, CustomError> {
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

        async fn query_thread_state(
            &self,
            _query: &crate::ports::graph_authority::GraphDerivedMemoryThreadQuery,
            _limit: usize,
        ) -> Result<
            (
                Vec<crate::ports::graph_authority::GraphMemoryRank>,
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
        ) -> Result<
            (
                Vec<crate::ports::graph_authority::GraphMemoryRank>,
                Vec<GraphExpansionFilteredNode>,
            ),
            CustomError,
        > {
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
