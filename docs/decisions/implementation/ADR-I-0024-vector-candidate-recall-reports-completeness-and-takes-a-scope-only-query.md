---
status: accepted
adr_type: implementation
date: 2026-09-02
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely add a vector-layer predicate as a three-valued hint filter that matches unknown values, or let an adapter truncate an unclosed equal-score cohort without saying so, because both are the natural first implementation and both have already happened in this repository"
  detected_signals: "cross-boundary contract shape (port postcondition) with tempting alternatives; rejected alternative likely to be re-proposed; premises likely to expire (no vector-layer predicate is needed yet)"
  cost_of_violation: "a prefilter that matches unknown values admits stale candidates that graph verification then silently discards, and an unreported open cohort makes top-K membership vary between runs — both surface as unexplained retrieval nondeterminism in evaluation evidence long after the cause is forgotten"
  cost_of_wrong_preservation: "if a retrieval route needs a scoped or time-bounded semantic search and the scope-only query is preserved as a rule rather than a current state, retrieval will starve at scale (an unfiltered top-K contains only the in-scope fraction) and callers will overfetch instead of adding the predicate"
  cost_of_over_extension: "treating the completeness verdict as an error condition would fail retrieval on a determinism caveat about non-authoritative candidates"
depends_on: [implementation/ADR-I-0018-responsibility-boundary-modules-with-enforced-dependency-direction.md, implementation/ADR-I-0022-retain-measured-retrieval-defaults.md]
implements: []
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-I-0024: Vector candidate recall reports completeness and takes a scope-only query

## Context and Problem Statement

The vector candidate port promised deterministic admission: at most `limit` unique object-and-surface matches in canonical order, with equal-score cohorts at the cutoff closed before truncation (ADR-I-0022 records the fix).
The service adapter closes the cohort by growing its fetch up to a bound, but when the bound is hit it returns the truncated set with no signal, and the port's result type — a bare candidate list — cannot carry the difference between "this top-K is determinate" and "membership may vary between runs".
Separately, the port once carried a filter type whose currentness predicates were `Option<bool>` values implemented as match-or-unknown: a record whose payload lacked the field satisfied a positive predicate in both the service adapter and the test fake.
Those filters were deleted as speculative in the structured-verdict phase because no caller used them; the query is now an embedding, a limit, and an object-type scope.
An embedded adapter (ADR-I-0023) makes the gap visible: an exact scan is exhaustive by construction and needs a way to say so, and a second adapter needs a query contract that cannot drift.

## Decision Drivers

