# Roadmap: v0.1 Starter Episodic Memory Implementation

- status: draft
- generated: 2026-04-27
- last_updated: 2026-04-27
- work_type: mixed
- roadmap_scope: full v0.1 implementation phase
- concrete_plan_policy: draft one separate implementation plan per chunk when that chunk is reached

## Goal
- Guide the full v0.1 implementation phase at roadmap granularity.
- Keep concrete execution plans separate so each chunk can be drafted, approved, executed, reviewed, and completed independently.
- Preserve the handoff's constraints: Rust library crate, chat/voice-transcript scope, Qdrant for vector candidate recall, Oxigraph for graph authority, stable cross-store IDs, deterministic tests, and no required LLM dependency in core.

## Current State Assessment
- The current public facade is `CharacterMemory` in `src/lib.rs`, with flat methods such as `create_memory`, `search_memories`, `update_memory`, and `delete_memory`; this is now legacy surface for the v0.1 rewrite rather than the canonical model.
- Canonical v0.1 domain objects now live under `src/api/types/domain.rs`, with store/embedder contracts and deterministic fake-store fixtures under `src/internal/repositories/**`.
- The current live persistence adapter is still vector-only and legacy-shaped: `MemoryRepository` delegates to `VectorMemoryRepository`, and the concrete live adapter is Qdrant.
- Qdrant currently stores flat payload fields such as `id`, `memory_type`, `content`, `timestamp`, `location_text`, and `participants`; it does not store v0.1 object types, graph URIs, lifecycle/currentness, derived types, or schema versions.
- There are v0.1 graph authority contracts and deterministic fake graph expansion support, but no current Oxigraph adapter, RDF vocabulary, SPARQL query layer, or authoritative persisted provenance graph.
- The current tests cover Qdrant-backed legacy create/read/update/delete/search/filter behavior, v0.1 domain model behavior, and deterministic store/fake-store support, but not live v0.1 Qdrant payloads, RDF/Oxigraph persistence, grouped retrieval, correction supersession pipelines, or live bounded graph expansion.
- `Cargo.toml` already includes core async/serde/uuid/chrono/qdrant dependencies; the adapter-foundation chunk is expected to add Oxigraph/RDF support and an embedded/in-memory graph-store foundation.

## Resolved Decisions
- Breaking changes are acceptable for v0.1. Compatibility with the old flat API is not a goal; legacy implementations that do not contribute to the new v0.1 architecture can and should be removed as replacement chunks land.
- Raw inputs should remain consumer-owned. Core objects should store only a `raw_ref` or equivalent reference ID.
- For tests in this first version, raw text may be stored in a temporary file fixture and linked through a reference ID.
- Graph validation phasing is accepted: use trait-backed graph fakes for pipeline tests, embedded/in-memory Oxigraph tests for RDF/SPARQL behavior where practical, and prerequisite-gated service-backed Oxigraph checks for adapter PR evidence.
- The vector/graph adapter-foundation chunk should add the Oxigraph crate with the other adapter code.
- Live Qdrant/Oxigraph smoke checks should run in CI before merge or locally before PR creation; they are not optional merge evidence.

## Roadmap Chunks

### Domain Foundation And Breaking API Direction
- Purpose: establish the v0.1 object vocabulary, ID strategy, schema versioning, lifecycle enums, raw reference policy, and validation invariants.
- Expected outcome: typed model foundation and deterministic tests, with old flat memory concepts no longer treated as canonical and no compatibility promise for legacy flat APIs.
- Concrete plan: completed in [docs/coding-agent/plans/completed/v0-1-domain-foundation-plan.md](../completed/v0-1-domain-foundation-plan.md)

### Store Contracts And Deterministic Test Harness
- Purpose: define vector, graph, raw-reference, and embedder contracts around the v0.1 objects.
- Expected outcome: fake/in-memory stores and fixtures that support remember/retrieve/lifecycle tests without Qdrant, Oxigraph, OpenAI, or network services, while identifying legacy repository/model pieces that should be removed once replaced.
- Concrete plan: completed in [docs/coding-agent/plans/completed/v0-1-store-contracts-test-harness-plan.md](../completed/v0-1-store-contracts-test-harness-plan.md)

