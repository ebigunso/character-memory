---
status: accepted
adr_type: implementation
date: 2026-09-02
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely add a public raw vector search to the facade for the evaluation baseline, or let evaluation tooling read a store's physical schema directly again, because each is the shortest path to a number"
  detected_signals: "externally observable contract shape with a tempting alternative; rejected alternative likely to be re-proposed; cross-repository obligation; deliberately bounded scope (no product use case for raw recall exists)"
  cost_of_violation: "a raw-recall facade makes the library a vector-database abstraction and exposes unverified candidates as if they were memory; a schema-reading baseline breaks silently the moment a second vector adapter ships a different physical schema, and it reimplements canonical ordering the library already owns"
  cost_of_wrong_preservation: "if a product use case for candidate-level recall arrives and this record is preserved as a blanket prohibition, the diagnostic surface the observability phase plans would be blocked instead of designed"
  cost_of_over_extension: "reading this record as forbidding evaluation tooling from using the trace at all would leave the baseline with no honest data source"
depends_on: [implementation/ADR-I-0020-restart-identity-via-caller-supplied-ids-not-a-lookup-surface.md, implementation/ADR-I-0024-vector-candidate-recall-reports-completeness-and-takes-a-scope-only-query.md]
implements: []
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-I-0026: Raw vector baselines read the retrieval trace; the library exposes no candidate-search surface

## Context and Problem Statement

The companion evaluation repository (a development aid, not core library functionality) runs a vector-only baseline: ingest through the library, then rank by plain vector similarity to measure what hybrid retrieval adds.
As built, that baseline held its own client to the vector service, ran one filtered search per object kind against the library's collection, read three payload fields by hard-coded name, re-implemented best-score-per-object deduplication and score ordering, and took item text from a payload column.
That is a hidden capability: the baseline depended on an adapter-private schema, duplicated ordering the library owns, and could not run at all against an embedded store (ADR-I-0023).
The question is what capability the library must expose so the baseline stops reaching into a store.

## Decision Drivers

