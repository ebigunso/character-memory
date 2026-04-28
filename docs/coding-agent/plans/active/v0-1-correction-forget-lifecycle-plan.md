# Plan: v0.1 Correction And Forget Lifecycle

- status: draft
- generated: 2026-04-29
- last_updated: 2026-04-29
- work_type: mixed

## Goal
- Implement non-destructive correction and forget lifecycle behavior on top of the landed graph-authoritative remember/link/retrieve path.
- Preserve provenance and source references while marking replaced derived memories non-current and suppressing forgotten memories by default.
- Keep this chunk focused on lifecycle mutation and retrieval visibility; do not implement production raw storage, reflection scheduling, belief/evidence ontology, or broad legacy facade removal beyond necessary isolation.

## Definition of Done
- Backend-free correction and forget request/outcome DTOs exist without Qdrant/Oxigraph/RDF types.
- Correction creates replacement canonical objects/links, records supersession, marks replaced derived memories non-current where applicable, and preserves provenance/source references.
- Forget defaults to suppression rather than hard deletion, with explicit redaction/delete semantics reserved for deliberately named policy paths if included.
- Graph authority remains the source of truth for supersession, suppression, lifecycle, provenance, and currentness; Qdrant payloads/candidates are updated or removed only to keep candidate recall from surfacing stale hints.
- Normal retrieve behavior excludes suppressed/deleted and non-current/superseded memories after correction/forget, while opt-in historical policies remain inspectable through rationale/trace.
- Required Rust checks, deterministic lifecycle tests, embedded Oxigraph smoke, gated Qdrant candidate smoke, and Reviewer approval are complete, or blockers/required-check waivers are explicitly recorded.

## Scope / Non-goals
- Scope:
  - Backend-free correction/forget request, policy, and outcome DTOs.
  - Provider-neutral graph lifecycle mutation support needed for currentness, supersession, suppression, and optional explicit hard deletion/redaction semantics.
  - Vector candidate maintenance needed after correction/forget so Qdrant remains a candidate hint store rather than authority.
  - Internal correction and forget pipelines over `GraphAuthorityStore`, `VectorCandidateStore`, and `MemoryEmbedder` where replacement objects need indexing.
  - Injected `CharacterMemory::correct` / `CharacterMemory::forget` facade shape if selected by Task_1.
  - Deterministic fake-store tests plus embedded Oxigraph and gated Qdrant smoke evidence.
- Non-goals:
  - Full belief/evidence ontology or contradiction-resolution system.
  - Reflection scheduling, background summarization, external LLM extraction, or LLM-based correction generation.
  - Production raw transcript storage.
  - Public production constructor rewiring unless narrowly required to keep lifecycle behavior testable.
  - UI/E2E/browser validation.

