---
status: accepted
adr_type: implementation
date: 2026-09-02
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely add a vector-layer predicate as a three-valued hint filter that matches unknown values, or let an adapter truncate an unclosed equal-score cohort without saying so, because both are the natural first implementation and both have already happened in this repository"
  detected_signals: "cross-boundary contract shape (port postcondition) with tempting alternatives; rejected alternative likely to be re-proposed; premises likely to expire (no vector-layer predicate has a caller, and stored values are not guaranteed present)"
  cost_of_violation: "a prefilter that matches unknown values admits stale candidates that graph verification then silently discards, and an unreported open cohort makes top-K membership vary between runs — both surface as unexplained retrieval nondeterminism in evaluation evidence long after the cause is forgotten"
  cost_of_wrong_preservation: "if the unknown-never-matches rule is preserved after every stored value is guaranteed present and synchronised, adapters carry a defensive arm for a case that cannot occur"
  cost_of_over_extension: "treating the completeness verdict as an error condition would fail retrieval on a determinism caveat about non-authoritative candidates"
depends_on: [implementation/ADR-I-0018-responsibility-boundary-modules-with-enforced-dependency-direction.md, implementation/ADR-I-0022-retain-measured-retrieval-defaults.md]
implements: []
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-I-0024: Vector candidate recall reports its completeness, and a vector-layer prefilter never matches an unknown value

## Context and Problem Statement

The vector candidate port promised deterministic admission: at most `limit` unique object-and-surface matches in canonical order, with equal-score cohorts at the cutoff closed before truncation (ADR-I-0022 records the fix).
The service adapter closes the cohort by growing its fetch up to a bound, but when the bound is hit it returns the truncated set with no signal, so a caller cannot tell "this top-K is determinate" from "membership may vary between runs", and evaluation evidence later attributes the variation to retrieval.
Separately, the port once carried currentness predicates implemented as match-or-unknown: a record whose payload lacked the field satisfied a positive predicate in both the service adapter and the test fake, so the filter admitted records under a rationale ("current") that was not true of them; and because the field was written only at upsert, a record whose value had since changed in graph authority was filtered on a stale value, admitted when it should not have been or excluded when it should have been returned.
Those filters were deleted as speculative when no caller used them.
A second adapter (ADR-I-0023) makes both gaps matter: below its indexing threshold an embedded shard scans exhaustively and needs a way to say so, and two adapters must agree on what a prefilter may do.

## Decision Drivers

- Retrieval rationale must be inspectable: the philosophy asks that a developer can see why a memory was or was not retrieved, and an unreported open cohort is unexplained recall.
- Candidate recall is non-authoritative; graph authority verifies every candidate, so a determinism caveat must never become a retrieval failure.
- A prefilter false negative is a memory that silently never returns — a continuity loss nobody can inspect — while a false positive costs one root slot and is discarded by verification; the asymmetry decides what a vector-layer predicate may do.
- Each port owns its stated postconditions; upper layers never repair lower-layer output.

## Decision

The search result carries, beside the canonical candidates, a completeness verdict stated by the adapter.
The verdict distinguishes four situations: no search was issued because the limit was zero or the scope was empty; every stored record in scope was scored, so the requested top-K is determinate over the population; an index answered with a prefix whose cutoff cohort was closed, so the returned set is determinate for that index state although an approximate index may have omitted records it never surfaced; and the overfetch bound was reached with the cutoff cohort still open, so membership may vary.
Adapters state the verdict truthfully: exhaustive only when the adapter knows the shard is unindexed and the cutoff cohort was closed, with the scanned count taken from the scope rather than the rows returned; an exhaustive scan whose cohort stays open at the bound reports open.
The retrieval pipeline records the verdict in retrieval telemetry beside the returned candidate count and never repairs, retries, or fails on it.

A vector-layer predicate may be evaluated only over stored values that are immutable or synchronised on every mutation, and an unknown or missing stored value never satisfies a positive predicate.
A predicate that needs a stored value the write paths do not keep current is not a prefilter; it is a graph-authority question.

## Character Memory Relevance

