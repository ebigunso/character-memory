# Plan: v0.1.1 Persistent Graph Authority

- status: in_progress
- generated: 2026-05-01
- last_updated: 2026-05-01
- work_type: mixed

## Goal
- Make the Oxigraph graph authority configurable and durable across restarts while preserving graph-authoritative retrieval, lifecycle/currentness filtering, provenance validation, bounded graph expansion, and Qdrant-as-candidate-only behavior.

## Definition of Done
- Persistent and in-memory graph modes are configurable without exposing backend-specific types through public/domain contracts.
- Persistent graph mode survives restart for objects, links, provenance, suppression, supersession, currentness, and bounded expansion.
- Retrieval after graph restart rejects vector-only candidates and excludes suppressed, deleted, non-current, and superseded records by default.
- Reconciliation diagnostics report vector-only, graph-only, graph URI mismatch, stale lifecycle/currentness hints, unsupported schema, and missing-provenance cases.
- Partial-persistence behavior is explicit: degraded state may be repairable, but ungrounded vector-only memory must not influence behavior.
- Required Rust checks, targeted restart/reconciliation tests, reviewer approval, and gated Qdrant/Oxigraph smoke-test expectations are recorded.

## Scope / Non-goals
- Scope:
  - Graph store mode/settings with Oxigraph service graph authority as the application default, embedded persistent mode as an explicit filesystem-backed option, and in-memory mode available for tests or explicit environment configuration.
  - Docker-backed Oxigraph service configuration aligned with the existing Qdrant service pattern.
  - Separate Docker-backed Oxigraph test service configuration for live smoke tests so production/default service data is not touched by tests.
  - Test cleanup for durable external-service and filesystem-backed data created by each test.
  - Persistent Oxigraph construction and restart-safe graph-authority behavior for both service-backed and embedded persistent modes.
  - Durable RDF/Oxigraph-backed canonical object/link hydration for reopened graph stores.
  - Restart-safe retrieval and lifecycle/currentness regression coverage.
  - Diagnostic Qdrant/Oxigraph reconciliation.
  - Documentation for persistent graph setup and partial-persistence policy.
- Non-goals:
  - New memory object types or v0.2 continuity/reflection concepts.
  - Distributed transactions across Qdrant and Oxigraph.
  - Physical hard-delete/redaction policy beyond existing lifecycle semantics.
  - Public SPARQL APIs or backend-specific public DTOs.
  - Full raw transcript storage in graph/vector stores.
  - Automated drift repair beyond diagnostics unless explicitly replanned.
  - Persisted sidecar stores for canonical object/link hydration.

## Context (workspace)
- Related files/areas:
  - `Cargo.toml`
  - `Cargo.lock`
  - `.env.example`
  - `docker-compose.oxigraph.yml`
  - `docker-compose.oxigraph.test.yml`
  - `src/lib.rs`
  - `src/config/**`
  - `src/internal/infrastructures/graph/**`
  - `src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs`
  - `src/internal/repositories/graph_authority_store.rs`
  - `src/internal/repositories/vector_candidate_store.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/internal/repositories/correction_forget_pipeline.rs`
  - `tests/v0_1_public_facade_tests.rs`
  - `docs/design/database/**`
