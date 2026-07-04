# v0.1.1 Design Draft: Persistent Oxigraph Graph Authority

## Version intent

Make the v0.1 graph-authoritative architecture durable across process restarts before adding richer continuity and reflection features.

v0.1 established the starter Character Memory object model and retrieval behavior:

```text
Episode
Observation
Entity
MemoryThread
DerivedMemory
MemoryLink
ContinuityContextPack
```

It also established the storage split:

```text
Qdrant   = vector candidate recall
Oxigraph = graph authority for objects, links, provenance, lifecycle, currentness, and correction
```

The current graph authority is embedded/in-memory. This creates a durability gap: Qdrant candidates may survive restart while the graph state required to validate provenance, lifecycle, currentness, supersession, and links may not.

v0.1.1 closes that gap without adding new memory concepts.

---

# 1. Why this comes before v0.2

v0.2 introduces stronger continuity structures:

```text
RelationshipState
CharacterSignal
OpenLoop
Commitment
ReflectionJob
CurrentContinuityView
```

These should not be built on a volatile graph authority layer.

If graph state is lost across process restart, the system can no longer reliably answer:

```text
Does this memory still exist?
Is this memory current?
Was it suppressed?
Was it superseded?
What episode or observation supports this derived memory?
Which thread/entity links make this relevant?
```

For Character Memory, this is not only a storage concern. It affects whether the assistant behaves from grounded continuity or from stale/unvalidated vector candidates.

---

# 2. Scope

## 2.1 Existing concepts hardened

No new memory concepts are introduced in this phase.

The phase hardens existing v0.1 concepts:

```text
Episode
Observation
Entity
MemoryThread
DerivedMemory
MemoryLink
Retention state
Currentness
Supersession
Suppression
Provenance
Bounded graph expansion
```

## 2.2 Non-goals

Do not implement in v0.1.1:

```text
new memory object types
relationship-state model
character-signal reinforcement
reflection scheduler
separate Assertion / Claim / EvidenceLink / BeliefAssessment classes
domain-scoped source credibility
advanced association graph
retention governance beyond existing lifecycle state
multimodal observation model
distributed transactions across Qdrant and Oxigraph
heavy ontology or OWL reasoning
```

This phase is operational hardening of the v0.1 storage model, not expansion of the ontology.

---

# 3. Deliverables

```text
configurable Oxigraph graph store mode
persistent Oxigraph graph authority implementation
restart-safe graph authority tests
retrieval behavior tests after graph restart
Qdrant/Oxigraph reconciliation diagnostics
partial-persistence visibility gates
documentation for persistent graph setup
```

---

# 4. Configuration direction

The graph authority should support three modes:

```rust
GraphStoreMode::Service { endpoint: Url }
GraphStoreMode::Persistent { path: PathBuf }
GraphStoreMode::InMemory
```

or equivalent settings:

```toml
[graph]
mode = "service"
endpoint = "http://localhost:7878"
```

In-memory mode remains useful for deterministic unit tests and local fast fixtures.

Service mode is the default recommendation for applications that expect memory to survive process restart. Embedded filesystem persistence remains available for explicit local or isolated deployments.

Implemented default service configuration:

```text
GRAPH_STORE_MODE=service
OXIGRAPH_CONNECTION_STRING=http://localhost:7878
```

Start the local service with:

```text
docker compose -f docker-compose.oxigraph.yml up -d
```

Service-mode graph reads use targeted remote SPARQL and named-graph hydration rather than a whole-dataset application-side snapshot. Ordinary object queries, provenance/thread lookups, bounded expansion, and diagnostics should remain scoped to the refs, IDs, link frontiers, or categories being requested.

`GRAPH_STORE_MODE=persistent` is the explicit embedded filesystem-backed mode, where `OXIGRAPH_CONNECTION_STRING` is a local path such as `./data/oxigraph`. `GRAPH_STORE_MODE=in_memory` is the explicit test/fixture override.

---

# 5. Required behavior

Persistent graph mode must preserve:

