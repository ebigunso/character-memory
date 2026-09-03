---
status: accepted
adr_type: implementation
date: 2026-09-02
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely replace the embedded engine with a lighter exhaustive-scan store the first time the dependency weight is questioned, or treat the embedded store as a test convenience whose semantics may drift from the service adapter, because both are the shortest path at small corpus sizes and both were measured as viable"
  detected_signals: "cross-boundary contract shape with tempting alternatives; rejected alternatives likely to be re-proposed (the two spiked candidates); costly reversal (an engine switch rebuilds every embedded store); premises likely to expire (the engine is beta; the weight measurement is unstripped); deliberately bounded scope (single process, index tuning deferred)"
  cost_of_violation: "an engine switch after embedded stores exist in the field rebuilds every character's recall index from graph authority and re-embeds it; two adapters with different admission semantics produce different continuity packs from the same memory, which evaluation evidence would attribute to retrieval regressions"
  cost_of_wrong_preservation: "if the engine's beta API breaks or its footprint proves unacceptable on a target platform and this record is preserved as settled, local deployments carry a dependency that no longer earns its place"
  cost_of_over_extension: "treating the embedded mode as validated for multi-process access misrepresents what the library has validated; treating the index knobs as tuned when this phase leaves them at their exact-scan setting would ship approximate recall nobody measured"
depends_on: [implementation/ADR-I-0003-qdrant-oxigraph-defaults.md, implementation/ADR-I-0021-embedded-persistent-oxigraph-default.md]
implements: []
supersedes: [implementation/ADR-I-0003-qdrant-oxigraph-defaults.md]
superseded_by: null
supersession_scope: partial   # ADR-I-0003's vector-backend default only; its backend roles and its embedded graph description stay authoritative
---

# ADR-I-0023: Embedded Qdrant Edge is the default vector candidate store; the service adapter remains the service mode

## Context and Problem Statement

After the embedded persistent graph store became the validated default (ADR-I-0021) and retrieval statistics were already file-backed (ADR-I-0009), the vector candidate store was the only component that still required an external service.
That conflicts with the intended deployment shapes: desktop companions and game or simulation characters run on end-user machines where a container runtime cannot be assumed, and it keeps a service dependency in the default test path.
ADR-I-0003's own revisit clause, "operating two stores becomes too heavy for target users", was recorded as triggered at the close of the eval-driven family closeout.
The vector layer is candidate recall only: the vector store suggests, retrieval statistics guide fanout, and graph authority decides final inclusion, so an embedded adapter has a low correctness bar for any single query — it must prefilter and rank candidates well, never be authoritative for anything.
The bar that matters is longevity: a character's memory is expected to accumulate continuously for years to decades and to outlive several generations of embedding model, so the embedded recall index must be chosen for where a memory ends up, not for where it starts.
Two feasibility spikes were run on 2026-09-02 against the same probe set (build weight, lifecycle and reopen, exactness control, a thirty-way identical-vector tie cohort, five filtered queries against the live service, API shape): the in-process build of the service backend (Qdrant Edge 0.8.0, beta) and a SQLite vector extension (sqlite-vec 0.1.9, stable).

## Decision Drivers

- The decade standard: at continuous-accumulation scale the recall index plausibly reaches hundreds of thousands to a million vectors, where an exhaustive scan costs seconds per query, so approximate indexing, quantization, and memory-mapped read-only segments are the baseline for an interactive character, not an escalation.
- Embedding models will change several times over a memory's life; named vectors that let two embedding spaces coexist during lazy re-embedding are a required capability, not a nicety.
- The store is a rebuildable cache over graph authority, which bounds the cost of an engine switch but does not eliminate it: every embedded store in the field is rebuilt and re-embedded.
- Library over in-house: the library does not own a vector engine; it owns the port contract, the tie-closure loop, the canonical ordering, the verdict mapping, and the error classification, and it holds any engine to those.
- The embedded adapter must satisfy the same port contract as the service adapter, proven by a shared parity suite, or the evaluation suite stops being a regression instrument.
- Defaults must match validation evidence (ADR-I-0021's rule): once the default test path runs on the embedded adapter, a service default would repeat the asymmetry that record corrected, so the default and its evidence land in the same change.
- Portability across deployment shapes: an engine that can synchronise with the service backend keeps a path from a local character to a hosted one.

## Decision

Add an embedded vector candidate store mode behind the existing vector candidate port, implemented on the in-process build of the service backend (Qdrant Edge), selected by a dedicated store-mode setting.
Embedded mode is the default vector mode from the phase that ships it.
The service adapter remains fully supported as the service and cloud mode, selected explicitly; this decision adds a mode and deprecates nothing.
The evidence that licenses the default is produced by the same phase: the shared parity suite proves both adapters identical below their indexing thresholds, including identical-vector tie cohorts, and the library's integration suite runs on the embedded adapter without a service, so the shipped default is the validated path (ADR-I-0021's rule).
The companion evaluation repository's cross-mode run is closeout evidence and a revisit trigger, not a gate: a difference between modes on its continuity suite reopens this record.
The embedded store is single-process, matching the embedded graph store's expectation.