- Existing patterns or references:
  - `CharacterMemory::new_with_embedding_provider` originally constructed `OxigraphGraphAuthorityStore::new_in_memory()`.
  - Existing Qdrant live-service validation is configured through `docker-compose.qdrant.yml`.
  - `OxigraphGraphAuthorityStore` materializes RDF quads into Oxigraph named graphs, but canonical object/link hydration still depends on sidecar in-memory maps.
  - `RetrievePipeline` already treats Qdrant candidates as non-authoritative and omits missing graph roots.
  - `CorrectionForgetPipeline` already mutates graph first and treats vector maintenance failures as degraded state.
  - Oxigraph `Store::open` requires the `rocksdb` feature in the current dependency family.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`
  - `docs/roadmap/development_roadmap.md`
  - `docs/project_philosophy.md`
  - `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md`
  - `docs/design/database/graph_schema_design.md`
  - `docs/design/database/vector_payload_design.md`
  - `docs/design/database/schema_cheat_sheet.md`
  - `docs/decisions/implementation/ADR-I-0003-qdrant-oxigraph-defaults.md`
  - `docs/decisions/implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md`
  - `docs/decisions/implementation/ADR-I-0006-bounded-graph-expansion.md`
  - `docs/decisions/design/ADR-D-0006-supersession-and-suppression.md`
  - `docs/decisions/design/ADR-D-0008-preserve-source-references.md`

## Open Questions (max 3)
- None.

## Resolved Decisions
- Oxigraph service graph authority is the application default when using public construction.
- Live Oxigraph smoke tests use a separate test service endpoint, not the application default endpoint.
- Embedded persistent graph authority remains available as an explicit filesystem-backed mode.
- In-memory graph authority remains available for tests and for explicit environment configuration.
- Durable canonical hydration must reconstruct domain objects and links from RDF/Oxigraph state.
- Persisted sidecar hydration is not an acceptable implementation path for this phase.
- Reconciliation diagnostics remain internal/admin-facing for this phase; public facade exposure is deferred unless separately planned.
- `OXIGRAPH_CONNECTION_STRING` is treated as the Oxigraph HTTP endpoint for service graph mode and as the filesystem path only for embedded persistent graph mode.
  - `OXIGRAPH_TEST_CONNECTION_STRING` is treated as the Oxigraph HTTP endpoint for live smoke tests and defaults to a separate local test service.
- Tests that create durable external-service or filesystem-backed data must clean up the data they created, either per test case or within an explicitly bounded test case group.
- Docker-backed Oxigraph services use `ghcr.io/oxigraph/oxigraph:0.5.8`.
- `GRAPH_STORE_MODE=service` is the default application mode.
- `GRAPH_STORE_MODE=persistent` is the explicit embedded filesystem-backed persistent mode.
- `GRAPH_STORE_MODE=in_memory` is the explicit environment/config override for in-memory graph authority.
- Oxigraph persistence support is enabled through the crate dependency feature required for `Store::open`.

## Assumptions
- A1: Stable object IDs and graph IRI mapping must remain unchanged.
- A2: Backend-specific Oxigraph/Qdrant/RDF types stay inside infrastructure modules.
- A3: In-memory graph mode remains available for deterministic unit tests and fast local fixtures.
- A4: Qdrant and Oxigraph live smoke tests remain prerequisite-gated and documented, not mandatory for ordinary local unit-test completion.
- A5: Roadmap version labels are acceptable in plan/docs artifacts, but durable production comments and identifiers should use stable domain language.

## Tasks

### Task_1: Select Persistence Boundary And Configuration Strategy
- type: design
- owns:
  - `docs/coding-agent/plans/active/v0-1-1-persistent-graph-authority-plan.md`
- depends_on: []
- description: |
  Resolve the remaining implementation boundary before code changes: graph store mode config names, Oxigraph service-vs-embedded persistence strategy, public constructor behavior, and diagnostic exposure boundary. Use implementation files as read-only decision inputs and record decisions in this plan before dispatching implementation tasks.
- acceptance:
  - Plan records the selected graph mode/config surface for service-backed persistent-by-default behavior, explicit embedded persistent mode, and explicit in-memory override.
  - Plan records whether Oxigraph service mode, embedded `rocksdb`, or both are supported.
  - Plan confirms RDF/Oxigraph-backed canonical hydration remains the only accepted restart-safe hydration strategy.
  - Plan records whether reconciliation diagnostics remain internal/admin for this phase or require a separately scoped public exposure task.
  - Decision explicitly rejects treating Qdrant payloads as authoritative fallback data.
  - Decision explicitly rejects persisted sidecar hydration as a fallback path.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Decision log updated with selected boundary, tradeoffs, and impact on later Task_X owns scopes"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review selected boundary against v0.1.1 design, backend ADRs, and repo rules before implementation dispatch"

### Task_2: Add Graph Store Mode Configuration
- type: impl
- owns:
  - `src/config/**`
  - `.env.example`
  - `docker-compose.oxigraph.yml`
  - `docker-compose.oxigraph.test.yml`
  - `src/lib.rs`
- depends_on: [Task_1]
- description: |
  Add application settings for graph authority mode and Oxigraph endpoint/path, then wire public construction to select service-backed, embedded persistent, or in-memory graph authority without exposing backend-specific types through public/domain APIs.
- acceptance:
  - Settings can express service-backed persistent, embedded persistent, and in-memory graph authority modes.
  - Service mode has a required, validated HTTP endpoint.
  - Live smoke tests use a separate test endpoint setting and do not default to the production/application endpoint.
  - Embedded persistent mode has a required, validated filesystem path.
  - In-memory mode remains available for deterministic tests and fast fixtures.
  - Existing v0.1 public APIs continue to compile and use service-backed Oxigraph when no graph mode is configured.
  - `.env.example` documents the graph settings without implying Qdrant is authoritative.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib config"

### Task_3: Implement Persistent Oxigraph Authority Construction
- type: impl
- owns:
  - `Cargo.toml`
  - `Cargo.lock`
  - `docker-compose.oxigraph.yml`
  - `docker-compose.oxigraph.test.yml`
  - `src/internal/infrastructures/graph/**`
- depends_on: [Task_1, Task_2]
- description: |
  Add Oxigraph authority construction using the selected service-backed default and embedded persistent strategy. Preserve the in-memory constructor and keep Oxigraph/RDF details inside graph infrastructure.
- acceptance:
  - Service graph constructor targets a durable Oxigraph HTTP service at the configured endpoint.
  - Embedded persistent graph constructor opens or creates a durable Oxigraph store at the configured path.
  - Docker Compose configuration can start an Oxigraph service with a persistent volume.
  - Docker Compose configuration can start a separate Oxigraph test service with a distinct port and volume.
  - Live smoke test deletes its created smoke graphs before completing, including failure paths that can be represented without panicking before cleanup.
  - Filesystem-backed persistence tests remove their temporary graph directories through cleanup guards.
  - In-memory graph constructor remains available and used by tests that do not need persistence.
  - Store initialization errors are surfaced through existing error patterns.
  - Persistent mode does not require public API consumers to construct Oxigraph-specific values.
  - Tests use isolated temporary graph paths and avoid Windows file-lock assumptions.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::graph --lib"
  - kind: command
    required: false
    owner: worker
    detail: "docker compose -f docker-compose.oxigraph.test.yml up -d"
  - kind: command
    required: false
    owner: worker
    detail: "OXIGRAPH_TEST_CONNECTION_STRING=http://localhost:7879 cargo test --lib oxigraph_http_service_live_smoke_upserts_queries_and_filters -- --ignored"

### Task_4: Make RDF/Oxigraph Hydration Restart-Safe
- type: impl
- owns:
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `src/internal/infrastructures/graph/rdf_mapping.rs`
  - `src/internal/infrastructures/graph/sparql_selectors.rs`
  - `src/internal/infrastructures/graph/vocabulary.rs`
- depends_on: [Task_3]
- description: |
  Remove the restart-safety gap where canonical objects and links only exist in sidecar in-memory maps. Reconstruct canonical objects and links from durable RDF/Oxigraph state so reopened embedded persistent graph stores and service-backed graph snapshots can satisfy graph-authority methods without persisted sidecar hydration.
- acceptance:
  - Reopened embedded graph store can retrieve stored Episode, Observation, Entity, MemoryThread, and DerivedMemory objects by ID.
  - Service-backed graph store can retrieve stored Episode, Observation, Entity, MemoryThread, and DerivedMemory objects by ID through the HTTP-backed adapter.
  - Reopened/service-backed graph store can retrieve MemoryLink relations needed for provenance, entity links, thread links, supersession, and bounded expansion.
  - Suppression, deletion/retention state, currentness, and supersession state survive restart.
  - Bounded expansion after restart returns graph-validated context using durable graph state.
  - Hydration remains deterministic and preserves stable ID to graph IRI mapping.
  - No persisted sidecar store is introduced for canonical object/link hydration.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::graph::oxigraph_authority_store --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::graph --lib"
  - kind: test
    required: true
    owner: worker
    detail: "Targeted persistent restart tests cover objects, links, provenance, lifecycle/currentness, supersession, and bounded expansion"

### Task_5: Add Retrieval-After-Restart And Lifecycle Coverage
- type: test
- owns:
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/internal/repositories/correction_forget_pipeline.rs`
  - `src/internal/repositories/test_support.rs`
  - `tests/**`
- depends_on: [Task_4]
- description: |
  Add regression coverage that proves retrieval after graph restart still flows through graph validation and default lifecycle/currentness filtering. Use fake or fixture vector candidates where possible so graph behavior is deterministic without live Qdrant.
- acceptance:
  - Retrieval after graph restart returns current graph-valid records with provenance.
  - Retrieval after graph restart excludes suppressed, deleted, non-current, and superseded records by default.
  - Vector candidates whose graph objects are missing are rejected from normal retrieval.
  - Stale vector lifecycle/currentness hints cannot override graph authority.
  - Existing public facade behavior remains compatible with the new graph configuration.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::repositories::retrieve_pipeline --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::repositories::correction_forget_pipeline --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --test v0_1_public_facade_tests"
  - kind: test
    required: true
    owner: worker
    detail: "Targeted retrieval-after-restart tests prove graph validation and lifecycle filtering"

### Task_6: Add Qdrant/Oxigraph Reconciliation Diagnostics
- type: impl
- owns:
  - `src/internal/repositories/graph_authority_store.rs`
  - `src/internal/repositories/vector_candidate_store.rs`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**reconciliation**`
  - `src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs`
  - `src/internal/infrastructures/graph/**`
- depends_on: [Task_1, Task_4]
- description: |
  Add internal/admin diagnostic reconciliation for cross-store drift. Prefer backend-neutral diagnostic DTOs and fake-store test coverage. Do not expose diagnostics through the public `CharacterMemory` facade in this task; if Task_1 selects public exposure, split that into a separate follow-up task with explicit public facade ownership.
- acceptance:
  - Diagnostics report Qdrant point exists but graph object is missing.
  - Diagnostics report graph object exists but Qdrant point is missing.
  - Diagnostics report Qdrant graph URI mismatch against canonical graph URI.
  - Diagnostics report stale Qdrant lifecycle/currentness hints when graph says suppressed, superseded, deleted, or non-current.
  - Diagnostics report unsupported vector payload schema versions.
  - Diagnostics report graph objects with missing required provenance.
  - Diagnostics are internal/admin-facing and do not alter normal retrieval behavior.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: test
    required: true
    owner: worker
    detail: "Fake-store reconciliation diagnostics tests cover all required drift classes"
  - kind: command
    required: false
    owner: worker
    detail: "docker compose -f docker-compose.qdrant.yml up -d"
  - kind: command
    required: false
    owner: worker
    detail: "QDRANT_CONNECTION_STRING=http://localhost:6334 cargo test --lib qdrant_candidate_store_live_smoke_upserts_filters_searches_and_deletes -- --ignored"

### Task_7: Document Persistent Graph Setup And Partial-Persistence Policy
- type: docs
- owns:
  - `.env.example`
  - `docs/design/database/**`
  - `docs/roadmap/development_roadmap.md`
  - `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md`
  - `docs/coding-agent/plans/active/v0-1-1-persistent-graph-authority-plan.md`
- depends_on: [Task_2, Task_3, Task_4, Task_5, Task_6]
- description: |
  Update operational documentation to explain persistent graph configuration, restart expectations, reconciliation diagnostics, and partial-persistence visibility rules.
- acceptance:
  - Docs show how to configure service-backed, embedded persistent, and in-memory graph modes.
  - Docs show how to start Oxigraph with Docker Compose.
  - Docs explain that Oxigraph is authoritative and Qdrant payloads are candidate hints only.
  - Docs explain acceptable degraded states and unacceptable visible states.
  - Docs document reconciliation diagnostic categories and the initial report-not-repair boundary.
  - Roadmap/design docs remain aligned with the implemented behavior.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Documentation cross-check against v0.1.1 acceptance criteria"

### Task_8: Full Validation And Reviewer Gate
- type: review
- owns: []
- depends_on: [Task_2, Task_3, Task_4, Task_5, Task_6, Task_7]
- description: |
  Run closeout validation and review the implementation against the phase plan, roadmap acceptance criteria, and backend authority invariants.
- acceptance:
  - Required validation evidence is present for all implementation and test tasks.
  - Reviewer status is APPROVED.
  - Review confirms no v0.2 concepts, distributed transaction semantics, or backend-specific public leaks were introduced.
  - Review confirms vector-only candidates cannot become behavior-influencing memory.
  - Any skipped live Qdrant or Oxigraph smoke test is explicitly documented with prerequisites.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo check"
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
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review against plan acceptance, v0.1.1 design, and graph-authority invariants"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (parallel): [Task_4]
- Wave 5 (parallel): [Task_5, Task_6]
- Wave 6 (parallel): [Task_7]
- Wave 7 (parallel): [Task_8]

## E2E / Visual Validation Spec

- Not applicable. Rust library/storage behavior only.

## Rollback / Safety
- Keep in-memory graph mode as a test fixture mode and explicit environment-configured override only.
- If Oxigraph persistent dependency enablement causes platform/build instability, stop after Task_1/Task_3 and replan dependency strategy before changing retrieval behavior.
- If full RDF-to-domain hydration proves too broad, stop and replan scope or sequencing; do not introduce persisted sidecar hydration or vector payload authority as a fallback.
- Reconciliation diagnostics must report drift before any repair behavior is added.

## Progress Log (append-only)

- 2026-05-01 19:43 Draft created.
  - Summary: Created v0.1.1 persistent graph authority phase plan on `feature/2026-05-01/persistent-graph-authority-plan`.
  - Validation evidence: Researcher completed repository context scan; plan-only change, commands not run.
  - Notes: Awaiting user approval before implementation.

- 2026-05-01 19:50 Reviewer revision completed.
  - Summary: Narrowed Task_1 to plan-only decisions, constrained Task_6 to internal/admin diagnostics, added repository module ownership for reconciliation wiring, and kept retrieval behavior ownership in Task_5/Task_8.
  - Validation evidence: Reviewer had returned NEEDS_REVISION before this update; follow-up review pending.
  - Notes: No implementation dispatched.

- 2026-05-02 00:00 User decision recorded.
  - Summary: Recorded persistent graph authority as the default, explicit in-memory mode for tests/env configuration, and RDF/Oxigraph-backed hydration as the only accepted implementation path.
  - Validation evidence: Plan-only update; commands not run.
  - Notes: Persisted sidecar hydration is now an explicit non-goal.

- 2026-05-02 00:00 Open questions resolved.
  - Summary: Resolved the final open question by keeping reconciliation diagnostics internal/admin-facing for this phase.
  - Validation evidence: Plan-only update; commands not run.
  - Notes: Public diagnostics exposure would require a separate future plan/task.

- 2026-05-02 00:00 Wave 1 completed: [Task_1]
  - Summary: Marked the plan in progress and recorded the remaining implementation-boundary decisions for graph configuration, Oxigraph persistence support, RDF/Oxigraph hydration, and internal diagnostics.
  - Validation evidence: Orchestrator reviewed the decision entries against the plan acceptance; reviewer check pending before implementation dispatch.
  - Notes: No production code changed in Task_1.

- 2026-05-02 00:00 Wave 2/3/4 partially implemented: [Task_2, Task_3, Task_4]
  - Summary: Added graph store mode settings, persistent-by-default constructor wiring, Oxigraph persistent constructor, RocksDB-backed Oxigraph feature, RDF/Oxigraph-backed graph hydration, named-graph replacement after reopen, and a persistent reopen regression test.
  - Validation evidence: `cargo fmt --check` passed. `cargo check` is blocked by `oxrocksdb-sys` requiring `libclang.dll`; Visual Studio developer environment is available, but no local libclang installation was found.
  - Notes: Do not mark Task_2, Task_3, or Task_4 done until required Cargo validation runs successfully.

- 2026-05-02 00:00 Wave 5 partially implemented: [Task_6]
  - Summary: Added internal graph/vector reconciliation diagnostics, backend-neutral diagnostic records, Qdrant scroll-backed diagnostic enumeration, fake-store diagnostic support, Oxigraph diagnostic object/link listing, and fake-store coverage for vector-only, graph-only, graph URI mismatch, stale lifecycle/currentness hints, unsupported vector schema, and missing provenance drift classes.
  - Validation evidence: `cargo fmt --check` passed; `git diff --check` passed with line-ending warnings only. `cargo check` remains blocked by `oxrocksdb-sys`/`bindgen` failing to find `clang.dll` or `libclang.dll`.
  - Notes: Do not mark Task_6 done until required Cargo validation and reconciliation tests run successfully.

- 2026-05-02 00:00 Native validation dependency installed.
  - Summary: Installed LLVM 22.1.4 and persisted `LIBCLANG_PATH=C:\Program Files\LLVM\bin` so `bindgen` can load `libclang.dll`.
  - Validation evidence: `cargo check -q` passed from the Visual Studio developer environment with `LIBCLANG_PATH` set.
  - Notes: Continue with targeted Cargo tests before marking Task_2, Task_3, Task_4, or Task_6 complete.

- 2026-05-02 00:00 Wave 2/3/4/5/6 implementation validated: [Task_2, Task_3, Task_4, Task_5, Task_6, Task_7]
  - Summary: Completed persistent-by-default graph configuration, RocksDB-backed Oxigraph persistence, RDF/Oxigraph object and link hydration, restart-safe retrieval coverage, internal reconciliation diagnostics, and persistent graph setup/partial-persistence documentation updates.
  - Validation evidence: `cargo fmt --check` passed; `cargo check` passed; `cargo test --lib config` passed; `cargo test internal::infrastructures::graph::oxigraph_authority_store --lib` passed; `cargo test internal::infrastructures::graph --lib` passed; `cargo test internal::repositories::retrieve_pipeline --lib` passed; `cargo test internal::repositories::correction_forget_pipeline --lib` passed; `cargo test --test v0_1_public_facade_tests` passed; `cargo test internal::repositories::reconciliation --lib` passed.
  - Notes: Live Qdrant smoke test remains prerequisite-gated and was not run.

- 2026-05-02 00:00 Closeout validation completed: [Task_8]
  - Summary: Re-ran full local closeout validation after adding the restart-retrieval regression.
  - Validation evidence: `cargo check` passed; `cargo test --no-run` passed; `cargo test --lib` passed with 217 passed and 1 ignored live Qdrant smoke test; `cargo clippy --all-targets -- -D warnings` passed.
  - Notes: External reviewer approval is still pending; no subagent reviewer was dispatched under the current delegation policy.

- 2026-05-02 00:00 Plan delta applied for Docker-backed Oxigraph service default.
  - Summary: Replanned graph authority configuration to make Oxigraph HTTP service mode the application default, keep embedded filesystem persistence as explicit `GRAPH_STORE_MODE=persistent`, and keep in-memory as explicit test/fixture mode.
  - Validation evidence: Plan updated before continuing further validation; `cargo check`, `cargo test --lib config`, `cargo test internal::infrastructures::graph::oxigraph_authority_store --lib`, and `cargo test --test v0_1_public_facade_tests` had passed for the current code direction before this plan correction. Full closeout rerun pending after docs and plan alignment.
  - Notes: Live Oxigraph service smoke is prerequisite-gated with `docker compose -f docker-compose.oxigraph.yml up -d` and `OXIGRAPH_CONNECTION_STRING=http://localhost:7878`.

- 2026-05-03 00:00 Plan delta applied for separate Oxigraph test service.
  - Summary: Replanned live Oxigraph smoke validation to use a dedicated Docker Compose service, distinct port, distinct volume, and explicit per-test cleanup instead of the application/default Oxigraph service.
  - Validation evidence: Plan updated before code changes; implementation and validation pending.
  - Notes: Production/default service remains `docker-compose.oxigraph.yml` on port 7878; live smoke service is planned as `docker-compose.oxigraph.test.yml` on port 7879; tests that create durable data must clean up what they create.

- 2026-05-02 00:00 Plan-aligned service default validation completed.
  - Summary: Revalidated after aligning plan, implementation, README, database docs, and roadmap docs around Oxigraph service mode as the default and embedded filesystem persistence as explicit mode.
  - Validation evidence: Stale-default grep found no remaining embedded/filesystem-default wording in current README/design/roadmap/active-plan docs; `docker compose -f docker-compose.oxigraph.yml config` passed with a Docker config-file access warning; `cargo fmt --check` passed; `cargo check` passed; `cargo test --lib config` passed; `cargo test internal::infrastructures::graph::oxigraph_authority_store --lib` passed with 25 passed and 1 ignored live Oxigraph smoke; `cargo test --test v0_1_public_facade_tests` passed; `cargo test internal::repositories::reconciliation --lib` passed; `cargo test --no-run` passed; `cargo test --lib` passed with 218 passed and 2 ignored live service smokes; `cargo clippy --all-targets -- -D warnings` passed.
  - Notes: Live Qdrant and live Oxigraph smoke tests remain prerequisite-gated and were not run.

- 2026-05-03 00:50 Live Oxigraph Docker service verified.
  - Summary: Started `docker-compose.oxigraph.yml`, verified the container is running, verified the HTTP `/query` endpoint responds, and ran the ignored live Oxigraph graph-authority smoke test.
  - Validation evidence: Superseded by the later `0.5.8` Docker validation entry below. The original default-service smoke started `charactermemory-oxigraph-1`, verified `0.0.0.0:7878->7878/tcp`, verified the `/query` endpoint, and passed the then-current ignored live smoke before the smoke was moved to the dedicated test service.
  - Notes: The Oxigraph container remains running after verification.

- 2026-05-03 01:05 Separate Oxigraph test service and per-test cleanup verified.
  - Summary: Added dedicated `docker-compose.oxigraph.test.yml`, moved the live smoke to `OXIGRAPH_TEST_CONNECTION_STRING`, and made the live smoke clean up the named graphs it creates. Filesystem persistence tests now use a cleanup guard for temporary graph directories.
  - Validation evidence: `docker compose -f docker-compose.oxigraph.test.yml config` passed; `docker compose -f docker-compose.oxigraph.test.yml up -d` started `charactermemory-oxigraph-test-1` on `0.0.0.0:7879->7878/tcp`; `OXIGRAPH_TEST_CONNECTION_STRING=http://localhost:7879 cargo test --lib oxigraph_http_service_live_smoke_upserts_queries_and_filters -- --ignored` passed with 1 passed; `SELECT (COUNT(*) AS ?count) WHERE { GRAPH ?g { ?s ?p ?o } }` against `http://localhost:7879/query` returned `0` after the smoke; `cargo fmt --check`, `cargo check`, `cargo test internal::infrastructures::graph::oxigraph_authority_store --lib`, `cargo test --lib`, and `cargo clippy --all-targets -- -D warnings` passed.
  - Notes: The production/default service on port 7878 and the test service on port 7879 are separate containers and volumes. Both containers remain running after verification.

- 2026-05-03 00:00 Follow-up image/doc-scope correction applied.
  - Summary: Updated Docker-backed Oxigraph services to `ghcr.io/oxigraph/oxigraph:0.5.8` and removed runtime graph-store configuration settings from the schema cheat sheet.
  - Validation evidence: `docker compose -f docker-compose.oxigraph.yml config` and `docker compose -f docker-compose.oxigraph.test.yml config` both resolve `ghcr.io/oxigraph/oxigraph:0.5.8`; stale schema-cheat-sheet config grep found no `GRAPH_STORE_MODE` or `OXIGRAPH_CONNECTION_STRING`; `docker compose -f docker-compose.oxigraph.test.yml up -d` pulled/recreated the test service with `0.5.8`; `OXIGRAPH_TEST_CONNECTION_STRING=http://localhost:7879 cargo test --lib oxigraph_http_service_live_smoke_upserts_queries_and_filters -- --ignored` passed with 1 passed; test endpoint count returned `0` quads after smoke cleanup; `docker compose -f docker-compose.oxigraph.yml up -d` recreated the default service with `0.5.8`; default endpoint responded to SPARQL `ASK`; both default and test containers report image `ghcr.io/oxigraph/oxigraph:0.5.8`.
  - Notes: Runtime configuration belongs in README/env/operational docs and the active phase plan, not in the schema cheat sheet.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-01 19:43 Decision:
  - Trigger / new insight: User requested a branch and concrete plan for the persistent graph authority phase.
  - Plan delta (what changed): Created a dedicated active execution plan rather than splitting the phase into separate plan files.
  - Tradeoffs considered: A single plan keeps config, persistence, retrieval, reconciliation, and docs tied to one graph-authority contract while still splitting execution into Task_X waves.
  - User approval: no

- 2026-05-01 19:43 Decision:
  - Trigger / new insight: Research found the current Oxigraph adapter persists RDF quads but hydrates canonical objects/links from in-memory sidecar maps.
  - Plan delta (what changed): Added a dedicated restart-safe canonical hydration task before retrieval-after-restart and reconciliation work.
  - Tradeoffs considered: Treating persistent Oxigraph as only `Store::open` would not satisfy restart-safe graph-authority behavior.
  - User approval: no

- 2026-05-01 19:50 Decision:
  - Trigger / new insight: Reviewer found Task_6 ownership and Wave 5 dependency risks, plus Task_1 overbroad owns for a design task.
  - Plan delta (what changed): Task_1 now owns only the active plan, Task_6 is internal/admin diagnostics only and owns `src/internal/repositories.rs`, and retrieval behavior acceptance stays in Task_5/Task_8.
  - Tradeoffs considered: Public diagnostics exposure may be useful later, but including it here would blur owns scopes and public API decisions before the persistence boundary is settled.
  - User approval: no

- 2026-05-02 00:00 Decision:
  - Trigger / new insight: User resolved graph mode default and ruled out persisted sidecar hydration entirely.
  - Plan delta (what changed): Added Resolved Decisions, made persistent graph authority the default, kept in-memory mode for tests or explicit environment configuration, and made RDF/Oxigraph-backed hydration mandatory.
  - Tradeoffs considered: Persisted sidecar hydration may have reduced implementation effort, but it would split authority and weaken the v0.1.1 persistence contract.
  - User approval: yes

- 2026-05-02 00:00 Decision:
  - Trigger / new insight: User accepted the remaining recommendations and asked to resolve the open question.
  - Plan delta (what changed): Reconciliation diagnostics remain internal/admin-facing in this phase; public `CharacterMemory` facade exposure is deferred.
  - Tradeoffs considered: Internal/admin diagnostics satisfy v0.1.1 drift detection without prematurely stabilizing a public maintenance API.
  - User approval: yes

- 2026-05-02 00:00 Decision:
  - Trigger / new insight: Implementation started and Task_1 required concrete configuration/dependency decisions before code dispatch.
  - Plan delta (what changed): `OXIGRAPH_CONNECTION_STRING` is the persistent path setting, `GRAPH_STORE_MODE=in_memory` is the explicit in-memory override, and Oxigraph persistence support is enabled through the dependency feature needed for `Store::open`.
  - Tradeoffs considered: Reusing the existing Oxigraph setting avoids adding a second path variable; a mode override keeps tests/local fixtures deterministic while making persistence the application default.
  - User approval: yes

- 2026-05-02 00:00 Decision:
  - Trigger / new insight: Local validation cannot compile `oxrocksdb-sys` because `bindgen` cannot find `clang.dll`/`libclang.dll`.
  - Plan delta (what changed): Implementation can continue, but required validation is blocked until LLVM/libclang is installed or validation runs in an environment with `LIBCLANG_PATH` configured.
  - Tradeoffs considered: Reverting the `rocksdb` feature would avoid the local native dependency but would violate the persistent graph authority requirement.
  - User approval: no

- 2026-05-02 00:00 Decision:
  - Trigger / new insight: User authorized installing the missing native dependency.
  - Plan delta (what changed): LLVM/libclang is installed locally and `LIBCLANG_PATH` is persisted for future shells.
  - Tradeoffs considered: Keeping the RocksDB-backed Oxigraph feature now has a viable Windows validation path; no dependency-strategy replan is needed for this blocker.
  - User approval: yes

- 2026-05-02 00:00 Decision:
  - Trigger / new insight: Task_6 needs cross-store drift detection without exposing an admin API through `CharacterMemory`.
  - Plan delta (what changed): Added internal-only diagnostic listing methods to graph/vector store contracts and a repository-level reconciliation helper; public facade exposure remains deferred.
  - Tradeoffs considered: The small internal contract extension lets Qdrant and Oxigraph adapters report drift without changing normal retrieval behavior or treating vector payloads as authority.
  - User approval: yes

- 2026-05-02 00:00 Decision:
  - Trigger / new insight: User requested Docker-backed Oxigraph service configuration like the existing Qdrant setup.
  - Plan delta (what changed): Changed the default application graph mode from embedded filesystem persistence to Oxigraph HTTP service mode, added explicit embedded persistent mode, added Docker Compose service expectations, and added prerequisite-gated live Oxigraph smoke validation.
  - Tradeoffs considered: Service mode aligns local development with Qdrant and avoids application-owned database directory concerns; embedded persistent mode remains useful for isolated local tests and does not introduce vector-payload authority or sidecar hydration.
  - User approval: yes

- 2026-05-02 00:00 Decision:
  - Trigger / new insight: User corrected the workflow after implementation started drifting ahead of the active plan.
  - Plan delta (what changed): Updated plan scope, decisions, task acceptance, validation expectations, and progress log before continuing further implementation/validation.
  - Tradeoffs considered: Continuing code edits without plan alignment would make validation evidence ambiguous and violate the harness workflow.
  - User approval: yes

- 2026-05-03 00:00 Decision:
  - Trigger / new insight: User asked whether production usage and tests can go to different Docker containers.
  - Plan delta (what changed): Added a separate Oxigraph test Docker service and endpoint variable for live smoke tests so test writes do not target the application/default Oxigraph service, and required tests to clean up the durable data they create.
  - Tradeoffs considered: A separate test container/volume plus per-test cleanup is safer than deleting known fixture graphs from the application service because it avoids accidental cleanup against production-like data and avoids accumulating test residue.
  - User approval: yes

- 2026-05-03 00:00 Decision:
  - Trigger / new insight: User requested Oxigraph container version `0.5.8` and clarified that schema cheat sheet should not carry runtime configuration explanations.
  - Plan delta (what changed): Docker Compose Oxigraph image tags are updated to `ghcr.io/oxigraph/oxigraph:0.5.8`; schema cheat sheet is restored to database schema/design reference scope.
  - Tradeoffs considered: Keeping configuration in operational docs avoids diluting the schema reference and makes service setup easier to find in README/env docs.
  - User approval: yes

## Notes
- Risks:
  - Oxigraph persistence requires a dependency feature decision and may affect build behavior.
  - Oxigraph service mode uses HTTP writes plus targeted remote SPARQL reads to remain graph-authoritative and may need live smoke validation against the official container before closeout.
  - The previous service-mode whole-dataset snapshot bridge was removed by `service-remote-sparql-graph-authority-plan`; future service-read optimization should tune query shape/frontier batching rather than reintroduce unbounded graph snapshots.
  - RDF/Oxigraph-to-domain hydration is the main architectural risk.
  - Reconciliation diagnostic access is now implemented but still requires Cargo/test validation after the native RocksDB build blocker is resolved.
  - Live Qdrant smoke tests require Docker and `QDRANT_CONNECTION_STRING`.
  - Live Oxigraph smoke tests require Docker and `OXIGRAPH_CONNECTION_STRING`.
  - Live Oxigraph smoke tests must use the test service endpoint (`OXIGRAPH_TEST_CONNECTION_STRING`) rather than the application/default endpoint.
- Edge cases:
  - Vector point exists but graph object is missing.
  - Graph object exists but vector point is missing.
  - Qdrant graph URI points to the wrong canonical graph object.
  - Qdrant payload says active/current while graph says suppressed, deleted, superseded, or non-current.
  - Derived memory exists without required provenance.
  - Duplicate retry writes must not inflate salience or apparent recurrence.