- Each port owns its stated postconditions; upper layers never repair lower-layer output (the structured-verdict contract's ruling), so completeness must be stated by the adapter, not inferred by the pipeline.
- Candidate recall is non-authoritative; graph authority verifies every candidate, so a determinism caveat must never become a retrieval failure.
- Prefilter false negatives are unrecoverable while false positives cost one root slot, so a vector-layer predicate is only safe on data that is immutable or synchronised on every mutation.
- Two adapters must be held to one query contract with one parity suite.
- The retrieval telemetry and trace vocabulary is the one API surface ports may import (ADR-I-0018), and it is where callers already read the returned candidate count.

## Decision

`search_candidates` returns a result envelope: the canonical candidates (the constructor-owned canonical newtype survives as the field type) together with a typed completeness verdict.

```rust
pub enum VectorRecallCompleteness {
    NotRequested,                                        // the limit was zero; no search was issued
    Exhaustive { scanned: usize },                       // every stored record in scope was scored
    BoundaryTieClosed { fetched: usize },                // an index returned a prefix and the cutoff cohort was closed
    BoundaryTieOpen { fetched: usize, fetch_bound: usize }, // the overfetch bound was reached with the cohort open
}
```

Adapters own canonicalisation and must state the verdict truthfully: not requested only when the limit is zero and no search was issued (an admitted configuration, so the verdict must be total over it), exhaustive only when the whole scoped population was scored, closed only when the cutoff cohort was verified closed or the index returned fewer rows than asked, open at the bound.
The retrieval pipeline records the verdict in retrieval telemetry beside the returned candidate count and never repairs, retries, or fails on it.
The verdict type lives in the public retrieval telemetry vocabulary so the port can name it without a mirror type.

The query is the embedding, the limit, and an object-type scope, and nothing else.
An empty scope selects zero candidates; wildcard-on-empty is prohibited, matching the graph query rule, and the retrieval context rejects an empty configured object-type set at the boundary.
Zero-norm vectors are defined on both sides of the port: the vector indexing service rejects a zero-norm record embedding as a typed per-record indexing failure before any adapter sees it (so adapters may normalise at write without a division-by-zero path), and a zero-norm query scores every candidate zero and returns a truthful verdict; the parity suite carries both cases.
Three-valued hint predicates are prohibited.
Any future vector-layer predicate arrives as an explicit enum whose unknown arm is spelled out, an unknown or missing stored value never satisfies a positive predicate, and the predicate lands with its mapping in both adapters and a parity fixture in the same change.

Two re-entry paths are named now so the scope-only query is read as a current state, not a rule:

1. A synchronised scope predicate, owned by the scoped-continuity phase: a scope-id column written at upsert and kept in sync by the link and reflection write paths, because the existing relationship hints were frozen at upsert and never updated by linking, which made them unusable as a prefilter.
2. An immutable time-window predicate over `created_at` and `observed_at`, owned by whichever phase first ships a time-bounded retrieval route: immutability makes a write-time column correct without a sync path, and the columns are backfilled from graph authority if they are ever needed (ADR-I-0025).

## Implementation Impact

- The port trait's search method changes its return type; the pipeline reads the candidates field and copies the verdict into telemetry; the test fakes wrap their existing value in the exhaustive variant.
- The service adapter's fetch-decision enum maps one-to-one onto the closed and open variants.
- Retrieval telemetry gains a completeness field with a manual default; the companion evaluation repository mirrors the field in its telemetry record (ADR-I-0026 records the obligation).
- The port doc comment stops describing a "documented bounded-overfetch degradation policy" because the type now says it.

## Considered Options

1. A typed completeness verdict in a result envelope; scope-only query with the predicate rule and named re-entry paths.
2. Silent degradation at the fetch bound (as built).
3. A boolean `complete` flag on the result.
4. Fail closed with an error when the cohort is open at the bound.
5. Resurrect the deleted hint filters for the embedded adapter, which can evaluate them exactly.

## Decision Outcome

Chosen option: **Option 1**.
It makes the postcondition expressible by the type that owns it, distinguishes the exhaustive case the embedded adapter introduces from the closed-cohort case the service adapter can promise, and keeps every consumer a field access away from unchanged code.

### Rejected Alternatives

Option 2 hides a determinism caveat that evaluation evidence later attributes to retrieval; rejected outright.
Option 3 loses the exhaustive-versus-closed distinction and the fetch counts that explain overfetch cost; rejected outright.
Option 4 fails retrieval on a caveat about non-authoritative candidates that graph authority verifies anyway; rejected outright.
Option 5 recreates a prefilter over hints that no write path other than upsert keeps in sync; it is reopened only through the named re-entry paths, each of which brings its own synchronisation obligation.

## Consequences

- Positive: top-K determinism is observable per retrieval; the parity suite can assert the verdict per adapter.
- Positive: the query contract is small enough to hold two adapters to by set equality.
- Negative / tradeoffs: callers that need scoped or time-bounded semantic recall must wait for the named predicate rather than overfetching; the re-entry paths exist to make that wait short and the shape predictable.

## Decision Boundary

Invariant: the search result carries a typed completeness verdict stated by the adapter; the pipeline never repairs or fails on it; the query carries no three-valued predicate; a new predicate lands in both adapters with a parity fixture.

Not covered: the service adapter's overfetch bound constants (calibrated values), the exact telemetry field name, and the internal fetch-decision mechanics.

## Validation

- Unit tests on the service adapter's fetch decision assert the mapping to closed and open verdicts, including the all-tied cohort at the bound.
- A retrieval test asserts the telemetry verdict for each variant using the fakes.
- The parity suite asserts exhaustive for the embedded adapter and closed for the service adapter on the identical-vector tie fixture.
- A census of the vector adapters shows no match-or-unknown condition and no filter type beyond the object-type scope.

## Revisit When

- A retrieval route needs a scoped or time-bounded semantic search — take the matching re-entry path above rather than reopening the predicate rule.
- An adapter appears that cannot classify its own cutoff (for example a remote index without a fetch count) — the verdict vocabulary may need a variant for "unknown", which must still never be treated as an error.

## Consultation impact

Question asked: whether the deleted hint filters should return for the embedded adapter; ruling adopted the scope-only query with the two named re-entry paths as recommended.

## More Information

- ADR-I-0022 (tie-cohort closure and canonical ordering at the adapter boundary, the postcondition this record makes expressible).
- ADR-I-0025 (the stored record whose columns the re-entry paths would extend).
- ADR-I-0023 (the embedded adapter that always reports exhaustive completeness).