### Vector And Graph Adapter Foundations
- Purpose: migrate Qdrant payloads to `VectorRecord`, add natural-language embedding surfaces, and introduce RDF/Oxigraph graph authority behavior.
- Expected outcome: Qdrant remains candidate/filter infrastructure while Oxigraph becomes authoritative for relationships, provenance, lifecycle, currentness, supersession, and bounded graph expansion.
- Concrete plan: [docs/coding-agent/plans/active/v0-1-vector-graph-adapter-foundations-plan.md](v0-1-vector-graph-adapter-foundations-plan.md)

### Remember And Link Pipelines
- Purpose: implement caller-supplied draft inputs and persistence ordering for entities, episodes, observations, links, derived memories, and selected vector records.
- Expected outcome: `remember` and typed `link` behavior can persist v0.1 memory objects with provenance and relationship links.
- Concrete plan: draft after store contracts and adapter foundations have enough working surface.

### Retrieve And ContinuityContextPack
- Purpose: implement vector-to-graph retrieval, bounded expansion, lifecycle/currentness filtering, deterministic reranking, grouped context pack assembly, rationale, and optional trace.
- Expected outcome: `retrieve` returns `ContinuityContextPack` and excludes suppressed/deleted and superseded/non-current memories by default.
- Concrete plan: draft after remember/link and graph expansion behavior are stable.

### Correction And Forget Lifecycle
- Purpose: implement non-destructive correction through supersession and lifecycle updates, plus `forget` with suppression as the default.
- Expected outcome: corrections preserve provenance, old derived memories become non-current, and suppressed memories are hidden from normal retrieval.
- Concrete plan: draft after retrieval filtering semantics are stable.

### Documentation, Migration Cleanup, And Release Validation
- Purpose: update README and roadmap docs, remove or rewrite old flat memory examples, remove non-contributing legacy implementations, and run final deterministic plus gated integration validation.
- Expected outcome: v0.1 is documented as chat-native episodic continuity memory, with Qdrant/Oxigraph responsibilities and old flat concepts clearly retired or removed.
- Concrete plan: draft after implementation behavior is substantially complete.

## Cross-Cutting Validation Expectations
- Every concrete implementation plan should include `cargo fmt --check`, `cargo check`, and `cargo test --no-run` unless explicitly waived.
- Deterministic unit/fake-store tests are required before relying on service-backed integration tests.
- Qdrant and Oxigraph live-service checks should be prerequisite-gated and documented, but required before PR merge through CI or locally before PR creation.
- Reviewer gates are required for non-trivial implementation chunks before marking a plan complete.

## Rollback / Safety
- Keep future chunk plans independent so individual chunks can be completed, paused, revised, or moved to completed plans without rewriting the whole roadmap.
- Draft each next concrete plan from the code and decisions that actually landed in prior chunks.
- Do not add compatibility wrappers for the old flat API; remove legacy implementations when they no longer serve the v0.1 architecture.
- Keep raw input storage consumer-owned; tests may use files only to prove reference preservation.

## Progress Log

- 2026-04-27 Roadmap separated from concrete plans.
  - Summary: Split the combined roadmap/Chunk 1 plan into this roadmap-only artifact plus a separate domain-foundation execution plan.
  - Validation evidence: Reviewer approved the roadmap/plan split and confirmed the roadmap stays at high-level chunk granularity.
  - Notes: Graph validation phasing accepted by user and recorded as resolved.
- 2026-04-27 Domain foundation completed; store-contracts plan drafted.
  - Summary: Domain foundation plan completed and moved to completed plans. Drafted the next concrete plan for store contracts and deterministic test harness.
  - Validation evidence: Domain foundation final Reviewer approved with no findings; required Rust checks and targeted domain tests passed.
  - Notes: The roadmap now links to the completed domain foundation plan and active store-contracts plan.
