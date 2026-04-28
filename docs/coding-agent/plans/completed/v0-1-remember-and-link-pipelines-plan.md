# Plan: v0.1 Remember And Link Pipelines

- status: done
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

## Open Questions (max 3)
- None. The initial API construction and partial-failure questions are resolved below.

## Resolved Decisions
- Draft DTOs should generate IDs/timestamps when omitted while still allowing caller-supplied values for deterministic tests, migrations, import/replay workflows, and cross-store ID stability.
- Task_1 draft API boundary: keep canonical v0.1 domain objects in `src/api/types/domain.rs` and add caller-facing draft DTOs in new direct module files under `src/api/types/` rather than mixing construction-only draft state into the canonical domain module. Draft DTO modules must be re-exported through the existing API type surface when public, must remain backend-free, and must convert into canonical `MemoryObject`/`MemoryLink` values before internal pipelines run.
- Task_1 facade boundary: expose async `CharacterMemory::remember` and `CharacterMemory::link` as crate-visible write-surface wiring until a public graph/vector/embedder construction path is selected. `remember` accepts a backend-free remember draft and returns an explicit outcome containing persisted graph object IDs, persisted link IDs, vector-indexed object IDs, and any vector-indexing failure when graph persistence succeeded. `link` accepts a backend-free typed-link draft and returns the persisted canonical `MemoryLink` or equivalent link outcome. The old flat methods (`create_memory`, `bulk_create_memories`, `get_memory_by_id`, `get_memories_by_ids`, `search_memories`, `update_memory`, and `delete_memory`) are replacement targets, not compatibility promises: Task_5 should retire them when the new facade supersedes them, or temporarily isolate them behind clearly named legacy code only if needed to keep this chunk focused and compiling.
- Add an injectable v0.1 construction path now as a durable API boundary, not as a legacy-compatibility workaround. A `from_parts`- or builder-style path that accepts `GraphAuthorityStore`, `VectorCandidateStore`, and `MemoryEmbedder` remains useful after legacy removal for tests, alternate backends, alternate embedders, and application-owned wiring.
- Defer redesigning the default production `CharacterMemory::new` until the remember/link path lands and the old flat facade can be removed or replaced deliberately.
- If graph persistence succeeds but vector indexing fails, return an explicit partial-success outcome with persisted graph IDs plus indexing failure status, rather than hiding the authoritative graph write behind a hard error only.
- Task_1 dependency direction: draft DTOs depend only on public API/domain types and convert into canonical domain objects; remember/link pipelines depend inward on `GraphAuthorityStore`, `VectorCandidateStore`, and `MemoryEmbedder`; domain types and draft DTOs must not depend on Qdrant, Oxigraph, RDF, infrastructure adapters, or pipeline modules.
- Task_1 persistence policy: validate all drafts before any store write, then upsert graph objects, upsert graph links, build and embed selected vector records, and finally call `VectorCandidateStore::upsert_vector_records`. Graph object failure prevents link and vector writes; graph link failure prevents vector writes; vector embedding or vector upsert failure after graph success returns the explicit partial-success outcome and does not attempt automatic rollback in this chunk.
- Task_1 deferred-finding policy: replace the legacy flat facade in Task_5 rather than extending it; retire `QdrantVectorMemoryRepository` with the flat facade when possible, or split/namespace its legacy mapping helpers away from Qdrant I/O if temporary retention is required; keep new remember/link test helpers module-local or split reusable support by responsibility instead of growing `src/internal/repositories/test_support.rs` further; represent the ADR vocabulary `assistant_preference` directly before public draft serialized names harden, with any compatibility alias treated as temporary migration scaffolding.
- Comments must distinguish temporary migration scaffolding from durable production API documentation. Code that should be removed or changed later needs clear removal-condition comments; code intended to remain after the complete refactor should use stable, production-ready comments that should not require churn.

