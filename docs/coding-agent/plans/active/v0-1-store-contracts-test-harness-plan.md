# Plan: v0.1 Store Contracts And Deterministic Test Harness

- status: done
- generated: 2026-04-27
- last_updated: 2026-04-27
- work_type: mixed

## Goal
- Define the v0.1 store and embedding contracts around the landed public domain model in `src/api/types/domain`.
- Add deterministic fake/in-memory implementations and fixtures that can support later remember/retrieve/lifecycle pipeline tests without requiring Qdrant, Oxigraph, OpenAI, or network services.
- Keep concrete Qdrant and Oxigraph adapters out of this chunk; those belong to the next adapter-foundation plan.

## Definition of Done
- Store contract traits compile for vector recall, graph authority, raw reference fixture access, and embedding.
- Deterministic fake/in-memory implementations exist for tests and remain clearly separate from production adapters.
- Fixture helpers can construct representative v0.1 episodes, observations, entities, threads, derived memories, links, and raw references using the landed domain types.
- Required Rust checks and targeted fake-store/fixture tests pass, or blockers are recorded with evidence.
- Reviewer approves the store-contracts/test-harness diff and validation evidence.

## Scope / Non-goals
- Scope:
  - Traits or equivalent contracts for vector store, graph store, raw reference fixture access, and embedder behavior.
  - Deterministic fake/in-memory stores for service-free tests.
  - Shared fixtures for the v0.1 Character Memory design scenario.
  - Tests proving fake stores preserve object IDs, links, lifecycle fields, raw references, and deterministic embeddings.
- Non-goals:
  - Qdrant payload migration or live Qdrant adapter changes.
  - Oxigraph dependency, RDF triple generation, SPARQL query builders, or live graph adapter behavior.
  - `remember`, `retrieve`, `link`, `correct`, or `forget` public pipeline implementation.
  - Production raw input storage.
  - Compatibility wrappers or compatibility preservation for old flat APIs.

## Context (workspace)
- Related files/areas:
  - `src/api/types/domain.rs`
  - `src/internal/repositories/**`
  - `src/internal/models/**`
  - `src/internal.rs`
  - `tests/**`
  - `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap.md`
  - `docs/coding-agent/plans/completed/v0-1-domain-foundation-plan.md`
- Existing patterns or references:
  - Canonical v0.1 domain types are public under `src/api/types/domain` and re-exported from `src/lib.rs`.
  - Existing flat vector repository traits live under `src/internal/repositories/**` and are legacy-oriented; preserve them only where they directly support the new v0.1 architecture or current compilation during replacement.
  - Integration tests already use deterministic embeddings, but this chunk should keep new tests service-free.