## Context (workspace)
- Related files/areas:
  - `src/api/types/domain.rs`
  - `src/api/types/draft.rs`
  - `src/api/types/retrieval.rs`
  - `src/api/types.rs`
  - `src/lib.rs`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal/infrastructures/external_services/**`
  - `src/internal/infrastructures/graph/**`
  - `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap.md`
  - `docs/coding-agent/plans/completed/v0-1-retrieve-continuity-context-pack-plan.md`
- Existing patterns or references:
  - `RetrieveOutcome` and rationale/trace behavior now expose graph-authoritative omission and supersession evidence.
  - `GraphExpansionQuery` and `RetrievePipeline` already exclude suppressed/deleted/non-current/superseded records by default and fail closed on bounded graph failures when degraded results are disabled.
  - `DerivedMemory.supersedes` means the current memory replaces older memories; graph-sourced `superseded_by` evidence identifies memories replaced by newer ones.
  - Qdrant payload lifecycle/currentness/supersession fields are candidate hints only; graph authority decides final lifecycle/currentness truth.
  - Legacy flat `update_memory` / `delete_memory` behavior conflicts with the lifecycle direction and must stay isolated or be retired when replacement lifecycle facades supersede it.
- Repo reference docs to consult:
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/decisions/design/ADR-D-0002-derived-memory-provenance.md`
  - `docs/decisions/design/ADR-D-0006-supersession-and-suppression.md`
  - `docs/decisions/design/ADR-D-0008-preserve-source-references.md`
  - `docs/decisions/implementation/ADR-I-0004-typed-memory-links.md`
  - `docs/decisions/implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md`
  - `docs/decisions/implementation/ADR-I-0006-bounded-graph-expansion.md`

## Open Questions (max 3)
- Q1: Should this chunk expose crate-visible injected `correct`/`forget` facades only, matching current `remember`/`link`/`retrieve`, or make any lifecycle method public now?
- Q2: Should explicit hard deletion/redaction be modeled in this chunk as a backend-free policy path, or deferred until migration cleanup/release validation?
- Q3: After graph-authoritative suppression/supersession, should vector candidates be deleted immediately for affected object IDs, or upserted with stale lifecycle hints plus graph verification still excluding them?

## Assumptions
- A1: Correction is non-destructive by default: create replacement memory, preserve old provenance, and mark old derived memories non-current/superseded rather than overwriting them.
- A2: Forget means suppression by default; hard deletion/redaction requires explicit policy naming and stronger validation.
- A3: Retrieval defaults from the completed retrieve plan remain the acceptance oracle for lifecycle visibility.
- A4: Vector candidate maintenance should reduce stale recall noise but must not become lifecycle authority.

## Tasks

### Task_1: Select lifecycle API and policy boundary
- type: design
- owns:
  - `docs/coding-agent/plans/active/v0-1-correction-forget-lifecycle-plan.md`
- depends_on: []
- description: |
  Decide correction/forget request names, facade visibility, suppression versus hard-delete policy, vector maintenance strategy, and failure semantics before implementation edits.
- acceptance:
  - Decision records whether correction/forget facades are crate-visible injected surfaces or public API in this chunk.
  - Decision records default suppression and supersession behavior plus any explicit redaction/delete policy boundaries.
  - Decision records graph-authority lifecycle mutation semantics and Qdrant candidate maintenance strategy.
  - Decision records dependency direction: public DTOs stay backend-free; pipelines depend on provider-neutral graph/vector/embedder contracts; Qdrant/Oxigraph/RDF remain infrastructure details.
  - Decision records how retrieval rationale/trace should expose corrected, superseded, suppressed, and historical opt-in behavior.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Record lifecycle API, policy, vector-maintenance, and failure-policy decisions before implementation edits."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review lifecycle boundary against ADR-D-0006, retrieval behavior, and Qdrant/Oxigraph authority split."

### Task_2: Add correction and forget DTOs
- type: impl
- owns:
  - `src/api/types.rs`
  - `src/api/types/**`
  - `src/lib.rs`
- depends_on: [Task_1]
- description: |
  Add backend-free correction/forget request, policy, and outcome DTOs aligned with canonical domain objects and retrieval lifecycle vocabulary.
- acceptance:
  - DTOs can express replacement derived memory/object data, superseded source IDs, rationale, lifecycle policy, suppression policy, and optional trace flags where selected.
  - DTOs preserve source/raw references without introducing raw transcript storage.
  - Outcomes report graph-mutated object IDs, link IDs, vector-maintained object IDs, and partial vector maintenance failures where graph mutation succeeded.
  - Pure DTO tests cover serialization, defaults, validation, non-destructive correction semantics, suppression defaults, and explicit hard-delete/redaction naming if included.
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
    detail: "Run targeted correction/forget DTO tests added by this task."

### Task_3: Extend graph lifecycle mutation support
- type: impl
- owns:
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/infrastructures/graph.rs`
  - `src/internal/infrastructures/graph/**`
- depends_on: [Task_1]
- description: |
  Add provider-neutral graph authority behavior needed to mark lifecycle/currentness, record supersession, and support suppression/default forget semantics.
- acceptance:
  - Graph authority can persist correction supersession links and lifecycle/currentness updates without hard-deleting by default.
  - Fake graph and embedded Oxigraph behavior keep old and replacement objects inspectable under explicit historical policy.
  - Normal retrieval after lifecycle mutation excludes suppressed/deleted and non-current/superseded records by default.
  - Tests cover supersession directionality, suppression, optional historical inclusion, and no accidental traversal through suppressed/non-current records.
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
    detail: "Run targeted graph lifecycle mutation tests, including embedded Oxigraph coverage."

### Task_4: Add correction and forget pipelines
- type: impl
- owns:
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
- depends_on: [Task_2, Task_3]
- description: |
  Implement internal correction and forget pipelines with ordered graph mutation, vector candidate maintenance, and deterministic partial-failure behavior.
- acceptance:
  - Correction validates all inputs before graph mutation, writes replacement objects/links, records supersession, marks replaced derived memories non-current where applicable, and indexes replacement vector records when selected.
  - Forget applies suppression by default and updates or deletes vector candidates according to Task_1 policy.
  - Graph mutation failure prevents vector maintenance; vector maintenance failure after graph success returns explicit partial-success outcome.
  - Tests cover ordering, graph-authoritative lifecycle changes, vector maintenance, partial failure, and retrieval-after-mutation defaults.
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
    detail: "Run targeted fake-store correction/forget pipeline tests added by this task."

### Task_5: Wire injected lifecycle facade and retrieval regression tests
- type: impl
- owns:
  - `src/lib.rs`
  - `src/api/types.rs`
  - `src/api/types/**`
  - `tests/**`
  - `README.md`
- depends_on: [Task_4]
- description: |
  Expose selected injected correction/forget facade behavior and prove normal retrieval reflects lifecycle mutations without extending legacy flat update/delete paths.
- acceptance:
  - Selected `CharacterMemory::correct` / `CharacterMemory::forget` facade shape uses injected provider-neutral parts and returns selected backend-free outcomes.
  - Legacy flat `update_memory` / `delete_memory` remain isolated/deprecated and are not used by lifecycle pipelines.
  - Deterministic facade tests cover correction, forget/suppression, retrieval exclusion, historical opt-in visibility if selected, and legacy isolation.
  - README examples are updated only if the public runnable surface is production-usable enough to document without misleading users.
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
    detail: "Run targeted injected lifecycle facade and retrieval regression tests added by this task."

### Task_6: Final lifecycle review and plan lifecycle
- type: review
- owns:
  - `docs/coding-agent/plans/active/**`
  - `docs/coding-agent/plans/completed/**`
- depends_on: [Task_5]
- description: |
  Run required smoke evidence, review lifecycle implementation, complete plan lifecycle updates, and draft the next Documentation/Migration Cleanup plan without implementing cleanup.
- acceptance:
  - Required deterministic validation evidence from Tasks 1-5 is recorded.
  - Embedded Oxigraph lifecycle/retrieve smoke passes.
  - Live Qdrant candidate maintenance smoke passes locally or via CI evidence, or a required-check waiver records risk, mitigation, owner, and expiration.
  - Reviewer approves no raw-storage/reflection/belief-ontology scope creep and confirms retrieval lifecycle behavior is inspectable through rationale/trace.
  - Next concrete plan for Documentation, Migration Cleanup, And Release Validation is drafted from the landed lifecycle shape.
  - Lifecycle plan evidence and decision/progress logs are complete before moving this plan from active to completed.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "Run embedded Oxigraph lifecycle/retrieve smoke."
  - kind: command
    required: true
    owner: worker
    detail: "Run live Qdrant candidate maintenance smoke locally before PR creation or provide CI job evidence before merge."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review lifecycle implementation against roadmap, ADRs, validation evidence, and non-goals."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Confirm evidence completeness, retrieval lifecycle behavior, and next-plan independence."

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (design gate): [Task_1]
- Wave 2 (public DTOs): [Task_2]
- Wave 3 (graph lifecycle support): [Task_3]
- Wave 4 (lifecycle pipelines): [Task_4]
- Wave 5 (facade and retrieval regression): [Task_5]
- Wave 6 (smoke, review, next-plan draft): [Task_6]

## E2E / Visual Validation Spec

- Not applicable. This is Rust library lifecycle behavior with no UI/user-flow surface.

## Rollback / Safety
- Keep correction/forget DTOs and facade backend-free.
- Preserve non-destructive correction and suppression-by-default forget behavior unless Task_1 explicitly selects named hard-delete/redaction semantics.
- Treat Qdrant lifecycle/supersession fields as hints; graph authority remains final truth.
- Normal retrieval must exclude suppressed/deleted and non-current/superseded memories by default after lifecycle mutation.
- Keep production raw storage, reflection scheduling, external LLM behavior, and belief/evidence ontology out of this plan.

## Quality Routing Note
- Routing level: L2
- In-scope docs: Rust API/internal pipeline architecture, graph-authoritative lifecycle mutation, vector candidate maintenance, data-integrity filtering, provenance/source-reference preservation, deterministic fake-store validation, smoke evidence.
- Out-of-scope docs: UI/E2E, frontend/browser checks, production raw storage, reflection scheduling, external LLM extraction/reranking, full belief/evidence ontology.
- Top risks: data-integrity lifecycle mutation, correction provenance, graph/vector drift, legacy update/delete drift, public API shape.
- Risk profile: medium-high because this chunk mutates lifecycle/currentness state and must keep retrieval visibility correct across graph and vector stores.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted DTO/graph/pipeline/facade/retrieval regression tests, embedded Oxigraph lifecycle smoke, live Qdrant candidate maintenance smoke or CI evidence/required-check waiver, Reviewer gate.
- Optional recommended checks: `cargo clippy --all-targets -- -D warnings`.

## Progress Log

- 2026-04-29 Plan drafted.
  - Summary: Created the next concrete plan for correction and forget lifecycle behavior from the landed retrieve/context-pack implementation.
  - Validation evidence: Documentation-only draft after completed retrieve plan validation and final Reviewer approval.
  - Notes: Draft status; requires user approval before execution.

## Decision Log

- 2026-04-29 Decision: Draft correction/forget lifecycle plan after retrieval implementation
  - Trigger / new insight: Retrieval now exposes graph-authoritative lifecycle/currentness omission behavior, rationale, trace, and fail-closed bounded expansion policy, making correction and forget lifecycle mutation the next roadmap chunk.
  - Plan delta: Added this active concrete plan for non-destructive correction, suppression-by-default forget, vector candidate maintenance, retrieval regression tests, and final lifecycle review.
  - Tradeoffs considered: Implementing lifecycle mutation before documentation cleanup keeps retrieval behavior grounded in actual graph state before retiring or rewriting legacy flat update/delete examples.
  - User approval: pending.

## Notes
- Risks:
  - Graph/vector drift can make stale candidates noisy even when graph authority excludes them correctly.
  - Hard deletion/redaction semantics can accidentally bypass provenance preservation if not explicitly named and tested.
  - Legacy flat update/delete methods may confuse users until the public replacement lifecycle surface and documentation cleanup land.
- Edge cases:
  - Correcting a derived memory should not erase the old source-reference chain.
  - Forget should suppress normal retrieval but preserve historical/audit visibility when policy explicitly includes it.
  - Supersession direction must remain clear: new memory supersedes old memory; old memory may be described as superseded_by the new memory in rationale/trace.
  - Vector candidates for suppressed/superseded objects must not become lifecycle authority even if they remain in Qdrant temporarily.
