// Continuity retrieval pipeline used by the public facade and internal tests.
// Some helper APIs are intentionally retained for retrieval policy validation.
use std::collections::{HashMap, HashSet};

use crate::api::types::{
    ContextPackSection, ContinuityContextPack, DerivedMemory, DerivedType, FanoutUtilizationTrace,
    GraphExpansionBoundedFailureTrace, GraphExpansionBoundedReason, GraphExpansionOutcome,
    GraphExpansionTelemetry, GraphExpansionTrace, IncludedDerivedMemory, LifecycleFilterAction,
    LifecycleFilterDecision, LifecycleFilterReason, LifecycleOmissionSummary, MemoryId,
    MemoryObject, MemoryObjectRef, MemoryThread, ObjectType, RationaleCategory, RelationType,
    RetentionState, RetrievalContext, RetrievalLifecyclePolicy, RetrievalRationale,
    RetrievalTelemetry, RetrievalTrace, RetrieveOutcome, SectionAssignment, SectionPressureSummary,
    SelectivityTelemetry, StaleCandidateOmission, StaleCandidateOmissionSummary,
    StaleCandidateReason, ThreadStatus, VectorCandidateTrace,
};
use crate::errors::CustomError;
use crate::models::vector::{
    canonicalize_vector_candidates, EmbeddingInput, VectorCandidateFilters, VectorCandidateMatch,
    VectorCandidateSearch, VectorSurface,
};
use crate::policy::{
    selectivity_plan_for_candidate, RetrievalSelectivityPolicy, SelectivityPlan,
    SelectivityStatsContext,
};
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphExpansion, GraphExpansionBoundedFailure,
    GraphExpansionBoundedFailureReason, GraphExpansionFailurePolicy, GraphExpansionFilteredReason,
    GraphExpansionLifecyclePolicy, GraphExpansionQuery, GraphObjectRef,
};
use crate::ports::retrieval_stats::RetrievalStatsStore;
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
        context: RetrievalContext,
    ) -> Result<RetrieveOutcome, CustomError> {
        let query_embedding = self.embed_query(&context).await?;
        let query_embedding_dimension = query_embedding.len();
        let vector_search = VectorCandidateSearch::new(
            query_embedding,
            context.candidate_limits.max_vector_candidates,
        )
        .with_object_types(context.object_type_defaults.clone())
        .with_filters(VectorCandidateFilters::new());
        let vector_candidates = canonicalize_vector_candidates(
            self.vector_store.search_candidates(&vector_search).await?,
        );
        let include_trace = context.include_trace;

        let root_selection =
            select_candidate_roots(&vector_candidates, context.candidate_limits.max_graph_roots);
        let candidate_roots = root_selection.roots;
        let mut assembly = RetrieveAssembly::new(include_trace);
        let mut graph_expansion_telemetry = GraphExpansionTelemetry::default();
        let mut selectivity_telemetry = SelectivityTelemetry::default();
        let mut graph_expansion_traces = include_trace.then(Vec::new);
        let mut fanout_utilization_traces = include_trace.then(Vec::new);
        let mut selectivity_traces = include_trace.then(Vec::new);
        let selectivity_stats_context = if candidate_roots
            .iter()
            .any(|candidate| candidate.object_type == ObjectType::Entity)
        {
            Some(
                SelectivityStatsContext::load_with_scope(
                    self.stats_store,
                    &context.object_type_defaults,
                    &context.graph_limits.allowed_relation_types,
                )
                .await?,
            )
        } else {
            None
        };

        for candidate in &candidate_roots {
            let selectivity_plan = if let Some(stats_context) = &selectivity_stats_context {
                selectivity_plan_for_candidate(
                    candidate,
                    context.graph_limits.max_fanout_per_node,
                    self.stats_store,
                    self.selectivity_policy,
                    stats_context,
                    context.lifecycle_policy,
                    include_trace,
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
                    .with_fanout_utilization_recording(include_trace);
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
                        if !context.graph_limits.allow_degraded_results {
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

        let ranked_objects = assembly.ranked_objects(&context.lifecycle_policy);
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
            query_embedding_dimension,
            returned_vector_candidate_count: vector_candidates.len(),
            unique_graph_root_candidate_count: root_selection.unique_count,
            selected_graph_root_count: candidate_roots.len(),
            graph_root_omission_count: root_selection.omitted_count,
            graph_expansion: graph_expansion_telemetry,
            selectivity: selectivity_telemetry,
            section_pressure,
        };
        let trace = include_trace.then(|| RetrievalTrace {
            vector_candidates: vector_candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| VectorCandidateTrace {
                    object: memory_object_ref(candidate.object_type, candidate.object_id),
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

        Ok(RetrieveOutcome {
            pack,
            rationale,
            trace,
        })
    }

    async fn embed_query(&self, context: &RetrievalContext) -> Result<Vec<f32>, CustomError> {
        let text = match context.current_context.as_deref() {
            Some(current_context) if !current_context.trim().is_empty() => {
                format!("{}\n{}", context.query_text.trim(), current_context.trim())
            }
            _ => context.query_text.trim().to_owned(),
        };
        let input = EmbeddingInput::new(None, None, VectorSurface::Query, text);
        self.embedder.embed(&input).await
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
    objects: HashMap<GraphObjectRef, RankedObject>,
    candidate_scores: HashMap<GraphObjectRef, f32>,
    candidate_refs: HashSet<GraphObjectRef>,
    superseded_by: HashMap<MemoryId, Vec<MemoryId>>,
    lifecycle_decisions: Vec<LifecycleFilterDecision>,
    stale_omissions: Vec<StaleCandidateOmission>,
    graph_relations: Option<Vec<crate::api::types::GraphRelationTrace>>,
}

impl RetrieveAssembly {
    fn new(collect_trace: bool) -> Self {
        Self {
            graph_relations: collect_trace.then(Vec::new),
            ..Self::default()
        }
    }

    fn absorb_expansion(&mut self, candidate: &VectorCandidateMatch, expansion: GraphExpansion) {
        let bounded_failure = expansion.bounded_failure;
        let candidate_ref = GraphObjectRef::new(candidate.object_id, candidate.object_type);
        self.candidate_refs.insert(candidate_ref);
        self.candidate_scores
            .entry(candidate_ref)
            .and_modify(|score| *score = score.max(candidate.score))
            .or_insert(candidate.score);

        for relation in &expansion.relations {
            if relation.relation == RelationType::Supersedes
                && relation.from.object_type == ObjectType::DerivedMemory
                && relation.to.object_type == ObjectType::DerivedMemory
            {
                self.superseded_by
                    .entry(relation.to.object_id)
                    .or_default()
                    .push(relation.from.object_id);
            }
        }

        if let Some(graph_relations) = &mut self.graph_relations {
            for relation in &expansion.relations {
                graph_relations.push(crate::api::types::GraphRelationTrace {
                    from: memory_object_ref(relation.from.object_type, relation.from.object_id),
                    to: memory_object_ref(relation.to.object_type, relation.to.object_id),
                    relation: relation.relation,
                    proximity: relation.proximity,
                });
            }
        }

        let mut proximity_by_ref = HashMap::new();
        proximity_by_ref.insert(candidate_ref, 0_u8);
        let graph_rationale_by_ref = self
            .graph_relations
            .as_ref()
            .map(|_| graph_provenance(candidate_ref, &expansion.relations));
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
            let object_ref = graph_object_ref(&object);
            if object_ref == candidate_ref {
                root_verified = true;
            }
            let graph_component = proximity_by_ref
                .get(&object_ref)
                .copied()
                .map(graph_component)
                .unwrap_or(0.0);
            let inherited_vector = if object_ref == candidate_ref {
                candidate.score
            } else {
                candidate.score * 0.75
            };
            let candidate_score = (object_ref == candidate_ref).then_some(candidate.score);
            let graph_rationale = graph_rationale_by_ref
                .as_ref()
                .and_then(|rationale_by_ref| rationale_by_ref.get(&object_ref))
                .copied()
                .unwrap_or_default();
            self.objects
                .entry(object_ref)
                .and_modify(|ranked| {
                    ranked.vector_component = ranked.vector_component.max(inherited_vector);
                    ranked.graph_component = ranked.graph_component.max(graph_component);
                    ranked.graph_rationale.merge(graph_rationale);
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
                        inherited_vector,
                        graph_component,
                        candidate_score,
                        graph_rationale,
                    )
                });
        }

        let mut root_filtered = false;
        for filtered in expansion.filtered_nodes {
            let superseded_by = self
                .superseded_by
                .get(&filtered.object_ref.object_id)
                .cloned()
                .unwrap_or_default();
            let decision =
                filtered_lifecycle_decision(filtered.object_ref, filtered.reason, &superseded_by);
            if filtered.object_ref == candidate_ref {
                root_filtered = true;
                let stale_reason = stale_reason_from_filtered(filtered.reason);
                self.stale_omissions.push(StaleCandidateOmission {
                    candidate: memory_object_ref(candidate.object_type, candidate.object_id),
                    vector_score: Some(candidate.score),
                    reason: stale_reason,
                    rationale_categories: rationale_categories_for_stale_reason(stale_reason),
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

    fn omit_bounded_candidate(&mut self, candidate: &VectorCandidateMatch) {
        self.stale_omissions.push(StaleCandidateOmission {
            candidate: memory_object_ref(candidate.object_type, candidate.object_id),
            vector_score: Some(candidate.score),
            reason: StaleCandidateReason::GraphExpansionBounded,
            rationale_categories: rationale_categories_for_stale_reason(
                StaleCandidateReason::GraphExpansionBounded,
            ),
        });
        self.lifecycle_decisions.push(LifecycleFilterDecision {
            object: memory_object_ref(candidate.object_type, candidate.object_id),
            retention_state: None,
            is_current: None,
            superseded_by: Vec::new(),
            action: LifecycleFilterAction::Omitted,
            reason: LifecycleFilterReason::GraphExpansionBounded,
        });
    }

    fn omit_missing_candidate(&mut self, candidate: &VectorCandidateMatch) {
        self.stale_omissions.push(StaleCandidateOmission {
            candidate: memory_object_ref(candidate.object_type, candidate.object_id),
            vector_score: Some(candidate.score),
            reason: StaleCandidateReason::GraphObjectMissing,
            rationale_categories: rationale_categories_for_stale_reason(
                StaleCandidateReason::GraphObjectMissing,
            ),
        });
        self.lifecycle_decisions.push(LifecycleFilterDecision {
            object: memory_object_ref(candidate.object_type, candidate.object_id),
            retention_state: None,
            is_current: None,
            superseded_by: Vec::new(),
            action: LifecycleFilterAction::Omitted,
            reason: LifecycleFilterReason::GraphObjectMissing,
        });
    }

    fn ranked_objects(&mut self, policy: &RetrievalLifecyclePolicy) -> Vec<RankedObject> {
        for superseded in self.superseded_by.values_mut() {
            superseded.sort();
            superseded.dedup();
        }

        let mut ranked_objects = Vec::new();
        for (object_ref, mut ranked) in std::mem::take(&mut self.objects) {
            let superseded_by = self
                .superseded_by
                .get(&object_ref.object_id)
                .cloned()
                .unwrap_or_default();
            let decision = lifecycle_decision(&ranked.object, &superseded_by, *policy);
            if decision.action == LifecycleFilterAction::Included {
                ranked.superseded_by = superseded_by;
                ranked_objects.push(ranked);
            } else if self.candidate_refs.contains(&object_ref) {
                let stale_reason = stale_reason_from_decision(decision.reason);
                self.stale_omissions.push(StaleCandidateOmission {
                    candidate: memory_object_ref(object_ref.object_type, object_ref.object_id),
                    vector_score: self.candidate_scores.get(&object_ref).copied(),
                    reason: stale_reason,
                    rationale_categories: rationale_categories_for_stale_reason(stale_reason),
                });
            }
            self.lifecycle_decisions.push(decision);
        }

        ranked_objects.sort_by_key(|ranked| ranked.rank_key());
        let included_refs = ranked_objects
            .iter()
            .map(|ranked| graph_object_ref(&ranked.object))
            .collect::<HashSet<_>>();
        self.lifecycle_decisions.retain(|decision| {
            decision.action != LifecycleFilterAction::Omitted
                || !included_refs.contains(&GraphObjectRef::new(
                    decision.object.id,
                    decision.object.object_type,
                ))
        });
        self.stale_omissions.retain(|omission| {
            !included_refs.contains(&GraphObjectRef::new(
                omission.candidate.id,
                omission.candidate.object_type,
            ))
        });
        self.lifecycle_decisions.sort_by_key(|decision| {
            (
                decision.object.id,
                object_type_rank(decision.object.object_type),
                lifecycle_action_rank(decision.action),
                lifecycle_reason_rank(decision.reason),
            )
        });
        self.lifecycle_decisions.dedup_by_key(|decision| {
            (
                decision.object.object_type,
                decision.object.id,
                decision.action,
            )
        });
        self.stale_omissions.sort_by_key(|omission| {
            (
                omission.candidate.id,
                object_type_rank(omission.candidate.object_type),
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
    vector_component: f32,
    vector_candidate_score: Option<f32>,
    graph_component: f32,
    graph_rationale: GraphRationaleSignals,
    salience_component: f32,
    superseded_by: Vec<MemoryId>,
}

impl RankedObject {
    fn new(
        object: MemoryObject,
        vector_component: f32,
        graph_component: f32,
        vector_candidate_score: Option<f32>,
        graph_rationale: GraphRationaleSignals,
    ) -> Self {
        let salience_component = salience_component(&object);
        Self {
            object,
            vector_component,
            vector_candidate_score,
            graph_component,
            graph_rationale,
            salience_component,
            superseded_by: Vec::new(),
        }
    }

    fn final_score(&self) -> f32 {
        (self.vector_component * 0.65)
            + (self.graph_component * 0.25)
            + (self.salience_component * 0.10)
    }

    fn rank_key(&self) -> RankKey {
        let (object_id, object_type) = object_identity(&self.object);
        RankKey {
            score: SortableScore(self.final_score()),
            object_type_rank: object_type_rank(object_type),
            object_id,
        }
    }

    fn assignment_reason(&self) -> String {
        format!(
            "score={:.6}; vector={:.6}; graph={:.6}; salience={:.6}",
            self.final_score(),
            self.vector_component,
            self.graph_component,
            self.salience_component
        )
    }

    fn rationale_categories(&self) -> Vec<RationaleCategory> {
        let mut categories = Vec::new();
        if self.vector_candidate_score.is_some() {
            push_unique_category(&mut categories, RationaleCategory::Semantic);
        }
        if self.graph_rationale.entity {
            push_unique_category(&mut categories, RationaleCategory::Entity);
        }
        if self.graph_rationale.thread {
            push_unique_category(&mut categories, RationaleCategory::Thread);
        }
        if self.graph_rationale.graph_bound {
            push_unique_category(&mut categories, RationaleCategory::GraphBound);
        }
        if self.salience_component > 0.0 {
            push_unique_category(&mut categories, RationaleCategory::Salience);
        }
        categories
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct GraphRationaleSignals {
    entity: bool,
    thread: bool,
    graph_bound: bool,
}

impl GraphRationaleSignals {
    fn merge(&mut self, other: Self) {
        self.entity |= other.entity;
        self.thread |= other.thread;
        self.graph_bound |= other.graph_bound;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GraphPathSignals {
    entity: bool,
    thread: bool,
}

impl GraphPathSignals {
    fn root(candidate_ref: GraphObjectRef) -> Self {
        Self {
            entity: candidate_ref.object_type == ObjectType::Entity,
            thread: false,
        }
    }

    fn through(
        self,
        relation: RelationType,
        source: GraphObjectRef,
        target: GraphObjectRef,
    ) -> Self {
        let mut signals = self;
        match relation_rationale(relation, source, target) {
            RelationRationale::Thread => signals.thread = true,
            RelationRationale::Generic => {}
        }
        signals.entity |=
            source.object_type == ObjectType::Entity || target.object_type == ObjectType::Entity;
        signals
    }
}

#[derive(Debug, Clone, Copy)]
enum RelationRationale {
    Thread,
    Generic,
}

fn relation_rationale(
    relation: RelationType,
    source: GraphObjectRef,
    target: GraphObjectRef,
) -> RelationRationale {
    match relation {
        RelationType::PartOfThread
            if source.object_type == ObjectType::MemoryThread
                || target.object_type == ObjectType::MemoryThread =>
        {
            RelationRationale::Thread
        }
        RelationType::PartOfThread => RelationRationale::Generic,
        RelationType::HasObservation
        | RelationType::ObservedIn
        | RelationType::Mentions
        | RelationType::Involves
        | RelationType::About
        | RelationType::DerivedFrom
        | RelationType::Supports
        | RelationType::Contradicts
        | RelationType::Supersedes
        | RelationType::Resolves
        | RelationType::CreatesOpenLoop
        | RelationType::FulfillsCommitment
        | RelationType::AssociatedWith => RelationRationale::Generic,
    }
}

/// Rationale-category provenance semantics (the complete contract; tests derive from it):
///
/// - An object's admitting paths are the discovery paths from the vector-candidate root (`candidate_ref`) to the object as walked by graph expansion; categories are the union over all admitting paths, and each path contributes only signals actually ON that path — side branches off a path contribute nothing to its endpoint.
/// - `Semantic` is assigned elsewhere, iff the object is itself a vector-candidate root.
/// - `Entity` requires an `Entity`-typed node on an admitting path; relation names alone do not imply endpoint types.
/// - `Thread` requires `PartOfThread` with a `MemoryThread` endpoint on an admitting path because domain validation does not otherwise constrain relation endpoint types.
/// - `GraphBound` is the explicit fallback for graph admission whose relations map to no more specific category (see `relation_rationale`, which must stay exhaustive with no wildcard so new relation types force a conscious classification).
/// - `Temporal` is never produced by retrieval today (no temporal admission signal exists); a regression asserts this.
/// - The candidate root is excluded from its own expansion's graph provenance.
/// - Results are independent of same-depth relation iteration order: each BFS depth is built from the prior depth's snapshot, and same-depth path states union without mutating parent state.
/// - Across multiple candidates admitting the same object, signals OR-merge.
fn graph_provenance(
    candidate_ref: GraphObjectRef,
    relations: &[crate::ports::graph_authority::GraphExpansionRelation],
) -> HashMap<GraphObjectRef, GraphRationaleSignals> {
    let mut depth_by_ref = HashMap::from([(candidate_ref, 0_u8)]);
    let mut paths_by_ref = HashMap::from([(
        candidate_ref,
        HashSet::from([GraphPathSignals::root(candidate_ref)]),
    )]);
    let max_proximity = relations
        .iter()
        .map(|relation| relation.proximity)
        .max()
        .unwrap_or(0);

    // Build each BFS depth from the prior depth's snapshot so sibling branches
    // cannot leak signals into one another through relation iteration order.
    for proximity in 1..=max_proximity {
        let parent_depth = proximity - 1;
        let mut next_paths: HashMap<GraphObjectRef, HashSet<GraphPathSignals>> = HashMap::new();
        for relation in relations
            .iter()
            .filter(|relation| relation.proximity == proximity)
        {
            for (source, target) in [(relation.from, relation.to), (relation.to, relation.from)] {
                if depth_by_ref.get(&source) != Some(&parent_depth)
                    || depth_by_ref
                        .get(&target)
                        .is_some_and(|depth| *depth < proximity)
                {
                    continue;
                }
                let Some(source_paths) = paths_by_ref.get(&source) else {
                    continue;
                };
                next_paths.entry(target).or_default().extend(
                    source_paths
                        .iter()
                        .map(|signals| signals.through(relation.relation, source, target)),
                );
            }
        }

        for (object_ref, paths) in next_paths {
            depth_by_ref.entry(object_ref).or_insert(proximity);
            paths_by_ref.entry(object_ref).or_default().extend(paths);
        }
    }

    paths_by_ref
        .into_iter()
        .filter(|(object_ref, _)| *object_ref != candidate_ref)
        .map(|(object_ref, paths)| {
            let mut rationale = GraphRationaleSignals::default();
            for path in paths {
                rationale.entity |= path.entity;
                rationale.thread |= path.thread;
                rationale.graph_bound |= !path.entity && !path.thread;
            }
            (object_ref, rationale)
        })
        .collect()
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
                object: memory_object_ref_from_object(&ranked.object),
                section: ContextPackSection::Omitted,
                rank: None,
                reason: Some(section_omission_reason(&ranked.object)),
                rationale_categories: rationale_categories_for_section_omission(),
            });
            continue;
        };

        let count = section_counts.count_mut(section);
        if *count >= section_limit(section, limits) {
            increment_section_omitted_by_limit(section_pressure, section);
            details
                .stale_candidate_omissions
                .push(StaleCandidateOmission {
                    candidate: memory_object_ref_from_object(&ranked.object),
                    vector_score: ranked.vector_candidate_score,
                    reason: StaleCandidateReason::SectionLimit,
                    rationale_categories: rationale_categories_for_stale_reason(
                        StaleCandidateReason::SectionLimit,
                    ),
                });
            details.section_assignments.push(SectionAssignment {
                object: memory_object_ref_from_object(&ranked.object),
                section: ContextPackSection::Omitted,
                rank: None,
                reason: Some(format!(
                    "section limit reached for {}",
                    context_pack_section_name(section)
                )),
                rationale_categories: rationale_categories_for_section_limit(),
            });
            continue;
        }

        *count += 1;
        increment_section_included(section_pressure, section);
        let rank = *count;
        details.section_assignments.push(SectionAssignment {
            object: memory_object_ref_from_object(&ranked.object),
            section,
            rank: Some(rank),
            reason: Some(ranked.assignment_reason()),
            rationale_categories: ranked.rationale_categories(),
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

#[derive(Debug)]
struct CandidateRootSelection {
    roots: Vec<VectorCandidateMatch>,
    unique_count: usize,
    omitted_count: usize,
}

fn select_candidate_roots(
    candidates: &[VectorCandidateMatch],
    max_graph_roots: usize,
) -> CandidateRootSelection {
    let mut by_ref: HashMap<GraphObjectRef, VectorCandidateMatch> = HashMap::new();
    for candidate in candidates {
        let object_ref = GraphObjectRef::new(candidate.object_id, candidate.object_type);
        by_ref
            .entry(object_ref)
            .and_modify(|existing| {
                if candidate.score.total_cmp(&existing.score).is_gt() {
                    *existing = candidate.clone();
                }
            })
            .or_insert_with(|| candidate.clone());
    }

    let mut roots = by_ref.into_values().collect::<Vec<_>>();
    roots.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| {
                object_type_rank(left.object_type).cmp(&object_type_rank(right.object_type))
            })
            .then_with(|| left.object_id.cmp(&right.object_id))
    });
    let unique_count = roots.len();
    roots.truncate(max_graph_roots);
    CandidateRootSelection {
        omitted_count: unique_count.saturating_sub(roots.len()),
        roots,
        unique_count,
    }
}

fn graph_query_for_candidate(
    candidate: &VectorCandidateMatch,
    context: &RetrievalContext,
    fanout_overrides: Vec<crate::ports::graph_authority::GraphExpansionFanoutOverride>,
) -> GraphExpansionQuery {
    GraphExpansionQuery::new(
        candidate.object_id,
        candidate.object_type,
        context.graph_limits.max_depth,
        context.graph_limits.max_nodes,
    )
    .with_allowed_object_types(context.object_type_defaults.clone())
    .with_allowed_relation_types(context.graph_limits.allowed_relation_types.clone())
    .with_fanout_overrides(fanout_overrides)
    .with_max_fanout_per_node(context.graph_limits.max_fanout_per_node)
    .with_max_hub_edges(context.graph_limits.max_hub_edges)
    .with_lifecycle_policy(GraphExpansionLifecyclePolicy {
        include_archived: context.lifecycle_policy.include_archived,
        include_suppressed: context.lifecycle_policy.include_suppressed,
        include_deleted: context.lifecycle_policy.include_deleted,
        include_non_current: context.lifecycle_policy.include_non_current,
        include_superseded: context.lifecycle_policy.include_superseded,
    })
    .with_failure_policy(GraphExpansionFailurePolicy {
        timeout_ms: context.graph_limits.timeout_ms,
        allow_partial_results: context.graph_limits.allow_degraded_results,
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
    let location = failure
        .at
        .map(|object_ref| {
            format!(
                " at object_type={} object_id={}",
                object_type_name(object_ref.object_type),
                object_ref.object_id
            )
        })
        .unwrap_or_default();

    CustomError::GraphExpansionBounded {
        reason: bounded_failure_reason_name(failure.reason).to_owned(),
        location,
    }
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
    candidate: &VectorCandidateMatch,
    expansion: &GraphExpansion,
) -> GraphExpansionTrace {
    GraphExpansionTrace {
        root: memory_object_ref(candidate.object_type, candidate.object_id),
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
            root: memory_object_ref(entry.root.object_type, entry.root.object_id),
            relation: entry.relation,
            object_type: entry.object_type,
            configured_cap: entry.configured_cap,
            selected_cap: entry.selected_cap,
            retained_count: entry.retained_count,
            omitted_by_fanout_count: entry.omitted_by_fanout_count,
        })
        .collect()
}

fn missing_root_expansion_trace(candidate: &VectorCandidateMatch) -> GraphExpansionTrace {
    GraphExpansionTrace {
        root: memory_object_ref(candidate.object_type, candidate.object_id),
        object_count: 0,
        relation_count: 0,
        filtered_node_count: 0,
        bounded_failure: None,
        outcome: GraphExpansionOutcome::MissingRoot,
    }
}

fn graph_expansion_bounded_failure_trace(
    failure: GraphExpansionBoundedFailure,
) -> GraphExpansionBoundedFailureTrace {
    GraphExpansionBoundedFailureTrace {
        reason: public_bounded_failure_reason(failure.reason),
        at: failure
            .at
            .map(|object_ref| memory_object_ref(object_ref.object_type, object_ref.object_id)),
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

fn bounded_failure_reason_name(
    reason: crate::ports::graph_authority::GraphExpansionBoundedFailureReason,
) -> &'static str {
    match reason {
        crate::ports::graph_authority::GraphExpansionBoundedFailureReason::NodeLimit => {
            "node_limit"
        }
        crate::ports::graph_authority::GraphExpansionBoundedFailureReason::Timeout => "timeout",
        crate::ports::graph_authority::GraphExpansionBoundedFailureReason::HubLimit => "hub_limit",
    }
}

fn lifecycle_decision(
    object: &MemoryObject,
    superseded_by: &[MemoryId],
    policy: RetrievalLifecyclePolicy,
) -> LifecycleFilterDecision {
    let (object_id, object_type) = object_identity(object);
    let (retention_state, is_current) = object_lifecycle_fields(object);
    let reason = match object {
        MemoryObject::Episode(object) => retention_reason(object.retention_state, policy),
        MemoryObject::Observation(object) => retention_reason(object.retention_state, policy),
        MemoryObject::MemoryThread(object) => thread_reason(object, policy),
        MemoryObject::DerivedMemory(object) => retention_reason(object.retention_state, policy)
            .or_else(|| currentness_reason(object.is_current, policy))
            .or_else(|| supersession_reason(superseded_by, policy)),
        MemoryObject::Entity(_) | MemoryObject::MemoryLink(_) => None,
    };
    let action = if reason.is_some_and(omission_reason) {
        LifecycleFilterAction::Omitted
    } else {
        LifecycleFilterAction::Included
    };

    LifecycleFilterDecision {
        object: memory_object_ref(object_type, object_id),
        retention_state,
        is_current,
        superseded_by: superseded_by.to_vec(),
        action,
        reason: reason.unwrap_or(LifecycleFilterReason::Active),
    }
}

fn retention_reason(
    retention_state: RetentionState,
    policy: RetrievalLifecyclePolicy,
) -> Option<LifecycleFilterReason> {
    match retention_state {
        RetentionState::Active => None,
        RetentionState::Archived if policy.include_archived => {
            Some(LifecycleFilterReason::ArchivedIncludedByPolicy)
        }
        RetentionState::Archived => Some(LifecycleFilterReason::ArchivedOmitted),
        RetentionState::Suppressed if policy.include_suppressed => {
            Some(LifecycleFilterReason::SuppressedIncludedByPolicy)
        }
        RetentionState::Suppressed => Some(LifecycleFilterReason::SuppressedOmitted),
        RetentionState::Deleted if policy.include_deleted => {
            Some(LifecycleFilterReason::DeletedIncludedByPolicy)
        }
        RetentionState::Deleted => Some(LifecycleFilterReason::DeletedOmitted),
    }
}

fn thread_reason(
    thread: &MemoryThread,
    policy: RetrievalLifecyclePolicy,
) -> Option<LifecycleFilterReason> {
    if thread.status == ThreadStatus::Archived && !policy.include_archived {
        Some(LifecycleFilterReason::ArchivedOmitted)
    } else if thread.status == ThreadStatus::Archived {
        Some(LifecycleFilterReason::ArchivedIncludedByPolicy)
    } else {
        None
    }
}

fn currentness_reason(
    is_current: bool,
    policy: RetrievalLifecyclePolicy,
) -> Option<LifecycleFilterReason> {
    if !is_current && !policy.include_non_current {
        Some(LifecycleFilterReason::NonCurrentOmitted)
    } else if !is_current {
        Some(LifecycleFilterReason::NonCurrentIncludedByPolicy)
    } else {
        None
    }
}

fn supersession_reason(
    superseded_by: &[MemoryId],
    policy: RetrievalLifecyclePolicy,
) -> Option<LifecycleFilterReason> {
    if !superseded_by.is_empty() && !policy.include_superseded {
        Some(LifecycleFilterReason::SupersededOmitted)
    } else if !superseded_by.is_empty() {
        Some(LifecycleFilterReason::SupersededIncludedByPolicy)
    } else {
        None
    }
}

fn omission_reason(reason: LifecycleFilterReason) -> bool {
    matches!(
        reason,
        LifecycleFilterReason::ArchivedOmitted
            | LifecycleFilterReason::SuppressedOmitted
            | LifecycleFilterReason::DeletedOmitted
            | LifecycleFilterReason::NonCurrentOmitted
            | LifecycleFilterReason::SupersededOmitted
            | LifecycleFilterReason::GraphObjectMissing
            | LifecycleFilterReason::GraphExpansionBounded
    )
}

fn filtered_lifecycle_decision(
    object_ref: GraphObjectRef,
    reason: GraphExpansionFilteredReason,
    superseded_by: &[MemoryId],
) -> LifecycleFilterDecision {
    LifecycleFilterDecision {
        object: memory_object_ref(object_ref.object_type, object_ref.object_id),
        retention_state: None,
        is_current: None,
        superseded_by: superseded_by.to_vec(),
        action: LifecycleFilterAction::Omitted,
        reason: match reason {
            GraphExpansionFilteredReason::Archived => LifecycleFilterReason::ArchivedOmitted,
            GraphExpansionFilteredReason::Suppressed => LifecycleFilterReason::SuppressedOmitted,
            GraphExpansionFilteredReason::Deleted => LifecycleFilterReason::DeletedOmitted,
            GraphExpansionFilteredReason::NonCurrent => LifecycleFilterReason::NonCurrentOmitted,
            GraphExpansionFilteredReason::Superseded => LifecycleFilterReason::SupersededOmitted,
        },
    }
}

fn stale_reason_from_filtered(reason: GraphExpansionFilteredReason) -> StaleCandidateReason {
    match reason {
        GraphExpansionFilteredReason::Archived
        | GraphExpansionFilteredReason::Suppressed
        | GraphExpansionFilteredReason::Deleted => StaleCandidateReason::LifecycleMismatch,
        GraphExpansionFilteredReason::NonCurrent => StaleCandidateReason::CurrentnessMismatch,
        GraphExpansionFilteredReason::Superseded => StaleCandidateReason::Superseded,
    }
}

fn stale_reason_from_decision(reason: LifecycleFilterReason) -> StaleCandidateReason {
    match reason {
        LifecycleFilterReason::NonCurrentOmitted => StaleCandidateReason::CurrentnessMismatch,
        LifecycleFilterReason::SupersededOmitted => StaleCandidateReason::Superseded,
        LifecycleFilterReason::GraphObjectMissing => StaleCandidateReason::GraphObjectMissing,
        LifecycleFilterReason::GraphExpansionBounded => StaleCandidateReason::GraphExpansionBounded,
        _ => StaleCandidateReason::LifecycleMismatch,
    }
}

fn rationale_categories_for_stale_reason(reason: StaleCandidateReason) -> Vec<RationaleCategory> {
    match reason {
        StaleCandidateReason::GraphObjectMissing => vec![RationaleCategory::Semantic],
        StaleCandidateReason::LifecycleMismatch
        | StaleCandidateReason::CurrentnessMismatch
        | StaleCandidateReason::Superseded => vec![RationaleCategory::Lifecycle],
        StaleCandidateReason::SectionLimit => vec![RationaleCategory::Scope],
        StaleCandidateReason::GraphExpansionBounded => vec![RationaleCategory::GraphBound],
    }
}

fn rationale_categories_for_section_omission() -> Vec<RationaleCategory> {
    vec![RationaleCategory::Scope]
}

fn rationale_categories_for_section_limit() -> Vec<RationaleCategory> {
    vec![RationaleCategory::Scope]
}

fn push_unique_category(categories: &mut Vec<RationaleCategory>, category: RationaleCategory) {
    if !categories.contains(&category) {
        categories.push(category);
    }
}

fn object_lifecycle_fields(object: &MemoryObject) -> (Option<RetentionState>, Option<bool>) {
    match object {
        MemoryObject::Episode(object) => (Some(object.retention_state), None),
        MemoryObject::Observation(object) => (Some(object.retention_state), None),
        MemoryObject::DerivedMemory(object) => {
            (Some(object.retention_state), Some(object.is_current))
        }
        MemoryObject::Entity(_) | MemoryObject::MemoryThread(_) | MemoryObject::MemoryLink(_) => {
            (None, None)
        }
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

fn section_omission_reason(object: &MemoryObject) -> String {
    match object {
        MemoryObject::MemoryThread(thread) => format!(
            "memory_thread status {} is not included in active_threads",
            thread_status_name(thread.status)
        ),
        MemoryObject::Entity(_) => "entity has no prompt-ready context-pack section".to_owned(),
        MemoryObject::MemoryLink(_) => {
            "memory_link is graph-only and has no prompt-ready context-pack section".to_owned()
        }
        MemoryObject::Episode(_)
        | MemoryObject::Observation(_)
        | MemoryObject::DerivedMemory(_) => {
            "object has no prompt-ready context-pack section".to_owned()
        }
    }
}

fn thread_status_name(status: ThreadStatus) -> &'static str {
    match status {
        ThreadStatus::Active => "active",
        ThreadStatus::Dormant => "dormant",
        ThreadStatus::Resolved => "resolved",
        ThreadStatus::Archived => "archived",
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

fn memory_object_ref(object_type: ObjectType, object_id: MemoryId) -> MemoryObjectRef {
    MemoryObjectRef::new(object_type, object_id)
}

fn memory_object_ref_from_object(object: &MemoryObject) -> MemoryObjectRef {
    let (object_id, object_type) = object_identity(object);
    memory_object_ref(object_type, object_id)
}

fn graph_object_ref(object: &MemoryObject) -> GraphObjectRef {
    let (object_id, object_type) = object_identity(object);
    GraphObjectRef::new(object_id, object_type)
}

fn object_identity(object: &MemoryObject) -> (MemoryId, ObjectType) {
    match object {
        MemoryObject::Episode(object) => (object.id, object.object_type),
        MemoryObject::Observation(object) => (object.id, object.object_type),
        MemoryObject::Entity(object) => (object.id, object.object_type),
        MemoryObject::MemoryThread(object) => (object.id, object.object_type),
        MemoryObject::DerivedMemory(object) => (object.id, object.object_type),
        MemoryObject::MemoryLink(object) => (object.id, object.object_type),
    }
}

fn object_type_rank(object_type: ObjectType) -> u8 {
    match object_type {
        ObjectType::Episode => 0,
        ObjectType::Observation => 1,
        ObjectType::Entity => 2,
        ObjectType::MemoryThread => 3,
        ObjectType::DerivedMemory => 4,
        ObjectType::MemoryLink => 5,
    }
}

fn object_type_name(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Episode => "episode",
        ObjectType::Observation => "observation",
        ObjectType::Entity => "entity",
        ObjectType::MemoryThread => "memory_thread",
        ObjectType::DerivedMemory => "derived_memory",
        ObjectType::MemoryLink => "memory_link",
    }
}

fn context_pack_section_name(section: ContextPackSection) -> &'static str {
    match section {
        ContextPackSection::ActiveThreads => "active_threads",
        ContextPackSection::RelevantEpisodes => "relevant_episodes",
        ContextPackSection::SalientObservations => "salient_observations",
        ContextPackSection::DerivedMemories => "derived_memories",
        ContextPackSection::Preferences => "preferences",
        ContextPackSection::RelationshipNotes => "relationship_notes",
        ContextPackSection::OpenLoops => "open_loops",
        ContextPackSection::Commitments => "commitments",
        ContextPackSection::CharacterSignals => "character_signals",
        ContextPackSection::Omitted => "omitted",
    }
}

fn lifecycle_action_rank(action: LifecycleFilterAction) -> u8 {
    match action {
        LifecycleFilterAction::Included => 0,
        LifecycleFilterAction::Omitted => 1,
    }
}

fn lifecycle_reason_rank(reason: LifecycleFilterReason) -> u8 {
    match reason {
        LifecycleFilterReason::Active => 0,
        LifecycleFilterReason::ArchivedIncludedByPolicy => 1,
        LifecycleFilterReason::SuppressedIncludedByPolicy => 2,
        LifecycleFilterReason::DeletedIncludedByPolicy => 3,
        LifecycleFilterReason::NonCurrentIncludedByPolicy => 4,
        LifecycleFilterReason::SupersededIncludedByPolicy => 5,
        LifecycleFilterReason::ArchivedOmitted => 6,
        LifecycleFilterReason::SuppressedOmitted => 7,
        LifecycleFilterReason::DeletedOmitted => 8,
        LifecycleFilterReason::NonCurrentOmitted => 9,
        LifecycleFilterReason::SupersededOmitted => 10,
        LifecycleFilterReason::GraphObjectMissing => 11,
        LifecycleFilterReason::GraphExpansionBounded => 12,
    }
}

fn stale_reason_rank(reason: StaleCandidateReason) -> u8 {
    match reason {
        StaleCandidateReason::GraphObjectMissing => 0,
        StaleCandidateReason::LifecycleMismatch => 1,
        StaleCandidateReason::CurrentnessMismatch => 2,
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
        "Embedded the retrieval query, evaluated {vector_candidate_count} vector candidates, included {graph_verified_count} final context-pack objects, omitted {stale_candidate_omission_count} stale or unresolved candidates, and recorded {lifecycle_omission_count} lifecycle omission decisions with deterministic vector, graph proximity, and salience scoring."
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
    use crate::api::types::{ContinuitySectionLimits, RetrievalCandidateLimits};
    use crate::models::vector::{
        VectorCandidateRecord, VectorPayloadHints, VectorRelationshipHints,
    };
    use crate::policy::RetrievalSelectivityPolicy;
    use crate::ports::retrieval_stats::RetrievalStatsEdge;
    use crate::test_support::{
        high_fanout_graph_fixture, representative_fixtures, FakeGraphAuthorityStore,
        FakeVectorCandidateStore,
    };

    #[tokio::test]
    async fn vector_to_graph_flow_groups_sections_and_records_trace() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = RecordingVectorStore::new(vec![
            candidate(fixtures.episode.id, ObjectType::Episode, 0.92),
            candidate(
                fixtures.salient_observation.id,
                ObjectType::Observation,
                0.91,
            ),
            candidate(fixtures.soft_thread.id, ObjectType::MemoryThread, 0.90),
            candidate(fixtures.user_preference.id, ObjectType::DerivedMemory, 0.89),
        ]);
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("deterministic store contracts").with_trace())
            .await
            .unwrap();

        assert_eq!(embedder.inputs()[0].surface, VectorSurface::Query);
        assert_eq!(vector.searches()[0].filters, VectorCandidateFilters::new());
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
        assert!(trace
            .section_assignments
            .iter()
            .all(|assignment| assignment.reason.is_some()));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.episode.id
                && assignment
                    .rationale_categories
                    .contains(&RationaleCategory::Semantic)
                && !assignment
                    .rationale_categories
                    .contains(&RationaleCategory::Temporal)
        }));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.user_preference.id
                && assignment
                    .rationale_categories
                    .contains(&RationaleCategory::Semantic)
                && !assignment
                    .rationale_categories
                    .contains(&RationaleCategory::Scope)
        }));
        assert_eq!(trace.vector_candidates.len(), 4);
        assert_eq!(trace.graph_expansions.len(), 4);
    }

    #[tokio::test]
    async fn trace_collection_does_not_change_retrieval_results() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = RecordingVectorStore::new(vec![
            candidate(fixtures.hub_entity.id, ObjectType::Entity, 0.93),
            candidate(fixtures.episode.id, ObjectType::Episode, 0.92),
            candidate(fixtures.user_preference.id, ObjectType::DerivedMemory, 0.91),
        ]);
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
        let vector = RecordingVectorStore::new(vec![candidate(
            fixture.hub_entity.id,
            ObjectType::Entity,
            0.99,
        )]);
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
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
        let pipeline = RetrievePipeline::new_with_stats(
            &graph,
            &vector,
            &embedder,
            &stats,
            RetrievalSelectivityPolicy::default(),
        );

        let outcome = pipeline
            .retrieve(RetrievalContext::new("broad hub").with_trace())
            .await
            .unwrap();

        assert!(outcome.pack.derived_memories.is_empty());
        assert!(outcome
            .trace
            .as_ref()
            .unwrap()
            .selectivity_decisions
            .iter()
            .any(|decision| {
                decision.relation == RelationType::About
                    && decision.object_type == ObjectType::DerivedMemory
                    && decision.chosen_fanout == 0
                    && decision.decision
                        == crate::api::types::SelectivityDecision::LowSelectivityRejected
            }));
        let fanout_utilization = &outcome.trace.as_ref().unwrap().fanout_utilization;
        assert!(fanout_utilization.iter().any(|entry| {
            entry.root.id == fixture.hub_entity.id
                && entry.relation == RelationType::About
                && entry.object_type == ObjectType::DerivedMemory
                && entry.selected_cap == 0
                && entry.retained_count == 0
                && entry.omitted_by_fanout_count > 0
        }));
    }

    #[tokio::test]
    async fn selectivity_allows_high_selectivity_entity_about_expansion() {
        let fixture = high_fanout_graph_fixture();
        let graph = graph_with(&fixture.objects(), &fixture.links).await;
        let vector = RecordingVectorStore::new(vec![candidate(
            fixture.hub_entity.id,
            ObjectType::Entity,
            0.99,
        )]);
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let stats = InMemoryRetrievalStatsStore::new();
        record_about_edges(
            &stats,
            fixture.hub_entity.id,
            &[fixture.derived_memories[0].id],
        )
        .await;
        record_other_about_edges(&stats, 80).await;
        let pipeline = RetrievePipeline::new_with_stats(
            &graph,
            &vector,
            &embedder,
            &stats,
            RetrievalSelectivityPolicy::default(),
        );

        let outcome = pipeline
            .retrieve(RetrievalContext::new("specific hub").with_trace())
            .await
            .unwrap();

        assert!(!outcome.pack.derived_memories.is_empty());
        assert!(
            outcome
                .rationale
                .telemetry
                .selectivity
                .high_selectivity_count
                > 0
        );
        let trace = outcome.trace.as_ref().unwrap();
        assert!(trace.fanout_utilization.iter().any(|entry| {
            entry.root.id == fixture.hub_entity.id
                && entry.relation == RelationType::About
                && entry.object_type == ObjectType::DerivedMemory
                && entry.selected_cap <= entry.configured_cap
                && entry.retained_count > 0
        }));
    }

    #[tokio::test]
    async fn graph_relation_trace_collection_is_trace_gated() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let query = GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 2, 10);
        let expansion = graph.expand_bounded(&query).await.unwrap();
        assert!(!expansion.relations.is_empty());

        let candidate = candidate(fixtures.hub_entity.id, ObjectType::Entity, 0.95);
        let mut without_trace = RetrieveAssembly::new(false);
        without_trace.absorb_expansion(&candidate, expansion.clone());
        assert!(without_trace.graph_relations.is_none());

        let mut with_trace = RetrieveAssembly::new(true);
        with_trace.absorb_expansion(&candidate, expansion);
        assert!(!with_trace.graph_relations.unwrap().is_empty());
    }

    #[test]
    fn rationale_categories_follow_vector_and_graph_provenance() {
        let fixtures = representative_fixtures();
        let mut preference = fixtures.user_preference.clone();
        preference.salience_score = 0.0;
        let preference_ref = GraphObjectRef::new(preference.id, ObjectType::DerivedMemory);
        let preference_candidate = candidate(preference.id, ObjectType::DerivedMemory, 0.95);
        let hub_candidate = candidate(fixtures.hub_entity.id, ObjectType::Entity, 0.90);
        let episode_candidate = candidate(fixtures.episode.id, ObjectType::Episode, 0.89);
        let entity_linked_expansion = || {
            let mut expansion = GraphExpansion::new(
                vec![
                    MemoryObject::Entity(fixtures.hub_entity.clone()),
                    MemoryObject::DerivedMemory(preference.clone()),
                ],
                Vec::new(),
            );
            expansion
                .relations
                .push(crate::ports::graph_authority::GraphExpansionRelation {
                    link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0210),
                    from: GraphObjectRef::new(fixtures.hub_entity.id, ObjectType::Entity),
                    to: preference_ref,
                    relation: RelationType::About,
                    proximity: 1,
                });
            expansion
        };
        let non_entity_linked_expansion = || {
            let mut expansion = GraphExpansion::new(
                vec![
                    MemoryObject::Episode(fixtures.episode.clone()),
                    MemoryObject::DerivedMemory(preference.clone()),
                ],
                Vec::new(),
            );
            expansion
                .relations
                .push(crate::ports::graph_authority::GraphExpansionRelation {
                    link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0211),
                    from: preference_ref,
                    to: GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode),
                    relation: RelationType::DerivedFrom,
                    proximity: 1,
                });
            expansion
        };

        let mut pure_vector = RetrieveAssembly::new(true);
        pure_vector.absorb_expansion(&preference_candidate, non_entity_linked_expansion());
        let pure_vector_ranked = pure_vector
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == preference_ref)
            .unwrap();
        let pure_vector_categories = pure_vector_ranked.rationale_categories();
        assert_eq!(pure_vector_categories, vec![RationaleCategory::Semantic]);

        let mut episode = fixtures.episode.clone();
        episode.salience_score = 0.0;
        let episode_ref = GraphObjectRef::new(episode.id, ObjectType::Episode);
        let mut semantic_episode = RetrieveAssembly::new(true);
        semantic_episode.absorb_expansion(
            &candidate(episode.id, ObjectType::Episode, 0.94),
            GraphExpansion::new(vec![MemoryObject::Episode(episode)], Vec::new()),
        );
        let episode_categories = semantic_episode
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == episode_ref)
            .unwrap()
            .rationale_categories();
        assert_eq!(episode_categories, vec![RationaleCategory::Semantic]);
        assert!(!episode_categories.contains(&RationaleCategory::Temporal));

        let mut thread = fixtures.soft_thread.clone();
        thread.salience_score = 0.0;
        let thread_ref = GraphObjectRef::new(thread.id, ObjectType::MemoryThread);
        let mut semantic_thread = RetrieveAssembly::new(true);
        semantic_thread.absorb_expansion(
            &candidate(thread.id, ObjectType::MemoryThread, 0.93),
            GraphExpansion::new(vec![MemoryObject::MemoryThread(thread)], Vec::new()),
        );
        let thread_categories = semantic_thread
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == thread_ref)
            .unwrap()
            .rationale_categories();
        assert_eq!(thread_categories, vec![RationaleCategory::Semantic]);
        assert!(!thread_categories.contains(&RationaleCategory::Thread));

        let mut non_entity_graph_expanded = RetrieveAssembly::new(true);
        non_entity_graph_expanded
            .absorb_expansion(&episode_candidate, non_entity_linked_expansion());
        let non_entity_graph_expanded_ranked = non_entity_graph_expanded
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == preference_ref)
            .unwrap();
        let non_entity_graph_categories = non_entity_graph_expanded_ranked.rationale_categories();
        assert!(!non_entity_graph_categories.contains(&RationaleCategory::Semantic));
        assert!(!non_entity_graph_categories.contains(&RationaleCategory::Entity));
        assert!(non_entity_graph_categories.contains(&RationaleCategory::GraphBound));

        let mut entity_graph_expanded = RetrieveAssembly::new(true);
        entity_graph_expanded.absorb_expansion(&hub_candidate, entity_linked_expansion());
        let entity_graph_expanded_ranked = entity_graph_expanded
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == preference_ref)
            .unwrap();
        let entity_graph_categories = entity_graph_expanded_ranked.rationale_categories();
        assert!(!entity_graph_categories.contains(&RationaleCategory::Semantic));
        assert!(entity_graph_categories.contains(&RationaleCategory::Entity));
        assert!(!entity_graph_categories.contains(&RationaleCategory::GraphBound));

        let mut both = RetrieveAssembly::new(true);
        both.absorb_expansion(&hub_candidate, entity_linked_expansion());
        both.absorb_expansion(
            &preference_candidate,
            GraphExpansion::new(vec![MemoryObject::DerivedMemory(preference)], Vec::new()),
        );
        let both_ranked = both
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == preference_ref)
            .unwrap();
        let both_categories = both_ranked.rationale_categories();
        assert!(both_categories.contains(&RationaleCategory::Semantic));
        assert!(both_categories.contains(&RationaleCategory::Entity));
    }

    #[test]
    fn entity_side_branch_does_not_affect_target_and_is_order_independent() {
        let fixtures = representative_fixtures();
        let root_ref = GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode);
        let bridge_ref =
            GraphObjectRef::new(fixtures.derived_reflection.id, ObjectType::DerivedMemory);
        let entity_ref = GraphObjectRef::new(fixtures.hub_entity.id, ObjectType::Entity);
        let target_ref =
            GraphObjectRef::new(fixtures.user_preference.id, ObjectType::DerivedMemory);
        let categories_with_ids = |entity_link_id: u128, target_link_id: u128| {
            let mut expansion = GraphExpansion::new(
                vec![
                    MemoryObject::Episode(fixtures.episode.clone()),
                    MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
                    MemoryObject::Entity(fixtures.hub_entity.clone()),
                    MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
                ],
                Vec::new(),
            );
            expansion.relations = vec![
                crate::ports::graph_authority::GraphExpansionRelation {
                    link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0220),
                    from: root_ref,
                    to: bridge_ref,
                    relation: RelationType::DerivedFrom,
                    proximity: 1,
                },
                crate::ports::graph_authority::GraphExpansionRelation {
                    link_id: Uuid::from_u128(entity_link_id),
                    from: bridge_ref,
                    to: entity_ref,
                    relation: RelationType::About,
                    proximity: 2,
                },
                crate::ports::graph_authority::GraphExpansionRelation {
                    link_id: Uuid::from_u128(target_link_id),
                    from: bridge_ref,
                    to: target_ref,
                    relation: RelationType::DerivedFrom,
                    proximity: 2,
                },
            ];
            expansion
                .relations
                .sort_by_key(|relation| (relation.proximity, relation.link_id));

            let mut assembly = RetrieveAssembly::new(true);
            assembly.absorb_expansion(
                &candidate(fixtures.episode.id, ObjectType::Episode, 0.90),
                expansion,
            );
            assembly
                .ranked_objects(&RetrievalLifecyclePolicy::default())
                .into_iter()
                .find(|ranked| graph_object_ref(&ranked.object) == target_ref)
                .unwrap()
                .rationale_categories()
        };

        let entity_first = categories_with_ids(
            0x550e_8400_e29b_41d4_a716_4466_5544_0221,
            0x550e_8400_e29b_41d4_a716_4466_5544_0222,
        );
        let target_first = categories_with_ids(
            0x550e_8400_e29b_41d4_a716_4466_5544_0222,
            0x550e_8400_e29b_41d4_a716_4466_5544_0221,
        );

        assert_eq!(entity_first, target_first);
        assert!(!entity_first.contains(&RationaleCategory::Entity));
        assert!(entity_first.contains(&RationaleCategory::GraphBound));
    }

    #[test]
    fn mentions_path_uses_entity_nodes_not_relation_name_for_entity_rationale() {
        let fixtures = representative_fixtures();
        let target_ref =
            GraphObjectRef::new(fixtures.salient_observation.id, ObjectType::Observation);
        let categories_for_root = |root_ref: GraphObjectRef, root: MemoryObject| {
            let mut expansion = GraphExpansion::new(
                vec![
                    root,
                    MemoryObject::Observation(fixtures.salient_observation.clone()),
                ],
                Vec::new(),
            );
            expansion
                .relations
                .push(crate::ports::graph_authority::GraphExpansionRelation {
                    link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0223),
                    from: root_ref,
                    to: target_ref,
                    relation: RelationType::Mentions,
                    proximity: 1,
                });
            let mut assembly = RetrieveAssembly::new(true);
            assembly.absorb_expansion(
                &candidate(root_ref.object_id, root_ref.object_type, 0.90),
                expansion,
            );
            assembly
                .ranked_objects(&RetrievalLifecyclePolicy::default())
                .into_iter()
                .find(|ranked| graph_object_ref(&ranked.object) == target_ref)
                .unwrap()
                .rationale_categories()
        };

        let entityless = categories_for_root(
            GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode),
            MemoryObject::Episode(fixtures.episode.clone()),
        );
        let entity_backed = categories_for_root(
            GraphObjectRef::new(fixtures.hub_entity.id, ObjectType::Entity),
            MemoryObject::Entity(fixtures.hub_entity.clone()),
        );

        assert!(!entityless.contains(&RationaleCategory::Entity));
        assert!(entityless.contains(&RationaleCategory::GraphBound));
        assert!(entity_backed.contains(&RationaleCategory::Entity));
        assert!(!entity_backed.contains(&RationaleCategory::GraphBound));
    }

    #[test]
    fn part_of_thread_path_emits_thread_without_graph_bound() {
        let fixtures = representative_fixtures();
        let root_ref = GraphObjectRef::new(fixtures.soft_thread.id, ObjectType::MemoryThread);
        let target_ref =
            GraphObjectRef::new(fixtures.user_preference.id, ObjectType::DerivedMemory);
        let mut expansion = GraphExpansion::new(
            vec![
                MemoryObject::MemoryThread(fixtures.soft_thread.clone()),
                MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            ],
            Vec::new(),
        );
        expansion
            .relations
            .push(crate::ports::graph_authority::GraphExpansionRelation {
                link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0230),
                from: target_ref,
                to: root_ref,
                relation: RelationType::PartOfThread,
                proximity: 1,
            });

        let mut assembly = RetrieveAssembly::new(true);
        assembly.absorb_expansion(
            &candidate(fixtures.soft_thread.id, ObjectType::MemoryThread, 0.90),
            expansion,
        );
        let categories = assembly
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == target_ref)
            .unwrap()
            .rationale_categories();

        assert!(categories.contains(&RationaleCategory::Thread));
        assert!(!categories.contains(&RationaleCategory::GraphBound));
        assert!(!categories.contains(&RationaleCategory::Entity));
    }

    #[test]
    fn part_of_thread_without_thread_endpoint_falls_back_to_graph_bound() {
        let fixtures = representative_fixtures();
        let root_ref = GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode);
        let target_ref =
            GraphObjectRef::new(fixtures.salient_observation.id, ObjectType::Observation);
        let mut expansion = GraphExpansion::new(
            vec![
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Observation(fixtures.salient_observation.clone()),
            ],
            Vec::new(),
        );
        expansion
            .relations
            .push(crate::ports::graph_authority::GraphExpansionRelation {
                link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0233),
                from: root_ref,
                to: target_ref,
                relation: RelationType::PartOfThread,
                proximity: 1,
            });

        let mut assembly = RetrieveAssembly::new(true);
        assembly.absorb_expansion(
            &candidate(fixtures.episode.id, ObjectType::Episode, 0.90),
            expansion,
        );
        let categories = assembly
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == target_ref)
            .unwrap()
            .rationale_categories();

        assert!(!categories.contains(&RationaleCategory::Thread));
        assert!(categories.contains(&RationaleCategory::GraphBound));
        assert!(!categories.contains(&RationaleCategory::Entity));
    }

    #[test]
    fn graph_categories_union_across_distinct_candidate_paths() {
        let fixtures = representative_fixtures();
        let target_ref =
            GraphObjectRef::new(fixtures.user_preference.id, ObjectType::DerivedMemory);
        let mut generic_expansion = GraphExpansion::new(
            vec![
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            ],
            Vec::new(),
        );
        generic_expansion
            .relations
            .push(crate::ports::graph_authority::GraphExpansionRelation {
                link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0231),
                from: GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode),
                to: target_ref,
                relation: RelationType::DerivedFrom,
                proximity: 1,
            });
        let mut thread_expansion = GraphExpansion::new(
            vec![
                MemoryObject::MemoryThread(fixtures.soft_thread.clone()),
                MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            ],
            Vec::new(),
        );
        thread_expansion
            .relations
            .push(crate::ports::graph_authority::GraphExpansionRelation {
                link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0232),
                from: target_ref,
                to: GraphObjectRef::new(fixtures.soft_thread.id, ObjectType::MemoryThread),
                relation: RelationType::PartOfThread,
                proximity: 1,
            });

        let mut assembly = RetrieveAssembly::new(true);
        assembly.absorb_expansion(
            &candidate(fixtures.episode.id, ObjectType::Episode, 0.90),
            generic_expansion,
        );
        assembly.absorb_expansion(
            &candidate(fixtures.soft_thread.id, ObjectType::MemoryThread, 0.89),
            thread_expansion,
        );
        let categories = assembly
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == target_ref)
            .unwrap()
            .rationale_categories();

        assert!(categories.contains(&RationaleCategory::Thread));
        assert!(categories.contains(&RationaleCategory::GraphBound));
        assert!(!categories.contains(&RationaleCategory::Entity));
    }

    #[test]
    fn entity_on_admitting_path_emits_entity_for_target() {
        let fixtures = representative_fixtures();
        let root_ref = GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode);
        let entity_ref = GraphObjectRef::new(fixtures.hub_entity.id, ObjectType::Entity);
        let target_ref =
            GraphObjectRef::new(fixtures.user_preference.id, ObjectType::DerivedMemory);
        let mut expansion = GraphExpansion::new(
            vec![
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Entity(fixtures.hub_entity.clone()),
                MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            ],
            Vec::new(),
        );
        expansion.relations = vec![
            crate::ports::graph_authority::GraphExpansionRelation {
                link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0240),
                from: root_ref,
                to: entity_ref,
                relation: RelationType::AssociatedWith,
                proximity: 1,
            },
            crate::ports::graph_authority::GraphExpansionRelation {
                link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0241),
                from: entity_ref,
                to: target_ref,
                relation: RelationType::DerivedFrom,
                proximity: 2,
            },
        ];

        let mut assembly = RetrieveAssembly::new(true);
        assembly.absorb_expansion(
            &candidate(fixtures.episode.id, ObjectType::Episode, 0.90),
            expansion,
        );
        let categories = assembly
            .ranked_objects(&RetrievalLifecyclePolicy::default())
            .into_iter()
            .find(|ranked| graph_object_ref(&ranked.object) == target_ref)
            .unwrap()
            .rationale_categories();

        assert!(categories.contains(&RationaleCategory::Entity));
        assert!(!categories.contains(&RationaleCategory::GraphBound));
    }

    #[test]
    fn temporal_rationale_is_not_emitted_by_current_retrieval_signals() {
        let fixtures = representative_fixtures();
        let ranked = RankedObject::new(
            MemoryObject::DerivedMemory(fixtures.user_preference),
            0.9,
            0.8,
            Some(0.9),
            GraphRationaleSignals {
                entity: true,
                thread: true,
                graph_bound: true,
            },
        );
        let mut category_sets = vec![
            ranked.rationale_categories(),
            rationale_categories_for_section_omission(),
            rationale_categories_for_section_limit(),
        ];
        category_sets.extend(
            [
                StaleCandidateReason::GraphObjectMissing,
                StaleCandidateReason::LifecycleMismatch,
                StaleCandidateReason::CurrentnessMismatch,
                StaleCandidateReason::Superseded,
                StaleCandidateReason::SectionLimit,
                StaleCandidateReason::GraphExpansionBounded,
            ]
            .map(rationale_categories_for_stale_reason),
        );

        assert!(category_sets
            .iter()
            .all(|categories| !categories.contains(&RationaleCategory::Temporal)));
    }

    #[test]
    fn salience_rationale_tracks_positive_salience_component() {
        let fixtures = representative_fixtures();
        let mut positive_salience = fixtures.user_preference.clone();
        positive_salience.salience_score = 0.8;
        let mut zero_salience = positive_salience.clone();
        zero_salience.salience_score = 0.0;

        for (object, expected) in [(positive_salience, true), (zero_salience, false)] {
            let categories = RankedObject::new(
                MemoryObject::DerivedMemory(object),
                0.0,
                0.0,
                None,
                GraphRationaleSignals::default(),
            )
            .rationale_categories();

            assert_eq!(categories.contains(&RationaleCategory::Salience), expected);
        }
    }

    #[tokio::test]
    async fn omits_unresolved_and_lifecycle_stale_candidates() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let missing_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9999);
        let vector = RecordingVectorStore::new(vec![
            candidate(missing_id, ObjectType::DerivedMemory, 0.95),
            candidate(fixtures.suppressed_seed.id, ObjectType::DerivedMemory, 0.94),
        ]);
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
                && omission.reason == StaleCandidateReason::GraphObjectMissing
                && omission
                    .rationale_categories
                    .contains(&RationaleCategory::Semantic)));
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
                    && omission
                        .rationale_categories
                        .contains(&RationaleCategory::Lifecycle)
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
        let vector = RecordingVectorStore::new(vec![
            candidate(missing_id, ObjectType::DerivedMemory, 0.95),
            candidate(fixtures.suppressed_seed.id, ObjectType::DerivedMemory, 0.94),
        ]);
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
        assert!(outcome
            .rationale
            .summary
            .contains("final context-pack objects"));
        assert!(outcome.rationale.summary.contains("omitted 2"));
    }

    #[test]
    fn included_objects_prune_prior_stale_omission_trace_details() {
        let fixtures = representative_fixtures();
        let candidate = candidate(fixtures.user_preference.id, ObjectType::DerivedMemory, 0.99);
        let mut assembly = RetrieveAssembly::new(true);
        assembly.omit_bounded_candidate(&candidate);
        assembly.absorb_expansion(
            &candidate,
            GraphExpansion::new(
                vec![MemoryObject::DerivedMemory(
                    fixtures.user_preference.clone(),
                )],
                Vec::new(),
            ),
        );

        let ranked = assembly.ranked_objects(&RetrievalLifecyclePolicy::default());

        assert_eq!(ranked.len(), 1);
        assert_eq!(
            object_identity(&ranked[0].object).0,
            fixtures.user_preference.id
        );
        assert!(!assembly
            .stale_omissions
            .iter()
            .any(|omission| omission.candidate.id == fixtures.user_preference.id));
        assert!(!assembly.lifecycle_decisions.iter().any(|decision| {
            decision.object.id == fixtures.user_preference.id
                && decision.action == LifecycleFilterAction::Omitted
        }));
    }

    #[test]
    fn filtered_superseded_decision_uses_relation_evidence_without_links() {
        let fixtures = representative_fixtures();
        let candidate = candidate(fixtures.suppressed_seed.id, ObjectType::DerivedMemory, 0.99);
        let mut expansion = GraphExpansion::new(Vec::new(), Vec::new());
        expansion
            .relations
            .push(crate::ports::graph_authority::GraphExpansionRelation {
                link_id: Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0120),
                from: GraphObjectRef::new(fixtures.correction.id, ObjectType::DerivedMemory),
                to: GraphObjectRef::new(fixtures.suppressed_seed.id, ObjectType::DerivedMemory),
                relation: RelationType::Supersedes,
                proximity: 1,
            });
        expansion
            .filtered_nodes
            .push(crate::ports::graph_authority::GraphExpansionFilteredNode {
                object_ref: GraphObjectRef::new(
                    fixtures.suppressed_seed.id,
                    ObjectType::DerivedMemory,
                ),
                reason: GraphExpansionFilteredReason::Superseded,
            });
        let mut assembly = RetrieveAssembly::new(true);

        assembly.absorb_expansion(&candidate, expansion);

        assert!(assembly.lifecycle_decisions.iter().any(|decision| {
            decision.object.id == fixtures.suppressed_seed.id
                && decision.reason == LifecycleFilterReason::SupersededOmitted
                && decision.superseded_by == vec![fixtures.correction.id]
        }));
        assert!(assembly.stale_omissions.iter().any(|omission| {
            omission.candidate.id == fixtures.suppressed_seed.id
                && omission.reason == StaleCandidateReason::Superseded
        }));
    }

    #[tokio::test]
    async fn bounded_expansion_failure_errors_when_degraded_results_are_disabled() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = RecordingVectorStore::new(vec![candidate(
            fixtures.user_preference.id,
            ObjectType::DerivedMemory,
            0.99,
        )]);
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("fail closed");
        context.graph_limits.timeout_ms = Some(0);
        context.graph_limits.allow_degraded_results = false;

        let error = pipeline.retrieve(context).await.unwrap_err();

        assert!(matches!(
            error,
            CustomError::GraphExpansionBounded { reason, location }
                if reason == "timeout"
                    && location.contains("object_type=derived_memory")
                    && location.contains(&fixtures.user_preference.id.to_string())
        ));
    }

    #[test]
    fn graph_query_failure_policy_follows_degraded_results_setting() {
        let object_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0110);
        let candidate = candidate(object_id, ObjectType::DerivedMemory, 0.99);
        let mut context = RetrievalContext::new("fail closed query");
        context.graph_limits.allow_degraded_results = false;

        let query = graph_query_for_candidate(&candidate, &context, Vec::new());

        assert!(!query.failure_policy.allow_partial_results);

        context.graph_limits.allow_degraded_results = true;
        let query = graph_query_for_candidate(&candidate, &context, Vec::new());

        assert!(query.failure_policy.allow_partial_results);
    }

    #[tokio::test]
    async fn bounded_empty_expansion_omits_without_reporting_graph_missing() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = RecordingVectorStore::new(vec![candidate(
            fixtures.user_preference.id,
            ObjectType::DerivedMemory,
            0.99,
        )]);
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("bounded graph limits");
        context.graph_limits.max_nodes = 0;
        context.graph_limits.allow_degraded_results = true;
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
        let graph = ErrorGraphStore::new("graph expansion rejected unsupported root");
        let vector =
            RecordingVectorStore::new(vec![candidate(object_id, ObjectType::MemoryLink, 0.99)]);
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .retrieve(RetrievalContext::new("propagate graph errors"))
            .await
            .unwrap_err();

        assert!(
            matches!(error, CustomError::MemoryValidation(message) if message.contains("unsupported root"))
        );
    }

    #[test]
    fn sortable_score_equality_uses_total_float_ordering() {
        let left = SortableScore(f32::NAN);
        let right = SortableScore(f32::NAN);

        assert_eq!(left, right);
        assert_eq!(left.cmp(&right), std::cmp::Ordering::Equal);
    }

    #[tokio::test]
    async fn graph_authority_overrides_stale_vector_payload_hints() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = FakeVectorCandidateStore::new();
        vector
            .upsert_candidates(&[VectorCandidateRecord::new(
                fixtures.user_preference.id,
                ObjectType::DerivedMemory,
                VectorSurface::DerivedText,
                vec![1.0, 0.0],
            )
            .with_filter_hints(
                Some(RetentionState::Suppressed),
                Some(false),
                VectorRelationshipHints::default(),
                VectorPayloadHints {
                    is_superseded: Some(true),
                    ..VectorPayloadHints::default()
                },
            )])
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
        let vector = RecordingVectorStore::new(vec![
            candidate(fixtures.episode.id, ObjectType::Episode, 0.92),
            candidate(
                fixtures.salient_observation.id,
                ObjectType::Observation,
                0.91,
            ),
        ]);
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("root truncation");
        context.candidate_limits.max_graph_roots = 1;

        let outcome = pipeline.retrieve(context).await.unwrap();
        let telemetry = &outcome.rationale.telemetry;

        assert_eq!(RetrievalCandidateLimits::default().max_graph_roots, 12);
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
        let first_vector = RecordingVectorStore::new(vec![
            candidate(second_preference.id, ObjectType::DerivedMemory, 0.90),
            candidate(fixtures.user_preference.id, ObjectType::DerivedMemory, 0.90),
        ]);
        let second_vector = RecordingVectorStore::new(vec![
            candidate(fixtures.user_preference.id, ObjectType::DerivedMemory, 0.90),
            candidate(second_preference.id, ObjectType::DerivedMemory, 0.90),
        ]);
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
                && omission.vector_score == Some(0.90)
                && omission.reason == StaleCandidateReason::SectionLimit
                && omission.rationale_categories == vec![RationaleCategory::Scope]));
        assert!(trace
            .section_assignments
            .iter()
            .any(|assignment| assignment.object.id == second_preference.id
                && assignment.section == ContextPackSection::Omitted
                && assignment.reason.as_deref() == Some("section limit reached for preferences")
                && assignment.rationale_categories == vec![RationaleCategory::Scope]));
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
        let vector = RecordingVectorStore::new(vec![candidate(
            fixtures.hub_entity.id,
            ObjectType::Entity,
            0.99,
        )]);
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
                && assignment.reason.as_deref()
                    == Some("section limit reached for relevant_episodes")));
    }

    #[tokio::test]
    async fn graph_verified_count_tracks_final_pack_inclusions() {
        let fixtures = representative_fixtures();
        let graph = graph_with(&fixtures.objects(), &fixtures.links()).await;
        let vector = RecordingVectorStore::new(vec![candidate(
            fixtures.hub_entity.id,
            ObjectType::Entity,
            0.99,
        )]);
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
        let vector = RecordingVectorStore::new(vec![candidate(
            relationship_note.id,
            ObjectType::DerivedMemory,
            0.99,
        )]);
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
        let mut archived_thread = fixtures.soft_thread.clone();
        archived_thread.status = ThreadStatus::Archived;
        let mut objects = fixtures.objects();
        objects.retain(|object| match object {
            MemoryObject::MemoryThread(thread) => thread.id != archived_thread.id,
            _ => true,
        });
        objects.push(MemoryObject::MemoryThread(archived_thread.clone()));
        let graph = graph_with(&objects, &fixtures.links()).await;
        let vector = RecordingVectorStore::new(vec![candidate(
            archived_thread.id,
            ObjectType::MemoryThread,
            0.99,
        )]);
        let embedder = RecordingEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&graph, &vector, &embedder);
        let mut context = RetrievalContext::new("archived thread").with_trace();
        context.lifecycle_policy.include_archived = true;

        let outcome = pipeline.retrieve(context).await.unwrap();
        let trace = outcome.trace.as_ref().unwrap();

        assert!(outcome.pack.active_threads.is_empty());
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == archived_thread.id
                && assignment.section == ContextPackSection::Omitted
                && assignment.reason.as_deref()
                    == Some("memory_thread status archived is not included in active_threads")
        }));
    }

    async fn graph_with(
        objects: &[MemoryObject],
        links: &[crate::api::types::MemoryLink],
    ) -> FakeGraphAuthorityStore {
        let graph = FakeGraphAuthorityStore::new();
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

    fn candidate(object_id: MemoryId, object_type: ObjectType, score: f32) -> VectorCandidateMatch {
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
    struct RecordingVectorStore {
        candidates: Vec<VectorCandidateMatch>,
        searches: Arc<Mutex<Vec<VectorCandidateSearch>>>,
    }

    impl RecordingVectorStore {
        fn new(candidates: Vec<VectorCandidateMatch>) -> Self {
            Self {
                candidates,
                searches: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn searches(&self) -> Vec<VectorCandidateSearch> {
            lock(&self.searches).unwrap().clone()
        }
    }

    #[async_trait]
    impl VectorCandidateStore for RecordingVectorStore {
        async fn upsert_vector_records(
            &self,
            _records: &[crate::models::vector::VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn search_candidates(
            &self,
            query: &VectorCandidateSearch,
        ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
            lock(&self.searches)?.push(query.clone());
            let mut candidates = self.candidates.clone();
            candidates.truncate(query.limit);
            Ok(candidates)
        }

        async fn list_candidate_diagnostics(
            &self,
        ) -> Result<Vec<crate::models::vector::VectorCandidateDiagnosticRecord>, CustomError>
        {
            Ok(Vec::new())
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct ErrorGraphStore {
        message: String,
    }

    impl ErrorGraphStore {
        fn new(message: impl Into<String>) -> Self {
            Self {
                message: message.into(),
            }
        }
    }

    #[async_trait]
    impl GraphAuthorityStore for ErrorGraphStore {
        async fn upsert_objects(&self, _objects: &[MemoryObject]) -> Result<(), CustomError> {
            Ok(())
        }

        async fn upsert_links(
            &self,
            _links: &[crate::api::types::MemoryLink],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn upsert_objects_and_links(
            &self,
            _objects: &[MemoryObject],
            _links: &[crate::api::types::MemoryLink],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn query_objects(
            &self,
            _query: &crate::ports::graph_authority::GraphObjectQuery,
        ) -> Result<Vec<MemoryObject>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_provenance(
            &self,
            _query: &crate::ports::graph_authority::GraphDerivedMemoryProvenanceQuery,
        ) -> Result<Vec<crate::api::types::DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_thread(
            &self,
            _query: &crate::ports::graph_authority::GraphDerivedMemoryThreadQuery,
        ) -> Result<Vec<crate::api::types::DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn expand_bounded(
            &self,
            _query: &GraphExpansionQuery,
        ) -> Result<GraphExpansion, CustomError> {
            Err(CustomError::MemoryValidation(self.message.clone()))
        }

        async fn list_diagnostic_objects(&self) -> Result<Vec<MemoryObject>, CustomError> {
            Ok(Vec::new())
        }

        async fn list_diagnostic_links(
            &self,
        ) -> Result<Vec<crate::api::types::MemoryLink>, CustomError> {
            Ok(Vec::new())
        }
    }

    fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, CustomError> {
        mutex
            .lock()
            .map_err(|error| CustomError::DatabaseError(format!("test lock poisoned: {error}")))
    }
}