## Assumptions
- A1: Draft types may be public if Task_1 selects a public `remember`/`link` facade, but they must stay canonical-domain-aligned and backend-free.
- A2: Persistence ordering should be validate all drafts, upsert graph objects, upsert graph links, embed selected vector records, then upsert vectors.
- A3: This chunk should fail closed on validation or store errors and explicitly document any non-atomic multi-store behavior.
- A4: Live Qdrant smoke evidence remains required before PR creation/merge; embedded Oxigraph smoke has no external service prerequisite.
- A5: `DerivedType` currently exposes `AssistantBehaviorNote`; Task_1 must decide whether to rename, alias, or intentionally defer the ADR-listed `assistant_preference` vocabulary before any public draft surface hardens serialized names.

## Deferred Review Findings To Address In This Plan
- Public flat facade replacement: the strict ADR review found that `CharacterMemory::{create_memory, search_memories, update_memory, delete_memory}` still makes the old `MemoryInput`/`MemoryType` and flat `Memory`/`ScoredMemory` model the practical public entry point. Task_1 and Task_5 must decide and implement removal or replacement when `remember`/`link` supersede it; isolation/deprecation is acceptable only with an explicit short-lived architecture or validation-scope reason.
- Legacy Qdrant repository retirement: `QdrantVectorMemoryRepository` still mixes old flat payload mapping, filter/index policy, point deserialization, and Qdrant I/O. Prefer retiring it with the flat facade. If it must survive temporarily, split legacy mapping helpers from I/O so it cannot be mistaken for the v0.1 Qdrant candidate adapter.
- Test-support split: `src/internal/repositories/test_support.rs` is a broad shared harness containing fakes, fixtures, raw-reference resolution, deterministic embedding, helper algorithms, and tests. As remember/link tests are added, split reusable test support by responsibility or keep new helpers module-local so the monolithic file does not become permanent debt.
- Derived type naming: resolve whether the ADR-listed `assistant_preference` should be represented directly before public draft DTOs harden serialized names.

## Tasks

### Task_1: Select draft API and pipeline boundary
- type: design
- owns:
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
  - Decision records the dependency direction: draft DTOs convert into canonical domain objects; pipelines depend on `GraphAuthorityStore`, `VectorCandidateStore`, and `MemoryEmbedder`; domain types do not depend on Qdrant, Oxigraph, RDF, or pipeline modules.
  - Decision records comment policy for this chunk: temporary migration comments name the removal/change condition, while durable constructor/API comments are written as production documentation.
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
  - The pipeline explicitly skips `MemoryObject::MemoryLink` during vector record construction.
  - Embeddings use `MemoryEmbedder::embed_batch` or an equivalent deterministic-test-friendly path before `VectorCandidateStore::upsert_vector_records`.
  - Tests assert ordering and no vector write when graph object/link write fails.
  - Tests assert selected vector records are paired with embeddings in stable order and fail clearly if the embedder returns the wrong number of vectors.
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
  - `tests/**`
  - `README.md`
- depends_on: [Task_3, Task_4]
- description: |
  Expose the selected `remember`/`link` surface or a clearly scoped transitional equivalent and prevent accidental extension of the old flat API path.
- acceptance:
  - Public or crate-visible construction path can inject graph store, vector store, and embedder for deterministic tests.
  - Injectable construction is documented as a durable composition boundary, not as temporary legacy compatibility scaffolding.
  - `CharacterMemory` exposes the selected `remember`/`link` surface or a clearly scoped transitional equivalent.
  - Old flat `create_memory` path is removed or replaced when the v0.1 surface supersedes it; isolation/deprecation is allowed only with an explicit architecture or validation-scope justification from Task_1.
  - The legacy `QdrantVectorMemoryRepository` path is retired with the flat facade when possible; if retained, its legacy mapping responsibilities are split or clearly namespaced so it cannot be confused with the v0.1 `VectorCandidateStore` adapter.
  - README examples are updated only if public surface changes require it.
  - No retrieve/correct/forget behavior is introduced.
  - Pure Rust behavior tests remain in source-module test files; `tests/**` is used only for public facade or integration-style coverage.
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
- Wave 3 (remember pipeline): [Task_3]
- Wave 4 (typed link pipeline): [Task_4]
- Wave 5 (facade and legacy isolation): [Task_5]
- Wave 6 (live smoke, review, next-plan draft): [Task_6]

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
- In-scope docs: Rust API/internal pipeline architecture, architecture gates, persistence ordering, deterministic fake-store validation, live smoke evidence, data-integrity boundaries.
- Out-of-scope docs: UI/E2E, frontend/browser checks, retrieval/ranking behavior, correction/forget lifecycle, production raw storage.
- Top risks: public API shape, data-integrity, partial failure across graph/vector stores, legacy flat facade drift, live-service validation prerequisites.
- Risk profile: medium-high because this chunk writes across graph and vector stores and begins public/transitional API behavior.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted draft/pipeline/link/facade tests, live Qdrant smoke, embedded Oxigraph smoke, Reviewer gate.
- Optional recommended checks: `cargo clippy --all-targets -- -D warnings`, gated CI workflow update if live smoke routing needs changes.