- 2026-04-27 Compatibility direction clarified.
  - Summary: Recorded that legacy compatibility is not a v0.1 goal and that legacy implementations which do not contribute to the new architecture should be removed as replacement chunks land.
  - Validation evidence: Documentation-only roadmap update.
  - Notes: Future concrete plans should not preserve old flat API behavior unless it directly serves the new architecture.
- 2026-04-28 Vector and graph adapter foundations plan drafted.
  - Summary: Drafted the active concrete plan for Qdrant v0.1 vector payloads, natural-language embedding surfaces, RDF/Oxigraph graph mapping, and bounded graph expansion adapter behavior.
  - Validation evidence: Documentation-only roadmap update after Researcher context gathering and direct source inspection.
  - Notes: The current source tree still needs adapter-foundation implementation despite a stale completed plan artifact; the active plan includes reconciliation work.
- 2026-04-28 Adapter validation and Oxigraph scope clarified.
  - Summary: Recorded that the adapter-foundation chunk should add the Oxigraph crate and that live Qdrant/Oxigraph smoke checks should run in CI before merge or locally before PR creation.
  - Validation evidence: Documentation-only roadmap update from user guidance.
  - Notes: Concrete smoke-check commands/jobs remain to be selected during adapter implementation.

## Decision Log

- 2026-04-27 Decision: Separate roadmap from execution plans
  - Trigger / new insight: User clarified that the roadmap should be independent from concrete implementation plans so each plan can be drafted and completed separately.
  - Plan delta: Created this roadmap-only file and moved concrete Chunk 1 work into `v0-1-domain-foundation-plan.md`.
  - Tradeoffs considered: Embedding concrete tasks in the roadmap made completion tracking awkward; separate files keep roadmap direction stable while allowing per-chunk lifecycle management.
  - User approval: yes.
- 2026-04-27 Decision: Accept graph validation phasing
  - Trigger / new insight: User accepted the recommendation for graph validation phasing.
  - Plan delta: Marked trait-backed graph fakes, embedded/in-memory Oxigraph tests, and later gated service checks as the v0.1 validation direction.
  - Tradeoffs considered: Fake-only graph tests would be fast but too weak; service-only graph tests would slow early development and increase local setup friction.
  - User approval: yes.
- 2026-04-27 Decision: Remove non-contributing legacy implementations
  - Trigger / new insight: User clarified that compatibility is not a concern for v0.1.
  - Plan delta: Explicitly direct future chunks to remove legacy flat API implementations that do not contribute to the new v0.1 architecture.
  - Tradeoffs considered: Keeping compatibility shims may reduce short-term disruption but increases architectural drag and test burden during the v0.1 rewrite.
  - User approval: yes.

- 2026-04-28 Decision: Include Oxigraph and require live smoke evidence for adapter PRs
  - Trigger / new insight: User answered the open adapter-foundation planning questions.
  - Plan delta: The adapter-foundation chunk should add Oxigraph/RDF support directly, and live Qdrant/Oxigraph smoke checks should be run before PR merge in CI or locally before PR creation.
  - Tradeoffs considered: Requiring live smoke evidence increases setup burden but keeps storage adapter PRs from merging with only deterministic mapping tests.
  - User approval: yes.

## Notes
- Risks:
  - Qdrant payload hints can drift from Oxigraph graph truth unless correction/forget flows update both sides predictably in later chunks.
  - Adding Oxigraph may require dependency/config/test-environment decisions before adapter work can proceed.
  - Hub entities such as primary user/assistant can create unbounded graph expansion without strict fanout/depth tests.
  - Existing integration tests may fail without local services even when compile/unit checks pass.
- Edge cases:
  - Observations should be salient excerpts, not every turn by default.
  - Threads must remain optional, many-to-many, and confidence-scored.
  - Suppressed/deleted memories must be excluded from normal retrieval.
  - Superseded derived memories must not appear as current context unless policy explicitly includes them.
  - Raw references should be preserved without forcing raw transcript storage into Qdrant or Oxigraph.