- Repo reference docs consulted:
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/decisions/implementation/ADR-I-0003-qdrant-oxigraph-defaults.md`
  - `docs/decisions/implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md`
  - `docs/decisions/implementation/ADR-I-0006-bounded-graph-expansion.md`

## Open Questions
- None for Task_1. Q1 resolved: reusable fake graph expansion and reusable fake stores/fixtures should live under internal `cfg(test)` test-support modules when later pipeline tests need to share them; module-local `tests.rs` files remain acceptable for tests that are not reusable.

## Review Mode
- mode: remediation
- scope: final-plan-review
- max_iterations: 2
- status: completed

## Assumptions
- A1: Store contracts may live under `src/internal/repositories` or a new internal store module, but must depend on canonical public domain types rather than old flat DTOs.
- A2: Fake stores are test/support infrastructure, not production adapters.
- A3: Raw input remains consumer-owned; this chunk may provide test fixture helpers for file-backed raw refs but not production raw persistence.
- A4: Live Qdrant/Oxigraph checks are optional or out of scope for this chunk.

## Tasks

### Task_1: Select store contract boundaries
- type: design
- owns:
  - `src/internal/repositories/**`
  - `src/internal/models/**`
  - `src/internal.rs`
  - `docs/coding-agent/plans/active/v0-1-store-contracts-test-harness-plan.md`
- depends_on: []
- description: |
  Inspect the existing repository traits and choose where v0.1 store contracts and test fakes should live, including which legacy flat repository/model pieces can be removed once replaced.
- acceptance:
  - Contract placement is recorded in the plan's Decision Log or Progress Log.
  - Decision keeps canonical domain types as inputs/outputs for v0.1 contracts.
  - Decision scopes old flat repository traits as removable legacy unless needed for the new v0.1 architecture or current compilation during replacement.
  - Decision identifies what belongs in production internal modules vs test-only support.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Inspect current repository/internal model layout and record the selected v0.1 contract/test-support boundary."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review boundary decision for consistency with roadmap and domain foundation."

### Task_2: Define v0.1 store and embedder contracts
- type: impl
- owns:
  - `src/internal/repositories/**`
  - `src/internal/models/**`
  - `src/internal.rs`
  - `tests/**`
- depends_on: [Task_1]
- description: |
  Add trait contracts for vector recall, graph authority, raw reference fixture access where appropriate, and deterministic embedding behavior.
- acceptance:
  - Contracts compile and use canonical v0.1 domain types or IDs.
  - Vector contract can upsert/search/delete candidate records without Qdrant-specific types.
  - Graph contract can upsert objects/links and expose bounded expansion/query placeholders needed by later chunks.
  - Embedder contract is deterministic-test friendly and does not require OpenAI.
  - No live adapter behavior is implemented.
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

### Task_3: Add deterministic fake stores and fixtures
- type: impl
- owns:
  - `tests/**`
  - `src/internal/repositories/**`
  - `src/internal/models/**`
- depends_on: [Task_2]
- description: |
  Add fake/in-memory implementations and representative fixtures for later pipeline tests, keeping them service-free and clearly non-production.
- acceptance:
  - Fixtures cover simple episode, salient observation, entity, soft thread link, derived reflection, user preference, open loop/commitment, correction/suppression seeds, and hub entity scenario seeds where feasible.
  - Fake vector behavior is deterministic and does not call external services.
  - Fake graph behavior preserves objects, links, lifecycle fields, and raw references.
  - Tests prove fake stores can persist and retrieve representative domain objects and links by stable IDs.
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
    detail: "Run targeted fake-store and fixture tests added by this task."

### Task_4: Review store contracts and draft adapter-foundation plan
- type: review
- owns:
  - `docs/coding-agent/plans/active/**`
- depends_on: [Task_3]
- description: |
  Review the store contracts/test harness diff and validation evidence. If approved, draft the next concrete plan for vector and graph adapter foundations from the landed contract shape.
- acceptance:
  - Reviewer approves the store contracts/test harness diff or blocking issues are resolved/waived.
  - Required validation evidence from Tasks 1-3 is present.
  - This plan's Progress Log and Decision Log are updated with outcomes.
  - A separate active plan for vector and graph adapter foundations is drafted from the landed contract shape.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Review store contract/test harness implementation against this plan and v0.1 storage roadmap requirements."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Confirm the next concrete plan is independent and based on landed store contract code."

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (design gate): [Task_1]
- Wave 2 (contracts): [Task_2]
- Wave 3 (fakes and fixtures): [Task_3]
- Wave 4 (review and next-plan draft): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. This is Rust library test/support infrastructure with no UI/user-flow surface.

## Rollback / Safety
- Keep this chunk away from live Qdrant/Oxigraph adapter behavior.
- Keep fake implementations clearly scoped as test/support infrastructure.
- Do not introduce production raw input storage.
- Do not preserve old flat repository paths for compatibility alone; remove or replace legacy pieces when this chunk makes them unnecessary.

## Quality Routing Note
- Routing level: L2
- In-scope docs: Rust internal architecture, test harness design, validation evidence, data-integrity boundaries.
- Out-of-scope docs: live Qdrant/Oxigraph adapter details, UI/E2E, auth/security, production raw storage.
- Top risks: data-integrity, architectural drift from legacy compatibility, external dependency/integration if scope leaks into live adapters.
- Risk profile: medium; this chunk shapes later pipeline tests but should remain deterministic and service-free.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted fake-store/fixture tests, Reviewer gate.
- Optional recommended checks: none for this chunk.
- At Risk items: fake graph placement and avoiding accidental production semantics.

## Progress Log (append-only)

- 2026-04-27 Plan drafted.
  - Summary: Created a concrete execution plan for v0.1 store contracts and deterministic test harness.
  - Validation evidence: Pending approval and execution.
  - Notes: Drafted from the landed domain foundation model shape.

- 2026-04-27 Task_1 completed: selected store contract/test-support boundary.
  - Summary: Inspected the current internal repository and model layout, confirmed the existing `MemoryRepository`, `VectorMemoryRepository`, `MemoryEntry`, `VectorMetadata`, and `ScoredMemoryEntry` stack is legacy flat API infrastructure, and recorded the v0.1 boundary before implementation.
  - Validation evidence: Manual review of `src/api/types/domain.rs`, `src/api/types/domain/tests.rs`, `src/internal/repositories.rs`, `src/internal/repositories/memory_repository.rs`, `src/internal/repositories/vector_memory_repository.rs`, `src/internal/models.rs`, `src/internal/models/memory/**`, `src/internal/models/vector/**`, `src/internal.rs`, and `tests/test_utils.rs`.
  - Notes: No Rust code or live Qdrant/Oxigraph adapter behavior was changed; cargo commands were not required for this docs/design-only task.

- 2026-04-27 Task_1 reviewer gate approved.
  - Summary: Reviewer approved the selected production/test-support/legacy boundary with no findings.
  - Validation evidence: Reviewer confirmed the decision is recorded in the Progress Log and Decision Log, keeps v0.1 contracts tied to canonical domain types and IDs, scopes old flat repository/model pieces as replacement-target legacy, separates production contracts from `cfg(test)` support, and resolves Q1.
  - Notes: Residual risk is limited to Task_3 deciding whether later integration tests need a public test-helper route or should stay source-module-local.

- 2026-04-27 Task_2 complete: defined v0.1 store and embedder contracts.
  - Summary: Added provider-neutral contracts for vector candidate storage, graph authority storage, deterministic embedding, and raw reference resolution. Added vector candidate/search/match and embedding input model types under the internal vector model boundary.
  - Validation evidence: Worker ran `cargo fmt --check`, `cargo check`, and `cargo test --no-run`; all passed after formatting the new Rust modules.
  - Notes: Contracts use canonical v0.1 `MemoryId`, `ObjectType`, `MemoryObject`, and `MemoryLink` types and do not add live Qdrant/Oxigraph adapters, production raw storage, or pipeline behavior.

- 2026-04-27 Task_3 complete: added deterministic fake stores and fixtures.
  - Summary: Added `cfg(test)` repository test support with in-memory fake vector, graph, embedder, and raw reference resolver implementations plus representative v0.1 fixtures covering episode, observation, entities, soft thread link, derived reflection, user preference, open loop, commitment, correction/suppression, and hub entity seeds.
  - Validation evidence: Worker ran final `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and targeted `cargo test internal::repositories::test_support`; all passed. Targeted tests covered fake vector upsert/search/delete, fake graph object/link/lifecycle/raw-ref preservation, deterministic embedding, file-backed raw reference resolution, and fixture scenario coverage.
  - Notes: Initial formatting and `cfg(test)` compile issues were fixed before final validation; no service-backed tests or live adapters were introduced.

- 2026-04-27 Task_4 complete: final review approved and adapter-foundation plan drafted.
  - Summary: Final Reviewer approved the store contracts and deterministic test harness with no findings, and remediation mode completed with zero follow-up iterations. Drafted the next active plan for vector and graph adapter foundations.
  - Validation evidence: Reviewer reviewed changed code and validation evidence, reran `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and `cargo test internal::repositories::test_support`; all passed. Structured remediation appendix reported `structured_findings: []`, `status: APPROVED`, and `highest_severity: NONE`.
  - Notes: The next plan is independent and keeps remember/retrieve/link/correct/forget pipeline behavior out of scope.

## Review Remediation Log

### Iteration 0 Result
- Reviewer status: APPROVED
- Highest severity: NONE
- Kept findings: none
- Discarded findings: none
- Directives: none
- Follow-up task: none
- Validation result after follow-up: not applicable
- Stop reason: approved

## Decision Log (append-only; re-plans and major discoveries)

- 2026-04-27 Decision: Draft store contracts plan from landed domain model
  - Trigger / new insight: Domain foundation completed with canonical public domain types available under `src/api/types/domain`.
  - Plan delta: Created `v0-1-store-contracts-test-harness-plan.md` as the next concrete plan.
  - Tradeoffs considered: Defining contracts and fakes before live adapters keeps later Qdrant/Oxigraph work testable without external services.
  - User approval: pending.

- 2026-04-27 Decision: Place v0.1 store contracts in internal repositories with test-only reusable support
  - Trigger / new insight: Task_1 review found the current repository/model stack is organized around legacy flat `MemoryInput`/`MemoryType` DTOs and vector database concerns rather than the canonical v0.1 object model.
  - Production boundary: Add v0.1 contract traits under direct files in `src/internal/repositories/` and re-export them from `src/internal/repositories.rs`. These contracts must depend on canonical domain types and IDs from `src/api/types/domain.rs`; do not duplicate `Episode`, `Observation`, `Entity`, `MemoryThread`, `DerivedMemory`, or `MemoryLink` under `src/internal/models`.
  - Vector candidate boundary: If Task_2 needs a vector candidate record that is not itself a canonical domain object, place it under `src/internal/models/vector/` using a direct filename and re-export it through `src/internal/models/vector.rs`.
  - Test-support boundary: Put reusable deterministic fakes, fixture builders, and fake graph expansion support in `src/internal/repositories/test_support.rs` behind `cfg(test)` when they need reuse across later pipeline tests. Keep one-off assertions in module-local `tests.rs` files when they are not reusable.
  - Legacy boundary: Treat `MemoryRepository`, `VectorMemoryRepository`, `MemoryEntry`, `VectorMetadata`, `ScoredMemoryEntry`, and the current Qdrant adapter as replacement targets for v0.1. Keep them only while needed for current compilation during replacement, and do not add compatibility wrappers for the old flat API.
  - Non-goals preserved: This decision does not implement live Qdrant/Oxigraph adapters, remember/retrieve/link/correct/forget pipelines, or production raw storage.
  - Tradeoffs considered: Keeping reusable fakes under internal `cfg(test)` support avoids service dependencies and duplication in later pipeline tests, while keeping production contracts in repository modules preserves a narrow internal boundary for adapter work.
  - User approval: directed by Task_1 objective.

## Notes
- Risks:
  - Fake stores can accidentally become alternate production semantics if placed ambiguously.
  - Graph expansion behavior may be tempting to overbuild here; keep only enough structure for later tests.
  - Existing legacy repository traits serve the old flat API and should be removed or replaced when they no longer contribute to the v0.1 architecture.
- Edge cases:
  - Hub entity fixtures should seed high-fanout scenarios but do not need full retrieval behavior yet.
  - Suppression/correction fixtures should seed lifecycle states and supersession links without implementing lifecycle pipelines.
