use std::collections::HashMap;

use crate::api::types::{
    MemoryObjectRef, ObjectType, RelationType, RetrievalLifecyclePolicy, SelectivityCountScope,
    SelectivityDecision, SelectivityTelemetry, SelectivityTrace,
};
use crate::errors::CustomError;
use crate::internal::models::vector::VectorCandidateMatch;
use crate::internal::repositories::{
    GraphExpansionFanoutOverride, RetrievalStatsCounter, RetrievalStatsCounterKey,
    RetrievalStatsHealth, RetrievalStatsHealthState, RetrievalStatsStore,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct RetrievalSelectivityPolicy {
    smoothing_alpha: f64,
    gamma: f64,
}

impl RetrievalSelectivityPolicy {
    pub(crate) fn new(smoothing_alpha: f64, gamma: f64) -> Self {
        Self::try_new(smoothing_alpha, gamma)
            .expect("selectivity smoothing_alpha and gamma must be finite positive numbers")
    }

    pub(crate) fn try_new(smoothing_alpha: f64, gamma: f64) -> Result<Self, CustomError> {
        validate_positive_f64("selectivity_smoothing_alpha", smoothing_alpha)?;
        validate_positive_f64("selectivity_gamma", gamma)?;
        Ok(Self {
            smoothing_alpha,
            gamma,
        })
    }
}

impl Default for RetrievalSelectivityPolicy {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SelectivityPlan {
    pub(crate) fanout_overrides: Vec<GraphExpansionFanoutOverride>,
    pub(crate) traces: Vec<SelectivityTrace>,
    pub(crate) telemetry: SelectivityTelemetry,
}

#[derive(Debug, Clone)]
pub(crate) struct SelectivityStatsContext {
    health: RetrievalStatsHealth,
    specs: Vec<FanoutSpec>,
    global_counters: HashMap<(RelationType, ObjectType), Option<RetrievalStatsCounter>>,
}

impl SelectivityStatsContext {
    #[cfg(test)]
    pub(crate) async fn load(stats_store: &dyn RetrievalStatsStore) -> Result<Self, CustomError> {
        Self::load_with_scope(stats_store, &[], &[]).await
    }

    pub(crate) async fn load_with_scope(
        stats_store: &dyn RetrievalStatsStore,
        allowed_object_types: &[ObjectType],
        allowed_relation_types: &[RelationType],
    ) -> Result<Self, CustomError> {
        let specs = fanout_specs()
            .iter()
            .copied()
            .filter(|spec| {
                spec_allowed_by_graph_scope(spec, allowed_object_types, allowed_relation_types)
            })
            .collect::<Vec<_>>();
        let health = match stats_store.health().await {
            Ok(health) => health,
            Err(error) => {
                let _ = stats_store.mark_unhealthy(error.to_string()).await;
                return Ok(Self {
                    health: RetrievalStatsHealth {
                        state: RetrievalStatsHealthState::Unhealthy,
                        last_error_message: Some(error.to_string()),
                    },
                    specs,
                    global_counters: HashMap::new(),
                });
            }
        };
        let mut global_counters = HashMap::new();
        if health.state == RetrievalStatsHealthState::Healthy {
            for spec in &specs {
                let global_counter = match stats_store
                    .global_counter(spec.relation, spec.object_type)
                    .await
                {
                    Ok(counter) => counter,
                    Err(error) => {
                        let _ = stats_store.mark_unhealthy(error.to_string()).await;
                        return Ok(Self {
                            health: RetrievalStatsHealth {
                                state: RetrievalStatsHealthState::Unhealthy,
                                last_error_message: Some(error.to_string()),
                            },
                            specs,
                            global_counters: HashMap::new(),
                        });
                    }
                };
                global_counters.insert((spec.relation, spec.object_type), global_counter);
            }
        }
        Ok(Self {
            health,
            specs,
            global_counters,
        })
    }

    fn global_counter(
        &self,
        relation: RelationType,
        object_type: ObjectType,
    ) -> Option<RetrievalStatsCounter> {
        self.global_counters
            .get(&(relation, object_type))
            .copied()
            .flatten()
    }
}

pub(crate) async fn selectivity_plan_for_candidate(
    candidate: &VectorCandidateMatch,
    static_max_fanout: usize,
    stats_store: &dyn RetrievalStatsStore,
    policy: RetrievalSelectivityPolicy,
    stats_context: &SelectivityStatsContext,
    lifecycle_policy: RetrievalLifecyclePolicy,
    include_trace: bool,
) -> Result<SelectivityPlan, CustomError> {
    if candidate.object_type != ObjectType::Entity {
        return Ok(SelectivityPlan::default());
    }

    let mut plan = SelectivityPlan::default();
    let count_scope = SelectivityCountScope::from(lifecycle_policy);
    let mut stats_reads_failed = stats_context.health.state != RetrievalStatsHealthState::Healthy;
    let support_factor = semantic_support_factor(candidate.score);
    for spec in &stats_context.specs {
        let (score, entity_count, global_count, fallback) = if !stats_reads_failed {
            let key = RetrievalStatsCounterKey {
                entity_id: candidate.object_id,
                relation_kind: spec.relation,
                object_type: spec.object_type,
            };
            let entity = match stats_store.counter(&key).await {
                Ok(counter) => counter,
                Err(error) => {
                    let _ = stats_store.mark_unhealthy(error.to_string()).await;
                    stats_reads_failed = true;
                    None
                }
            };
            let global = stats_context.global_counter(spec.relation, spec.object_type);
            match (entity, global) {
                (Some(entity), Some(global)) => {
                    let entity_count = count_scope.count(entity);
                    let global_count = count_scope.count(global);
                    if global_count > 0 {
                        (
                            Some(selectivity_score(
                                entity_count,
                                global_count,
                                policy.smoothing_alpha,
                            )),
                            Some(entity_count),
                            Some(global_count),
                            false,
                        )
                    } else {
                        (None, None, None, true)
                    }
                }
                _ => (None, None, None, true),
            }
        } else {
            (None, None, None, true)
        };

        let max_fanout = spec.max_fanout.min(static_max_fanout);
        let chosen_fanout = match score {
            Some(score) => smooth_fanout_budget(
                score,
                support_factor,
                spec.min_fanout,
                max_fanout,
                policy.gamma,
            ),
            None => conservative_fallback_fanout(max_fanout),
        };
        let decision = selectivity_decision(score, support_factor, chosen_fanout, fallback);
        increment_telemetry(&mut plan.telemetry, decision);
        plan.fanout_overrides.push(GraphExpansionFanoutOverride {
            relation: spec.relation,
            object_type: spec.object_type,
            max_fanout: chosen_fanout,
        });
        if include_trace {
            plan.traces.push(SelectivityTrace {
                root: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
                relation: spec.relation,
                object_type: spec.object_type,
                count_scope,
                score,
                entity_count,
                global_count,
                support_factor,
                chosen_fanout,
                max_fanout,
                decision,
                fallback,
            });
        }
    }

    Ok(plan)
}

fn validate_positive_f64(name: &str, value: f64) -> Result<(), CustomError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(CustomError::ConfigParseError(format!(
            "{name} must be a finite positive number, got {value}"
        )));
    }
    Ok(())
}

