---
status: accepted
adr_type: implementation
date: 2026-09-02
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely either treat the embedded vector store as a test convenience and let it drift from the service adapter's contract, or reach for an embedded approximate-nearest-neighbour library the moment a corpus feels large, discarding the exact-scan determinism the parity contract depends on"
  detected_signals: "cross-boundary contract shape with a tempting alternative; rejected alternative likely to be re-proposed; premises likely to expire (corpus scale, in-process build of the service backend); deliberately bounded scope (single process, opt-in default)"
  cost_of_violation: "two vector adapters with different admission semantics silently produce different continuity packs from the same memory, which the evaluation suite would attribute to retrieval regressions rather than backend divergence"
  cost_of_wrong_preservation: "once corpora exceed the exact-scan guidance or the service backend ships a stable in-process build, keeping the exact scan as the only embedded option would make local deployments slow for no contractual reason"
  cost_of_over_extension: "applying the single-process expectation to multi-replica deployments, or treating the embedded mode as the validated default before parity evidence exists, misrepresents what the library has validated"
depends_on: [implementation/ADR-I-0009-use-sqlite-as-default-retrieval-stats-store.md, implementation/ADR-I-0021-embedded-persistent-oxigraph-default.md]
implements: []
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-I-0023: Embedded SQLite exact-scan vector candidate store as the opt-in local mode

## Context and Problem Statement

After the embedded persistent graph store became the validated default (ADR-I-0021) and retrieval statistics were already file-backed (ADR-I-0009), the vector candidate store was the only component that still required an external service.
That conflicts with the intended deployment shapes: desktop companions and game or simulation characters run on end-user machines where a container runtime cannot be assumed, and it keeps a service dependency in the default test path.
ADR-I-0003's own revisit clause, "operating two stores becomes too heavy for target users", was recorded as triggered at the close of the eval-driven family closeout.
The vector layer is candidate recall only: the vector store suggests, retrieval statistics guide fanout, and graph authority decides final inclusion, so an embedded adapter has a low correctness bar — it must prefilter and rank candidates well, never be authoritative for anything.

## Decision Drivers