Recall that silently varies between runs, or that admits and excludes memories on values nobody keeps true, is the unexplained recall the philosophy forbids: a character that forgets an episode because a stale column excluded it looks like a character that never lived it, and a filter that admits on a blank gives a rationale that is false.
The verdict keeps determinism inspectable; the prefilter rule keeps a candidate stage from being the reason a memory is unreachable, and keeps every stated filter rationale true.

## Implementation Impact

- The port's search method returns candidates plus verdict; the pipeline copies the verdict into telemetry; test fakes report exhaustive.
- The service adapter's existing fetch decision maps onto the closed and open situations.
- The evaluation repository mirrors the telemetry field (ADR-I-0026).

## Considered Options

1. A completeness verdict beside the candidates, plus the prefilter rule.
2. Silent degradation at the fetch bound (as built).
3. A boolean complete flag.
4. Fail with an error when the cohort is open at the bound.
5. Resurrect the deleted hint filters for the embedded adapter, which can evaluate them exactly.

## Decision Outcome

Chosen option: **Option 1**.
It makes the postcondition expressible by the layer that owns it, distinguishes the exhaustive case an unindexed shard can report from the closed-cohort case an index can promise, and keeps every consumer a field access away from unchanged code.

### Rejected Alternatives

Option 2 hides a determinism caveat that evaluation evidence later attributes to retrieval; rejected outright.
Option 3 loses the exhaustive-versus-closed distinction that tells a caller whether population-level determinacy was achieved; rejected outright.
Option 4 fails retrieval on a caveat about non-authoritative candidates that graph authority verifies anyway; rejected outright.
Option 5 recreates a prefilter over values that only the upsert path wrote, which the rule above forbids; a predicate is admitted when its stored value is kept in sync or is immutable.

## Consequences

- Positive: top-K determinism is observable per retrieval and per adapter.
- Positive: any future prefilter has one admission test — is the value it reads always current — instead of a case-by-case argument.
- Negative / tradeoffs: callers wanting a scoped or time-bounded semantic search wait for a synchronised or immutable column rather than filtering on what happens to be stored.

## Decision Boundary

Invariant: the search result carries a completeness verdict stated truthfully by the adapter, and the pipeline never repairs, retries, or fails on it; a vector-layer predicate reads only immutable or synchronised values, and an unknown value never satisfies a positive predicate.

Not covered: the current query shape (an embedding, a limit, and an object-type scope, with an empty scope selecting zero — a current state, not a rule), the verdict's type and wire shape (the appendix is a reference, not a contract), the telemetry field name, and the service adapter's overfetch bound.

## Validation

- Unit tests on the service adapter's fetch decision assert the closed and open verdicts, including the all-tied cohort at the bound.
- A retrieval test asserts the telemetry verdict for each situation using the fakes.
- The parity suite asserts exhaustive for the embedded adapter below its indexing threshold and closed for the service adapter on the identical-vector tie fixture.
- A census of the vector adapters shows no match-or-unknown condition.

## Revisit When

- An adapter appears that cannot classify its own cutoff (a remote index without a fetch count) — the verdict may need an "unknown" situation, which must still never be treated as an error.
- Every stored value a predicate could read is guaranteed present and synchronised — the unknown arm becomes unreachable and may be removed.

## Consultation impact

Question asked: whether the deleted hint filters should return for the embedded adapter; ruling adopted the prefilter rule instead. Revised 2026-09-03 on the decider's review: the type shape is an appendix and the scope-only query is recorded as current state, not as a rule.

## More Information

- ADR-I-0022 (tie-cohort closure and canonical ordering, the postcondition this record makes expressible); ADR-I-0023 (the embedded adapter); ADR-I-0025 (the stored record a predicate would extend); ADR-I-0026 (the evaluation reader of the verdict).
- Candidate predicates that satisfy the rule, noted for whichever phase needs them and binding on none: a scope id written at upsert and kept in sync by the link and reflection write paths (scoped continuity); an immutable time window over creation and observation time (a time-bounded retrieval route).

## Appendix: reference shape (non-binding)

```rust
pub struct VectorCandidateRecall { pub candidates: CanonicalCandidates, pub completeness: VectorRecallCompleteness }

pub enum VectorRecallCompleteness {
    NotRequested,
    Exhaustive { scanned: usize },
    BoundaryTieClosed { fetched: usize },
    BoundaryTieOpen { fetched: usize, fetch_bound: usize },
}
```