pub(crate) fn selectivity_score(entity_count: u64, global_count: u64, alpha: f64) -> f64 {
    if global_count == 0 {
        return 0.0;
    }
    let n = entity_count as f64;
    let total = global_count as f64;
    let denominator = (total + alpha).ln();
    if denominator <= 0.0 {
        return 0.0;
    }
    (((total + alpha) / (n + alpha)).ln() / denominator).clamp(0.0, 1.0)
}

fn smooth_fanout_budget(
    score: f64,
    support_factor: f64,
    min_fanout: usize,
    max_fanout: usize,
    gamma: f64,
) -> usize {
    let specificity_factor = score.clamp(0.0, 1.0).powf(gamma);
    let budget = ((max_fanout as f64) * specificity_factor * support_factor).floor() as usize;
    budget.clamp(min_fanout, max_fanout)
}

fn semantic_support_factor(score: f32) -> f64 {
    1.0 + score.clamp(0.0, 1.0) as f64
}

fn conservative_fallback_fanout(max_fanout: usize) -> usize {
    max_fanout.min(1)
}

fn spec_allowed_by_graph_scope(
    spec: &FanoutSpec,
    allowed_object_types: &[ObjectType],
    allowed_relation_types: &[RelationType],
) -> bool {
    (allowed_object_types.is_empty() || allowed_object_types.contains(&spec.object_type))
        && (allowed_relation_types.is_empty() || allowed_relation_types.contains(&spec.relation))
}