- Evaluation tooling must not grow library surface that no product use case has demanded (ADR-I-0020's driver).
- The library is not a vector-database abstraction, and vector-only candidates must never become behavior-influencing memory without graph verification (project philosophy; the persistent-graph-authority phase's acceptance criteria).
- The retrieval trace already is the raw vector recall: the canonical, pre-verification top-K with object reference, surface, score, and rank, scoped by the configured object types and sized by the candidate limit, and ADR-I-0024 adds the completeness verdict that says whether that top-K was determinate.
- Every store's physical schema is adapter-private; two adapters must not create two baseline implementations.

## Decision

The library exposes no raw candidate-search surface and no facade change.
The evaluation repository's vector-only baseline issues one ordinary `retrieve` with tracing enabled per measured object kind, each with a singleton object-type scope, and reads each retrieval's completeness verdict from telemetry; a single mixed-kind top-K is not used, because a global cutoff can exclude an underrepresented kind's valid candidates without any open verdict.
The trace's vector candidates are object-and-surface pairs recorded before the pipeline's object-level deduplication, so the baseline's candidate limit per kind is that kind's section budget multiplied by the maximum number of embedding surfaces one object of that kind can have (a constant declared by the embedding-surface policy; one for every kind at the time of this record), and the baseline deduplicates by object keeping the best-scoring surface and truncates to the budget.
That limit is sufficient by construction: an object ranked within the budget by its best surface has that surface inside the surface top-K of budget times surfaces, because the surfaces above it belong to fewer than the budget's worth of objects; therefore a closed or exhaustive surface-level verdict at that limit makes the object-level top-budget determinate, and the contract is pinned by a fixture whose objects carry every surface.
Item text comes from the evaluation repository's own ingest records, keyed by the external identity it already reverse-maps, never from a store payload (ADR-I-0025's third sentence: consumers needing candidate content hydrate by object id).
The evaluation repository's vector-service client shrinks to collection lifecycle operations (existence and deletion), which the embedded mode replaces with file operations through the durable-store path list the adapter already maintains.

What this record asks of the evaluation repository, recorded as the library-facing contract and nothing more:

- The evaluation repository consumes the retrieval trace and the completeness telemetry as an ordinary caller; the library adds no surface for it.
- A raw-vector baseline that wants per-kind top-K uses one singleton-scoped traced retrieval per kind with the multiplied limit and object-level deduplication described above; any other reading of the trace is not covered by the parity claim.
- How the evaluation repository migrates its baseline, mirrors telemetry, labels its rows, or guards its cleanup is planned and tracked in that repository.


## Character Memory Relevance

Retrieval that bypasses graph authority is exactly the "generic RAG wrapper" and "unexplained recall" the philosophy warns against; the trace exists so that every candidate a developer sees is one the library can explain, whether it was admitted or not.
Keeping the baseline inside the traced retrieval path means the measurement of "what does the graph add" is taken from the same recall the character actually experiences, not from a parallel search that may drift from it.

## Implementation Impact

- Library: none beyond ADR-I-0024's telemetry field; the acceptance criterion "no public facade change" holds.
- Evaluation repository: its baseline moves onto the trace under its own plan; nothing in this repository depends on how.

## Considered Options

1. The baseline consumes the retrieval trace; no library surface.
2. A public candidate-recall method on the facade returning references, surfaces, and scores.
3. A retrieval mode that skips graph verification.
4. Publish the payload manifest so evaluation tooling can keep reading the store.

## Decision Outcome

Chosen option: **Option 1**.
It holds the no-facade-change line, covers every vector adapter automatically, deletes a duplicate implementation, and takes the measurement from the recall the character actually experiences.

### Rejected Alternatives

Option 2 is an evaluation-driven surface with no product use case and a vector-database-abstraction shape; it is reopened only by a product use case for candidate-level recall, at which point it lands as a designed diagnostic surface in the retrieval-observability phase, not as a search method.
Option 3 contradicts the acceptance criterion that candidates whose graph objects are missing are rejected from normal retrieval; rejected outright.
Option 4 leaves two implementations of one capability and breaks with the first adapter whose physical schema differs; rejected outright.

## Consequences

- Positive: one retrieval entry point; both adapters covered; the baseline reports the completeness of the top-K it measured.
- Negative / tradeoffs: the baseline pays for graph expansion it discards, an evaluation-run cost accepted in exchange for not inventing a second retrieval path.
- Negative / tradeoffs: the baseline issues one traced retrieval per measured kind instead of one search, so its cost scales with the number of kinds; if a kind's budget cannot be closed within the service adapter's fetch bound, that retrieval's completeness verdict says so and the run records it.

## Decision Boundary

Invariant: no public raw candidate-search surface; evaluation baselines consume the retrieval trace and telemetry; store schemas are adapter-private; candidate content is hydrated by object id from the consumer's own records or graph authority.

Not covered: any headroom the baseline adds to a kind's limit, the shape of the evaluation repository's ingest record store, and any future diagnostic surface the observability phase designs on product demand.

## Validation

- An A/B run of the vector-only configuration (direct search versus trace-derived) shows identical item identities and ranks per question before the direct path is deleted.
- After the switch, the evaluation adapter contains no search call against the vector service and no payload field constant.
- The baseline runs unchanged in embedded mode.

## Revisit When

- A product use case demands candidate-level recall — design a diagnostic surface in the retrieval-observability phase and supersede this record's prohibition for that surface only.
- One retrieval per kind proves too costly on a real dataset, or a kind's budget cannot be closed within the service adapter's fetch bound — reopen whether the port needs per-type limits in a single query.

## Consultation impact

Question asked: trace-derived baseline versus a public candidate-recall method; ruling adopted the trace as recommended.

## More Information

- ADR-I-0020 (the precedent: evaluation needs met without a lookup surface).
- ADR-I-0024 (the completeness verdict the baseline reads), ADR-I-0025 (why item text is not a payload column).
- The companion evaluation repository's vector-only baseline plan (historical record of the direct-search implementation this record replaces).
