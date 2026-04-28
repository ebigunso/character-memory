# Plan: v0.1 Remember And Link Pipelines

- status: draft
- generated: 2026-04-28
- last_updated: 2026-04-28
- work_type: mixed

## Goal
- Implement caller-supplied draft inputs and ordered persistence for v0.1 entities, episodes, observations, memory threads, derived memories, typed links, and selected vector records.
- Use the landed v0.1 store contracts, deterministic fakes, Qdrant vector candidate adapter, and embedded Oxigraph graph authority foundation.
- Keep this chunk focused on write/link behavior; do not implement retrieval, reranking, `ContinuityContextPack`, correction/forget lifecycle, or production raw storage.

## Definition of Done
- Draft input types or builders exist for remember and typed link behavior without exposing Qdrant/Oxigraph/RDF types.
- Drafts validate into canonical `MemoryObject` and `MemoryLink` values with stable IDs, schema version, raw refs, lifecycle/currentness fields where applicable, and derived-memory provenance.
- Remember pipeline persists graph objects before graph links, embeds selected vector records, and upserts vectors using `VectorCandidateStore::upsert_vector_records`.
- Typed link pipeline writes canonical `MemoryLink` records through `GraphAuthorityStore` and does not create vector records for links.
- Deterministic fake-store tests cover success, ordering, vector selection, validation failures, and partial failure behavior.
- Required Rust checks, targeted deterministic tests, live Qdrant smoke evidence, embedded Oxigraph smoke evidence, and Reviewer approval are complete, or blockers/waivers are explicitly recorded.

## Scope / Non-goals
- Scope:
  - Caller-supplied draft DTOs/builders for entities, episodes, observations, memory threads, derived memories, and links.
  - Internal remember/link pipeline service using `GraphAuthorityStore`, `VectorCandidateStore`, and `MemoryEmbedder`.
  - Ordered persistence and deterministic failure behavior.
  - Minimal public or transitional facade shape for `remember`/`link` if selected by Task_1.
  - Deterministic fake-store tests plus gated live Qdrant and embedded Oxigraph smoke checks.
- Non-goals:
  - `retrieve`, reranking, graph-expanded context assembly, or `ContinuityContextPack`.
  - `correct`, `forget`, suppression/supersession lifecycle APIs beyond preserving supplied lifecycle/currentness fields.
  - Production raw input storage.
  - Broad old flat facade removal unless narrowly required to avoid API ambiguity while preserving focused checks.
  - Multi-store transaction guarantees or automatic rollback unless explicitly implemented and tested in this chunk.

