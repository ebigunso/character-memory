# v0.1.6 Design: Embedded Vector Candidate Recall

Status: decided 2026-09-02 (ADR-I-0023 through ADR-I-0026); supersedes the 2026-07 draft of this document.

## Version intent

Complete the zero-infrastructure local deployment story by adding an embedded vector candidate store mode behind the existing vector candidate port, and settle the port contract that both adapters must satisfy before a second adapter exists.
With graph authority defaulting to embedded persistent storage (ADR-I-0021) and retrieval statistics already file-backed (ADR-I-0009), the vector candidate store is the only component that still requires an external service.
That conflicts with the desktop-companion and game or simulation use cases, where end users cannot be expected to operate containers, and it keeps a service dependency in the default test path.

Sequencing: this phase runs before scoped continuity, so the vector record is mirrored across two adapters while it is five fields, and so the scoped-continuity evaluation fixtures know which vector backend they validate against.

## Why this is safe to do before scoped continuity

The vector layer is candidate recall only: the vector store suggests, statistics guide fanout, and graph authority decides final inclusion.
An embedded adapter therefore has a low correctness bar: it must prefilter and rank candidates well, never be authoritative for anything.
The port is small (upsert, scoped search, delete), provider-neutral, and already exercised by deterministic fakes and a live smoke surface.
The write path already removes the vectors of superseded and suppressed objects, so the live vector population is the active population by construction, and stale residue from failed maintenance is caught by graph verification; the embedded adapter inherits both guarantees without new code.

## Design direction

### The port contract (ADR-I-0024, ADR-I-0025)

This phase fixes the port contract deliberately, because two adapters cannot be held to an implicit one.

Query: the embedding, the limit, and an object-type scope, and nothing else; an empty scope selects zero candidates, and the retrieval context rejects an empty configured object-type set at the boundary.
Three-valued hint predicates are prohibited; a future predicate arrives as an explicit enum whose unknown arm never matches, in both adapters, with a parity fixture.

Result: a completeness envelope, the canonical candidates plus a typed verdict — not requested (the limit was zero and no search was issued), exhaustive (every scoped record was scored), boundary tie closed (an index returned a prefix and the cutoff cohort was verified closed), or boundary tie open (the overfetch bound was reached with the cohort open).
The service adapter's existing tie-cohort loop maps onto the last two verdicts; before this phase it returned the truncated set silently at its bound, and the port's bare list type could not say whether top-K membership was determinate.
The retrieval pipeline records the verdict in telemetry beside the returned candidate count and never repairs, retries, or fails on it, because candidate recall is non-authoritative.
The canonical-candidates newtype introduced for deterministic admission survives as the envelope's candidates field; canonical ordering (score, object-type rank, object id, surface rank) is unchanged and applies identically to both adapters.

Record: both adapters persist exactly five fields — object id, object type, surface, schema version, and the embedded text.
Read-out text lives in graph authority; the vector record stores only the embedded surface, as provenance of what was ranked; consumers needing candidate content hydrate by object id.
The relationship, lifecycle, time, ranking, object-specific, graph-URI, and raw-reference hints leave the write path: the library read none of them, the relationship hints were frozen at upsert and never updated by linking, the lifecycle hints described vectors the write path deletes, and the readable text column duplicated graph text.

Two re-entry paths are named so the scope-only query and five-field record are read as current state, not prohibition:

1. A synchronised scope predicate, owned by the scoped-continuity phase: a scope-id column written at upsert and kept in sync by the link and reflection write paths.
2. An immutable time-window predicate over `created_at` and `observed_at`, owned by whichever phase first ships a time-bounded retrieval route; immutability makes a write-time column correct without a sync path, and the columns are backfilled from graph authority if ever needed.

### The embedded adapter (ADR-I-0023)