```text
memory object existence
object type triples
episode -> observation links
derived memory -> episode/observation provenance links
entity links
thread links
supersession links
retention state
currentness
bounded expansion behavior
```

After restart, retrieval must still obey:

```text
Qdrant candidate
  -> graph validation
  -> lifecycle/currentness/provenance filtering
  -> bounded graph expansion
  -> ContinuityContextPack
```

Retrieval must not fall back to treating Qdrant payloads as authoritative memory if graph validation fails.

---

# 6. Qdrant/Oxigraph reconciliation

This phase should add at least diagnostic reconciliation for cross-store drift.

Detect:

```text
Qdrant point exists but graph object is missing
graph object exists but Qdrant point is missing
Qdrant graph_uri does not match canonical graph URI
Qdrant payload says active but graph says suppressed
Qdrant payload says current but graph says superseded or non-current
Qdrant payload schema_version is unsupported
graph object has missing required provenance
```

Initial reconciliation may report rather than fully repair all cases.

This phase keeps reconciliation admin-facing. It does not expose diagnostics through the public `CharacterMemory` facade.

Retrieval diagnostics in this phase remain reconciliation-oriented and admin-facing. The v0.1 family may expose light per-retrieval rationale or telemetry, but durable first-class `RetrievalTrace` objects are deferred to v0.4.

Minimum behavior:

```text
vector-only candidates are excluded from normal retrieval
graph-only records remain valid but may have degraded semantic recall
suppressed or superseded graph records are excluded even if Qdrant payload is stale
```

---

# 7. Partial persistence policy

v0.1.1 should make this rule explicit:

```text
Partial persistence may create repairable degraded state.
It must not create behavior-influencing ungrounded memory.
```

Acceptable degraded states:

```text
graph object exists but vector point is missing
optional thread/entity link is missing
reflection/vector indexing failed
Qdrant is unavailable but graph lookup still works
```

Unacceptable visible states:

```text
vector point exists and is used without graph validation
DerivedMemory influences behavior without provenance
suppressed/deleted memory appears in normal retrieval
superseded memory appears as current
Qdrant payload joins to the wrong graph object
duplicate retry writes inflate salience or apparent recurrence
```

---

# 8. Acceptance criteria

```text
Persistent graph mode can be configured.
In-memory graph mode remains available.
Episode objects survive graph store restart.
Observation links survive graph store restart.
DerivedMemory provenance links survive graph store restart.
Entity and thread links survive graph store restart.
Suppression and supersession state survive graph store restart.
Currentness filtering works after restart.
Retrieval after restart excludes suppressed, deleted, non-current, and superseded records by default.
Qdrant candidates whose graph objects are missing are rejected from normal retrieval.
Reconciliation diagnostics can report vector-only and graph-only drift.
Stable object ID to graph IRI mapping remains unchanged.
Existing v0.1 public APIs continue to work.
```

---

# 9. Validation expectations

Concrete implementation plans for this phase should include:

```text
cargo fmt --check
cargo check
cargo test --no-run
cargo test --lib
targeted persistent Oxigraph restart tests
targeted retrieval-after-restart tests
targeted reconciliation diagnostics tests
bounded graph expansion regression tests
lifecycle filtering regression tests
```

If live Qdrant is required for reconciliation smoke tests, those tests should remain prerequisite-gated and documented, as in the existing v0.1 validation approach.

---

# 10. Design constraints

```text
Qdrant remains candidate recall only.
Oxigraph remains authoritative for object existence, links, provenance, lifecycle, currentness, and correction.
Stable IDs and deterministic graph IRIs must not change.
Persistent graph mode must not require changing the public v0.1 object model.
In-memory mode must remain available for tests.
No new v0.2 continuity concepts should be introduced in this phase.
```

---

# 11. Expected outcome

After v0.1.1, the starter Character Memory model is operationally durable.

The system can safely move to v0.2 continuity and reflection work because the graph authority needed for provenance, lifecycle, currentness, and retrieval validation survives process restarts.

In short:

```text
v0.1 made the starter memory model feature-complete.
v0.1.1 makes the starter memory model persistence-safe.
```