fn selectivity_decision(
    score: Option<f64>,
    support_factor: f64,
    chosen_fanout: usize,
    fallback: bool,
) -> SelectivityDecision {
    if fallback {
        return SelectivityDecision::ConservativeFallback;
    }
    if chosen_fanout == 0 {
        return SelectivityDecision::LowSelectivityRejected;
    }
    let score = score.unwrap_or_default();
    if score >= 0.5 {
        SelectivityDecision::HighSelectivity
    } else if chosen_fanout > 0 && support_factor > 1.0 {
        SelectivityDecision::LowSelectivitySupported
    } else {
        SelectivityDecision::LowSelectivityRejected
    }
}

impl From<RetrievalLifecyclePolicy> for SelectivityCountScope {
    fn from(policy: RetrievalLifecyclePolicy) -> Self {
        if policy.include_archived || policy.include_suppressed || policy.include_deleted {
            Self::Total
        } else if policy.include_non_current || policy.include_superseded {
            Self::Active
        } else {
            Self::Current
        }
    }
}

impl SelectivityCountScope {
    fn count(self, counter: RetrievalStatsCounter) -> u64 {
        match self {
            Self::Current => counter.current_count,
            Self::Active => counter.active_count,
            Self::Total => counter.total_count,
        }
    }
}

fn increment_telemetry(telemetry: &mut SelectivityTelemetry, decision: SelectivityDecision) {
    telemetry.decision_count += 1;
    match decision {
        SelectivityDecision::HighSelectivity => telemetry.high_selectivity_count += 1,
        SelectivityDecision::LowSelectivitySupported => {
            telemetry.low_selectivity_supported_count += 1
        }
        SelectivityDecision::LowSelectivityRejected => {
            telemetry.low_selectivity_rejected_count += 1
        }
        SelectivityDecision::ConservativeFallback => telemetry.fallback_count += 1,
    }
}

#[derive(Debug, Clone, Copy)]
struct FanoutSpec {
    relation: RelationType,
    object_type: ObjectType,
    min_fanout: usize,
    max_fanout: usize,
}

