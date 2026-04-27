# Plan: v0.1 Vector And Graph Adapter Foundations

- status: done
- generated: 2026-04-27
- last_updated: 2026-04-27
- work_type: mixed

## Goal
- Build the first live-adapter foundation on top of the v0.1 store contracts and deterministic test harness.
- Keep Qdrant responsible for vector candidate recall and filterable payload hints.
- Introduce Oxigraph/RDF graph authority behavior for canonical v0.1 memory objects and links without implementing remember/retrieve/link/correct/forget pipelines.

## Definition of Done
- Qdrant-facing vector payload mapping exists for indexed v0.1 object records without making Qdrant authoritative for graph relationships.
- Natural-language embedding surface builders exist for indexed v0.1 objects and remain deterministic in tests.
- Oxigraph/RDF mapping exists for canonical domain objects and memory links.
- Adapter tests cover payload/RDF mapping, stable IDs/graph URIs, lifecycle/currentness fields, and bounded graph expansion behavior at the adapter-foundation level.
- Required Rust checks and targeted adapter-foundation tests pass, or blockers are recorded with evidence.
- Reviewer approves the adapter-foundation diff and validation evidence.

## Scope / Non-goals
- Scope:
  - Qdrant payload record/mapping for `Episode`, `Observation`, `DerivedMemory`, `MemoryThread`, and `Entity` vector candidates.
  - Natural-language embedding surface construction for vector-indexed records.
  - Oxigraph/RDF vocabulary and mapping for canonical v0.1 objects and `MemoryLink` relationships.
  - Adapter-foundation tests that can run deterministically without requiring live services where practical.
  - Gated or documented live-service checks only where adapter behavior requires them.
- Non-goals:
  - Public `remember`, `retrieve`, `link`, `correct`, or `forget` pipeline implementation.
  - Full hybrid retrieval, reranking, continuity context pack assembly, or rationale generation.
  - Production raw input storage.
  - Compatibility wrappers for the old flat API.
  - Broad removal of legacy flat API surfaces unless directly required to integrate the new adapter foundation.

## Context (workspace)
- Related files/areas:
  - `src/api/types/domain.rs`
  - `src/internal/models/vector/**`
  - `src/internal/repositories/**`
  - `src/internal/infrastructures/external_services/**`
  - `src/config/**`
  - `tests/**`
  - `docs/coding-agent/plans/completed/v0-1-store-contracts-test-harness-plan.md`
- Existing patterns or references:
  - Store contracts now live under `src/internal/repositories/` and use canonical v0.1 domain types.
  - Deterministic fakes and fixtures live under `src/internal/repositories/test_support.rs` behind `cfg(test)`.
  - Existing Qdrant adapter code is legacy flat-memory-shaped and should be replaced or isolated as v0.1 adapter work lands.
