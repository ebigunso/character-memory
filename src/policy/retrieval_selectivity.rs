use std::collections::HashMap;

use crate::api::types::{
    RetrievalLifecyclePolicy, SelectivityCountScope, SelectivityDecision, SelectivityTelemetry,
    SelectivityTrace,
};
use crate::domain::{MemoryObjectRef, ObjectType, RelationType};
#[cfg(test)]
use crate::errors::RetrievalStatsStoreError;
use crate::errors::{
    ConfigValidationError, ConfigValidationReason, CustomError, RetrievalStatsHealthCause,
};
use crate::ports::graph_authority::GraphExpansionFanoutOverride;
use crate::ports::graph_authority::TraceMode;
use crate::ports::retrieval_stats::{
    RetrievalStatsCounter, RetrievalStatsCounterKey, RetrievalStatsHealth,
    RetrievalStatsHealthState, RetrievalStatsStore,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct RetrievalSelectivityPolicy {
    smoothing_alpha: f64,
    gamma: f64,
    fanout_budgets: [FanoutSpec; 3],
}

impl RetrievalSelectivityPolicy {
    pub(crate) fn new(smoothing_alpha: f64, gamma: f64) -> Self {
        Self::try_new(smoothing_alpha, gamma)
            .expect("selectivity smoothing_alpha and gamma must be finite positive numbers")
    }

    pub(crate) fn try_new(smoothing_alpha: f64, gamma: f64) -> Result<Self, CustomError> {
        Self::try_new_with_fanout_budgets(smoothing_alpha, gamma, [])
    }

    pub(crate) fn try_new_with_fanout_budgets(
        smoothing_alpha: f64,
        gamma: f64,
        fanout_budgets: impl IntoIterator<Item = (RelationType, ObjectType, usize, usize)>,
    ) -> Result<Self, CustomError> {
        validate_positive_f64("selectivity_smoothing_alpha", smoothing_alpha)?;
        validate_positive_f64("selectivity_gamma", gamma)?;
        let mut configured_budgets = DEFAULT_FANOUT_SPECS;
        for (relation, object_type, min_fanout, max_fanout) in fanout_budgets {
            validate_fanout_budget(relation, object_type, min_fanout, max_fanout)?;
            if let Some(spec) = configured_budgets
                .iter_mut()
                .find(|spec| spec.relation == relation && spec.object_type == object_type)
            {
                spec.min_fanout = min_fanout;
                spec.max_fanout = max_fanout;
            } else {
                return Err(ConfigValidationError {
                    keys: vec!["retrieval.fanout"],
                    reason: ConfigValidationReason::OutOfDomain {
                        expected: "an implemented retrieval fanout target",
                        actual: format!("{relation:?}->{object_type:?}"),
                    },
                }
                .into());
            }
        }
        Ok(Self {
            smoothing_alpha,
            gamma,
            fanout_budgets: configured_budgets,
        })
    }

    pub(crate) fn state_scope_limit(&self) -> usize {
        self.fanout_budget(RelationType::About, ObjectType::DerivedMemory)
            .max_fanout
    }

    fn fanout_budget(&self, relation: RelationType, object_type: ObjectType) -> FanoutSpec {
        let (relation, object_type) = match (relation, object_type) {
            (RelationType::Mentions, ObjectType::Observation) => {
                (RelationType::About, ObjectType::DerivedMemory)
            }
            pair => pair,
        };
        self.fanout_budgets
            .iter()
            .copied()
            .find(|spec| spec.relation == relation && spec.object_type == object_type)
            .expect("every selectivity route maps to a configured fanout budget")
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
        let specs = fanout_routes()
            .iter()
            .copied()
            .filter(|spec| {
                spec_allowed_by_graph_scope(spec, allowed_object_types, allowed_relation_types)
            })
            .collect::<Vec<_>>();
        let health = match stats_store.health().await {
            Ok(health) => health,
            Err(error) => {
                return Ok(Self::failed(
                    stats_store,
                    specs,
                    RetrievalStatsHealthCause::HealthCheck { error },
                )
                .await);
            }
        };
        let mut global_counters = HashMap::new();
        if health.state == RetrievalStatsHealthState::Healthy {
            for spec in &specs {
                let bucket = spec.count_bucket();
                if global_counters.contains_key(&bucket) {
                    continue;
                }
                let counter = if bucket == (RelationType::Involves, ObjectType::Episode) {
                    stats_store.global_episode_counter().await
                } else {
                    stats_store.global_counter(bucket.0, bucket.1).await
                };
                let global_counter = match counter {
                    Ok(counter) => counter,
                    Err(error) => {
                        return Ok(Self::failed(
                            stats_store,
                            specs,
                            RetrievalStatsHealthCause::GlobalCounterRead { error },
                        )
                        .await);
                    }
                };
                global_counters.insert(bucket, global_counter);
            }
        }
        Ok(Self {
            health,
            specs,
            global_counters,
        })
    }

    async fn failed(
        stats_store: &dyn RetrievalStatsStore,
        specs: Vec<FanoutSpec>,
        cause: RetrievalStatsHealthCause,
    ) -> Self {
        let _ = stats_store.mark_unhealthy(cause.clone()).await;
        Self {
            health: RetrievalStatsHealth {
                state: RetrievalStatsHealthState::Unhealthy,
                last_error_cause: Some(cause),
            },
            specs,
            global_counters: HashMap::new(),
        }
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

// Scene participants supply entity roots; content recall does not index notions.
#[expect(
    clippy::too_many_arguments,
    reason = "identity and cue support are independent inputs to the entity policy"
)]
pub(crate) async fn selectivity_plan_for_entity(
    entity_id: crate::domain::MemoryId,
    cue_score: f32,
    static_max_fanout: usize,
    stats_store: &dyn RetrievalStatsStore,
    policy: RetrievalSelectivityPolicy,
    stats_context: &SelectivityStatsContext,
    lifecycle_policy: RetrievalLifecyclePolicy,
    trace_mode: TraceMode,
) -> Result<SelectivityPlan, CustomError> {
    let mut plan = SelectivityPlan::default();
    let count_scope = SelectivityCountScope::from(lifecycle_policy);
    let mut stats_reads_failed = stats_context.health.state != RetrievalStatsHealthState::Healthy;
    let support_factor = semantic_support_factor(cue_score);
    for spec in &stats_context.specs {
        if matches!(spec.relation, RelationType::About | RelationType::Mentions) {
            let max_fanout = policy.state_scope_limit().min(static_max_fanout);
            if !plan
                .fanout_overrides
                .iter()
                .any(|entry| entry.relation == RelationType::About)
            {
                plan.fanout_overrides.push(GraphExpansionFanoutOverride {
                    relation: RelationType::About,
                    object_type: ObjectType::DerivedMemory,
                    max_fanout,
                });
            }
            continue;
        }
        let (count_relation, count_object_type) = spec.count_bucket();
        let (score, entity_count, global_count, fallback) = if !stats_reads_failed {
            let key = RetrievalStatsCounterKey {
                entity_id,
                relation_kind: count_relation,
                object_type: count_object_type,
            };
            let entity = match stats_store.counter(&key).await {
                Ok(counter) => counter,
                Err(error) => {
                    let _ = stats_store
                        .mark_unhealthy(RetrievalStatsHealthCause::CounterRead { error })
                        .await;
                    stats_reads_failed = true;
                    None
                }
            };
            let global = stats_context.global_counter(count_relation, count_object_type);
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

        let budget_spec = policy.fanout_budget(count_relation, count_object_type);
        let max_fanout = budget_spec.max_fanout.min(static_max_fanout);
        let min_fanout = budget_spec.min_fanout.min(max_fanout);
        let chosen_fanout = match score {
            Some(score) => {
                smooth_fanout_budget(score, support_factor, min_fanout, max_fanout, policy.gamma)
            }
            None => conservative_fallback_fanout(min_fanout, max_fanout),
        };
        let decision = selectivity_decision(score, support_factor, chosen_fanout, fallback);
        increment_telemetry(&mut plan.telemetry, decision);
        plan.fanout_overrides.push(GraphExpansionFanoutOverride {
            relation: spec.relation,
            object_type: spec.object_type,
            max_fanout: chosen_fanout,
        });
        if trace_mode.is_enabled() {
            plan.traces.push(SelectivityTrace {
                root: MemoryObjectRef::new(ObjectType::Entity, entity_id),
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

fn validate_positive_f64(name: &'static str, value: f64) -> Result<(), CustomError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(ConfigValidationError {
            keys: vec![name],
            reason: ConfigValidationReason::OutOfDomain {
                expected: "a finite positive number",
                actual: value.to_string(),
            },
        }
        .into());
    }
    Ok(())
}

fn validate_fanout_budget(
    relation: RelationType,
    object_type: ObjectType,
    min_fanout: usize,
    max_fanout: usize,
) -> Result<(), CustomError> {
    if min_fanout > max_fanout {
        return Err(ConfigValidationError {
            keys: vec![fanout_config_key(relation, object_type)],
            reason: ConfigValidationReason::OutOfDomain {
                expected: "min <= max",
                actual: format!("min={min_fanout} max={max_fanout}"),
            },
        }
        .into());
    }
    Ok(())
}

fn fanout_config_key(relation: RelationType, object_type: ObjectType) -> &'static str {
    match (relation, object_type) {
        (RelationType::About, ObjectType::DerivedMemory) => {
            "retrieval.fanout.about_entity.derived_memory"
        }
        (RelationType::Involves, ObjectType::Episode) => {
            "retrieval.fanout.participant_entity.episode"
        }
        (RelationType::PartOfThread, ObjectType::DerivedMemory) => {
            "retrieval.fanout.part_of_thread.derived_memory"
        }
        _ => "retrieval.fanout",
    }
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

fn conservative_fallback_fanout(min_fanout: usize, max_fanout: usize) -> usize {
    max_fanout.min(min_fanout.max(1))
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
        if policy.include_suppressed {
            // ponytail: excluding superseded memories still uses totals that include their edges,
            // so fanout can skew either way; expansion separately enforces lifecycle eligibility.
            // Add a non-superseded-across-retention counter if a consumer sees fanout loss.
            Self::Total
        } else if policy.include_superseded {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FanoutSpec {
    relation: RelationType,
    object_type: ObjectType,
    min_fanout: usize,
    max_fanout: usize,
}

impl FanoutSpec {
    fn count_bucket(self) -> (RelationType, ObjectType) {
        (self.relation, self.object_type)
    }
}

fn fanout_routes() -> [FanoutSpec; 4] {
    // Mentions counts observations independently, with the aboutness budget.
    let [about, participant, thread] = DEFAULT_FANOUT_SPECS;
    [
        about,
        participant,
        thread,
        FanoutSpec {
            relation: RelationType::Mentions,
            object_type: ObjectType::Observation,
            ..about
        },
    ]
}

const DEFAULT_FANOUT_SPECS: [FanoutSpec; 3] = [
    FanoutSpec {
        relation: RelationType::About,
        object_type: ObjectType::DerivedMemory,
        min_fanout: 0,
        max_fanout: 20,
    },
    FanoutSpec {
        relation: RelationType::Involves,
        object_type: ObjectType::Episode,
        min_fanout: 1,
        max_fanout: 5,
    },
    FanoutSpec {
        relation: RelationType::PartOfThread,
        object_type: ObjectType::DerivedMemory,
        min_fanout: 0,
        max_fanout: 15,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::stats::InMemoryRetrievalStatsStore;
    use crate::ports::retrieval_stats::RetrievalStatsEdge;
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
    fn fallback_fanout_honors_configured_minimum_within_cap() {
        assert_eq!(conservative_fallback_fanout(0, 20), 1);
        assert_eq!(conservative_fallback_fanout(4, 20), 4);
        assert_eq!(conservative_fallback_fanout(4, 2), 2);
    }

    #[tokio::test]
    async fn selectivity_plan_builds_traces_only_when_requested() {
        let stats = InMemoryRetrievalStatsStore::new();
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let candidate = (
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462001").unwrap(),
            0.75,
        );

        let without_trace = selectivity_plan_for_entity(
            candidate.0,
            candidate.1,
            10,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            TraceMode::Disabled,
        )
        .await
        .unwrap();
        let with_trace = selectivity_plan_for_entity(
            candidate.0,
            candidate.1,
            10,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            TraceMode::Enabled,
        )
        .await
        .unwrap();

        assert_eq!(without_trace.telemetry.decision_count, 2);
        assert!(without_trace.traces.is_empty());
        assert_eq!(with_trace.traces.len(), 2);
    }

    #[tokio::test]
    async fn selectivity_plan_uses_conservative_fanout_when_stats_are_missing() {
        let stats = InMemoryRetrievalStatsStore::new();
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let candidate = (
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462002").unwrap(),
            0.95,
        );

        let plan = selectivity_plan_for_entity(
            candidate.0,
            candidate.1,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            TraceMode::Enabled,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.fallback_count, 2);
        assert_eq!(
            plan.traces
                .iter()
                .find(|trace| trace.relation == RelationType::PartOfThread)
                .unwrap()
                .max_fanout,
            15
        );
        assert!(plan
            .fanout_overrides
            .iter()
            .filter(|override_| override_.relation != RelationType::About)
            .all(|override_| override_.max_fanout == 1));
        assert!(plan.traces.iter().all(|trace| {
            trace.fallback
                && trace.count_scope == SelectivityCountScope::Current
                && trace.chosen_fanout == 1
                && trace.decision == SelectivityDecision::ConservativeFallback
        }));
    }

    #[tokio::test]
    async fn selectivity_plan_uses_lifecycle_scoped_counts() {
        let stats = InMemoryRetrievalStatsStore::new();
        let entity_id = crate::domain::MemoryId::from_u128(1);
        let edges = [
            (crate::domain::RetentionState::Active, true),
            (crate::domain::RetentionState::Active, false),
            (crate::domain::RetentionState::Suppressed, false),
        ]
        .into_iter()
        .enumerate()
        .map(
            |(index, (retention_state, is_current))| RetrievalStatsEdge {
                edge_key: format!("thread:{index}"),
                entity_id,
                relation_kind: RelationType::PartOfThread,
                object_id: crate::domain::MemoryId::from_u128(index as u128 + 2),
                object_type: ObjectType::DerivedMemory,
                retention_state,
                is_current,
                first_seen_at: chrono::DateTime::UNIX_EPOCH,
                last_seen_at: chrono::DateTime::UNIX_EPOCH,
            },
        )
        .collect::<Vec<_>>();
        stats.record_edges(&edges).await.unwrap();
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();

        for (include_suppressed, include_superseded, scope, count) in [
            (false, false, SelectivityCountScope::Current, 1),
            (false, true, SelectivityCountScope::Active, 2),
            (true, false, SelectivityCountScope::Total, 3),
            (true, true, SelectivityCountScope::Total, 3),
        ] {
            let plan = selectivity_plan_for_entity(
                entity_id,
                0.75,
                20,
                &stats,
                RetrievalSelectivityPolicy::default(),
                &stats_context,
                RetrievalLifecyclePolicy {
                    include_suppressed,
                    include_superseded,
                },
                TraceMode::Enabled,
            )
            .await
            .unwrap();
            let trace = plan
                .traces
                .iter()
                .find(|trace| trace.relation == RelationType::PartOfThread)
                .unwrap();
            assert_eq!(trace.count_scope, scope);
            assert_eq!(trace.entity_count, Some(count));
            assert_eq!(trace.global_count, Some(count));
            assert!(!trace.fallback);
        }
    }

    #[tokio::test]
    async fn subject_aboutness_has_one_configured_budget_even_for_mentions_only_scope() {
        let stats = InMemoryRetrievalStatsStore::new();
        let policy = RetrievalSelectivityPolicy::try_new_with_fanout_budgets(
            1.0,
            1.0,
            [(RelationType::About, ObjectType::DerivedMemory, 2, 2)],
        )
        .unwrap();
        for relations in [
            vec![RelationType::Mentions],
            vec![RelationType::About, RelationType::Mentions],
        ] {
            let stats_context = SelectivityStatsContext::load_with_scope(&stats, &[], &relations)
                .await
                .unwrap();
            let plan = selectivity_plan_for_entity(
                crate::domain::MemoryId::from_u128(1),
                1.0,
                16,
                &stats,
                policy,
                &stats_context,
                RetrievalLifecyclePolicy::default(),
                TraceMode::Enabled,
            )
            .await
            .unwrap();
            assert!(plan.traces.is_empty());
            assert_eq!(
                plan.fanout_overrides,
                [GraphExpansionFanoutOverride {
                    relation: RelationType::About,
                    object_type: ObjectType::DerivedMemory,
                    max_fanout: 2,
                }]
            );
        }
    }

    #[tokio::test]
    async fn selectivity_plan_uses_conservative_fanout_when_stats_reads_fail() {
        let stats = FailingRetrievalStatsStore;
        let stats_context = SelectivityStatsContext::load(&stats).await.unwrap();
        let candidate = (
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462021").unwrap(),
            0.95,
        );

        let plan = selectivity_plan_for_entity(
            candidate.0,
            candidate.1,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            TraceMode::Enabled,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.fallback_count, 2);
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
        let candidate = (
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462022").unwrap(),
            0.95,
        );

        let plan = selectivity_plan_for_entity(
            candidate.0,
            candidate.1,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            TraceMode::Enabled,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.fallback_count, 2);
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
        let candidate = (
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462023").unwrap(),
            0.95,
        );

        let plan = selectivity_plan_for_entity(
            candidate.0,
            candidate.1,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            TraceMode::Enabled,
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
        let candidate = (
            uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655462024").unwrap(),
            0.95,
        );

        let plan = selectivity_plan_for_entity(
            candidate.0,
            candidate.1,
            20,
            &stats,
            RetrievalSelectivityPolicy::default(),
            &stats_context,
            RetrievalLifecyclePolicy::default(),
            TraceMode::Enabled,
        )
        .await
        .unwrap();

        assert_eq!(plan.telemetry.decision_count, 0);
        assert!(plan.fanout_overrides.is_empty());
        assert!(plan.traces.is_empty());
    }

    #[test]
    fn zero_fanout_is_not_reported_as_high_selectivity() {
        let decision = selectivity_decision(Some(1.0), 2.0, 0, false);

        assert_eq!(decision, SelectivityDecision::LowSelectivityRejected);
    }

    struct FailingRetrievalStatsStore;

    #[async_trait]
    impl RetrievalStatsStore for FailingRetrievalStatsStore {
        async fn global_episode_counter(
            &self,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            self.global_counter(RelationType::Involves, ObjectType::Episode)
                .await
        }

        async fn record_edges(
            &self,
            _edges: &[RetrievalStatsEdge],
        ) -> Result<(), RetrievalStatsStoreError> {
            Ok(())
        }

        async fn record_object_states(
            &self,
            _states: &[crate::ports::retrieval_stats::RetrievalStatsObjectState],
        ) -> Result<(), RetrievalStatsStoreError> {
            Ok(())
        }

        async fn counter(
            &self,
            _key: &RetrievalStatsCounterKey,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Err(RetrievalStatsStoreError::Sqlite {
                detail: "stats counter read failed".to_owned(),
            })
        }

        async fn global_counter(
            &self,
            _relation_kind: RelationType,
            _object_type: ObjectType,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Err(RetrievalStatsStoreError::Sqlite {
                detail: "stats global counter read failed".to_owned(),
            })
        }

        async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
            Ok(RetrievalStatsHealth::default())
        }

        async fn mark_unhealthy(
            &self,
            _cause: RetrievalStatsHealthCause,
        ) -> Result<(), RetrievalStatsStoreError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct PartiallyFailingRetrievalStatsStore {
        counter_reads: Mutex<usize>,
    }

    #[async_trait]
    impl RetrievalStatsStore for PartiallyFailingRetrievalStatsStore {
        async fn global_episode_counter(
            &self,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            self.global_counter(RelationType::Involves, ObjectType::Episode)
                .await
        }

        async fn record_edges(
            &self,
            _edges: &[RetrievalStatsEdge],
        ) -> Result<(), RetrievalStatsStoreError> {
            Ok(())
        }

        async fn record_object_states(
            &self,
            _states: &[crate::ports::retrieval_stats::RetrievalStatsObjectState],
        ) -> Result<(), RetrievalStatsStoreError> {
            Ok(())
        }

        async fn counter(
            &self,
            _key: &RetrievalStatsCounterKey,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            let mut reads = self.counter_reads.lock().unwrap();
            *reads += 1;
            if *reads == 1 {
                return Err(RetrievalStatsStoreError::Sqlite {
                    detail: "first stats counter read failed".to_owned(),
                });
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
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(Some(RetrievalStatsCounter {
                total_count: 100,
                active_count: 100,
                current_count: 100,
            }))
        }

        async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
            Ok(RetrievalStatsHealth::default())
        }

        async fn mark_unhealthy(
            &self,
            _cause: RetrievalStatsHealthCause,
        ) -> Result<(), RetrievalStatsStoreError> {
            Ok(())
        }
    }
}
