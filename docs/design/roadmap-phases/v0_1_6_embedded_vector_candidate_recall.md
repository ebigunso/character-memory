# v0.1.6 Design: Embedded Vector Candidate Recall

Status: decided 2026-09-02 (ADR-I-0023 through ADR-I-0026); supersedes the 2026-07 draft of this document; the embedded engine ruling (Qdrant Edge over an in-house scan) was taken the same day on two feasibility spikes.

## Version intent

Complete the zero-infrastructure local deployment story by adding an embedded vector candidate store mode behind the existing vector candidate port, and settle the port contract that both adapters must satisfy before a second adapter exists.
With graph authority defaulting to embedded persistent storage (ADR-I-0021) and retrieval statistics already file-backed (ADR-I-0009), the vector candidate store is the only component that still requires an external service.
That conflicts with the desktop-companion and game or simulation use cases, where end users cannot be expected to operate containers, and it keeps a service dependency in the default test path.
The embedded engine is chosen for where a character's memory ends up, not where it starts: a memory expected to accumulate for years to decades across several generations of embedding model needs approximate indexing, quantization, memory-mapped segments, and coexisting embedding spaces as baseline capabilities, so the in-process build of the service backend is adopted rather than an exhaustive scan the library would own (ADR-I-0023).

Sequencing: this phase runs before scoped continuity, so the vector record is mirrored across two adapters while it is five fields, and so the scoped-continuity evaluation fixtures know which vector backend they validate against.

## Why this is safe to do before scoped continuity

The vector layer is candidate recall only: the vector store suggests, statistics guide fanout, and graph authority decides final inclusion.
An embedded adapter therefore has a low correctness bar for any single query: it must prefilter and rank candidates well, never be authoritative for anything.
The port is small (upsert, scoped search, delete), provider-neutral, and already exercised by deterministic fakes and a live smoke surface.
The write path already removes the vectors of superseded and suppressed objects, so the live vector population is the active population by construction, and stale residue from failed maintenance is caught by graph verification; the embedded adapter inherits both guarantees without new code.
The embedded engine is the same family as the service backend, so payload and filter conventions are shared rather than translated, and the spike measured identical id sets, identical order, and a score delta of 0.0 on five filtered queries against the live service.

## Design direction

### The port contract (ADR-I-0024, ADR-I-0025)

This phase fixes the port contract deliberately, because two adapters cannot be held to an implicit one.

Query: the embedding, the limit, and an object-type scope, and nothing else; an empty scope selects zero candidates, and the retrieval context rejects an empty configured object-type set at the boundary.
Three-valued hint predicates are prohibited; a future predicate arrives as an explicit enum whose unknown arm never matches, in both adapters, with a parity fixture.

Result: a completeness envelope, the canonical candidates plus a typed verdict — not requested (the limit was zero or the scope was empty, so no search was issued), exhaustive (every scoped record was scored, so the requested top-K is determinate), boundary tie closed (an index returned a prefix and the cutoff cohort was verified closed), or boundary tie open (the overfetch bound was reached with the cohort open).
Both adapters answer through the shared tie-closure loop and the canonical constructor; before this phase the service adapter returned the truncated set silently at its bound, and the port's bare list type could not say whether top-K membership was determinate.
The retrieval pipeline records the verdict in telemetry beside the returned candidate count and never repairs, retries, or fails on it, because candidate recall is non-authoritative.
The canonical-candidates newtype introduced for deterministic admission survives as the envelope's candidates field; canonical ordering (score, object-type rank, object id, surface rank) is unchanged and applies identically to both adapters.

Record: both adapters persist exactly five fields — object id, object type, surface, schema version, and the embedded text.
Read-out text lives in graph authority; the vector record stores only the embedded surface, as provenance of what was ranked; consumers needing candidate content hydrate by object id.
The relationship, lifecycle, time, ranking, object-specific, graph-URI, and raw-reference hints leave the write path: the library read none of them, the relationship hints were frozen at upsert and never updated by linking, the lifecycle hints described vectors the write path deletes, and the readable text column duplicated graph text.

Two re-entry paths are named so the scope-only query and five-field record are read as current state, not prohibition:

1. A synchronised scope predicate, owned by the scoped-continuity phase: a scope-id column written at upsert and kept in sync by the link and reflection write paths.
2. An immutable time-window predicate over `created_at` and `observed_at`, owned by whichever phase first ships a time-bounded retrieval route; immutability makes a write-time column correct without a sync path, and the columns are backfilled from graph authority if ever needed.

