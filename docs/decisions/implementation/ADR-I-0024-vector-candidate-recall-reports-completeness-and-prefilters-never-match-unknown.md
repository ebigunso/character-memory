---
status: accepted
adr_type: implementation
date: 2026-09-02
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely let an adapter truncate an unclosed equal-score cohort without reporting that loss of determinacy, because the service adapter did so before ADR-I-0024"
  detected_signals: "cross-boundary contract shape (port postcondition) with tempting alternatives; rejected alternative likely to be re-proposed; premises likely to expire when an adapter cannot classify its own cutoff"
  cost_of_violation: "an unreported open cohort makes top-K membership vary between runs and surfaces as unexplained retrieval nondeterminism in evaluation evidence after the cause is forgotten"
  cost_of_wrong_preservation: "if an adapter cannot distinguish an exhaustive population from an index-produced prefix, preserving the four-way vocabulary without an unknown verdict would force false precision"
  cost_of_over_extension: "treating the completeness verdict as an error condition would fail retrieval on a determinism caveat about non-authoritative candidates"
depends_on: [implementation/ADR-I-0018-responsibility-boundary-modules-with-enforced-dependency-direction.md, implementation/ADR-I-0022-retain-measured-retrieval-defaults.md]
implements: []
supersedes: []
superseded_by: implementation/ADR-I-0028-vector-prefilters-require-fully-populated-current-columns-and-never-match-unknown.md
supersession_scope: partial   # the prefilter-admission clause moved to ADR-I-0028; this record stays authoritative for completeness reporting
---

# ADR-I-0024: Vector candidate recall reports its completeness

## Context and Problem Statement

The vector candidate port promised deterministic admission: at most `limit` unique object-and-surface matches in canonical order, with equal-score cohorts at the cutoff closed before truncation (ADR-I-0022 records the fix).
The service adapter closes the cohort by growing its fetch up to a bound, but when the bound is hit it returns the truncated set with no signal, so a caller cannot tell "this top-K is determinate" from "membership may vary between runs", and subsequent evaluation evidence attributes the variation to retrieval.
A second adapter (ADR-I-0023) makes the gap matter in another direction: below its indexing threshold an embedded shard scans exhaustively and needs a way to report population-level determinacy.

## Decision Drivers

- Retrieval rationale must be inspectable: the philosophy asks that a developer can see why a memory was or was not retrieved, and an unreported open cohort is unexplained recall.
- Candidate recall is non-authoritative; graph authority verifies every candidate, so a determinism caveat must never become a retrieval failure.
- Each port owns its stated postconditions; upper layers never repair lower-layer output.

## Decision

The search result carries, beside the canonical candidates, a completeness verdict stated by the adapter.
The verdict distinguishes four situations: no search was issued because the limit was zero or the scope was empty; every stored record in scope was scored, so the requested top-K is determinate over the population; an index answered with a prefix whose cutoff cohort was closed, so the returned set is determinate for that index state although an approximate index may have omitted records it never surfaced; and the overfetch bound was reached with the cutoff cohort still open, so membership may vary.
Adapters state the verdict truthfully: exhaustive only when every record in the requested scope was scored through a path the adapter knows to be exhaustive (an unindexed scan or a full-scope scroll such as the zero-norm path) and the cutoff cohort was closed; `scanned` is the number of scoped records actually scored, never a truncated prefix. An exhaustive path whose cohort stays open at the bound reports open.
The retrieval pipeline records the verdict in retrieval telemetry beside the returned candidate count and never repairs, retries, or fails on it.
Degenerate vectors are defined on both sides of the port so that neither adapter has an undefined path: a zero-norm record embedding is rejected at indexing as a typed per-record failure before any adapter sees it, and a zero-norm query scores every candidate zero and reports a truthful verdict.

## Character Memory Relevance

Recall that silently varies between runs is the unexplained recall the philosophy forbids: a character that returns a different member of an equal-score cohort without disclosing the open boundary appears inconsistent for no inspectable reason.
The verdict keeps population-level and prefix-level determinacy visible without making non-authoritative candidate recall a failure.

## Implementation Impact

- The port's search method returns candidates plus verdict; the pipeline copies the verdict into telemetry; test fakes report exhaustive.
- The service adapter's existing fetch decision maps onto the closed and open situations.
- The evaluation repository mirrors the telemetry field (ADR-I-0026).

## Considered Options

1. A completeness verdict beside the candidates.
2. Silent degradation at the fetch bound (as built before this record).
3. A boolean complete flag.
4. Fail with an error when the cohort is open at the bound.

## Decision Outcome

Chosen option: **Option 1**.
It makes the postcondition expressible by the layer that owns it, distinguishes the exhaustive case an unindexed shard can report from the closed-cohort case an index can promise, and keeps every consumer a field access away from unchanged code.

### Rejected Alternatives

Option 2 hides a determinism caveat that subsequent evaluation evidence attributes to retrieval; rejected outright.
Option 3 loses the exhaustive-versus-closed distinction that tells a caller whether population-level determinacy was achieved; rejected outright.
Option 4 fails retrieval on a caveat about non-authoritative candidates that graph authority verifies anyway; rejected outright.

## Consequences

- Positive: top-K determinism is observable per retrieval and per adapter.
- Negative / tradeoffs: the verdict exposes a backend boundary that consumers may need to retain in telemetry even when they do not act on it.

## Decision Boundary

Invariant: the search result carries a completeness verdict stated truthfully by the adapter, and the pipeline never repairs, retries, or fails on it.

Not covered: the query shape established for v0.1.6, the verdict's type and wire shape (the appendix is a reference, not a contract), the telemetry field name, the service adapter's overfetch bound, and vector-layer prefilter admission (ADR-I-0028).

## Validation

- Unit tests on the service adapter's fetch decision assert the closed and open verdicts, including the all-tied cohort at the bound.
- A retrieval test asserts the telemetry verdict for each situation using the fakes.
- The parity suite asserts exhaustive for the embedded adapter below its indexing threshold and closed for the service adapter on the identical-vector tie fixture; the zero-norm parity fixture asserts exhaustive for both adapters because each scores the full requested scope.
- Parity fixtures cover the zero-norm query and the rejected zero-norm record in both adapters.

## Revisit When

- An adapter appears that cannot classify its own cutoff (a remote index without a fetch count) — the verdict may need an "unknown" situation, which must still never be treated as an error.

## Consultation impact

Question asked: how both adapters should expose an unclosed cutoff and an exhaustive population; the ruling adopted a typed verdict that remains telemetry rather than control flow. Revised 2026-09-03 on the decider's review: the type shape is an appendix and the scope-only query is recorded as the v0.1.6 state, not as a rule. Revised 2026-09-04 to separate the prefilter decision into ADR-I-0028.

## More Information

- ADR-I-0022 (tie-cohort closure and canonical ordering, the postcondition this record makes expressible); ADR-I-0023 (the embedded adapter); ADR-I-0025 (the stored vector record); ADR-I-0026 (the evaluation reader of the verdict); ADR-I-0028 (vector-layer prefilter admission).

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
