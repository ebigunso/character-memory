# Plan: v0.1 Documentation, Migration Cleanup, And Release Validation

- status: completed
- generated: 2026-04-30
- last_updated: 2026-04-30
- work_type: mixed

## Goal
- Document the landed v0.1 chat-native episodic memory shape after remember/link/retrieve and correction/forget lifecycle behavior are complete.
- Complete public migration to the graph/vector/embedder architecture, including production default wiring where needed.
- Remove or rewrite stale flat-memory examples and non-contributing legacy implementation paths that conflict with graph-authoritative lifecycle semantics.
- Run final deterministic and gated service validation before v0.1 release readiness.

## Definition of Done
- README and roadmap docs describe `CharacterMemory` as graph-authoritative episodic continuity memory with Qdrant as vector candidate recall and Oxigraph as relationship/lifecycle authority.
- Public `CharacterMemory` construction and facade methods use the graph-authoritative v0.1 architecture rather than the legacy flat `MemoryRepository` / `VectorMemoryRepository` path.
- Public v0.1 facade exposes `remember`, typed `link`, `retrieve`, `correct`, and `forget` through backend-free DTOs.
- ADR statuses and summaries are updated where landed behavior and tests now support acceptance; ADRs that remain proposed keep accurate rationale.
- Public docs avoid implying production raw transcript storage, reflection scheduling, external LLM extraction, belief/evidence ontology, or physical redaction/delete behavior exists in v0.1.
- Old flat API examples are removed or clearly marked legacy if still present for transitional reasons.
- Non-contributing legacy implementation paths are removed, without adding compatibility wrappers for v0.1.
- Correction/forget lifecycle documentation reflects non-destructive supersession, suppression/archive defaults, provenance preservation, vector hygiene, rationale aggregates, and trace-enabled inspectability.
- Final validation evidence includes deterministic Rust checks, embedded Oxigraph lifecycle/retrieve smoke, live Qdrant candidate maintenance smoke or CI evidence, and Reviewer approval.

## Scope / Non-goals
- Scope:
  - README and planning/roadmap documentation cleanup.
  - ADR status/summary cleanup where landed behavior and validation support it.
  - Public `CharacterMemory` constructor and facade migration to graph/vector/embedder composition.
  - Migration notes for removal of the old flat facade and legacy Qdrant vector-memory repository path.
  - Source cleanup for legacy code that no longer contributes to the v0.1 graph/vector/embedder architecture.
  - Release validation checklist and evidence collection.
- Non-goals:
  - New lifecycle semantics beyond the completed correction/forget plan.
  - Production raw transcript storage.
  - Reflection scheduling or background summarization.
  - Belief/evidence ontology or contradiction-resolution engine.
  - Physical hard deletion/redaction semantics.
  - New durable schema fields for structured external correction-origin/source refs; those require a separate domain/schema decision before docs can describe them as graph-persisted provenance.
  - UI/browser/E2E validation.