## Architecture Gate Notes
- Boundary map: public draft/facade types convert into canonical domain values; internal pipeline code coordinates provider-neutral store/embedder traits; Qdrant/Oxigraph/RDF details remain in infrastructure adapters.
- Data integrity gate: validation must complete before graph or vector writes; graph object/link writes are authoritative and precede vector indexing; vector payload relationship IDs remain filter hints.
- Failure containment gate: graph object failure prevents link/vector writes; graph link failure prevents vector writes; vector failure after graph success must return the Task_1-selected explicit non-atomic result or error shape.
- Observability/evidence gate: tests must prove write ordering, skipped link vectorization, partial failure behavior, and raw reference preservation without logging or persisting raw transcripts.

## Progress Log

- 2026-04-28 Plan drafted.
  - Summary: Created the next concrete plan for v0.1 remember and typed link pipelines from the completed domain/store contracts and final-reviewed vector/graph adapter foundation.
  - Validation evidence: Researcher plan-fill report and current adapter-foundation code review inputs.
  - Notes: Draft status; requires user approval before execution.

- 2026-04-28 Plan refreshed for approval.
  - Summary: Rechecked prior completed plans, roadmap/design documents, relevant ADRs, canonical domain types, store contracts, vector record builders, and the legacy `CharacterMemory` facade. Tightened open questions, task ownership, validation evidence, and sequential waves where owns overlap.
  - Validation evidence: Direct source inspection of `src/api/types/domain.rs`, `src/internal/repositories.rs`, `src/internal/repositories/graph_authority_store.rs`, `src/internal/repositories/vector_candidate_store.rs`, `src/internal/repositories/embedder.rs`, `src/internal/models/vector/record.rs`, `src/internal/models/vector/embedding_surface.rs`, and `src/lib.rs`; Researcher plan-fill report.
  - Notes: Draft status; requires user approval before execution.

- 2026-04-28 Initial open questions resolved.
  - Summary: User accepted generated IDs/timestamps with caller override, durable injectable construction, deferred default constructor redesign, explicit partial-success result for graph-success/vector-failure, and clear temporary-vs-durable code comment guidance.
  - Validation evidence: Planning decision update only; no Rust validation required.
  - Notes: Plan remains draft until the user approves execution.

- 2026-04-28 Task_1 complete: selected draft API and pipeline boundary.
  - Summary: Recorded the draft DTO module boundary, `CharacterMemory::remember`/`link` facade direction, legacy flat API and Qdrant retirement policy, test-support split policy, `assistant_preference` naming direction, write ordering, partial-success vector failure semantics, dependency direction, and temporary-vs-durable comment policy.
  - Validation evidence: Worker-owned design review passed; Reviewer approved Task_1 with no findings.
  - Notes: Plan status moved to `in_progress` before Rust implementation began.

