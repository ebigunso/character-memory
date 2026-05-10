use crate::api::types::{
    MemoryObjectRef, ObjectType, RelationType, SelectivityDecision, SelectivityTelemetry,
    SelectivityTrace,
};
use crate::errors::CustomError;
use crate::internal::models::vector::VectorCandidateMatch;
use crate::internal::repositories::{
    GraphExpansionFanoutOverride, RetrievalStatsCounterKey, RetrievalStatsHealthState,
    RetrievalStatsStore,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct RetrievalSelectivityPolicy {
    smoothing_alpha: f64,
    gamma: f64,
}

impl RetrievalSelectivityPolicy {
    pub(crate) fn new(smoothing_alpha: f64, gamma: f64) -> Self {
        Self {
            smoothing_alpha,
            gamma,
        }
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

pub(crate) async fn selectivity_plan_for_candidate(
    candidate: &VectorCandidateMatch,
    static_max_fanout: usize,
    stats_store: &dyn RetrievalStatsStore,
    policy: RetrievalSelectivityPolicy,
) -> Result<SelectivityPlan, CustomError> {
    if candidate.object_type != ObjectType::Entity {
        return Ok(SelectivityPlan::default());
    }

    let health = stats_store.health().await?;
    let mut plan = SelectivityPlan::default();
    for spec in fanout_specs() {
        let support_factor = semantic_support_factor(candidate.score);
        let (score, entity_count, global_count, fallback) =
            if health.state == RetrievalStatsHealthState::Healthy {
                let key = RetrievalStatsCounterKey {
                    entity_id: candidate.object_id,
                    relation_kind: spec.relation,
                    object_type: spec.object_type,
                };
                let entity = stats_store.counter(&key).await?;
                let global = stats_store
                    .global_counter(spec.relation, spec.object_type)
                    .await?;
                match (entity, global) {
                    (Some(entity), Some(global)) if global.current_count > 0 => (
                        Some(selectivity_score(
                            entity.current_count,
                            global.current_count,
                            policy.smoothing_alpha,
                        )),
                        Some(entity.current_count),
                        Some(global.current_count),
                        false,
                    ),
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
            None => max_fanout,
        };
        let decision = selectivity_decision(score, support_factor, chosen_fanout, fallback);
        increment_telemetry(&mut plan.telemetry, decision);
        plan.fanout_overrides.push(GraphExpansionFanoutOverride {
            relation: spec.relation,
            object_type: spec.object_type,
            max_fanout: chosen_fanout,
        });
        plan.traces.push(SelectivityTrace {
            root: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
            relation: spec.relation,
            object_type: spec.object_type,
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

    Ok(plan)
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

fn selectivity_decision(
    score: Option<f64>,
    support_factor: f64,
    chosen_fanout: usize,
    fallback: bool,
) -> SelectivityDecision {
    if fallback {
        return SelectivityDecision::ConservativeFallback;
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

fn fanout_specs() -> Vec<FanoutSpec> {
    vec![
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
    fn missing_stats_use_conservative_zero_floor() {
        let budget = smooth_fanout_budget(0.0, 2.0, 0, 20, 1.0);

        assert_eq!(budget, 0);
    }
}