fn fanout_specs() -> &'static [FanoutSpec] {
    &[
        FanoutSpec {
            relation: RelationType::About,
            object_type: ObjectType::DerivedMemory,
            min_fanout: 0,
            max_fanout: 20,
        },
        FanoutSpec {
            relation: RelationType::Involves,
            object_type: ObjectType::Episode,
            min_fanout: 0,
            max_fanout: 5,
        },
        FanoutSpec {
            relation: RelationType::PartOfThread,
            object_type: ObjectType::DerivedMemory,
            min_fanout: 0,
            max_fanout: 15,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::models::vector::VectorSurface;
    use crate::internal::repositories::{InMemoryRetrievalStatsStore, RetrievalStatsEdge};
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[test]
    fn selectivity_decreases_as_entity_count_increases() {
        let narrow = selectivity_score(1, 100, 1.0);
        let broad = selectivity_score(50, 100, 1.0);

        assert!(narrow > broad);
        assert!((0.0..=1.0).contains(&narrow));
        assert!((0.0..=1.0).contains(&broad));
    }

    #[test]
    fn support_can_raise_fanout_without_exceeding_cap() {
        let unsupported = smooth_fanout_budget(0.3, 1.0, 0, 20, 1.0);
        let supported = smooth_fanout_budget(0.3, 2.0, 0, 20, 1.0);

        assert!(supported > unsupported);
        assert!(supported <= 20);
    }

    #[test]
    fn zero_selectivity_score_uses_conservative_zero_floor() {
        let budget = smooth_fanout_budget(0.0, 2.0, 0, 20, 1.0);

        assert_eq!(budget, 0);
    }

    #[test]
    fn selectivity_policy_rejects_invalid_numbers() {
        let invalid_alpha = RetrievalSelectivityPolicy::try_new(0.0, 1.0);
        let invalid_gamma = RetrievalSelectivityPolicy::try_new(1.0, f64::NAN);

        assert!(matches!(
            invalid_alpha,
            Err(CustomError::ConfigParseError(message))
                if message.contains("selectivity_smoothing_alpha")
        ));
        assert!(matches!(
            invalid_gamma,
            Err(CustomError::ConfigParseError(message)) if message.contains("selectivity_gamma")
        ));
    }

    #[tokio::test]
    async fn selectivity_plan_builds_traces_only_when_requested() {
        let stats = InMemoryRetrievalStatsStore::new();
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let candidate = VectorCandidateMatch::new(
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462001").unwrap(),
            ObjectType::Entity,
            VectorSurface::Name,
            0.75,
        );

        let without_trace = selectivity_plan_for_candidate(
            &candidate,
            10,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            false,
        )
        .await
        .unwrap();
        let with_trace = selectivity_plan_for_candidate(
            &candidate,
            10,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(without_trace.telemetry.decision_count, fanout_specs().len());
        assert!(without_trace.traces.is_empty());
        assert_eq!(with_trace.traces.len(), fanout_specs().len());
    }

    #[tokio::test]
    async fn selectivity_plan_uses_conservative_fanout_when_stats_are_missing() {
        let stats = InMemoryRetrievalStatsStore::new();
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let candidate = VectorCandidateMatch::new(
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462002").unwrap(),
            ObjectType::Entity,
            VectorSurface::Name,
            0.95,
        );

        let plan = selectivity_plan_for_candidate(
            &candidate,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.fallback_count, fanout_specs().len());
        assert!(plan
            .fanout_overrides
            .iter()
            .all(|override_| override_.max_fanout == 1));
        assert!(plan.traces.iter().all(|trace| {
            trace.fallback
                && trace.count_scope == SelectivityCountScope::Current
                && trace.chosen_fanout == 1
                && trace.decision == SelectivityDecision::ConservativeFallback
        }));
    }

    #[tokio::test]
    async fn selectivity_plan_uses_conservative_fanout_when_stats_reads_fail() {
        let stats = FailingRetrievalStatsStore;
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let candidate = VectorCandidateMatch::new(
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462021").unwrap(),
            ObjectType::Entity,
            VectorSurface::Name,
            0.95,
        );

        let plan = selectivity_plan_for_candidate(
            &candidate,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.fallback_count, fanout_specs().len());
        assert!(plan.traces.iter().all(|trace| {
            trace.fallback
                && trace.chosen_fanout == 1
                && trace.decision == SelectivityDecision::ConservativeFallback
        }));
    }

    #[tokio::test]
    async fn selectivity_plan_uses_conservative_fanout_after_partial_stats_read_failure() {
        let stats = PartiallyFailingRetrievalStatsStore::default();
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let candidate = VectorCandidateMatch::new(
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462022").unwrap(),
            ObjectType::Entity,
            VectorSurface::Name,
            0.95,
        );

        let plan = selectivity_plan_for_candidate(
            &candidate,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.fallback_count, fanout_specs().len());
        assert!(plan.traces.iter().all(|trace| {
            trace.fallback
                && trace.chosen_fanout == 1
                && trace.decision == SelectivityDecision::ConservativeFallback
        }));
    }

    #[tokio::test]
    async fn selectivity_plan_skips_specs_excluded_by_graph_scope() {
        let stats = InMemoryRetrievalStatsStore::new();
        let stats_context = SelectivityStatsContext::load_with_scope(
            &stats,
            &[ObjectType::Episode],
            &[RelationType::Involves],
        )
        .await
        .unwrap();
        let candidate = VectorCandidateMatch::new(
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462023").unwrap(),
            ObjectType::Entity,
            VectorSurface::Name,
            0.95,
        );

        let plan = selectivity_plan_for_candidate(
            &candidate,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.decision_count, 1);
        assert_eq!(plan.fanout_overrides.len(), 1);
        assert_eq!(plan.fanout_overrides[0].relation, RelationType::Involves);
        assert_eq!(plan.fanout_overrides[0].object_type, ObjectType::Episode);
        assert_eq!(plan.traces.len(), 1);
        assert_eq!(plan.traces[0].relation, RelationType::Involves);
        assert_eq!(plan.traces[0].object_type, ObjectType::Episode);
    }

    #[tokio::test]
    async fn selectivity_plan_is_empty_when_graph_scope_excludes_all_specs() {
        let stats = InMemoryRetrievalStatsStore::new();
        let stats_context = SelectivityStatsContext::load_with_scope(
            &stats,
            &[ObjectType::Observation],
            &[RelationType::About],
        )
        .await
        .unwrap();
        let candidate = VectorCandidateMatch::new(
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462024").unwrap(),
            ObjectType::Entity,
            VectorSurface::Name,
            0.95,
        );

        let plan = selectivity_plan_for_candidate(
            &candidate,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.decision_count, 0);
        assert!(plan.fanout_overrides.is_empty());
        assert!(plan.traces.is_empty());
    }

    #[tokio::test]
    async fn selectivity_plan_uses_lifecycle_scoped_counts() {
        let stats = InMemoryRetrievalStatsStore::new();
        let entity_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462003").unwrap();
        stats
            .record_edges(&[
                RetrievalStatsEdge {
                    edge_key: format!("{entity_id}:about:derived_memory:current"),
                    entity_id,
                    relation_kind: RelationType::About,
                    object_id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462004")
                        .unwrap(),
                    object_type: ObjectType::DerivedMemory,
                    retention_state: crate::api::types::RetentionState::Active,
                    is_current: true,
                    first_seen_at: chrono::DateTime::UNIX_EPOCH,
                    last_seen_at: chrono::DateTime::UNIX_EPOCH,
                },
                RetrievalStatsEdge {
                    edge_key: format!("{entity_id}:about:derived_memory:non_current"),
                    entity_id,
                    relation_kind: RelationType::About,
                    object_id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462005")
                        .unwrap(),
                    object_type: ObjectType::DerivedMemory,
                    retention_state: crate::api::types::RetentionState::Active,
                    is_current: false,
                    first_seen_at: chrono::DateTime::UNIX_EPOCH,
                    last_seen_at: chrono::DateTime::UNIX_EPOCH,
                },
                RetrievalStatsEdge {
                    edge_key: format!("{entity_id}:about:derived_memory:suppressed"),
                    entity_id,
                    relation_kind: RelationType::About,
                    object_id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462006")
                        .unwrap(),
                    object_type: ObjectType::DerivedMemory,
                    retention_state: crate::api::types::RetentionState::Suppressed,
                    is_current: false,
                    first_seen_at: chrono::DateTime::UNIX_EPOCH,
                    last_seen_at: chrono::DateTime::UNIX_EPOCH,
                },
            ])
            .await
            .unwrap();
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let candidate =
            VectorCandidateMatch::new(entity_id, ObjectType::Entity, VectorSurface::Name, 0.75);

        let active_plan = selectivity_plan_for_candidate(
            &candidate,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy {
                include_non_current: true,
                ..RetrievalLifecyclePolicy::default()
            },
            true,
        )
        .await
        .unwrap();
        let total_plan = selectivity_plan_for_candidate(
            &candidate,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy {
                include_suppressed: true,
                ..RetrievalLifecyclePolicy::default()
            },
            true,
        )
        .await
        .unwrap();

        let active_about = active_plan
            .traces
            .iter()
            .find(|trace| {
                trace.relation == RelationType::About
                    && trace.object_type == ObjectType::DerivedMemory
            })
            .unwrap();
        assert_eq!(active_about.count_scope, SelectivityCountScope::Active);
        assert_eq!(active_about.entity_count, Some(2));
        assert_eq!(active_about.global_count, Some(2));

        let total_about = total_plan
            .traces
            .iter()
            .find(|trace| {
                trace.relation == RelationType::About
                    && trace.object_type == ObjectType::DerivedMemory
            })
            .unwrap();
        assert_eq!(total_about.count_scope, SelectivityCountScope::Total);
        assert_eq!(total_about.entity_count, Some(3));
        assert_eq!(total_about.global_count, Some(3));
    }

    #[test]
    fn zero_fanout_is_not_reported_as_high_selectivity() {
        let decision = selectivity_decision(Some(1.0), 2.0, 0, false);

        assert_eq!(decision, SelectivityDecision::LowSelectivityRejected);
    }

    struct FailingRetrievalStatsStore;

    #[async_trait]
    impl RetrievalStatsStore for FailingRetrievalStatsStore {
        async fn record_edges(&self, _edges: &[RetrievalStatsEdge]) -> Result<(), CustomError> {
            Ok(())
        }

        async fn record_object_states(
            &self,
            _states: &[crate::internal::repositories::RetrievalStatsObjectState],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn counter(
            &self,
            _key: &RetrievalStatsCounterKey,
        ) -> Result<Option<RetrievalStatsCounter>, CustomError> {
            Err(CustomError::DatabaseError(
                "stats counter read failed".to_owned(),
            ))
        }

        async fn global_counter(
            &self,
            _relation_kind: RelationType,
            _object_type: ObjectType,
        ) -> Result<Option<RetrievalStatsCounter>, CustomError> {
            Err(CustomError::DatabaseError(
                "stats global counter read failed".to_owned(),
            ))
        }

        async fn health(&self) -> Result<RetrievalStatsHealth, CustomError> {
            Ok(RetrievalStatsHealth::default())
        }

        async fn mark_unhealthy(&self, _message: String) -> Result<(), CustomError> {
            Ok(())
        }

        async fn record_rejected_low_information_link(&self) -> Result<(), CustomError> {
            Ok(())
        }

        async fn rejected_low_information_link_count(&self) -> Result<u64, CustomError> {
            Ok(0)
        }
    }

    #[derive(Default)]
    struct PartiallyFailingRetrievalStatsStore {
        counter_reads: Mutex<usize>,
    }

    #[async_trait]
    impl RetrievalStatsStore for PartiallyFailingRetrievalStatsStore {
        async fn record_edges(&self, _edges: &[RetrievalStatsEdge]) -> Result<(), CustomError> {
            Ok(())
        }

        async fn record_object_states(
            &self,
            _states: &[crate::internal::repositories::RetrievalStatsObjectState],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn counter(
            &self,
            _key: &RetrievalStatsCounterKey,
        ) -> Result<Option<RetrievalStatsCounter>, CustomError> {
            let mut reads = self.counter_reads.lock().unwrap();
            *reads += 1;
            if *reads == 1 {
                return Err(CustomError::DatabaseError(
                    "first stats counter read failed".to_owned(),
                ));
            }
            Ok(Some(RetrievalStatsCounter {
                total_count: 100,
                active_count: 100,
                current_count: 100,
            }))
        }

        async fn global_counter(
            &self,
            _relation_kind: RelationType,
            _object_type: ObjectType,
        ) -> Result<Option<RetrievalStatsCounter>, CustomError> {
            Ok(Some(RetrievalStatsCounter {
                total_count: 100,
                active_count: 100,
                current_count: 100,
            }))
        }

        async fn health(&self) -> Result<RetrievalStatsHealth, CustomError> {
            Ok(RetrievalStatsHealth::default())
        }

        async fn mark_unhealthy(&self, _message: String) -> Result<(), CustomError> {
            Ok(())
        }

        async fn record_rejected_low_information_link(&self) -> Result<(), CustomError> {
            Ok(())
        }

        async fn rejected_low_information_link_count(&self) -> Result<u64, CustomError> {
            Ok(0)
        }
    }
}
