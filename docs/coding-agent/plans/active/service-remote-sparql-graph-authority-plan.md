# Plan: Service Remote SPARQL Graph Authority

- status: draft
- generated: 2026-05-03
- last_updated: 2026-05-03
- work_type: code

## Goal

- Replace the Oxigraph HTTP service adapter's whole-dataset snapshot bridge with targeted remote SPARQL query/update behavior while preserving graph-authoritative retrieval, RDF-backed hydration, lifecycle/currentness filtering, bounded expansion semantics, and embedded Oxigraph parity.

## Definition of Done

- Service-mode reads no longer execute an unbounded `SELECT ?g ?s ?p ?o WHERE { GRAPH ?g { ?s ?p ?o } }` for ordinary object queries, provenance queries, thread queries, bounded expansion, or diagnostics.
- Service-mode graph reads use targeted SPARQL queries scoped by object refs, object types, provenance IDs, thread IDs, frontier refs, graph URIs, or diagnostic categories as appropriate.
- Embedded persistent/in-memory Oxigraph behavior remains unchanged except for any shared query contract improvements needed to preserve parity.
- Unit tests cover remote query construction or HTTP request bodies for each service read path using an in-repo test seam/fake client. Adding an external mock HTTP dependency is out of scope unless the plan is explicitly updated to own `Cargo.toml` and `Cargo.lock`.
- Live Oxigraph smoke validation proves service-mode retrieval and cleanup still work against the dedicated test container.
- Documentation and active roadmap notes identify the snapshot bridge as removed, not merely tracked.

## Scope / Non-goals

- Scope:
  - `Cargo.toml` and `Cargo.lock` only if Task_1 explicitly replans to allow a test-only HTTP mocking dependency
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `src/internal/infrastructures/graph/**`
  - `src/internal/infrastructures/graph/sparql_selectors.rs`
  - `src/internal/repositories/graph_authority_store.rs`
  - `src/internal/repositories/reconciliation.rs`
  - `docs/design/database/graph_schema_design.md`
  - `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md`
  - `docs/coding-agent/plans/active/v0-1-1-persistent-graph-authority-plan.md`
  - tests adjacent to changed production modules
- Non-goals:
  - Changing public `CharacterMemory` APIs or graph store mode names.
  - Reintroducing persisted sidecar hydration or Qdrant-payload authority.
  - Adding distributed transactions across Qdrant and Oxigraph.
  - Replacing embedded Oxigraph with HTTP service mode in unit tests that should stay deterministic.
  - Building a public/admin reconciliation facade.

## Context (workspace)

- Related files/areas:
  - Current HTTP adapter: `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - Shared embedded selectors: `src/internal/infrastructures/graph/sparql_selectors.rs`
  - Graph contract and bounded expansion helpers: `src/internal/repositories/graph_authority_store.rs`
  - Reconciliation diagnostics: `src/internal/repositories/reconciliation.rs`
- Existing patterns or references:
  - Embedded Oxigraph uses `SparqlGraphSelectors` over an in-process `Store`.
  - HTTP service mode currently snapshots all named graphs into an in-memory `Store`, then reuses embedded selector/hydration logic.
  - The current bridge is correctness-preserving for small/local datasets but is not acceptable as the long-term service adapter for larger graph datasets.
  - LongMemEval live benchmark logs showed retrieval latency rising as shard graph state grew, which is consistent with paying whole-dataset snapshot cost on ordinary service-mode reads.
  - Live Oxigraph validation uses `docker-compose.oxigraph.test.yml` and `OXIGRAPH_TEST_CONNECTION_STRING=http://localhost:7879`.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`
  - `docs/design/database/graph_schema_design.md`
  - `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md`
  - `docs/coding-agent/plans/active/v0-1-1-persistent-graph-authority-plan.md`

## Open Questions (max 3)

- Q1: Should service-mode remote query helpers live beside `OxigraphHttpGraphAuthorityStore`, or should selector abstractions be split into embedded and remote backends under `src/internal/infrastructures/graph/`?
- Q2: Should service-mode diagnostics remain full-scan by category for this refactor, or should they be paginated/streamed to avoid large in-memory diagnostic result sets?
- Q3: Should live Oxigraph smoke validation assert the absence of the whole-dataset snapshot query through test instrumentation, or is unit-level HTTP request-body verification sufficient?