- 2026-04-28 Task_2 complete: added caller-supplied draft input types.
  - Summary: Added backend-free draft DTOs and conversion defaults for entities, episodes, observations, memory threads, derived memories, memory links, and draft object wrappers. Public derived-memory serialization now uses `assistant_preference`, with a documented temporary compatibility alias for `assistant_behavior_note`.
  - Validation evidence: Worker ran `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and `cargo test api::types::draft::tests`; all passed. Problems diagnostics were clean for touched Rust files.
  - Notes: Pure draft behavior tests live beside the new source module; no service-backed checks were needed for this task.

- 2026-04-28 Task_3 complete: added internal remember pipeline service.
  - Summary: Added an internal remember pipeline that validates draft inputs before writes, persists graph objects before graph links, embeds selected non-link vector records, and returns explicit partial-success vector indexing failures after graph success.
  - Validation evidence: Worker ran final `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and `cargo test internal::repositories::remember_pipeline::tests`; all passed. Problems diagnostics were clean for touched internal repository files.
  - Notes: Module-local fake stores cover write ordering, validation short-circuiting, graph failure short-circuiting, link vector skipping, vector upsert partial failure, and embedding count mismatch handling.

- 2026-04-28 Task_4 complete: added typed link pipeline.
  - Summary: Added an internal graph-only link pipeline that converts `MemoryLinkDraft` into canonical `MemoryLink` and persists it through `GraphAuthorityStore::upsert_links`. Canonical link validation now rejects invalid confidence, self-links, and `MemoryLink` endpoints before store writes.
  - Validation evidence: Worker ran `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo test internal::repositories::link_pipeline::tests`, and adjacent `cargo test api::types`; all final runs passed. Problems diagnostics were clean for touched Rust files.
  - Notes: Embedded Oxigraph link persistence is deferred to Task_6 because Task_4 owns repository/API files, while the concrete Oxigraph store lives under infrastructure modules.

- 2026-04-28 Task_5 complete: wired transitional facade and isolated legacy flat API.
  - Summary: Added `RememberDraft`/`RememberOutcome`, crate-visible `CharacterMemory::remember`/`link`, and a crate-visible durable `from_parts` composition boundary for injected graph/vector/embedder parts. Legacy flat methods now route through optional legacy state and are deprecated with removal-condition comments.
  - Validation evidence: Worker ran final `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and `cargo test injected_facade --lib`; all passed. Orchestrator then fixed warning-as-error fallout in legacy integration tests and reran `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check`, `cargo test --no-run`, and `cargo test injected_facade --lib`; all passed.
  - Notes: Existing integration tests that intentionally exercise the legacy flat facade now allow deprecation warnings locally. README was not updated because default public production construction for remember/link remains deferred.

- 2026-04-28 Task_6 validation evidence collected.
  - Summary: Ran final smoke evidence and drafted the next active retrieve/context-pack plan from the landed remember/link shape.
  - Validation evidence: `cargo test embedded_in_memory_oxigraph_smoke_has_no_external_prerequisite --lib` passed. Initial ignored Qdrant smoke failed because `QDRANT_CONNECTION_STRING` was unset; after `docker compose -f docker-compose.qdrant.yml up -d`, `QDRANT_CONNECTION_STRING=http://localhost:6334 cargo test qdrant_candidate_store_live_smoke_upserts_searches_and_deletes --lib -- --ignored` passed. Qdrant service was stopped with `docker compose -f docker-compose.qdrant.yml down`.
  - Notes: Drafted `docs/coding-agent/plans/active/v0-1-retrieve-continuity-context-pack-plan.md` as the next independent concrete plan.

- 2026-04-28 Task_6 initial Reviewer gate blocked on plan evidence recording.
  - Summary: Reviewer found no implementation-blocking issues but could not approve final completion because Task_6 smoke evidence had not yet been recorded in this plan.
  - Validation evidence: Reviewer confirmed implementation behavior satisfied remember/link acceptance and noted a non-blocking vocabulary consistency follow-up for the internal vector embedding label.
  - Notes: Follow-up aligned the internal derived-memory vector label to `Assistant preference`; `cargo fmt --check` and `cargo test internal::models::vector --lib` passed afterward. Re-review is pending.

- 2026-04-28 Task_6 complete: final review approved and retrieve plan drafted.
  - Summary: Final Reviewer approved remember/link implementation and validation evidence with no blocking findings. Drafted the next active retrieve/context-pack plan.
  - Validation evidence: Reviewer status APPROVED; required deterministic checks and smoke evidence are recorded above.
  - Notes: Plan lifecycle is ready to move from active to completed.