Exactness is a threshold property, not a promise: below the configured indexing threshold a shard answers by exhaustive scan, above it the index answers, and in both cases the completeness verdict (ADR-I-0024) reports the boundary state of the returned top-K.
This phase ships the threshold at its exact-scan setting; index construction, quantization, and memory-mapped segments become available capabilities whose defaults are tuned on measured corpora in a later decision, never silently.

The adapter is held to the port contract by the shared library logic it may not re-implement: the tie-closure loop and the canonical constructor (the spike showed identical-vector cohorts stable within a shard and across reopen but not across fresh shards), the filter and payload conventions, the verdict mapping, and the error classification.
The engine's point, filter, condition, and scored-point types are crate-local engine types, not the service client's protocol types, so the type mapping is adapter-specific.
The adapter constructs only object payloads (the engine's point constructor panics on any other JSON shape) and uses only the general shard type (the update-only shard type carries an unimplemented path).
A contract canary test, in the pattern of the service client's erased-connect canary, pins the engine facts the adapter relies on — the meaning of the zero indexing threshold, the object-payload precondition, the shard-directory precondition, and the crate-local type provenance — so an upstream change fails a test rather than a character.
How the adapter runs the engine inside an async host and what makes a write durable is a separate decision (ADR-I-0027).

Configuration follows the one-key-per-backend pattern the graph and statistics stores already use: a mode setting (`embedded` or `service`) plus a path setting read only in embedded mode, with the service connection string required only in service mode.
The path names a directory; each collection is one engine shard directory inside it, named by the collection name the public constructor already takes, so the constructor's collection name is the backend-neutral namespace key in both modes (a server has collections, a directory has shard directories).
Embedded mode admits only names that are portable and unique on case-insensitive filesystems (lowercase allowlist and reserved-name rejection, specified in the phase document), records the name and the record schema version in an adapter-owned marker inside the shard directory, and requires the path setting to be present, with a missing path a configuration error rather than an implicit location, exactly as the embedded graph store's path.
A reopened shard is validated against the configured embedding model (vector size and distance from the shard's own configuration) and the supported record schema version before any query, failing with the same collection-compatibility error the service adapter raises and, for an unsupported schema version, the clear failure ADR-I-0007 requires.

## Implementation Impact

- A new adapter module implementing the vector candidate port on the embedded engine; the composition root gains a mode switch mirroring the statistics-store switch, defaulting to embedded.
- The library's toolchain pin moves to the minimum the engine compiles on (Rust 1.97 at decision time; the engine rejected the previous 1.95 pin).
- The settings type gains the mode and path keys; the service connection string becomes optional and is validated as present only in service mode.
- The vector database error vocabulary gains an engine-error kind for the embedded backend and reuses the existing filesystem and payload-shape kinds; the vocabulary is closed, so the companion evaluation repository's exhaustive conversion, planned in that repository, is a prerequisite to re-pinning its checkout to this wave's merge — not work of this wave.
- The port-conformance parity suite lives in the library's integration tests and runs against the embedded adapter unconditionally and against the service adapter when a service is configured; the deterministic test fake is retired in favour of the embedded adapter opened on a temporary directory, which removes the vector-service dependency from the default test path.
- Dependency weight is a recorded deliverable: the unstripped release delta is measured; the stripped delta and the effect of feature trimming are measured and recorded before closeout.
- Documentation states the embedded default, the single-process expectation, the threshold semantics, the measured latency guidance, and rebuild-from-graph-authority as the path between modes; the README's backend setup leads with the local path and presents the service as the explicit alternative.

## Considered Options

1. The in-process build of the service backend (Qdrant Edge) as the default, service mode retained.
2. An in-house SQLite exact cosine scan on the `rusqlite` dependency the statistics store already carries.
3. The SQLite vector extension (sqlite-vec).
4. A columnar embedded vector database (LanceDB).
5. An in-memory-only embedded store.
6. Ship embedded as an opt-in mode and decide the default separately.

## Decision Outcome

Chosen option: **Option 1**.
Measured on the shared probe set: lifecycle and reopen identical; the zero indexing threshold keeps a shard on a plain exhaustive scan while a threshold of one plus an optimise call builds the index; five filtered queries against the live service returned identical id sets and order with a maximum score delta of 0.0; cost of 489 additional dependency-tree lines and about 30.5 MB of unstripped release binary (stripped size unverified, feature trimming untested).
It is the only candidate that offers, in one engine, the capabilities the decade standard makes baseline — approximate indexing, quantization, named vectors, datetime payload indexes, memory-mapped read-only segments — plus synchronisation to the service backend, and it is the same engine family the service adapter already targets, so payload and filter conventions are shared rather than translated.

### Rejected Alternatives

Option 2 (in-house exact scan) violates the library-over-in-house driver: the library would own distance computation, blob encoding, and scan scheduling, and every capability the decade standard needs (index, quantization, named vectors) would have to be written or migrated to later; it is rejected outright, not deferred.
Option 3 (sqlite-vec 0.1.9) measured well — builds on both the previous and the new toolchain pin, 19 additional tree lines, about 6.2 MB unstripped, deterministic ties across fresh files and processes, parity five of five with score delta at or below 1.6e-7 — but its stable release is exhaustive-only with approximate indexing existing only in a pre-release, it carries limits on dimensions, result count, and metadata columns, and it is a pre-1.0 binding with a single maintainer; at decade scale its future is a second migration, so it is rejected for this role and reopened only if the chosen engine fails its Revisit When triggers and the extension has shipped a stable approximate index.
Option 4 (LanceDB) was rejected on dependency weight relative to the chosen engine without a spike, since the chosen engine already covers its capabilities; it is reopened only alongside Option 3's reopening.
Option 5 fails restart safety, which the persistent-graph-authority phase made a requirement for every store that survives a process; rejected outright.
Option 6 defers a decision whose deciding evidence this very phase produces: the parity suite and the service-free integration path are phase acceptance, so after the phase the validated path is embedded and a service default would be the unvalidated one, the exact asymmetry ADR-I-0021 corrected; deferral would also leave every consumer-facing document describing a default the evidence no longer supports. Rejected outright; the default is reopened only by the triggers under Revisit When.

## Consequences

- Positive: a fully self-contained local deployment exists; the default test path needs no running service; both adapters are held to one contract by one suite; the embedded store can grow into indexed, quantized, and memory-mapped operation without an engine switch.
- Positive: shared engine family means payload and filter conventions are written once and the service parity result (score delta 0.0) is structural, not coincidental.
- Negative / tradeoffs: the engine is beta and its API may change; the canary test and the pinned version turn that into a build-time failure rather than a runtime one.
- Negative / tradeoffs: about 30.5 MB of unstripped binary and a higher toolchain floor; the weight deliverable exists to establish the real number.
- Negative / tradeoffs: two adapters must be kept in parity for every port change; the parity suite is the cost of that guarantee.
- Negative / tradeoffs: consumers who followed the service-first setup must now set the store path or select service mode explicitly; with no external consumers (Compatibility Policy) no migration hint is carried.

## Decision Boundary

Invariant: embedded is the default vector mode and the embedded adapter implements the same port contract as the service adapter, proven by the shared parity suite; the store is single-process; the mode is selected by configuration, never inferred from the connection string; tie closure and canonical ordering come from the shared library loop and constructor, never from engine ordering; the indexing threshold ships at its exact-scan setting until a measured decision changes it.

Not covered: the index, quantization, and memory-map tuning values (calibrated through a later measured decision) and the latency guidance numbers (measured and revised through documentation).

## Validation

- Embedded mode constructs without any running service and survives process restart with identical search results.
- The parity suite produces identical admitted candidate sets and orderings from both adapters while both are below their indexing thresholds, including identical-vector tie cohorts closed through the shared loop; above the threshold a recall comparison against the exhaustive setting is recorded.
- A reopened shard with a different vector size or distance fails with the collection-compatibility error; an unsupported record schema version fails clearly.
- The contract canary passes on the pinned engine version and is re-run on every engine bump.
- The default-mode construction test asserts embedded mode; service mode requires the explicit mode value and a connection string.

## Revisit When

- The engine leaves beta or changes its API — re-pin, re-run the canary, and re-measure parity before adopting the new version.
- A stripped-footprint measurement or feature trimming changes the weight picture materially in either direction — revisit the weight tradeoff recorded above and, if the footprint is unacceptable on a target platform, reopen Option 3.
- A corpus benchmark contradicts the interactive-latency assumption behind the decade standard (exhaustive scan acceptable far beyond the assumed scale, or the index insufficient at it) — revisit the threshold default and the tuning decision.
- The companion evaluation repository's cross-mode run shows a difference between embedded and service mode on the continuity suite, or a corpus at the guidance size misses the interactive-latency assumption in embedded mode — reopen the default.
- A multi-replica deployment shape is designed (the remote graph-authority phase ADR-I-0021 anticipates) — the single-process expectation is reconsidered together with the graph and statistics stores, never alone.

## Consultation impact

Question asked: which embedded engine, on the two spikes' evidence; the consult's earlier recommendation of an in-house exact scan was overruled by the decider on the decade-scale portability standard, the settings shape was adopted as recommended, and the consult's proposal to defer the default was overruled because the deciding evidence is produced by the same phase.

## More Information

- ADR-I-0003 remains authoritative for the backend roles (vectors in the service backend family, graph authority in Oxigraph); this record supersedes only its vector-backend default, answering its own revisit clause; its embedded graph description is untouched (ADR-I-0021 made persistent embedded storage the validated graph default without superseding it).
- ADR-I-0024 (port contract this adapter implements, including the verdict rule), ADR-I-0025 (the record it stores), and ADR-I-0027 (how the adapter runs the engine and makes writes durable).
- The two spike reports (2026-09-02) are transient working artifacts; the numbers above are their record.
- The embedded vector candidate recall phase document in the roadmap-phases design directory.