### The embedded adapter (ADR-I-0023)

The adapter runs the in-process build of the service backend (the `qdrant-edge` crate, pinned exactly; beta at adoption) as one engine shard per collection.

Exactness is a threshold property: a shard below its configured indexing threshold answers by exhaustive scan, a shard above it answers from its index, and the completeness verdict reports the boundary state either way.
This phase ships the threshold at its exact-scan setting (the spike confirmed that a zero threshold leaves a shard unindexed while a threshold of one plus an optimise call builds the index); index construction, quantization, and memory-mapped segments are available capabilities whose defaults are tuned in a later measured decision, never silently.

Shard: cosine distance at the configured vector size; the five-field payload with keyword indexes on object id (the delete selector removes every surface of an object by it) and on object type (the scope predicate); the object-type scope expressed as a filter; the general shard type only (the update-only shard type carries an unimplemented path); object payloads only (the engine's point constructor panics on any other JSON shape).
Search: the query runs through the service adapter's tie-closure loop and the canonical constructor, because the spike found identical-vector cohorts stable within a shard and across reopen but not across freshly built shards — deterministic admission comes from closing the cutoff cohort and ordering it canonically, never from engine order.
Verdict mapping: exhaustive when the shard is unindexed and the scan returned fewer rows than the fetch limit (the whole scoped population was scored, so the requested top-K is determinate; the envelope stays capped at the limit), boundary tie closed or open from the loop otherwise, not requested at limit zero or empty scope.
Delete: remove every surface of each object id, matching the service adapter's selector.
Restart safety: opening an existing shard validates its recorded vector size and distance against the configured embedding model and the adapter-owned marker's record schema version against the supported version before any query, raising the collection-compatibility error or the clear unsupported-schema failure ADR-I-0007 requires.
Blocking discipline: every engine call is synchronous (upsert and delete write the log and update payload and field indexes; search scans or traverses; index build; and shutdown flushes on drop), so the adapter creates a dedicated blocking owner at construction, one blocking worker that holds the shard and serialises access, and routes every call through it; the adapter's own drop only signals that owner, so the shard's final drop happens on the owner's thread and no port or facade method is needed.
That non-blocking drop is safe because every write is durable in the engine's write-ahead log before the call returns (the final flush on drop is compaction, not persistence, so a process exit that pre-empts it loses nothing) and because a shard directory stays locked while an owner holds it, so a constructor that meets a locked directory waits with a bounded backoff for the previous owner to release it; both facts are pinned by the contract canary and proven by a close-then-reopen test that drops the facade inside an async runtime, reopens the same directory immediately, and finds every write.
The in-phase benchmark records executor responsiveness while a scan, a write burst, a build, and a close are in progress.
Type mapping: the engine's point, filter, condition, and scored-point types are crate-local engine types, not the service client's protocol types, so the conversion is adapter-specific; payload and filter conventions, the tie-closure loop, the verdict mapping, and the error classification are shared library logic that neither adapter re-implements.
Contract canary: a test in the pattern of the service client's erased-connect canary pins the engine facts the adapter relies on — zero threshold means unindexed, object-payload precondition, shard-directory precondition, crate-local type provenance — so an upstream change fails a test rather than a character; the pin is bumped only with a canary and parity re-run.
Score parity across adapters was measured at 0.0 delta on the spike; the parity suite still asserts it with non-unit query and record vectors rather than assuming it.

### Settings and composition (ADR-I-0023)

Follow the one-key-per-backend pattern the graph and statistics stores already use rather than overloading the service connection string.

```text
VECTOR_STORE_MODE   service | embedded   (default: service)
VECTOR_STORE_PATH   directory, required in embedded mode (missing is a configuration error, never an implicit default), ignored in service mode
QDRANT_CONNECTION_STRING   required only in service mode
```

`VECTOR_STORE_PATH` is a directory; each collection is one engine shard directory inside it named by the collection name the public constructor already takes, so `collection_name` is the backend-neutral namespace key in both modes.
Because the collection name becomes a directory name under the configured path, embedded mode validates it at construction with a contract owned here: lowercase ASCII letters, digits, underscore, and hyphen only (lowercase so that names stay unique on the case-insensitive filesystems of desktop targets), first character a letter or digit, at most 128 characters, not a reserved device name on Windows (con, prn, aux, nul, com1 to com9, lpt1 to lpt9), no path separators, dots, or empty name; anything else is rejected with the configuration error before any directory is touched, the name and the record schema version are recorded in an adapter-owned marker inside the shard directory, and a path-confinement test proves that separator and parent-directory inputs cannot escape the directory.
The engine requires the shard root directory to exist before opening; the adapter creates it, as the statistics store creates its parent directory.
The composition root gains a vector-store mode switch mirroring the statistics-store switch; the vector database error vocabulary gains an engine-error kind for the embedded backend and reuses the existing filesystem and payload-shape kinds.
The toolchain pin moves to the engine's minimum (Rust 1.97 at adoption; the previous pin did not compile it), in its own change before the adapter lands.

### Parity suite placement (ADR-I-0023)

Library: a port-conformance suite in the integration tests — scope filtering, empty scope selects zero, canonical order, identical-vector tie cohort closed through the shared loop, best-score-per-object-and-surface deduplication, delete removes all surfaces, restart reopen, completeness verdict per adapter — run against the embedded adapter unconditionally and against the service adapter when a service connection is configured.
Parity acceptance is identical admitted sets and orderings while both adapters are below their indexing thresholds (the suite's fixtures are small enough that both are), and a recorded recall comparison of the embedded adapter above its threshold against its exhaustive setting, informational this phase.
This follows the precedent that port conformance is enforced by contract tests run against every adapter, not by a runtime wrapper.
Evaluation repository: no second contract suite; it adds an embedded-mode configuration to the continuity scenarios and requires identical scenario results between modes, the behaviour-level regression instrument.

## Deliverables

```text
port contract: completeness envelope, scope-only query with empty-scope-selects-zero, retrieval telemetry completeness field
vector record read contract: five-field manifest shared by both adapters
QdrantEdgeVectorCandidateStore adapter: shard per collection at the exact-scan threshold, upsert/delete, scoped search through the shared tie-closure loop, verdict mapping, restart validation, blocking discipline
engine contract canary test, re-run on every engine bump
VectorStoreMode and VectorStorePath settings; composition mode switch; service connection string required only in service mode; toolchain pin at the engine minimum
port-conformance parity suite in the library integration tests, run against both adapters; above-threshold recall comparison recorded
restart-safety test for the embedded store; pipeline test over the embedded adapter with a deleted graph object
dependency-weight report: unstripped and stripped release deltas, effect of feature trimming
latency benchmark: exhaustive scan across corpus sizes at the configured dimension; executor responsiveness under a concurrent scan
documentation: settings, single-process expectation, threshold semantics, measured latency guidance, rebuild-from-graph-authority as the path between modes
four implementation ADRs (ADR-I-0023 through ADR-I-0026) with reciprocal partial-supersession frontmatter on ADR-I-0001, ADR-I-0002, and ADR-I-0005
```

Deletions that are deliverables, not side effects:

```text
the hint carriers on the vector record type and the surface builders' hint population
the readable text column and the per-field payload index creation for dropped fields
the test-only payload field constants and the prose-assertion note constant
the service adapter's private enum token mappers, replaced by one Display/FromStr per enum in the domain (the embedded adapter must not add another copy)
the deterministic vector fake and its embedding-bearing record type, replaced by the embedded adapter opened on a temporary directory (failure-injecting and recording fakes stay)
the port doc comment's "documented bounded-overfetch degradation policy" clause, now expressed by the type
```

## Non-goals

```text
changing the authority split, or any retrieval semantics for non-empty scopes (the empty-scope change in ADR-I-0024 is intended and in scope)
deprecating the service adapter, or altering it beyond what the shared port and record contracts require
tuning the embedded index, quantization, or memory-mapping defaults (available in the engine; shipped at the exact-scan threshold, tuned by a later measured decision)
named-vector coexistence of two embedding spaces (an engine capability this decision was taken for; its use lands with the first embedding-model migration)
migration tooling between modes or between record shapes (rebuild-from-graph-authority is the documented path)
changing the default vector mode in this phase (embedded ships opt-in; flipping the default is a separate evidence-gated decision)
multi-process access to the embedded store (same single-process expectation as embedded graph storage)
synchronisation between an embedded shard and a service collection (an engine capability; not exercised this phase)
any vector-layer predicate beyond the object-type scope (the two named re-entry paths belong to later phases)
any new public facade method (the evaluation baseline consumes the retrieval trace)
reconciliation diagnostics (the reconciliation slice was deleted in the structured-verdict phase; graph verification is the guard)
```

## Technology posture

- The in-process build of the service backend (Qdrant Edge, pinned exactly at 0.8.0, beta) is the embedded engine: measured on the spike as lifecycle-and-reopen identical, exactness controllable by the indexing threshold, five-of-five service parity at score delta 0.0, at a cost of 489 additional dependency-tree lines and about 30.5 MB of unstripped release binary (stripped size and feature trimming are measured in-phase), requiring Rust 1.97 or later.
- The decade standard behind the choice: at continuous-accumulation scale the recall index plausibly reaches hundreds of thousands to a million vectors, where exhaustive scan is seconds per query, so approximate indexing, quantization, and memory-mapped segments are the baseline for an interactive character; embedding models will change several times, so named vectors that let two spaces coexist during lazy re-embedding matter; the store is a rebuildable cache over graph authority, which bounds but does not eliminate the cost of an engine switch.
- Library over in-house: the library owns the port contract, the tie-closure loop, the canonical ordering, the verdict mapping, and the error classification, and holds any engine to them; it does not own distance computation, storage layout, or index construction.
- Rejected on the same probe set: an in-house SQLite exact scan (violates library-over-in-house and would re-implement every capability above later); the SQLite vector extension sqlite-vec 0.1.9 (builds on both toolchains, 19 tree lines, about 6.2 MB unstripped, deterministic ties across fresh files, parity five of five at delta at or below 1.6e-7, but exhaustive-only in its stable release with approximate indexing only in a pre-release, a pre-1.0 binding, and a single maintainer); LanceDB (weight, without a spike, since the chosen engine covers its capabilities); an in-memory-only store (no restart safety).
- Beta risk is handled structurally: exact pin, contract canary, parity re-run on every bump; the engine leaving beta or changing its API, a stripped-footprint measurement that changes the weight picture, or a corpus benchmark contradicting the interactive-latency assumption each reopen ADR-I-0023.
- Deployments that outgrow the embedded mode are exactly the deployments that should use the service mode; the engine's synchronisation to a service collection is the recorded portability path from a local character to a hosted one.

## Acceptance criteria

```text
Embedded mode is configurable and constructs without any running service.
The parity suite produces identical admitted candidate sets and orderings from both adapters while both are below their indexing thresholds, including identical-vector tie cohorts closed through the shared loop.
A recall comparison of the embedded adapter above its indexing threshold against its exhaustive setting is recorded.
Deterministic admission holds in embedded mode (equal-score cohorts canonically ordered; repeated runs byte-identical; no engine ordering relied on).
Retrieval telemetry reports the completeness verdict; the embedded adapter reports exhaustive below its threshold, the service adapter reports closed on the tie fixture.
Embedded state survives process restart; a reopened shard with a different vector size or distance fails with the collection-compatibility error; an unsupported record schema version fails clearly.
No engine call (upsert, delete, search, index build, or shutdown) occupies an async executor thread; all run on the adapter's dedicated blocking owner, dropping the facade signals that owner so the shard's final drop happens there, and the benchmark records executor responsiveness during a concurrent scan, a write burst, a build, and a close.
A zero-norm record embedding is rejected at indexing as a typed per-record failure before any adapter sees it, and a zero-norm query scores every candidate zero with a truthful verdict, both proven in both adapters by parity fixtures.
The engine contract canary passes on the pinned version.
The dependency-weight report records unstripped and stripped release deltas and the effect of feature trimming.
The default test path requires no vector service; service-gated suites continue to pass unchanged.
Both adapters persist exactly the five-field read contract; a census of both repositories shows no reader of a dropped field.
Documentation states the single-process expectation, the threshold semantics, the latency guidance, and the rebuild-from-authority path.
No public facade change beyond the telemetry field and the published maximum-surfaces-per-object-kind policy value; no retrieval behaviour change in service mode for non-empty scopes (an empty object-type scope now selects zero instead of searching unfiltered, and an empty configured scope is rejected at the boundary, both intended).
```

## What the evaluation repository provides and when it is used (ADR-I-0026)

The companion evaluation repository is a development aid; its own work is planned and tracked there, and this document records only what its measurements let this phase decide.

- The library exposes, through an ordinary traced retrieval, everything a raw-vector baseline needs: the vector candidates with scores and the completeness verdict in telemetry; the honest way to use them is one singleton-scoped traced retrieval per measured object kind with a limit of the section budget multiplied by the maximum surfaces per object, deduplicated by object.
- The cross-mode comparison (service mode against embedded mode on the continuity suite, identical baselines expected under the parity contract) is the evidence that gates the default flip recorded in ADR-I-0023; it is consumed at the closeout task and by that later decision, not produced by this plan.
- No candidate-search facade or configuration surface is added for the evaluation repository; the one public addition made for its trace reading is the published maximum-surfaces-per-object-kind policy value (ADR-I-0026), and if its measurements ever require more, that is a library decision taken on its own record.

## Evaluation tie-in

The evaluation repository is expected to run its continuity suite in both vector modes; identical scenario results are what the parity contract predicts, and the comparison is the evidence that gates the later default-flip decision recorded in ADR-I-0023.
How that configuration is built and run is planned in the evaluation repository; this phase consumes the comparison at closeout and cites nothing else from it.

## Deferral-reconfirmation checklist

Each item was parked on this phase by the structured-verdict phase; each row states the parked claim, what was re-verified at design time, and the evidence the implementation must produce.

1. Canonical-candidates newtype survival.
   Parked claim: the newtype survives the port redesign or is absorbed into its result envelope.
   Re-verified: every consumer is a slice read (the pipeline's count, telemetry, trace, and root selection sites, plus the test fakes); none relies on the newtype being the whole return value.
   Evidence: after the change a census shows only the envelope field, the constructor, and the fakes' exhaustive wrapping; the existing deduplication-and-ordering test is unchanged.
2. Dual text columns.
   Parked claim: the text columns' fate depends on the port's read contract.
   Re-verified: the readable text column had exactly one reader (the evaluation baseline) and the embedded text column none; the evaluation repository can source item text from its own ingest.
   Evidence: zero-hit census for the readable text column across both repositories; a vector-only run before and after produces identical item identities and text.
3. Search completeness.
   Parked claim: the port cannot express whether the top-K was determinate.
   Re-verified: the only degradation site is the service adapter's fetch bound; no pipeline path inspects or retries on it; the embedded engine's own tie order is not stable across fresh shards, which makes the shared loop a requirement for both adapters rather than a service-only workaround.
   Evidence: the fetch-decision unit test asserts the open verdict at the bound; a retrieval test asserts the telemetry field per variant; the live boundary test asserts the closed verdict; the parity tie fixture asserts exhaustive versus closed through the same loop.
4. Hint filter semantics.
   Parked claim: query-side hint semantics belong to the port contract.
   Re-verified: the filter type and both match-or-unknown implementations were deleted in the structured-verdict phase, and no consumer asks for a vector-layer predicate (the evaluation surface policy carries object types and budgets only).
   Evidence: zero-hit census for the filter type and for empty-or-null match conditions in the service adapter; the prohibition and re-entry paths are recorded in ADR-I-0024.
5. Evaluation baseline capability.
   Parked claim: the baseline re-implements a hidden raw-vector capability against the payload schema.
   Re-verified: one singleton-scoped traced retrieval per measured kind reproduces the direct per-kind search exactly, which a sliced mixed-kind top-K would not; each retrieval's completeness verdict reports whether that kind's top-K was determinate; the evaluation adapter can hold item text from ingest.
   Evidence: the A/B run with row-level diff of item identities and ranks; after the switch, zero-hit census for vector-service search calls and payload constants in the evaluation adapter.

## Decisions (the draft's open questions, resolved 2026-09-02)

- Embedded engine: the in-process build of the service backend (Qdrant Edge), on the decade-scale portability standard and the two spikes' measurements; the in-house exact scan and the SQLite vector extension are rejected alternatives (ADR-I-0023).
- Exactness: a threshold property shipped at the exact-scan setting; index, quantization, and memory-map tuning is a later measured decision (ADR-I-0023).
- Default mode: stays opt-in this phase; the flip is reopened by the evaluation suite running every dataset in embedded mode with identical results and one corpus at the guidance size (ADR-I-0023, Revisit When).
- Latency guidance: measured in-phase by a benchmark over a synthetic corpus at the configured dimension, published in documentation, revised through documentation.
- Dependency weight: the unstripped delta is recorded; the stripped delta and feature trimming are measured in-phase, and a material change reopens ADR-I-0023.
- Parity suite placement: contract parity in the library, behaviour parity in the evaluation repository (above).
- Settings shape: separate mode and path keys with `collection_name` as the backend-neutral namespace key naming one shard directory per collection, not a connection string interpreted by mode (ADR-I-0023).
- Hint families: all dropped from the vector record, with the two named re-entry paths (ADR-I-0024, ADR-I-0025).
- Text columns: readable text dropped, embedded text kept as provenance, governed by the three sentences in ADR-I-0025.
- Evaluation baseline: trace-sourced, no facade change (ADR-I-0026).