## Assumptions

- A1: RDF/Oxigraph remains the only canonical graph authority for service and embedded modes.
- A2: Domain hydration from RDF remains required; service mode may hydrate only the subset needed for each request.
- A3: The dedicated test Oxigraph service on port 7879 remains the live-service validation target.
- A4: This refactor should preserve public API behavior and retrieval outputs for existing tests.

## Tasks

### Task_1: Design Remote Query Boundary

- type: design
- owns:
  - `docs/coding-agent/plans/active/service-remote-sparql-graph-authority-plan.md`
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `src/internal/infrastructures/graph/sparql_selectors.rs`
- depends_on: []
- description: |
  Decide the internal boundary for service-mode remote SPARQL helpers and map each current `snapshot_store()` read path to targeted remote query requirements. Record the final query-helper shape before implementation.
- acceptance:
  - Each service read path has an explicit targeted query strategy: `query_objects`, provenance lookup, thread lookup, bounded expansion, diagnostic objects, and diagnostic links.
  - The plan identifies which logic stays shared with embedded selectors and which logic becomes service-specific.
  - The plan records whether diagnostic reads are category-scoped, paginated, or temporarily bounded.
  - The plan records how unit tests will assert the whole-dataset snapshot query is gone.
  - The plan records the HTTP query test strategy: in-repo fake client/test seam by default, or an explicit replan that adds `Cargo.toml` and `Cargo.lock` to the owned scope for a test-only mock dependency.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Plan decision log updated with remote query boundary, query strategy per read path, and test strategy"

### Task_2: Add Targeted Service Query Helpers

- type: impl
- owns:
  - `src/internal/infrastructures/graph/**`
  - `src/internal/repositories/graph_authority_store.rs`
  - `src/internal/repositories/reconciliation.rs`
  - tests adjacent to changed production modules
  - `Cargo.toml` and `Cargo.lock` only if Task_1 explicitly approves a test-only HTTP mocking dependency
- depends_on: [Task_1]
- description: |
  Implement HTTP service query helpers that POST targeted SPARQL to `/query`, parse SPARQL JSON results, and hydrate only the objects/links needed by each service read path.
- acceptance:
  - `query_objects` fetches only requested refs/ids/types and honors limits without snapshotting unrelated graphs.
  - Provenance and thread lookup fetch candidate derived memories plus only links needed for currentness/lifecycle/provenance filtering.
  - Bounded expansion fetches graph frontier/link evidence iteratively or through bounded targeted queries, not through full graph snapshot.
  - Diagnostic object/link reads do not use the whole-dataset snapshot bridge.
  - Request-body tests or query-builder tests fail if any ordinary service read path emits the unbounded `SELECT ?g ?s ?p ?o WHERE { GRAPH ?g { ?s ?p ?o } }`.
  - Existing service update behavior continues to replace named graphs atomically enough for current phase expectations.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib internal::infrastructures::graph::oxigraph_authority_store"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib live_smoke_endpoint_guard_allows_only_local_test_service"
  - kind: command
    required: true
    owner: worker
    detail: "rg -n 'SELECT \\?g \\?s \\?p \\?o WHERE \\{ GRAPH \\?g \\{ \\?s \\?p \\?o \\} \\}' src/internal/infrastructures/graph should return no matches"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review HTTP service read paths and confirm the full graph snapshot query is not used for ordinary reads"

### Task_3: Preserve Embedded/Service Parity

- type: test
- owns:
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `src/internal/infrastructures/graph/sparql_selectors.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/internal/repositories/reconciliation.rs`
- depends_on: [Task_2]
- description: |
  Add parity tests that compare embedded and service-targeted query behavior for representative graph authority operations without requiring broad public API changes.
- acceptance:
  - Tests cover object query, provenance query, thread query, bounded expansion, lifecycle/currentness filtering, supersession filtering, and diagnostic reads.
  - Tests include at least one stale/unrelated named graph that must not be fetched for a targeted object query.
  - Retrieval behavior remains graph-authoritative when vector candidates reference missing, suppressed, non-current, or superseded memories.
  - Reconciliation diagnostics still report vector-only, graph-only, URI mismatch, stale lifecycle/currentness, unsupported schema, and missing provenance.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib internal::infrastructures::graph"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib internal::repositories::reconciliation"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib internal::repositories::retrieve_pipeline"

