# v0.1.6 Design Draft: Embedded Vector Candidate Recall

## Version intent

Complete the zero-infrastructure local deployment story by adding an embedded vector candidate store mode behind the existing vector port.
With graph authority defaulting to embedded persistent storage and retrieval statistics already file-backed, the vector candidate store is the only component that still requires an external service.
That conflicts with the desktop-companion and game/simulation use cases, where end users cannot be expected to operate containers, and it keeps a service dependency in the default test path.

Sequencing: this phase runs before v0.2, so the payload surface is mirrored across two adapters while it is still small, and so v0.2 fixture work knows which vector backend it validates against.

## Why this is safe to do now

The vector layer is candidate recall only: Qdrant suggests, statistics guide fanout, and graph authority decides final inclusion.
An embedded adapter therefore has a low correctness bar — it must prefilter and rank candidates well, not be authoritative for anything.
The port is small (upsert, filtered search, diagnostics listing, delete), provider-neutral, and already exercised by deterministic fakes and a live parity surface.

## Design direction

- Add a `VectorStoreMode` setting (`service` | `embedded`) mirroring the graph store mode pattern, with the vector connection string interpreted as URL or local path accordingly.
- First embedded implementation: a SQLite-backed exact-scan adapter.
  The port's filter contract (object types, retention states, currentness, entity/thread/episode ID lists, time ranges) maps natively onto SQL predicates with junction tables for the ID lists; after prefiltering, exact cosine scan over the survivors.
  At character-memory scale (tens of thousands of vectors), exact scan is honest, fast enough, deterministic, and strictly better recall than approximate search.
- The Qdrant adapter remains fully supported as the service/cloud mode; this phase adds a mode, it does not deprecate one.
- The canonical candidate ordering contract (score, object type rank, object ID, surface rank) established for deterministic admission applies identically to the embedded adapter.
- Parity is the acceptance instrument: one shared filter-contract fixture suite runs against both adapters and must produce identical admitted sets; the embedded adapter runs it unconditionally (no service gating), which also removes the vector-service dependency from the default test path.

## Deliverables

```text
VectorStoreMode setting and configuration interpretation
SqliteVectorCandidateStore adapter (schema, upsert/delete, filtered exact-scan search, diagnostics)
composition wiring and mode selection
shared filter-contract parity suite exercised by both adapters
restart-safety and reconciliation coverage for embedded mode
documentation: payload mapping addendum, setup, corpus-size guidance
an implementation ADR recording the technology selection and its revisit triggers
```

## Non-goals

```text
changing the authority split or any retrieval semantics
deprecating or altering the Qdrant adapter
approximate-nearest-neighbor indexing (LanceDB is the recorded escalation path if embedded ANN ever becomes necessary)
migration tooling between modes (rebuild-from-graph-authority is the documented path)
changing the default vector mode in this phase (embedded ships opt-in first; flipping the default is a separate decision once parity evidence exists)
multi-process access to the embedded store (same single-process expectation as embedded graph storage)
```

## Technology posture (from the v0.1.5 closeout analysis)

- SQLite exact-scan first: zero heavyweight dependencies (`rusqlite` direction already exists via the statistics store), exact filter semantics, deterministic, restart-safe.
- LanceDB recorded as the embedded-ANN escalation path if corpora outgrow exact scan.
- The in-process edge build of the current vector backend is a revisit candidate once it stabilizes; it would maximize payload-convention reuse and add a cloud-sync story.
- Deployments that outgrow the embedded mode are exactly the deployments that should use the service mode; document a corpus-size guidance number rather than engineering for it.

## Acceptance criteria

```text
Embedded mode is configurable and constructs without any running service.
The shared parity suite produces identical admitted candidate sets from both adapters across the full filter contract.
Deterministic admission holds in embedded mode (equal-score cohorts canonically ordered; repeated runs byte-identical).
Embedded state survives process restart; reconciliation diagnostics work against the embedded store.
The default test path requires no vector service; Qdrant-gated suites continue to pass unchanged.
Documentation states the single-process expectation, the corpus-size guidance, and the rebuild-from-authority migration path.
No public facade change; no retrieval behavior change in service mode.
```

## Evaluation tie-in

The continuity evaluation suite gains an embedded-mode configuration so the confirmation scenarios (including restart) run against the embedded vector store; the frozen-embedding infrastructure applies unchanged.
Scenario baselines are expected to be identical between modes under the parity contract; any divergence is a finding, which makes the eval suite the cross-adapter regression instrument.

## Open questions

- Should embedded become the default vector mode once parity evidence exists, matching the embedded-default graph decision, or stay opt-in until a full release cycle passes?
- What corpus-size number goes in the guidance (the closeout analysis suggested exact-scan comfort up to low hundreds of thousands of vectors; measure rather than assume)?
- Does the parity suite live in the library's integration tests, the evaluation repository, or both (recommendation: shared fixtures in the library, evaluation reuse where cheap)?