- Zero-infrastructure local deployment is a product requirement, not a convenience.
- The embedded adapter must satisfy the same port contract as the service adapter, proven by a shared parity suite; anything less makes the evaluation suite an unreliable regression instrument.
- No heavyweight dependency for a first implementation; `rusqlite` with the bundled engine is already a dependency through the statistics store.
- Exact scan at character-memory scale (tens of thousands of vectors) is honest, deterministic, and strictly better recall than approximate search.
- Defaults must match validation evidence (ADR-I-0021's rule); flipping the default before parity evidence exists would repeat the mistake that ADR-I-0021 corrected.

## Decision

Add an embedded vector candidate store mode behind the existing vector candidate port, implemented as a SQLite-backed exact cosine scan, selected by a dedicated store-mode setting.
The service adapter remains fully supported as the service and cloud mode; this decision adds a mode and deprecates nothing.
The default mode stays service until the parity suite and the evaluation suite have produced identical results across modes; flipping the default is a separate, evidence-gated decision.
The embedded store is single-process, matching the embedded graph store's expectation.

Configuration follows the one-key-per-backend pattern the graph and statistics stores already use: a mode setting (`service` or `embedded`) plus a path setting read only in embedded mode, with the service connection string required only in service mode.
The path names a directory; each collection is one SQLite file inside it, named by the collection name the public constructor already takes, so the constructor's collection name is the backend-neutral namespace key in both modes (a server has collections, a directory has files).

Physical shape: one table keyed by object id and surface, one column per field of the vector record read contract (ADR-I-0025), the embedding stored as a fixed-width little-endian floating-point blob normalised at write, an index on object type for the scope predicate, and a metadata table recording vector size, distance, and schema version so a reopened file is validated against the configured embedding model with the same compatibility error the service adapter raises for a mismatched collection.
Search is a scan of the scoped rows scored by dot product in a fixed order, canonicalised by the shared constructor the port requires, and truncated to the requested limit; the result always reports exhaustive completeness (ADR-I-0024).

## Implementation Impact

- A new adapter module implementing the vector candidate port; the composition root gains a mode switch mirroring the statistics-store switch.
- The settings type gains the mode and path keys; the service connection string becomes optional and is validated as present only in service mode.
- The vector database error vocabulary gains an engine-error kind for the embedded backend and reuses the existing filesystem and payload-shape kinds; the vocabulary is closed, so the companion evaluation repository's exhaustive conversion is updated in the same wave.
- The port-conformance parity suite lives in the library's integration tests and runs against the embedded adapter unconditionally and against the service adapter when a service is configured.
- The deterministic test fake that reimplements cosine scoring and scope filtering is retired in favour of the embedded adapter opened in memory, which removes the vector-service dependency from the default test path.
- Documentation states the single-process expectation, the measured corpus-size guidance, and rebuild-from-graph-authority as the path between modes.

## Considered Options

1. SQLite exact cosine scan behind the existing port, opt-in, service mode retained.
2. An embedded approximate-nearest-neighbour library as the first embedded implementation.
3. The in-process build of the service backend.
4. An in-memory-only embedded store.
5. Flip the default to embedded in the same change.

## Decision Outcome

Chosen option: **Option 1**.
It reuses an existing dependency, gives exact filter and ranking semantics that make the parity contract checkable by set equality, is deterministic by construction, and is restart-safe through an ordinary file.

### Rejected Alternatives

Option 2 adds a heavyweight dependency and approximate membership semantics before any corpus has demonstrated that exact scan is on the critical path; it is the recorded escalation path, reopened by a measured corpus exceeding the published exact-scan guidance or a benchmark showing the scan dominating retrieval latency.
Option 3 was not stable at decision time; it is a revisit candidate once it ships a stable release, because it would maximise reuse of the service adapter's conventions.
Option 4 fails restart safety, which the persistent-graph-authority phase made a requirement for every store that survives a process.
Option 5 contradicts the defaults-match-evidence rule; it is reopened by the evidence named under Revisit When.

## Consequences

- Positive: a fully self-contained local deployment exists; the default test path needs no running service; both adapters are held to one contract by one suite.
- Positive: the embedded adapter is the honest reference implementation for the port's semantics because it computes them exactly.
- Negative / tradeoffs: exact scan is linear in corpus size and reads every scoped embedding per query; the guidance number is measured, not engineered around.
- Negative / tradeoffs: two adapters must be kept in parity for every port change; the parity suite is the cost of that guarantee.

## Decision Boundary

Invariant: the embedded adapter implements the same port contract as the service adapter and is proven by the shared parity suite; the store is single-process; the mode is selected by configuration, never inferred from the connection string.

Not covered: the corpus-size guidance number (measured and revised through documentation), the choice to add an in-memory embedding cache in front of the scan (an implementation optimisation), and the default mode (a separate evidence-gated decision).

## Validation

- Embedded mode constructs without any running service and survives process restart with identical search results.
- The parity suite produces identical admitted candidate sets and orderings from both adapters across the full contract, including identical-vector tie cohorts.
- A reopened embedded store with a different vector size fails with the collection-compatibility error.
- The default-mode construction test asserts service mode.

## Revisit When

- A measured corpus exceeds the published exact-scan guidance, or a benchmark shows the scan on the retrieval critical path — reopen the approximate-index escalation.
- The service backend's in-process build reaches a stable release — reopen the choice of embedded engine.
- The evaluation suite has run every dataset in embedded mode with results identical to service mode and one corpus at the guidance size — reopen the default mode.
- A multi-replica deployment shape is designed (the remote graph-authority phase ADR-I-0021 anticipates) — the single-process expectation is reconsidered together with the graph and statistics stores, never alone.

## Consultation impact

Question asked: whether the embedded store should overload the service connection string or take its own settings, and whether to flip the default now; ruling adopted the separate settings and the opt-in default as recommended.

## More Information

- ADR-I-0003 remains fully authoritative for the default backends; this record adds an opt-in mode in response to its revisit clause and changes no default, so it supersedes nothing. A later, evidence-gated record that flips the default would supersede ADR-I-0003's vector-backend default.
- ADR-I-0024 (port contract this adapter implements) and ADR-I-0025 (the record it stores).
- The embedded vector candidate recall phase document in the roadmap-phases design directory.