### Task_4: Live Service Validation

- type: test
- owns:
  - `docker-compose.oxigraph.test.yml`
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `docs/coding-agent/plans/active/service-remote-sparql-graph-authority-plan.md`
- depends_on: [Task_2, Task_3]
- description: |
  Validate the targeted remote query implementation against the dedicated Oxigraph test container and record cleanup evidence.
- acceptance:
  - Live smoke uses only `OXIGRAPH_TEST_CONNECTION_STRING=http://localhost:7879`.
  - Live smoke writes representative objects/links, exercises retrieval and lifecycle filters, and cleans up every named graph it creates.
  - Post-smoke count for smoke graph URIs is zero.
  - Validation evidence records the Oxigraph container image and endpoint used.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "docker compose -f docker-compose.oxigraph.test.yml up -d"
  - kind: command
    required: true
    owner: orchestrator
    detail: "OXIGRAPH_TEST_CONNECTION_STRING=http://localhost:7879 cargo test --lib oxigraph_http_service_live_smoke_upserts_queries_and_filters -- --ignored"
  - kind: manual
    required: true
    owner: orchestrator
    detail: "Verify smoke graph quad count is zero after cleanup"

### Task_5: Documentation And Debt Removal

- type: docs
- owns:
  - `docs/design/database/graph_schema_design.md`
  - `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md`
  - `docs/coding-agent/plans/active/v0-1-1-persistent-graph-authority-plan.md`
  - `docs/coding-agent/plans/active/service-remote-sparql-graph-authority-plan.md`
- depends_on: [Task_2, Task_3, Task_4]
- description: |
  Update documentation to say the service adapter uses targeted remote SPARQL reads, and remove or close the currently tracked snapshot-bridge debt note.
- acceptance:
  - Docs no longer describe full-graph service snapshots as current behavior.
  - Any remaining limits are described as explicit operational constraints rather than hidden debt.
  - The original persistent graph authority plan references this follow-up as completed or superseded.
  - Schema docs remain focused on database design and do not gain runtime configuration clutter.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Documentation cross-check against current implementation behavior and schema-cheat-sheet scope"

### Task_6: Closeout Review

- type: review
- owns: []
- depends_on: [Task_2, Task_3, Task_4, Task_5]
- description: |
  Review the implementation against graph authority invariants, scale/performance risk, and plan acceptance criteria.
- acceptance:
  - Reviewer confirms no ordinary service read path uses the whole-dataset snapshot query.
  - Reviewer confirms no sidecar hydration or Qdrant-payload authority was introduced.
  - Reviewer confirms service and embedded graph behavior remain aligned for covered operations.
  - Required validation evidence is complete or explicitly waived.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo test --lib"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo clippy --all-targets -- -D warnings"
  - kind: command
    required: true
    owner: orchestrator
    detail: "rg -n 'SELECT \\?g \\?s \\?p \\?o WHERE \\{ GRAPH \\?g \\{ \\?s \\?p \\?o \\} \\}' src/internal/infrastructures/graph should return no matches"
  - kind: review
    required: true
    owner: reviewer
    detail: "Full diff review against this plan and the v0.1.1 graph-authority roadmap"

## Task Waves (explicit parallel dispatch sets)

Interpretation:
- Tasks listed in the same wave are intended to be dispatched in parallel by default when `owns` are disjoint and dependencies are met.
- Waves are executed sequentially.

- Wave 1 (sequential): [Task_1]
- Wave 2 (sequential): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (sequential): [Task_4]
- Wave 5 (parallel): [Task_5]
- Wave 6 (sequential): [Task_6]

## E2E / Visual Validation Spec

- Not applicable. No UI or browser-visible behavior is in scope.

## Rollback / Safety

