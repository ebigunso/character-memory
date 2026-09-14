---
status: accepted
adr_type: implementation
date: 2026-09-14
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely add a re-canonicalisation, re-filtering, or de-duplication pass above a port as a cheap defensive fix, because it is the shortest change when an adapter misbehaves"
  detected_signals: "cross-boundary contract shape (port postconditions across every adapter); rejected alternative likely to be re-proposed; a decider's ruling setting a durable governance default"
  cost_of_violation: "a repair pass above a port masks the adapter defect it compensates for, duplicates policy in two layers that then drift, and makes the port's stated contract unverifiable because nothing depends on it"
  cost_of_over_extension: "treating every use-case invariant as a port postcondition pushes policy into adapters that should stay ignorant of it"
depends_on: [implementation/ADR-I-0018-responsibility-boundary-modules-with-enforced-dependency-direction.md]
implements: []
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-I-0030: Each port owns its stated postconditions and no upper layer repairs lower-layer output

## Context and Problem Statement

The state that the structured-verdict observability plan (`docs/coding-agent/plans/completed/structured-verdict-observability-plan.md`, July 2026) found was this: the retrieval pipeline re-canonicalised candidates the vector store returned as canonical, the retrieval assembly re-evaluated lifecycle filtering that graph expansion declared, removed contradictory omissions, and de-duplicated decisions a second time, and the graph selector post-filtered and re-sorted rows whose query owned those predicates. Each pass existed because an adapter failed to satisfy a postcondition the port stated, and the caller compensated instead of the adapter being fixed. The passes hid those adapter defects, duplicated policy in two places that could drift apart, and left the port contracts untested, since the pipeline no longer depended on them.

## Decision Drivers

- A port contract that nothing relies on cannot be verified and will decay.
- Silent repair converts an adapter bug into an invisible performance and correctness tax paid by every caller.
- Policy that lives in two layers drifts; the layer that declares a guarantee must be the one that enforces it.
- The module dependency direction (ADR-I-0018) places adapters below use cases; the postcondition ownership follows that direction.

## Decision

Each port owns the postconditions it states, and no layer above a port repairs, re-filters, re-sorts, or de-duplicates that port's output.

- A postcondition is enforced at the boundary that states it, by one of two mechanisms chosen per site by cost: a result type whose only constructor establishes the property (so an adapter cannot return a value that violates it), or a contract test that every adapter passes. Test code holds no second implementation of a port: a test double that records calls or injects failures either wraps a real adapter or returns only its injected failure, and makes no postcondition claim of its own.
- Canonical vector candidates are established by the constructor of the candidate result type; lifecycle filtering of graph expansion is owned by graph expansion as declared by its query; graph object selection owns its reference, identifier, type predicates and ordering in the query itself, a postcondition each adapter establishes independently.
- When an adapter violates a postcondition, the fix is in the adapter and its contract test, never a compensating pass above the port.
- Query semantics are total: an empty targeted selection selects nothing in every adapter, and no query silently widens to everything.

## Character Memory Relevance

Recall must be explainable from the trace. A candidate list altered by a repair pass after the trace was recorded, or a filter applied twice with different rules, produces recall whose reason cannot be read from the evidence.

## Implementation Impact

- Port contract tests run against every adapter; adding an adapter means passing them, not adding a pass above the port.
- A use case that needs a property the port does not promise asks for the property to become a stated postcondition rather than repairing locally.
- The candidate result type keeps its sole constructor; its shape may be absorbed into a result envelope without loosening the property.

## Considered Options

1. Ports own their postconditions, enforced by constructor or contract test; no repair above.
2. Defensive repair passes above every port.
3. Postconditions stated but unenforced, relying on adapter discipline.

## Decision Outcome

Chosen option: **Option 1**. It makes every port contract load-bearing and therefore tested, and keeps each policy in exactly one layer.

### Rejected Alternatives

Option 2 masks adapter defects, duplicates policy, and taxes every caller; rejected outright.

Option 3 is the state that produced the repair passes; a stated but unenforced postcondition is violated the first time an adapter is written by someone who did not read it; rejected outright.

## Consequences

- Positive: adapter defects surface where they are, in contract tests.
- Positive: pipeline code reads as composition of guaranteed results.
- Negative / tradeoffs: a new postcondition costs a constructor or a contract test rather than a local fix.

## Decision Boundary

Invariant: no layer above a port re-establishes a property the port states. Changing this requires a superseding record.

Not covered: which properties each port states (recorded in the port's documentation and contract tests); the choice between constructor and contract test at any given site; use-case invariants that no port promises, which the use case owns.

## Validation

- Every stated postcondition is validated either by a constructor that establishes it or by contract tests every adapter passes. The vector port has a shared suite (`tests/vector_port_contract_tests.rs`). Graph postconditions are validated by the Oxigraph adapter tests over the in-memory store (`src/adapters/oxigraph/tests.rs`), which is also a production configuration. The library keeps no second implementation of the graph port's postconditions in test code, so no parity suite is needed. Test doubles that only record calls make no postcondition claim.
- Review rejects a canonicalisation, filtering, ordering, or de-duplication pass above a port whose contract states the property.

## Revisit When

A port must serve adapters that cannot establish a property at all (for example a remote backend with non-deterministic ordering that cannot be closed); the answer then is to weaken the stated postcondition and move the property to the layer that can own it, recorded in a superseding record.

## More Information

- The graph test fake was deleted under this record because a second implementation of a port's postconditions in test code is the drift this record forbids.
- Design history: the structured-verdict observability phase (`docs/coding-agent/plans/completed/structured-verdict-observability-plan.md`), finding R2-09 in the Task_2 description and in the finding-disposition table of the appendix, which record the deletion of the three repair passes.
- ADR-I-0029 (structured outcomes are authoritative) records the other durable ruling of the structured-verdict observability plan named above.
