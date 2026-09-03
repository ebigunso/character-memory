use std::future::Future;

use crate::api::types::retrieval::VectorRecallCompleteness;
use crate::models::vector::{CanonicalCandidates, VectorCandidateMatch};

const TIE_COHORT_MIN_EXTRA_CANDIDATES: usize = 4_096;
const TIE_COHORT_LIMIT_MULTIPLIER: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FetchDecision {
    Return,
    ReturnAtBound,
    Grow(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TieClosure {
    Closed,
    OpenAtBound,
}

pub(crate) struct TieClosureResult {
    pub(crate) candidates: CanonicalCandidates,
    pub(crate) fetched: usize,
    fetch_bound: usize,
    closure: TieClosure,
}

impl TieClosureResult {
    pub(crate) fn completeness(
        &self,
        exhaustive_scanned: Option<usize>,
    ) -> VectorRecallCompleteness {
        match (self.closure, exhaustive_scanned) {
            (TieClosure::Closed, Some(scanned)) => VectorRecallCompleteness::Exhaustive { scanned },
            (TieClosure::Closed, None) => VectorRecallCompleteness::BoundaryTieClosed {
                fetched: self.fetched,
            },
            (TieClosure::OpenAtBound, _) => VectorRecallCompleteness::BoundaryTieOpen {
                fetched: self.fetched,
                fetch_bound: self.fetch_bound,
            },
        }
    }
}

pub(crate) async fn close_tie_cohort<F, Fut, E>(
    admitted_limit: usize,
    fetch_limit_cap: usize,
    mut fetch: F,
) -> Result<TieClosureResult, E>
where
    F: FnMut(usize) -> Fut,
    Fut: Future<Output = Result<Vec<VectorCandidateMatch>, E>>,
{
    let fetch_bound = tie_cohort_fetch_bound(admitted_limit, fetch_limit_cap);
    let mut fetch_limit = admitted_limit.saturating_add(1).min(fetch_bound);

    loop {
        let fetched = fetch(fetch_limit).await?;
        let fetched_count = fetched.len();
        let candidates = CanonicalCandidates::new(fetched);

        match fetch_decision(
            fetch_limit,
            fetch_bound,
            fetched_count,
            tie_cohort_is_closed(&candidates, admitted_limit),
        ) {
            FetchDecision::Grow(next_limit) => fetch_limit = next_limit,
            FetchDecision::Return => {
                return Ok(TieClosureResult {
                    candidates: candidates.truncated(admitted_limit),
                    fetched: fetched_count,
                    fetch_bound,
                    closure: TieClosure::Closed,
                });
            }
            FetchDecision::ReturnAtBound => {
                return Ok(TieClosureResult {
                    candidates: candidates.truncated(admitted_limit),
                    fetched: fetched_count,
                    fetch_bound,
                    closure: TieClosure::OpenAtBound,
                });
            }
        }
    }
}

fn tie_cohort_fetch_bound(limit: usize, fetch_limit_cap: usize) -> usize {
    limit
        .saturating_mul(TIE_COHORT_LIMIT_MULTIPLIER)
        .max(limit.saturating_add(TIE_COHORT_MIN_EXTRA_CANDIDATES))
        .min(fetch_limit_cap)
}

fn fetch_decision(
    fetch_limit: usize,
    fetch_bound: usize,
    fetched_count: usize,
    tie_cohort_closed: bool,
) -> FetchDecision {
    if fetched_count < fetch_limit || tie_cohort_closed {
        return FetchDecision::Return;
    }
    if fetch_limit >= fetch_bound {
        return FetchDecision::ReturnAtBound;
    }

    FetchDecision::Grow(fetch_limit.saturating_mul(2).min(fetch_bound))
}

fn tie_cohort_is_closed(candidates: &[VectorCandidateMatch], admitted_limit: usize) -> bool {
    if admitted_limit == 0 || candidates.len() <= admitted_limit {
        return false;
    }

    candidates.last().is_some_and(|tail| {
        tail.score
            .total_cmp(&candidates[admitted_limit - 1].score)
            .is_lt()
    })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::future::ready;

    use uuid::Uuid;

    use super::*;
    use crate::domain::ObjectType;
    use crate::models::vector::VectorSurface;

    fn candidate(id: u128, score: f32) -> VectorCandidateMatch {
        VectorCandidateMatch::new(
            Uuid::from_u128(id),
            ObjectType::Episode,
            VectorSurface::Summary,
            score,
        )
    }

    #[test]
    fn fetch_decision_returns_when_the_cutoff_cohort_is_closed() {
        let candidates =
            CanonicalCandidates::new([candidate(1, 1.0), candidate(2, 1.0), candidate(3, 0.5)]);

        assert_eq!(
            fetch_decision(3, 10, 3, tie_cohort_is_closed(&candidates, 2)),
            FetchDecision::Return
        );
        let result = TieClosureResult {
            candidates: candidates.truncated(2),
            fetched: 3,
            fetch_bound: 10,
            closure: TieClosure::Closed,
        };
        assert_eq!(
            result.completeness(None),
            VectorRecallCompleteness::BoundaryTieClosed { fetched: 3 }
        );
    }

    #[test]
    fn all_tied_cohort_at_fetch_bound_is_open() {
        let candidates = CanonicalCandidates::new((1..=6).rev().map(|id| candidate(id, 1.0)));

        assert_eq!(
            fetch_decision(6, 6, 6, tie_cohort_is_closed(&candidates, 2)),
            FetchDecision::ReturnAtBound
        );
        let result = TieClosureResult {
            candidates: candidates.truncated(2),
            fetched: 6,
            fetch_bound: 6,
            closure: TieClosure::OpenAtBound,
        };
        assert_eq!(
            result.completeness(None),
            VectorRecallCompleteness::BoundaryTieOpen {
                fetched: 6,
                fetch_bound: 6,
            }
        );
        assert_eq!(result.candidates[0].object_id, Uuid::from_u128(1));
        assert_eq!(result.candidates[1].object_id, Uuid::from_u128(2));
    }

    #[test]
    fn backend_fetch_cap_reports_an_open_boundary_without_allocating_rows() {
        let Ok(fetch_limit_cap) = usize::try_from(u32::MAX) else {
            return;
        };
        let Some(admitted_limit) = fetch_limit_cap.checked_add(1) else {
            return;
        };
        let fetch_bound = tie_cohort_fetch_bound(admitted_limit, fetch_limit_cap);

        assert_eq!(fetch_bound, fetch_limit_cap);
        assert_eq!(
            fetch_decision(fetch_limit_cap, fetch_bound, fetch_limit_cap, false),
            FetchDecision::ReturnAtBound
        );
        let result = TieClosureResult {
            candidates: CanonicalCandidates::new([]),
            fetched: fetch_limit_cap,
            fetch_bound,
            closure: TieClosure::OpenAtBound,
        };
        assert_eq!(
            result.completeness(Some(fetch_limit_cap)),
            VectorRecallCompleteness::BoundaryTieOpen {
                fetched: fetch_limit_cap,
                fetch_bound: fetch_limit_cap,
            }
        );
    }

    #[tokio::test]
    async fn closure_loop_grows_and_canonicalizes_before_returning() {
        let candidates = [candidate(2, 1.0), candidate(1, 1.0), candidate(3, 0.5)];
        let fetch_limits = RefCell::new(Vec::new());

        let result = close_tie_cohort(1, usize::MAX, |fetch_limit| {
            fetch_limits.borrow_mut().push(fetch_limit);
            ready(Ok::<_, ()>(
                candidates.iter().take(fetch_limit).cloned().collect(),
            ))
        })
        .await
        .unwrap();

        assert_eq!(*fetch_limits.borrow(), vec![2, 4]);
        assert_eq!(result.candidates[0].object_id, Uuid::from_u128(1));
        assert_eq!(
            result.completeness(None),
            VectorRecallCompleteness::BoundaryTieClosed { fetched: 3 }
        );
        assert_eq!(
            result.completeness(Some(3)),
            VectorRecallCompleteness::Exhaustive { scanned: 3 }
        );
    }
}