A SQLite-backed exact cosine scan, using the `rusqlite` dependency the statistics store already carries, with the same single-process, mutex-guarded connection model, except that the scan itself is a synchronous, potentially long operation behind an async port: it runs on a blocking worker (the runtime's blocking pool) with connection access still serialized, so a scan never occupies an async executor thread, and the in-phase benchmark records executor responsiveness while a scan is in progress.

Schema: one table keyed by object id and surface with a column per contract field and the embedding as a fixed-width little-endian floating-point blob normalised at write; an index on object type for the scope predicate; a metadata table recording vector size, distance, and schema version.
Search: normalise the query once (a zero-norm query scores every row zero and is reported, not rejected), select the scoped rows, score by dot product in a fixed order so the score equals the service adapter's cosine, canonicalise through the shared constructor, truncate to the limit, and report exhaustive completeness with the scanned count; a parity fixture with non-unit query and record vectors pins score equality across adapters.
Delete: remove every surface of each object id, matching the service adapter's selector.
Restart safety: opening an existing file validates the recorded vector size and distance against the configured embedding model and raises the same collection-compatibility error the service adapter raises for a mismatched collection.
Determinism: same inputs, same scores, same total sort; equal-score cohorts are ordered by the shared comparator, so the embedded adapter satisfies deterministic admission by construction and never needs an overfetch loop.
Score parity across adapters is not bitwise (the service computes cosine on its own normalised copy), so parity compares membership and order with a small score tolerance, and tie fixtures use identical vectors.

### Settings and composition (ADR-I-0023)

Follow the one-key-per-backend pattern the graph and statistics stores already use rather than overloading the service connection string.

```text
VECTOR_STORE_MODE   service | embedded   (default: service)
VECTOR_STORE_PATH   directory, read only in embedded mode
QDRANT_CONNECTION_STRING   required only in service mode
```

`VECTOR_STORE_PATH` is a directory; each collection is one SQLite file inside it named by the collection name the public constructor already takes, so `collection_name` is the backend-neutral namespace key in both modes.
Because the collection name becomes a file name under the configured directory, embedded mode validates it at construction with a contract owned here: ASCII letters, digits, underscore, and hyphen only, first character a letter or digit, at most 128 characters, no path separators, dots, or empty name; anything else is rejected with the configuration error before any file is touched, and a path-confinement test proves that separator and parent-directory inputs cannot escape the directory.
The composition root gains a vector-store mode switch mirroring the statistics-store switch; the vector database error vocabulary gains an engine-error kind for the embedded backend and reuses the existing filesystem and payload-shape kinds.

### Parity suite placement (ADR-I-0023)

Library: a port-conformance suite in the integration tests — scope filtering, empty scope selects zero, canonical order, identical-vector tie cohort, best-score-per-object-and-surface deduplication, delete removes all surfaces, restart reopen, completeness verdict per adapter — run against the embedded adapter unconditionally and against the service adapter when a service connection is configured.
This follows the precedent that port conformance is enforced by contract tests run against every adapter, not by a runtime wrapper.
Evaluation repository: no second contract suite; it adds an embedded-mode configuration to the continuity scenarios and requires identical scenario results between modes, the behaviour-level regression instrument.

## Deliverables

```text
port contract: completeness envelope, scope-only query with empty-scope-selects-zero, retrieval telemetry completeness field
vector record read contract: five-field manifest shared by both adapters
SqliteVectorCandidateStore adapter: schema, upsert/delete, scoped exact-scan search, restart validation
VectorStoreMode and VectorStorePath settings; composition mode switch; service connection string required only in service mode
port-conformance parity suite in the library integration tests, run against both adapters
restart-safety test for the embedded store; pipeline test over the embedded adapter with a deleted graph object
measured corpus-size guidance from an in-phase benchmark
documentation: settings, single-process expectation, corpus-size guidance, rebuild-from-graph-authority as the path between modes
four implementation ADRs (ADR-I-0023 through ADR-I-0026) with reciprocal partial-supersession frontmatter on ADR-I-0001, ADR-I-0002, and ADR-I-0005
```

Deletions that are deliverables, not side effects:

```text
the hint carriers on the vector record type and the surface builders' hint population
the readable text column and the per-field payload index creation for dropped fields
the test-only payload field constants and the prose-assertion note constant
the service adapter's private enum token mappers, replaced by one Display/FromStr per enum in the domain (the embedded adapter must not add another copy)
the deterministic vector fake and its embedding-bearing record type, replaced by the embedded adapter opened in memory (failure-injecting and recording fakes stay)
the port doc comment's "documented bounded-overfetch degradation policy" clause, now expressed by the type
```

## Non-goals

```text
changing the authority split or any retrieval semantics
deprecating or altering the service adapter
approximate-nearest-neighbour indexing (the recorded escalation path if embedded ANN ever becomes necessary)
migration tooling between modes or between record shapes (rebuild-from-graph-authority is the documented path)
changing the default vector mode in this phase (embedded ships opt-in; flipping the default is a separate evidence-gated decision)
multi-process access to the embedded store (same single-process expectation as embedded graph storage)
any vector-layer predicate beyond the object-type scope (the two named re-entry paths belong to later phases)
any new public facade method (the evaluation baseline consumes the retrieval trace)
reconciliation diagnostics (the reconciliation slice was deleted in the structured-verdict phase; graph verification is the guard)
```

## Technology posture

- SQLite exact scan first: zero new dependencies, exact filter semantics, deterministic, restart-safe.
- The first escalation inside the embedded mode is an in-memory normalised matrix loaded at open with write-through, an implementation optimisation that changes no contract.
- An embedded approximate-nearest-neighbour library is the recorded escalation path if corpora outgrow exact scan; a measured corpus exceeding the guidance or a benchmark showing the scan on the critical path reopens it.
- The in-process build of the service backend is a revisit candidate once it ships a stable release; it would maximise reuse of the service adapter's conventions.
- Deployments that outgrow the embedded mode are exactly the deployments that should use the service mode; publish a measured corpus-size guidance number rather than engineering for it (at a 3072-dimension model each ten thousand vectors is about 120 MB of embedding data read per query, which bounds the honest number).

## Acceptance criteria

```text
Embedded mode is configurable and constructs without any running service.
The parity suite produces identical admitted candidate sets and orderings from both adapters across the full contract, including identical-vector tie cohorts.
Deterministic admission holds in embedded mode (equal-score cohorts canonically ordered; repeated runs byte-identical).
Retrieval telemetry reports the completeness verdict; the embedded adapter reports exhaustive, the service adapter reports closed on the tie fixture.
Embedded state survives process restart; a reopened store with a different vector size fails with the collection-compatibility error.
The default test path requires no vector service; service-gated suites continue to pass unchanged.
Both adapters persist exactly the five-field read contract; a census of both repositories shows no reader of a dropped field.
Documentation states the single-process expectation, the corpus-size guidance, and the rebuild-from-authority path.
No public facade change; no retrieval behaviour change in service mode beyond the added telemetry field.
```

## What the evaluation repository provides and when it is used (ADR-I-0026)

The companion evaluation repository is a development aid; its own work is planned and tracked there, and this document records only what its measurements let this phase decide.

- The library exposes, through an ordinary traced retrieval, everything a raw-vector baseline needs: the vector candidates with scores and the completeness verdict in telemetry; the honest way to use them is one singleton-scoped traced retrieval per measured object kind with a limit of the section budget multiplied by the maximum surfaces per object, deduplicated by object.
- The cross-mode comparison (service mode against embedded mode on the continuity suite, identical baselines expected under the parity contract) is the evidence that gates the default flip recorded in ADR-I-0023; it is consumed at the closeout task and by that later decision, not produced by this plan.
- No public facade or configuration surface is added for the evaluation repository; if its measurements ever require one, that is a library decision taken on its own record.

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
   Re-verified: the only degradation site is the service adapter's fetch bound; no pipeline path inspects or retries on it.
   Evidence: the fetch-decision unit test asserts the open verdict at the bound; a retrieval test asserts the telemetry field per variant; the live boundary test asserts the closed verdict.
4. Hint filter semantics.
   Parked claim: query-side hint semantics belong to the port contract.
   Re-verified: the filter type and both match-or-unknown implementations were deleted in the structured-verdict phase, and no consumer asks for a vector-layer predicate (the evaluation surface policy carries object types and budgets only).
   Evidence: zero-hit census for the filter type and for empty-or-null match conditions in the service adapter; the prohibition and re-entry paths are recorded in ADR-I-0024.
5. Evaluation baseline capability.
   Parked claim: the baseline re-implements a hidden raw-vector capability against the payload schema.
   Re-verified: one singleton-scoped traced retrieval per measured kind reproduces the direct per-kind search exactly, which a sliced mixed-kind top-K would not; each retrieval's completeness verdict reports whether that kind's top-K was determinate; the evaluation adapter can hold item text from ingest.
   Evidence: the A/B run with row-level diff of item identities and ranks; after the switch, zero-hit census for vector-service search calls and payload constants in the evaluation adapter.

## Decisions (the draft's open questions, resolved 2026-09-02)

- Default mode: stays opt-in this phase; the flip is reopened by the evaluation suite running every dataset in embedded mode with identical results and one corpus at the guidance size (ADR-I-0023, Revisit When).
- Corpus-size guidance: measured in-phase by a benchmark over a synthetic corpus at the configured dimension, published in documentation, revised through documentation.
- Parity suite placement: contract parity in the library, behaviour parity in the evaluation repository (above).
- Settings shape: separate mode and path keys with `collection_name` as the backend-neutral namespace key, not a connection string interpreted by mode (ADR-I-0023).
- Hint families: all dropped from the vector record, with the two named re-entry paths (ADR-I-0024, ADR-I-0025).
- Text columns: readable text dropped, embedded text kept as provenance, governed by the three sentences in ADR-I-0025.
- Evaluation baseline: trace-sourced, no facade change (ADR-I-0026).