- 2026-04-28 Plan lifecycle closed.
  - Summary: Moved this plan from active to completed and updated the starter episodic memory roadmap to mark remember/link complete and retrieve/context-pack active.
  - Validation evidence: Final `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo clippy --all-targets -- -D warnings`, and Problems diagnostics all passed.
  - Notes: Implementation changes remain uncommitted after the earlier plan-decision commit.

- 2026-04-28 Task_1 completed: selected draft API and pipeline boundary.
  - Summary: Recorded the draft DTO module boundary, public remember/link facade shape, legacy flat API retirement/isolation policy, deferred review-finding handling, persistence ordering, partial-failure policy, backend-free dependency direction, and comment policy before implementation edits.
  - Validation evidence: Manual design review against this plan, the v0.1 roadmap, completed domain/store/adapter-foundation plans, and current facade/domain/store contract shapes; Reviewer approved the boundary with no findings.
  - Notes: No Rust source files, tests, README, rule files, or git state were changed by this task.

## Decision Log

- 2026-04-28 Decision: Draft remember/link plan after adapter foundation final review
  - Trigger / new insight: Adapter foundation final review approved Qdrant vector records, embedded Oxigraph graph authority, and bounded graph expansion with no remediation findings.
  - Plan delta: Added `v0-1-remember-and-link-pipelines-plan.md` as the next active concrete plan.
  - Tradeoffs considered: Starting with caller-supplied drafts keeps the write pipeline deterministic and avoids adding extractor/LLM dependencies. Keeping retrieval/correction out of scope protects this chunk from expanding into context-pack behavior.
  - User approval: pending.

- 2026-04-28 Decision: Resolve draft defaults, injectable construction, and partial indexing failure semantics
  - Trigger / new insight: User accepted generated ID/timestamp defaults with caller overrides and explicit partial-success outcomes, then clarified that injectable construction should be considered useful after legacy removal rather than only a backward-compatibility tool.
  - Plan delta: Replaced the initial open questions with resolved decisions, added durable injectable-constructor guidance, and added comment policy distinguishing temporary migration scaffolding from stable production-ready API comments.
  - Tradeoffs considered: Requiring caller-supplied IDs would improve explicitness but make normal use noisy; hard-erroring on vector failure would obscure graph-authoritative persistence; treating injectable construction as temporary would undercut testability and backend substitution after the refactor.
  - User approval: yes.

- 2026-04-28 Decision: Select remember/link draft API and pipeline boundary
  - Trigger / new insight: Implementation was approved, so Task_1 needed to lock the API/module boundary before source edits.
  - Plan delta: Status moved to `in_progress`; Resolved Decisions now select new direct draft DTO files under `src/api/types/`, async `CharacterMemory::remember`/`link` as the v0.1 facade, legacy flat facade retirement with temporary isolation only if necessary, explicit handling for the deferred review findings, graph-before-vector persistence ordering, non-atomic partial-success behavior, backend-free dependency direction, and temporary-vs-durable comment policy.
  - Tradeoffs considered: Keeping drafts out of `domain.rs` prevents construction-only API state from diluting canonical persisted domain objects; retiring the flat facade reduces ambiguity during the v0.1 rewrite; returning partial success makes graph-authoritative writes visible when vector indexing fails while avoiding unimplemented cross-store rollback guarantees.
  - User approval: yes.

## Notes
- Risks:
  - Draft DTOs may harden too early as public API; keep them minimal and canonical-domain-aligned.
  - Multi-store writes can leave graph/vector drift if vector indexing fails after graph success; tests must lock down the selected failure semantics.
  - Old flat `CharacterMemory` methods may confuse future implementation unless Task_1 explicitly isolates or retires them.
- Edge cases:
  - Derived memories without episode/observation provenance must fail validation before store writes.
  - Typed links should not be vector-indexed by default.
  - Submitted links may reference existing graph objects not included in the current batch; Task_1 should decide validation depth for those references.