## Context (workspace)
- Related files/areas:
  - `README.md`
  - `docs/roadmap/development_roadmap.md`
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/decisions/**`
  - `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap.md`
  - `docs/coding-agent/plans/completed/v0-1-remember-and-link-pipelines-plan.md`
  - `docs/coding-agent/plans/completed/v0-1-retrieve-continuity-context-pack-plan.md`
  - `docs/coding-agent/plans/completed/v0-1-correction-forget-lifecycle-plan.md`
  - `src/lib.rs`
  - `src/internal/infrastructures/external_services/qdrant_vector_memory_repository.rs`
  - `src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs`
  - `tests/**`
- Existing lifecycle shape to preserve:
  - `remember`, `link`, `retrieve`, `correct`, and `forget` use injected graph/vector/embedder paths where currently exposed.
  - Graph authority decides lifecycle/currentness/supersession/suppression/archive truth.
  - Episode correction/forget cascades include derived memories provenanced directly to the episode and derived memories provenanced only to observations in that episode.
  - Qdrant candidate maintenance is hygiene and recall reduction, not authority.
  - Normal retrieval excludes suppressed/deleted and non-current/superseded records by default, with detailed inspectability through trace.
  - Correction-origin and original-source external refs exist in lifecycle DTOs, but durable `DerivedMemory` provenance currently persists episode/observation IDs; do not document structured external correction-origin refs as graph-persisted unless the schema is extended.

## Open Questions (max 3)
- Resolved by user clarification: this step must leave the project fully migrated to the graph/vector/embedder architecture; implement public constructor/facade wiring if needed rather than retaining the old flat facade.
- Should release validation reuse the Task_6 local Qdrant candidate smoke evidence, or rerun the smoke as fresh release evidence before closing this plan?

## Tasks

### Task_1: Audit docs and legacy surface
- type: review
- owns:
  - `README.md`
  - `docs/**`
  - `docs/decisions/**`
  - `src/lib.rs`
  - `src/internal/**`
  - `tests/**`
- depends_on: []
- description: |
  Identify stale flat-memory docs/examples, legacy implementation paths, and release-validation gaps after lifecycle behavior lands.
- acceptance:
  - Audit lists README/doc sections that still describe flat memory behavior as canonical.
  - Audit lists ADR statuses and wording that need release-readiness cleanup, especially decisions now backed by landed tests.
  - Audit lists legacy source/test paths that should be removed, rewritten, or explicitly isolated.
  - Audit distinguishes public v0.1 graph/vector/embedder migration requirements from removable legacy flat behavior.
  - Audit records lifecycle DTO external correction-origin/source refs as deferred from durable graph persistence in this plan unless a separate user-approved schema plan supersedes that boundary.
  - Audit confirms no cleanup task requires adding compatibility wrappers.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Document migration cleanup findings before source/doc edits."

### Task_2: Update user-facing docs and roadmap state
- type: docs
- owns:
  - `README.md`
  - `docs/roadmap/**`
  - `docs/design/roadmap-phases/**`
  - `docs/decisions/**`
  - `docs/coding-agent/plans/active/**`
- depends_on: [Task_1]
- description: |
  Rewrite documentation so v0.1 is described as graph-authoritative episodic continuity memory with backend-free DTOs and provider-specific stores kept behind infrastructure boundaries.
- acceptance:
  - README examples do not promote old flat `create/search/update/delete` behavior as the v0.1 path.
  - Docs describe Qdrant and Oxigraph responsibilities accurately.
  - Lifecycle docs preserve non-destructive correction, suppression/archive defaults, provenance preservation, and trace inspectability.
  - Lifecycle docs describe episode cascades through observations and document structured external correction-origin/source refs as request metadata/deferred schema work rather than graph-persisted provenance.
  - Docs preserve soft memory threads, typed links, natural-language embedding surfaces, source/raw-reference distinction, schema-version expectations, and graph-over-vector authority.
  - ADR statuses move from `proposed` to `accepted` only where corresponding behavior exists and is covered by tests; otherwise wording remains accurate about pending work.
  - Non-goals remain explicit for raw storage, reflection scheduling, belief ontology, and physical redaction/delete.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Review docs for stale flat examples and lifecycle overclaims."

### Task_3: Remove or isolate non-contributing legacy implementation paths
- type: impl
- owns:
  - `src/lib.rs`
  - `src/internal/**`
  - `tests/**`
- depends_on: [Task_1]
- description: |
  Complete public graph/vector/embedder facade wiring and remove legacy flat facade and Qdrant vector-memory repository behavior that conflicts with the v0.1 graph-authoritative architecture.
- acceptance:
  - `CharacterMemory::new` and `new_with_embedding_provider` construct graph/vector/embedder composition with Oxigraph graph authority and Qdrant vector candidate recall.
  - Public `remember`, `link`, `retrieve`, `correct`, and `forget` facades call the graph-authoritative pipelines through backend-free DTOs.
  - Legacy public flat `create_memory`, `bulk_create_memories`, `search_memories`, `get_memory_by_id`, `get_memories_by_ids`, `update_memory`, and `delete_memory` facades are removed rather than preserved for compatibility.
  - Legacy `MemoryRepository`, `VectorMemoryRepository`, `QdrantVectorMemoryRepository`, and their flat test coverage are removed or no longer compiled unless a remaining internal dependency is explicitly justified by the graph/vector architecture.
  - Durable correction-origin/source external-ref schema work is not introduced in this cleanup chunk; retained docs and tests describe episode/observation provenance as the persisted correction trail.
  - Tests are updated to validate the public v0.1 path rather than preserving legacy flat behavior for compatibility.
  - No production raw storage, reflection scheduling, belief ontology, or physical redaction/delete semantics are introduced.
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
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib"
  - kind: command
    required: true
    owner: worker
    detail: "Run targeted migration cleanup tests."

### Task_4: Final release validation and plan lifecycle
- type: review
- owns:
  - `docs/coding-agent/plans/active/**`
  - `docs/coding-agent/plans/completed/**`
- depends_on: [Task_2, Task_3]
- description: |
  Run final release validation, record evidence, and complete v0.1 planning lifecycle updates.
- acceptance:
  - Required deterministic validation evidence is recorded.
  - Embedded Oxigraph lifecycle/retrieve smoke passes.
  - Live Qdrant candidate maintenance smoke passes locally or via CI evidence before release validation closes.
  - Reviewer approves docs/migration cleanup and confirms no lifecycle or non-goal scope creep.
  - Active roadmap links point to completed plans or the next explicitly drafted plan.
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
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo clippy --all-targets -- -D warnings"
  - kind: command
    required: true
    owner: worker
    detail: "Run embedded Oxigraph lifecycle/retrieve smoke."
  - kind: command
    required: true
    owner: worker
    detail: "Run live Qdrant candidate maintenance smoke locally or record CI evidence before release validation closes."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review final docs, migration cleanup, validation evidence, and non-goals."

## Task Waves (explicit parallel dispatch sets)
- Wave 1 (audit): [Task_1]
- Wave 2 (parallel cleanup): [Task_2, Task_3]
- Wave 3 (release validation): [Task_4]

## E2E / Visual Validation Spec
- Not applicable. This is Rust library documentation, migration cleanup, and release validation with no UI/user-flow surface.

## Rollback / Safety
- Do not preserve old flat API behavior for compatibility alone.
- The public graph/vector/embedder facade must exist before removing the old flat facade.
- Keep Qdrant lifecycle payloads as candidate hints and Oxigraph/graph authority as lifecycle truth.
- Keep raw inputs consumer-owned; do not introduce raw transcript storage.

## Progress Log
- 2026-04-30 Plan drafted.
  - Summary: Drafted the next concrete plan from the lifecycle implementation shape and remaining roadmap cleanup needs.
  - Validation evidence: Documentation-only plan draft by Task_6; no source edits performed.
  - Notes: This plan should start after the correction/forget lifecycle plan receives final Reviewer and Orchestrator closeout and moves to completed.
- 2026-04-30 Plan reviewed and scope corrected before implementation.
  - Summary: Initial research found the public constructor still used the legacy flat Qdrant path. User clarified this step must finish the public migration, so Task_3 was revised to require graph/vector/embedder production wiring and removal of the old flat facade rather than transitional retention.
  - Validation evidence: Researcher reports covered roadmap/design/ADR alignment and source/test migration requirements.
  - Notes: This replaced the prior narrower cleanup interpretation.
- 2026-04-30 Documentation and ADR cleanup completed.
  - Summary: README, roadmap/design docs, and v0.1 ADRs now describe the public graph-authoritative architecture, Qdrant candidate recall, Oxigraph authority, lifecycle semantics, source-reference boundary, and v0.1 non-goals.
  - Validation evidence: Worker doc searches found no stale `create_memory`, `search_memories`, `update_memory`, or `delete_memory` examples in owned docs; remaining flat-memory references are historical contrast text.
  - Notes: ADR statuses were marked accepted where landed behavior and tests support the decision.
- 2026-04-30 Public graph/vector/embedder migration completed.
  - Summary: `CharacterMemory::new` and `new_with_embedding_provider` now construct Oxigraph graph authority plus Qdrant vector candidate recall; `remember`, `link`, `retrieve`, `correct`, and `forget` are public v0.1 facades; old flat public facade methods, legacy repository modules, flat DTO re-exports, and legacy integration tests were removed.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, embedded Oxigraph smoke filters, live Qdrant candidate smoke, and public v0.1 facade integration tests passed.
  - Notes: Qdrant live evidence used Docker Compose service and `QDRANT_CONNECTION_STRING=http://localhost:6334`.
- 2026-04-30 Final review approved.
  - Summary: Reviewer requested fixes for stale roadmap API snippet and overclaimed Oxigraph durability; follow-up changed docs to backend-free draft/context facade examples and explicitly scoped default Oxigraph authority to embedded in-memory graph state with persistent storage deferred.
  - Validation evidence: Reviewer approved after `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, and stale-term search passed.
  - Notes: Residual risk is documented: Qdrant candidates can outlive in-memory graph state across restarts until persistent Oxigraph configuration lands.

## Decision Log
- 2026-04-30 Decision: Draft cleanup/release validation after lifecycle behavior lands.
  - Trigger / new insight: Correction/forget lifecycle behavior introduces the replacement semantics needed before retiring old flat update/delete examples.
  - Plan delta: Added a concrete plan for documentation, migration cleanup, and release validation.
  - Tradeoffs considered: Starting cleanup before lifecycle closeout would risk documenting unstable behavior; drafting now preserves the next-step shape while leaving execution gated on lifecycle completion.
  - User approval: directed by approved roadmap sequence.

## Notes
- Risks:
  - Removing legacy code may reveal integration tests that still depend on old flat Qdrant behavior.
  - Documentation can overstate crate-visible injected lifecycle surfaces if production constructors are not yet graph-authoritative.
  - Documentation can overstate correction-origin/source external-ref persistence if it does not distinguish DTO request metadata from durable episode/observation provenance and source refs.
  - Live Qdrant validation remains environment-sensitive and must be captured before release validation closes.
- Edge cases:
  - Retained legacy paths need explicit transitional ownership and should not be described as canonical v0.1 behavior.
  - Examples should preserve source references without embedding raw transcript content into Qdrant or Oxigraph.
  - Episode-level correction/forget examples should include observation-only derived memories so source cascades do not depend on duplicate episode IDs on every derived memory.