- Keep embedded Oxigraph behavior as the known-good parity baseline.
- Keep service writes scoped to named graph replacement behavior already covered by the persistent graph authority phase.
- Live tests must use the dedicated test service endpoint on port 7879 and must keep per-test cleanup.
- If targeted service reads cannot preserve bounded expansion/lifecycle parity cleanly, stop and replan around a shared query abstraction before changing public construction defaults.

## Progress Log (append-only)

- 2026-05-04 00:00 Planning branch created: `feature-2026-05-04-service-remote-sparql-graph-authority`.
  - Summary: Reused the existing active plan for the Oxigraph service read optimization scope and recorded the LongMemEval performance context.
  - Validation evidence: Planning artifact only; implementation validation pending.
  - Notes: This branch should remove snapshot-based service reads without changing retrieval breadth or eval semantics.

- 2026-05-03 00:00 Plan drafted: [Task_1, Task_2, Task_3, Task_4, Task_5, Task_6]
  - Summary: Created future-work plan to replace the Oxigraph HTTP service full-graph snapshot bridge with targeted remote SPARQL reads.
  - Validation evidence: Plan-format checklist applied; implementation intentionally not started.
  - Notes: Current worktree already contains review-fix edits from the persistent graph authority PR; this plan is a separate future-work artifact.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-04 00:00 Decision:
  - Trigger / new insight: LongMemEval retrieval timings showed service-mode graph reads are a core-library performance bottleneck, not an eval-only configuration concern.
  - Plan delta (what changed): Confirmed this existing plan is the implementation scope for removing whole-dataset remote snapshots from ordinary graph reads.
  - Tradeoffs considered: Optimize service graph reads while preserving graph-authoritative hybrid retrieval and avoiding vector-only fallbacks.
  - User approval: yes; user requested branches and committed plans for ready work scopes.

- 2026-05-04 00:00 Decision:
  - Trigger / new insight: Remaining service remote-SPARQL open questions were resolved before implementation.
  - Plan delta (what changed): Keep remote query helpers beside `OxigraphHttpGraphAuthorityStore` initially; make diagnostics category-scoped and explicitly bounded or paginated where practical; prefer smaller targeted frontier queries for bounded expansion before optimizing query shape; use unit/request-body verification plus the no-snapshot grep gate as the hard no-snapshot evidence, with live smoke focused on service behavior.
  - Tradeoffs considered: Avoid premature selector abstraction while still preventing hidden unbounded reads and preserving embedded/service parity.
  - User approval: yes; user accepted these recommendations.

- 2026-05-03 00:00 Decision:
  - Trigger / new insight: Review accepted the service snapshot bridge as tracked debt, but requested a concrete future plan to remove it.
  - Plan delta (what changed): Added a dedicated follow-up plan for targeted remote SPARQL service reads and validation.
  - Tradeoffs considered: A single broad implementation task would be too risky because object query, provenance, bounded expansion, diagnostics, and live-service validation have different query shapes.
  - User approval: pending

- 2026-05-03 00:00 Decision:
  - Trigger / new insight: Plan review found Task_2 owns was too narrow, no hard no-snapshot validation was required, and mock HTTP dependency scope was unspecified.
  - Plan delta (what changed): Expanded Task_2 owns to graph infrastructure plus graph contract/reconciliation files, added required `rg` validation for the forbidden snapshot query, and required an in-repo fake/test seam unless a replan explicitly owns dependency changes.
  - Tradeoffs considered: Keeping the test seam in-repo avoids adding dependencies for a future refactor plan; allowing a replan path keeps the option open if request-body testing becomes awkward without a mock server.
  - User approval: pending

## Notes

- Risks:
  - Remote SPARQL query construction can drift from embedded selector semantics if parity tests are too narrow.
  - Bounded expansion may need iterative remote calls; a single large query could recreate the same scale problem in a different form.
  - Diagnostics may still need pagination or streaming if graph size grows beyond local memory expectations.
  - SPARQL JSON literal handling currently preserves lexical values; typed/language literal support should be revisited if external writers add richer RDF terms.
- Edge cases:
  - Missing graph roots.
  - Vector-only candidates.
  - Suppressed, deleted, archived, non-current, and superseded derived memories.
  - Hub nodes with high fanout.
  - Duplicate relation triples owned by different memory link graphs.
  - Legacy or malformed vector payload diagnostics.