## Context (workspace)
- Related files/areas:
  - `src/api/types/domain.rs`
  - `src/api/types.rs`
  - `src/lib.rs`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal/infrastructures/**`
  - `tests/**`
  - `.github/workflows/**`
  - `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap.md`
  - `docs/coding-agent/plans/completed/v0-1-domain-foundation-plan.md`
  - `docs/coding-agent/plans/completed/v0-1-store-contracts-test-harness-plan.md`
  - `docs/coding-agent/plans/completed/v0-1-vector-graph-adapter-foundations-plan.md`
- Existing patterns or references:
  - Canonical v0.1 domain objects live in `src/api/types/domain.rs` and should not be duplicated internally.
  - `VectorRecord`, `VectorRecordEmbedding`, and natural-language vector builders live under `src/internal/models/vector/**`.
  - `VectorCandidateStore::upsert_vector_records`, `GraphAuthorityStore`, and `MemoryEmbedder` are the provider-neutral persistence contracts.
  - `FakeVectorCandidateStore`, `FakeGraphAuthorityStore`, `DeterministicMemoryEmbedder`, and fixtures live in `src/internal/repositories/test_support.rs` behind `cfg(test)`.
  - Qdrant payload fields are filter hints only; Oxigraph/`GraphAuthorityStore` remains authoritative for relationships, provenance, lifecycle, currentness, and graph expansion.
  - The old `CharacterMemory::create_memory` facade and flat `MemoryInput` path are legacy v0.1 replacement targets.
- Repo reference docs consulted:
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/decisions/design/ADR-D-0002-derived-memory-provenance.md`
  - `docs/decisions/design/ADR-D-0008-preserve-source-references.md`
  - `docs/decisions/implementation/ADR-I-0001-stable-cross-store-ids.md`
  - `docs/decisions/implementation/ADR-I-0004-typed-memory-links.md`
  - `docs/decisions/implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md`
  - `docs/decisions/implementation/ADR-I-0007-schema-versioning.md`

## Open Questions
- Q1: Should public draft DTOs generate IDs/timestamps when omitted, or should callers supply all durable IDs/timestamps for this first pipeline chunk?
- Q2: Should this chunk redesign `CharacterMemory::new` for v0.1 production stores, or add a test-first injectable constructor and defer default production wiring?
- Q3: On vector upsert failure after graph success, should the pipeline return a hard error only, or return persisted graph IDs plus an indexing failure status?
- Q4: Should `DerivedType::AssistantBehaviorNote` be renamed or aliased to the ADR-listed `assistant_preference` before draft DTOs become public, so behavior-influencing assistant preferences are not split across names?

## Assumptions
- A1: Draft types may be public if Task_1 selects a public `remember`/`link` facade, but they must stay canonical-domain-aligned and backend-free.
- A2: Persistence ordering should be validate all drafts, upsert graph objects, upsert graph links, embed selected vector records, then upsert vectors.
- A3: This chunk should fail closed on validation or store errors and explicitly document any non-atomic multi-store behavior.
- A4: Live Qdrant smoke evidence remains required before PR creation/merge; embedded Oxigraph smoke has no external service prerequisite.

## Deferred Review Findings To Address In This Plan
- Public flat facade replacement: the strict ADR review found that `CharacterMemory::{create_memory, search_memories, update_memory, delete_memory}` still makes the old `MemoryInput`/`MemoryType` and flat `Memory`/`ScoredMemory` model the practical public entry point. Task_1 and Task_5 must decide and implement removal or replacement when `remember`/`link` supersede it; isolation/deprecation is acceptable only with an explicit short-lived architecture or validation-scope reason.
- Legacy Qdrant repository retirement: `QdrantVectorMemoryRepository` still mixes old flat payload mapping, filter/index policy, point deserialization, and Qdrant I/O. Prefer retiring it with the flat facade. If it must survive temporarily, split legacy mapping helpers from I/O so it cannot be mistaken for the v0.1 Qdrant candidate adapter.
- Test-support split: `src/internal/repositories/test_support.rs` is a broad shared harness containing fakes, fixtures, raw-reference resolution, deterministic embedding, helper algorithms, and tests. As remember/link tests are added, split reusable test support by responsibility or keep new helpers module-local so the monolithic file does not become permanent debt.
- Derived type naming: resolve whether the ADR-listed `assistant_preference` should be represented directly before public draft DTOs harden serialized names.

## Tasks

### Task_1: Select draft API and pipeline boundary
- type: design
- owns:
  - `src/api/types/**`
  - `src/lib.rs`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `docs/coding-agent/plans/active/v0-1-remember-and-link-pipelines-plan.md`
- depends_on: []
- description: |
  Decide where draft DTOs/builders live, what public or transitional `remember`/`link` surface this chunk exposes, and the persistence/failure boundary before implementation edits.
- acceptance:
  - Decision records whether draft DTOs live in `src/api/types/domain.rs` or new direct files under `src/api/types/`.
  - Decision records public facade shape for `CharacterMemory::remember` and `CharacterMemory::link`, including how old flat methods remain isolated or are retired.
  - Decision records how the deferred review findings in this plan are handled, explicitly including legacy facade replacement, legacy Qdrant repository retirement/split, test-support split boundaries, and `assistant_preference` naming.
  - Decision records persistence ordering and partial-failure policy.
  - Decision keeps draft and pipeline contracts backend-free.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Record draft API, facade, persistence-order, and failure-policy decisions before implementation edits."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review boundary decision against roadmap, provenance/source-reference ADRs, and Qdrant/Oxigraph authority split."

### Task_2: Add caller-supplied draft input types
- type: impl
- owns:
  - `src/api/types.rs`
  - `src/api/types/**`
  - `src/lib.rs`
- depends_on: [Task_1]
- description: |
  Add draft input types/builders and conversion into canonical v0.1 domain objects and links without backend-specific fields.
- acceptance:
  - Draft types represent entities, episodes, observations, memory threads, derived memories, and memory links.
  - Draft conversion produces canonical `MemoryObject`/`MemoryLink` values with stable IDs, schema version, raw refs, lifecycle/currentness fields where applicable, and validation.
  - Derived memory drafts require at least one source episode or observation.
  - Derived memory draft type names align with ADR vocabulary, including resolving the `assistant_preference` naming question before the draft surface becomes public.
  - Drafts preserve raw references without introducing raw transcript storage.
  - Drafts do not expose Qdrant, Oxigraph, RDF, or SPARQL types.
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
    detail: "Run targeted draft validation/conversion tests added by this task."

### Task_3: Add internal remember pipeline service
- type: impl
- owns:
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal.rs`
- depends_on: [Task_2]
- description: |
  Add an internal remember pipeline that validates drafts, writes graph objects and links, embeds selected vector records, and upserts vector records in deterministic order.
- acceptance:
  - Pipeline accepts validated remember drafts and returns persisted object IDs, link IDs, and vector-indexed IDs.
  - Graph objects are persisted before links; links reference submitted objects or existing graph objects by ID/type.
  - Vector indexing uses `memory_object_vector_record` and indexes only `Episode`, `Observation`, `DerivedMemory`, `MemoryThread`, and `Entity`.
  - Embeddings use `MemoryEmbedder::embed_batch` or an equivalent deterministic-test-friendly path before `VectorCandidateStore::upsert_vector_records`.
  - Tests assert ordering and no vector write when graph object/link write fails.
  - New fake-store and fixture helpers are module-local or placed in narrower test-support modules rather than expanding the existing monolithic `test_support.rs` without a split plan.
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
    detail: "Run targeted module-local fake-store remember pipeline tests added by this task."

### Task_4: Add typed link pipeline
- type: impl
- owns:
  - `src/api/types.rs`
  - `src/api/types/**`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/lib.rs`
- depends_on: [Task_2]
- description: |
  Add typed link behavior that persists canonical `MemoryLink` records as graph-authoritative relationships without vector indexing.
- acceptance:
  - `link` accepts caller-supplied IDs/types/relation/confidence/rationale and persists a canonical `MemoryLink`.
  - Link validation rejects invalid confidence and invalid object-type/self-link cases selected by Task_1.
  - Graph store receives links as authoritative relationship records.
  - Link writes do not create vector records.
  - Tests cover fake graph and embedded Oxigraph link persistence where practical.
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
    detail: "Run targeted module-local typed-link pipeline tests added by this task."

### Task_5: Wire transitional facade and isolate legacy flat API
- type: impl
- owns:
  - `src/lib.rs`
  - `src/api/types.rs`
  - `src/api/types/**`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `tests/**` # public facade/integration tests only; unit tests stay beside source modules
  - `README.md`
- depends_on: [Task_3, Task_4]
- description: |
  Expose the selected `remember`/`link` surface or a clearly scoped transitional equivalent and prevent accidental extension of the old flat API path.
- acceptance:
  - Public or crate-visible construction path can inject graph store, vector store, and embedder for deterministic tests.
  - `CharacterMemory` exposes the selected `remember`/`link` surface or a clearly scoped transitional equivalent.
  - Old flat `create_memory` path is removed or replaced when the v0.1 surface supersedes it; isolation/deprecation is allowed only with an explicit architecture or validation-scope justification from Task_1.
  - The legacy `QdrantVectorMemoryRepository` path is retired with the flat facade when possible; if retained, its legacy mapping responsibilities are split or clearly namespaced so it cannot be confused with the v0.1 `VectorCandidateStore` adapter.
  - README examples are updated only if public surface changes require it.
  - No retrieve/correct/forget behavior is introduced.
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
    detail: "Run targeted facade-level deterministic tests added by this task."

### Task_6: Live smoke, review, and next-plan handoff
- type: review
- owns:
  - `.github/workflows/**`
  - `docs/coding-agent/plans/active/**`
  - `docs/coding-agent/plans/completed/**`
- depends_on: [Task_5]
- description: |
  Run required live/embedded smoke evidence, review the remember/link implementation, complete plan lifecycle updates, and draft the next Retrieve And ContinuityContextPack plan without implementing retrieval.
- acceptance:
  - Required deterministic validation evidence from Tasks 1-5 is recorded.
  - Live Qdrant remember-pipeline smoke passes locally or via CI evidence.
  - Embedded Oxigraph remember/link smoke passes.
  - Reviewer approves no retrieval/correction/raw-storage scope creep.
  - Next concrete plan for Retrieve And ContinuityContextPack is drafted from the landed remember/link shape.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "Run live Qdrant remember-pipeline smoke locally before PR creation or provide CI job evidence before merge."
  - kind: command
    required: true
    owner: worker
    detail: "Run embedded Oxigraph remember/link smoke."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review remember/link implementation against roadmap, ADRs, validation evidence, and non-goals."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Confirm evidence completeness, live-smoke route, no retrieval scope creep, and next-plan independence."

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (design gate): [Task_1]
- Wave 2 (draft inputs): [Task_2]
- Wave 3 (pipeline implementations): [Task_3, Task_4]
- Wave 4 (facade and legacy isolation): [Task_5]
- Wave 5 (live smoke, review, next-plan draft): [Task_6]

## E2E / Visual Validation Spec

- Not applicable. This is Rust library write/link pipeline behavior with no UI/user-flow surface.

## Rollback / Safety
- Keep draft and pipeline contracts backend-free; Qdrant/Oxigraph types stay inside infrastructure modules.
- Validate drafts before any store write.
- Treat graph and vector writes as non-atomic unless this chunk explicitly implements and tests rollback/compensation.
- Keep raw input storage consumer-owned and preserve only raw references.
- Keep retrieval, correction, forgetting, and context-pack behavior out of this plan.

## Quality Routing Note
- Routing level: L2
- In-scope docs: Rust API/internal pipeline architecture, persistence ordering, deterministic fake-store validation, live smoke evidence, data-integrity boundaries.
- Out-of-scope docs: UI/E2E, frontend/browser checks, retrieval/ranking behavior, correction/forget lifecycle, production raw storage.
- Top risks: public API shape, data-integrity, partial failure across graph/vector stores, legacy flat facade drift, live-service validation prerequisites.
- Risk profile: medium-high because this chunk writes across graph and vector stores and begins public/transitional API behavior.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted draft/pipeline/link/facade tests, live Qdrant smoke, embedded Oxigraph smoke, Reviewer gate.
- Optional recommended checks: gated CI workflow update if live smoke routing needs changes.

## Progress Log

- 2026-04-28 Plan drafted.
  - Summary: Created the next concrete plan for v0.1 remember and typed link pipelines from the completed domain/store contracts and final-reviewed vector/graph adapter foundation.
  - Validation evidence: Researcher plan-fill report and current adapter-foundation code review inputs.
  - Notes: Draft status; requires user approval before execution.

## Decision Log

- 2026-04-28 Decision: Draft remember/link plan after adapter foundation final review
  - Trigger / new insight: Adapter foundation final review approved Qdrant vector records, embedded Oxigraph graph authority, and bounded graph expansion with no remediation findings.
  - Plan delta: Added `v0-1-remember-and-link-pipelines-plan.md` as the next active concrete plan.
  - Tradeoffs considered: Starting with caller-supplied drafts keeps the write pipeline deterministic and avoids adding extractor/LLM dependencies. Keeping retrieval/correction out of scope protects this chunk from expanding into context-pack behavior.
  - User approval: pending.

## Notes
- Risks:
  - Draft DTOs may harden too early as public API; keep them minimal and canonical-domain-aligned.
  - Multi-store writes can leave graph/vector drift if vector indexing fails after graph success; tests must lock down the selected failure semantics.
  - Old flat `CharacterMemory` methods may confuse future implementation unless Task_1 explicitly isolates or retires them.
- Edge cases:
  - Derived memories without episode/observation provenance must fail validation before store writes.
  - Typed links should not be vector-indexed by default.
  - Submitted links may reference existing graph objects not included in the current batch; Task_1 should decide validation depth for those references.