- Repo reference docs consulted:
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/decisions/implementation/ADR-I-0003-qdrant-oxigraph-defaults.md`
  - `docs/decisions/implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md`
  - `docs/decisions/implementation/ADR-I-0006-bounded-graph-expansion.md`

## Open Questions
- Q1: Should this chunk add the Oxigraph crate and use an embedded/in-memory Oxigraph test path immediately, or first land RDF mapping helpers with a trait-backed fake and add the dependency in a narrower follow-up?
- Q2: Should Qdrant live-service checks remain fully gated in this chunk, or should an adapter smoke test be required when local Qdrant configuration is present?

## Assumptions
- A1: Qdrant payload mapping can be implemented as provider-specific adapter code while keeping the internal `VectorCandidateStore` contract provider-neutral.
- A2: Oxigraph graph authority should be introduced behind the `GraphAuthorityStore` contract and should not leak RDF/SPARQL types into canonical domain models.
- A3: Graph relationships and lifecycle/currentness remain authoritative in the graph store; duplicated Qdrant payload fields are filter hints.
- A4: Raw input remains consumer-owned; adapters should carry or preserve raw references only.

## Tasks

### Task_1: Select adapter module and dependency boundary
- type: design
- owns:
  - `src/internal/infrastructures/**`
  - `src/internal/repositories/**`
  - `src/internal/models/**`
  - `Cargo.toml`
  - `docs/coding-agent/plans/active/v0-1-vector-graph-adapter-foundations-plan.md`
- depends_on: []
- description: |
  Inspect the current Qdrant adapter/config layout and choose the concrete module/dependency boundary for v0.1 Qdrant and Oxigraph adapter foundations before implementation.
- acceptance:
  - Decision records where Qdrant payload mapping and Oxigraph/RDF mapping will live.
  - Decision records whether this chunk adds an Oxigraph dependency or defers it.
  - Decision keeps vendor-specific types out of canonical domain and provider-neutral repository contracts.
  - Decision identifies any legacy flat adapter pieces that can be replaced or must temporarily coexist for compilation.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Inspect adapter/config layout and record the selected v0.1 adapter boundary."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review adapter boundary for consistency with store contracts and storage ADRs."

### Task_2: Add Qdrant vector payload foundation
- type: impl
- owns:
  - `src/internal/models/vector/**`
  - `src/internal/infrastructures/external_services/**`
  - `src/internal/repositories/**`
- depends_on: [Task_1]
- description: |
  Add Qdrant-facing payload mapping for v0.1 vector candidate records and natural-language embedding surfaces without implementing retrieval pipelines.
- acceptance:
  - Payload mapping includes stable object ID, graph URI, object/record type, schema version, embedding text, content text, lifecycle/currentness hints where applicable, and relevant episode/observation/thread/entity IDs.
  - Embedding surface builders produce natural-language text rather than structured metadata templates.
  - Mapping tests cover representative v0.1 objects using deterministic fixtures.
  - Qdrant remains candidate/filter infrastructure and does not become graph authority.
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
    detail: "Run targeted vector payload/embedding-surface tests added by this task."

### Task_3: Add graph authority mapping foundation
- type: impl
- owns:
  - `Cargo.toml`
  - `src/internal/infrastructures/**`
  - `src/internal/repositories/**`
  - `src/internal/models/**`
- depends_on: [Task_1]
- description: |
  Add RDF/Oxigraph graph authority mapping or a pre-Oxigraph RDF mapping layer, depending on the Task_1 boundary decision.
- acceptance:
  - Mapping covers canonical objects and `MemoryLink` relationships needed for provenance, entity/thread links, supersession, retention state, and currentness.
  - Stable graph URIs use the canonical domain helper.
  - Bounded expansion adapter behavior or mapping-level expansion inputs preserve explicit max-depth/max-node limits.
  - No retrieval context-pack or public pipeline behavior is implemented.
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
    detail: "Run targeted graph mapping/adapter-foundation tests added by this task."

### Task_4: Validate adapter boundaries and update next plan
- type: review
- owns:
  - `docs/coding-agent/plans/active/**`
- depends_on: [Task_2, Task_3]
- description: |
  Review the adapter-foundation diff and validation evidence. If approved, draft the next concrete remember/link pipeline plan from the landed adapter shape.
- acceptance:
  - Reviewer approves the adapter-foundation diff or blocking issues are resolved/waived.
  - Required validation evidence from Tasks 1-3 is present.
  - This plan's Progress Log and Decision Log are updated with outcomes.
  - A separate active plan for remember/link pipelines is drafted from the landed code shape.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Review adapter-foundation implementation against storage contracts, ADRs, and validation evidence."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Confirm the next concrete plan is independent and based on landed adapter-foundation code."

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (design gate): [Task_1]
- Wave 2 (adapter mappings): [Task_2, Task_3]
- Wave 3 (review and next-plan draft): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. This is Rust storage adapter infrastructure with no UI/user-flow surface.

## Rollback / Safety
- Keep vendor-specific Qdrant/Oxigraph types outside canonical domain model modules.
- Keep graph authority semantics in graph mapping/adapter code; Qdrant payload fields are filter hints only.
- Keep raw input storage consumer-owned and preserve only raw references.
- Keep live-service checks gated and documented if they cannot run deterministically in local unit tests.

## Quality Routing Note
- Routing level: L2
- In-scope docs: Rust adapter architecture, storage boundary design, deterministic adapter/mapping tests, validation evidence, data-integrity boundaries.
- Out-of-scope docs: UI/E2E, auth/security, public pipeline behavior, production raw storage.
- Top risks: data-integrity, external dependency/integration, contract/API/schema compatibility, architectural drift between Qdrant payload hints and graph authority.
- Risk profile: medium-high; this chunk introduces live-backend adapter foundations and may add an Oxigraph dependency, but should keep behavior bounded to mappings/adapters.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted mapping/adapter tests, Reviewer gate.
- Optional recommended checks: gated Qdrant/Oxigraph live-service smoke checks if local prerequisites are available and documented.
- At Risk items: Oxigraph dependency timing, live-service validation prerequisites, and avoiding duplicated graph authority in Qdrant payloads.

## Progress Log

- 2026-04-27 Plan drafted.
  - Summary: Created the next concrete plan from the landed v0.1 store contracts and deterministic test harness shape.
  - Validation evidence: Pending approval and execution.
  - Notes: Draft keeps remember/retrieve/link/correct/forget pipelines out of scope.

## Decision Log

- 2026-04-27 Decision: Draft adapter-foundation plan from store contract shape
  - Trigger / new insight: Store contracts and deterministic fakes now define provider-neutral boundaries for vector candidates, graph authority, embeddings, and raw reference resolution.
  - Plan delta: Created `v0-1-vector-graph-adapter-foundations-plan.md` as the next concrete plan.
  - Tradeoffs considered: Splitting adapter foundations from remember/retrieve pipelines keeps storage mapping and backend responsibilities reviewable before behavior pipelines depend on them.
  - User approval: pending.

## Notes
- Risks:
  - Qdrant payload fields can drift from graph truth unless later write flows update both predictably.
  - Oxigraph dependency and test strategy may need a design gate before implementation.
  - Hub entities can create expansion pressure; adapter tests should preserve explicit bounds.
- Edge cases:
  - Suppressed/deleted and non-current/superseded state must be represented for later filtering, even if retrieval filtering is not implemented here.
  - Raw refs must survive mapping without adding raw text storage to Qdrant or Oxigraph.
  - Natural-language embedding surfaces should avoid metadata-template strings that hurt semantic recall.
