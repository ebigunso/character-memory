---
status: accepted
adr_type: implementation
date: 2026-09-04
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely restore a three-valued vector-layer hint predicate that matches unknown values because that shape appears to preserve recall, repeating the pre-ADR-I-0028 behavior"
  detected_signals: "cross-boundary contract shape (prefilter admission across two adapters); rejected alternative likely to be re-proposed; meaningful backfill and synchronisation cost; scope boundary is deliberate because graph authority may reason over incomplete knowledge"
  cost_of_violation: "a prefilter over missing or stale values silently excludes reachable memories or admits them under a false rationale, producing continuity loss and misleading retrieval evidence that are expensive to diagnose"
  cost_of_wrong_preservation: "if a stored predicate column is proven fully populated and cannot become unknown, preserving a special unknown-handling branch adds unreachable policy to both adapters"
  cost_of_over_extension: "applying this vector-candidate admission rule to graph-authority reasoning would confuse incomplete domain knowledge with a corrupt denormalized index column"
depends_on: [implementation/ADR-I-0024-vector-candidate-recall-reports-completeness-and-prefilters-never-match-unknown.md, implementation/ADR-I-0025-vector-record-is-a-read-contract.md]
implements: []
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-I-0028: A vector-layer prefilter reads only a fully populated, immutable or synchronised column, and an unknown value never matches

## Context and Problem Statement

The vector candidate port previously carried currentness predicates implemented as match-or-unknown: a record whose payload lacked the field satisfied a positive predicate in both the service adapter and the test fake, so the filter admitted records under a rationale that was not true of them.
Those fields were written only at upsert. A value changed in graph authority could therefore remain stale in the vector payload, causing the prefilter to admit a record that should be excluded or silently exclude a memory that should remain reachable.
The speculative filters were deleted when no caller used them. The five-field read contract in ADR-I-0025 deliberately leaves a re-entry path for a justified predicate, so both adapters need one durable admission rule for any additional prefilter column.

## Decision Drivers

- A prefilter false negative is a memory that silently never returns, while a false positive consumes candidate capacity and presents a rationale that is not true.
- A denormalized predicate column is trustworthy only if every searchable record has a value and every mutation preserves it, or if the value cannot change.
- Both vector adapters implement the same port and must apply the same predicate semantics.
- Graph authority verifies candidates and remains the correct layer for questions that cannot be represented by a complete, current vector column.

## Decision

A vector-layer predicate may read a column only after the column is fully populated for every searchable record, including any required backfill from graph authority before the predicate is enabled, and only when the value is immutable or synchronised on every mutation.
Under those conditions a missing or unknown value is a defect, not an admissible state, and it never satisfies a positive predicate. This rule produces no false negative on a correctly populated column and turns an incorrectly populated one into a visible failure rather than silent widening.
A predicate that needs a value the write paths do not keep current, or that is not populated for every searchable record, remains a graph-authority question and is not a vector-layer prefilter.
Any admitted predicate lands in both adapters with a parity fixture.

## Character Memory Relevance

A character that cannot retrieve an episode because a stale hint excluded it appears to have never lived it, while a blank value admitted as a match gives a false explanation for recall.
Requiring complete and current predicate columns keeps candidate recall from becoming the hidden reason a memory is unreachable and keeps every stated filter rationale true.

## Implementation Impact

- The v0.1.6 object-type scope satisfies the rule because every vector record carries its object type and that identity field does not mutate.
- An additional vector predicate owns its column, graph-authority backfill, mutation synchronisation when applicable, schema step, both adapter mappings, and parity evidence as one change.
- ADR-I-0025 remains the record contract; a returning column earns its place through a concrete predicate and reader rather than speculative storage.

## Considered Options

1. Require complete population plus immutability or synchronisation, and never match unknown values.
2. Match unknown values to avoid false negatives.
3. Reject unknown values but allow predicate columns without a backfill prerequisite.
4. Keep graph-derived hints in every vector record in anticipation of possible predicates.

## Decision Outcome

Chosen option: **Option 1**.
It makes a prefilter's rationale true at the storage boundary, prevents silent exclusion from incomplete backfills, and gives both adapters one testable admission contract.

### Rejected Alternatives

Option 2 widens results under a rationale the stored value does not establish and hides incomplete population; rejected outright.
Option 3 permits pre-existing searchable records to disappear from results as soon as the predicate is enabled; rejected outright.
Option 4 mirrors unread, stale state across two adapters without a consumer and repeats the payload design that ADR-I-0025 replaced; rejected outright.

## Consequences

- Positive: a vector prefilter cannot silently rely on missing or stale denormalized state.
- Positive: the column, its reader, and the work that keeps it trustworthy enter together.
- Negative / tradeoffs: a scoped or time-bounded semantic search must pay the backfill and synchronisation or immutability cost before it can filter in the vector layer.

## Decision Boundary

Invariant: a vector-layer prefilter reads only a fully populated column that is immutable or synchronised on every mutation, and an unknown value never satisfies a positive predicate.

Not covered: graph-authority query semantics, the v0.1.6 object-type scope shape, or the physical encoding of an admitted column in either adapter.

## Validation

- A census of both vector adapters shows no match-or-unknown condition.
- Any admitted predicate has a pre-enablement backfill or proof of complete population, a mutation-consistency design, and a parity fixture across both adapters.

## Revisit When

- Every value a predicate can read is structurally guaranteed present and cannot become unknown — the explicit unknown arm may be removed as unreachable while the complete-population rule remains.
- A vector backend cannot expose missing values distinctly enough to enforce the rule — that backend's admission evidence is re-derived before the predicate is implemented.

## Consultation impact

Question asked: whether the deleted hint filters should return for the embedded adapter; the ruling adopted this admission rule instead. The prefilter decision was separated from ADR-I-0024 on 2026-09-04 so each record has one governing claim.

## More Information

- ADR-I-0024 governs vector recall completeness and telemetry; ADR-I-0025 governs the five-field vector record.
- Candidate predicates that satisfy this rule, noted for whichever version needs them and binding on none: a scope id written at upsert and kept in sync by the link and reflection write paths; an immutable time window over `created_at` and `observed_at`, backfilled from graph authority before enablement.
