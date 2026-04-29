# Plan: v0.1 Documentation, Migration Cleanup, And Release Validation

- status: draft
- generated: 2026-04-30
- last_updated: 2026-04-30
- work_type: mixed

## Goal
- Document the landed v0.1 chat-native episodic memory shape after remember/link/retrieve and correction/forget lifecycle behavior are complete.
- Remove or rewrite stale flat-memory examples and non-contributing legacy implementation paths that conflict with graph-authoritative lifecycle semantics.
- Run final deterministic and gated service validation before v0.1 release readiness.

## Definition of Done
- README and roadmap docs describe `CharacterMemory` as graph-authoritative episodic continuity memory with Qdrant as vector candidate recall and Oxigraph as relationship/lifecycle authority.
- Public docs avoid implying production raw transcript storage, reflection scheduling, external LLM extraction, belief/evidence ontology, or physical redaction/delete behavior exists in v0.1.
- Old flat API examples are removed or clearly marked legacy if still present for transitional reasons.
- Non-contributing legacy implementation paths are removed or isolated with explicit migration notes, without adding compatibility wrappers for v0.1.
- Correction/forget lifecycle documentation reflects non-destructive supersession, suppression/archive defaults, provenance preservation, vector hygiene, rationale aggregates, and trace-enabled inspectability.
- Final validation evidence includes deterministic Rust checks, embedded Oxigraph lifecycle/retrieve smoke, live Qdrant candidate maintenance smoke or CI evidence, and Reviewer approval.

## Scope / Non-goals
- Scope:
  - README and planning/roadmap documentation cleanup.
  - Migration notes for the old flat facade and legacy Qdrant vector-memory repository path.
  - Source cleanup only where legacy code no longer contributes to the v0.1 graph/vector/embedder architecture.
  - Release validation checklist and evidence collection.
- Non-goals:
  - New lifecycle semantics beyond the completed correction/forget plan.
  - Production raw transcript storage.
  - Reflection scheduling or background summarization.
  - Belief/evidence ontology or contradiction-resolution engine.
  - Physical hard deletion/redaction semantics.
  - UI/browser/E2E validation.

## Context (workspace)
- Related files/areas:
  - `README.md`
  - `docs/roadmap/development_roadmap.md`
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap.md`
  - `docs/coding-agent/plans/completed/v0-1-remember-and-link-pipelines-plan.md`
  - `docs/coding-agent/plans/completed/v0-1-retrieve-continuity-context-pack-plan.md`
  - `docs/coding-agent/plans/active/v0-1-correction-forget-lifecycle-plan.md`
  - `src/lib.rs`
  - `src/internal/infrastructures/external_services/qdrant_vector_memory_repository.rs`
  - `src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs`
  - `tests/**`
- Existing lifecycle shape to preserve:
  - `remember`, `link`, `retrieve`, `correct`, and `forget` use injected graph/vector/embedder paths where currently exposed.
  - Graph authority decides lifecycle/currentness/supersession/suppression/archive truth.
  - Qdrant candidate maintenance is hygiene and recall reduction, not authority.
  - Normal retrieval excludes suppressed/deleted and non-current/superseded records by default, with detailed inspectability through trace.

## Open Questions (max 3)
- Should the old flat public facade be removed in this chunk or kept as explicitly deprecated while production v0.1 constructors are finalized?
- Should README document crate-visible injected lifecycle behavior only in architecture notes, or wait until a public production constructor exposes it safely?
- Should release validation reuse the Task_6 local Qdrant candidate smoke evidence, or rerun the smoke as fresh release evidence before closing this plan?

## Tasks

### Task_1: Audit docs and legacy surface
- type: review
- owns:
  - `README.md`
  - `docs/**`
  - `src/lib.rs`
  - `src/internal/**`
  - `tests/**`
- depends_on: []
- description: |
  Identify stale flat-memory docs/examples, legacy implementation paths, and release-validation gaps after lifecycle behavior lands.
- acceptance:
  - Audit lists README/doc sections that still describe flat memory behavior as canonical.
  - Audit lists legacy source/test paths that should be removed, rewritten, or explicitly isolated.
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
  - `docs/coding-agent/plans/active/**`
- depends_on: [Task_1]
- description: |
  Rewrite documentation so v0.1 is described as graph-authoritative episodic continuity memory with backend-free DTOs and provider-specific stores kept behind infrastructure boundaries.
- acceptance:
  - README examples do not promote old flat `create/search/update/delete` behavior as the v0.1 path.
  - Docs describe Qdrant and Oxigraph responsibilities accurately.
  - Lifecycle docs preserve non-destructive correction, suppression/archive defaults, provenance preservation, and trace inspectability.
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
  Remove, rewrite, or explicitly isolate legacy flat facade and Qdrant vector-memory repository behavior that conflicts with the v0.1 graph-authoritative architecture.
- acceptance:
  - Legacy `update_memory` / `delete_memory` paths no longer imply hard update/delete lifecycle semantics for v0.1.
  - Any retained legacy path has a clear transitional reason and test boundary.
  - Tests are updated to validate the v0.1 path rather than preserving legacy behavior for compatibility alone.
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
- Do not document crate-visible injected surfaces as stable public production APIs.
- Keep Qdrant lifecycle payloads as candidate hints and Oxigraph/graph authority as lifecycle truth.
- Keep raw inputs consumer-owned; do not introduce raw transcript storage.

## Progress Log
- 2026-04-30 Plan drafted.
  - Summary: Drafted the next concrete plan from the lifecycle implementation shape and remaining roadmap cleanup needs.
  - Validation evidence: Documentation-only plan draft by Task_6; no source edits performed.
  - Notes: This plan should start after the correction/forget lifecycle plan receives final Reviewer and Orchestrator closeout and moves to completed.

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
  - Live Qdrant validation remains environment-sensitive and must be captured before release validation closes.
- Edge cases:
  - Retained legacy paths need explicit transitional ownership and should not be described as canonical v0.1 behavior.
  - Examples should preserve source references without embedding raw transcript content into Qdrant or Oxigraph.
