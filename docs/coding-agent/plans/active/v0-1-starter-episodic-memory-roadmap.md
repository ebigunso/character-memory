# Roadmap: v0.1 Starter Episodic Memory Implementation

- status: completed
- generated: 2026-04-27
- last_updated: 2026-04-30
- work_type: mixed
- roadmap_scope: full v0.1 implementation phase
- concrete_plan_policy: draft one separate implementation plan per chunk when that chunk is reached

## Goal
- Guide the full v0.1 implementation phase at roadmap granularity.
- Keep concrete execution plans separate so each chunk can be drafted, approved, executed, reviewed, and completed independently.
- Preserve the handoff's constraints: Rust library crate, chat/voice-transcript scope, Qdrant for vector candidate recall, Oxigraph for graph authority, stable cross-store IDs, deterministic tests, and no required LLM dependency in core.

## Current State Assessment
- The current public facade is `CharacterMemory` in `src/lib.rs`; public `remember`, typed `link`, `retrieve`, `correct`, and `forget` now use the graph/vector/embedder path.
- Canonical v0.1 domain objects now live under `src/api/types/domain.rs`, with store/embedder contracts and deterministic fake-store fixtures under `src/internal/repositories/**`.
- The old flat live persistence path has been removed: `MemoryRepository`, `VectorMemoryRepository`, `QdrantVectorMemoryRepository`, flat DTO re-exports, and flat integration tests are no longer compiled.
- The v0.1 vector foundation now has provider-neutral `VectorRecord` and natural-language embedding surfaces, plus Qdrant v0.1 payload mapping and `VectorCandidateStore::upsert_vector_records` support for full record payloads.
- The v0.1 graph foundation now has RDF vocabulary/mapping and an embedded/in-memory `OxigraphGraphAuthorityStore` implementing `GraphAuthorityStore` for canonical objects, typed links, query, and bounded expansion foundation.
- The current tests cover v0.1 domain model behavior, draft DTO conversion, lifecycle DTOs, retrieval DTOs, deterministic fake-store support, v0.1 vector surfaces, Qdrant payload/candidate-store mapping, RDF/Oxigraph mapping, embedded Oxigraph retrieve/expansion/lifecycle smoke, bounded graph expansion, internal remember/link/retrieve/correction/forget pipelines, public facade-level remember/link/retrieve/correction/forget behavior, lifecycle retrieval regression behavior, and live Qdrant candidate smoke when the service prerequisite is available.
- `Cargo.toml` includes the selected Qdrant and Oxigraph dependencies for the adapter foundation.

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
- Concrete plan: completed in [docs/coding-agent/plans/completed/v0-1-vector-graph-adapter-foundations-plan.md](../completed/v0-1-vector-graph-adapter-foundations-plan.md)

### Remember And Link Pipelines
- Purpose: implement caller-supplied draft inputs and persistence ordering for entities, episodes, observations, links, derived memories, and selected vector records.
- Expected outcome: `remember` and typed `link` behavior can persist v0.1 memory objects with provenance and relationship links.
- Concrete plan: completed in [docs/coding-agent/plans/completed/v0-1-remember-and-link-pipelines-plan.md](../completed/v0-1-remember-and-link-pipelines-plan.md)

### Retrieve And ContinuityContextPack
- Purpose: implement vector-to-graph retrieval, bounded expansion, lifecycle/currentness filtering, deterministic reranking, grouped context pack assembly, rationale, and optional trace.
- Expected outcome: crate-visible injected `retrieve` returns `RetrieveOutcome` with a `ContinuityContextPack`, compact rationale, and optional trace; normal retrieval excludes suppressed/deleted and superseded/non-current memories by default.
- Concrete plan: completed in [docs/coding-agent/plans/completed/v0-1-retrieve-continuity-context-pack-plan.md](../completed/v0-1-retrieve-continuity-context-pack-plan.md)

### Correction And Forget Lifecycle
- Purpose: implement non-destructive correction through supersession and lifecycle updates, plus `forget` with suppression as the default.
- Expected outcome: corrections preserve original and correction-origin provenance, old or source-affected derived memories become non-current/suppressed or superseded, source-object forget cascades to behavior-influencing derived memories, archived threads are excluded, and suppressed/non-current/superseded memories are hidden from normal retrieval unless historical policy opts in.
- Deferred review findings to carry forward: hard delete/update behavior in the legacy flat facade conflicts with the v0.1 lifecycle direction. Correction/forget work should preserve supersession/suppression as default behavior and reserve hard deletion for explicit redaction/delete semantics.
- Concrete plan: completed in [docs/coding-agent/plans/completed/v0-1-correction-forget-lifecycle-plan.md](../completed/v0-1-correction-forget-lifecycle-plan.md)

### Documentation, Migration Cleanup, And Release Validation
- Purpose: update README and roadmap docs, remove or rewrite old flat memory examples, remove non-contributing legacy implementations, and run final deterministic plus gated integration validation.
- Expected outcome: v0.1 is documented as chat-native episodic continuity memory, with Qdrant/Oxigraph responsibilities and old flat concepts clearly retired or removed.
- Concrete plan: completed in [docs/coding-agent/plans/completed/v0-1-documentation-migration-cleanup-release-validation-plan.md](../completed/v0-1-documentation-migration-cleanup-release-validation-plan.md)

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
  - Notes: Historical planning entry; adapter-foundation implementation has since landed and the plan has moved to completed.
- 2026-04-28 Adapter validation and Oxigraph scope clarified.
  - Summary: Recorded that the adapter-foundation chunk should add the Oxigraph crate and that live Qdrant/Oxigraph smoke checks should run in CI before merge or locally before PR creation.
  - Validation evidence: Documentation-only roadmap update from user guidance.
  - Notes: Historical planning entry; concrete adapter validation evidence is recorded in the completed adapter-foundation plan.
- 2026-04-28 Vector and graph adapter foundations completed; remember/link plan drafted.
  - Summary: Completed the adapter-foundation plan, moved it to completed plans, and drafted the next active remember/link pipeline plan from the landed vector/Qdrant/RDF/Oxigraph code shape.
  - Validation evidence: Final structural review approved code organization, lesson-regression checks, and ADR adherence; required Rust checks and Problems diagnostics were clean.
  - Notes: The active roadmap now reflects adapter-foundation code as landed state rather than future work.
- 2026-04-28 Remember and link pipelines completed; retrieve plan drafted.
  - Summary: Completed caller-supplied draft DTOs, internal remember/link pipelines, crate-visible injected `CharacterMemory::remember`/`link` wiring, legacy flat facade isolation, and the next active retrieve/context-pack plan.
  - Validation evidence: Required Rust checks, targeted draft/remember/link/facade tests, clippy warning gate, embedded Oxigraph smoke, local Qdrant live smoke, and final Reviewer approval passed.
  - Notes: The retrieve plan carries forward graph expansion hardening before context-pack assembly.
- 2026-04-29 Retrieve and context-pack completed; correction/forget plan drafted.
  - Summary: Completed backend-free retrieval DTOs, bounded graph expansion hardening, vector candidate prefilters, provider-neutral retrieve pipeline, crate-visible injected `CharacterMemory::retrieve`, rationale/trace behavior, and the next active correction/forget lifecycle plan.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo test --lib`, embedded Oxigraph retrieve smoke, live Qdrant candidate smoke, `cargo clippy --all-targets -- -D warnings`, and final Reviewer approval passed.
  - Notes: Correction/forget planning now starts from graph-authoritative retrieval defaults that exclude suppressed/deleted and non-current/superseded records.
- 2026-04-30 Correction/forget lifecycle completed; cleanup plan drafted.
  - Summary: Completed backend-free lifecycle DTOs, graph-authoritative correction/forget pipelines, source-provenance cascade, correction-origin provenance, injected crate-visible lifecycle facades, retrieval lifecycle regressions, and the next documentation/migration cleanup/release validation plan.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo test --lib`, embedded Oxigraph lifecycle/retrieve smoke, `cargo clippy --all-targets -- -D warnings`, local live Qdrant candidate smoke, and final Reviewer approval passed; `cargo test --lib` reported 180 passed, 0 failed, 1 ignored, and the Qdrant smoke reported 1 passed, 0 failed.
  - Notes: The cleanup plan is active as a draft only; lifecycle plan has moved to completed.
- 2026-04-30 Documentation, migration cleanup, and release validation completed.
  - Summary: Public `CharacterMemory::new` and `new_with_embedding_provider` now construct graph/vector/embedder composition with Qdrant candidate recall and embedded in-memory Oxigraph graph authority. Public `remember`, `link`, `retrieve`, `correct`, and `forget` facades are graph-authoritative; legacy flat facades, flat DTO re-exports, legacy repositories, and legacy integration tests were removed.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, embedded Oxigraph lifecycle/retrieve smoke filters, public v0.1 facade integration tests, local live Qdrant candidate smoke, and final Reviewer approval passed.
  - Notes: Persistent Oxigraph storage configuration remains future work; current graph authority is embedded/in-memory.

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

- 2026-04-28 Decision: Move from remember/link to retrieval planning
  - Trigger / new insight: Remember/link final review approved the write pipeline and injected composition behavior with required deterministic and smoke validation evidence.
  - Plan delta: Moved the remember/link concrete plan to completed plans and drafted the retrieve/context-pack concrete plan as the next active chunk.
  - Tradeoffs considered: Retrieval should harden graph expansion policy before assembling context packs, rather than relying on depth/node-count bounds alone.
  - User approval: directed by approved roadmap sequence.
- 2026-04-29 Decision: Move from retrieval to correction/forget lifecycle planning
  - Trigger / new insight: Retrieval final review approved graph-authoritative vector-to-graph context assembly, lifecycle/currentness filtering, rationale/trace behavior, and injected facade isolation.
  - Plan delta: Moved the retrieve/context-pack concrete plan to completed plans and drafted the correction/forget lifecycle concrete plan as the next active chunk.
  - Tradeoffs considered: Correction/forget should mutate graph-authoritative lifecycle state before documentation/migration cleanup retires legacy flat update/delete examples.
  - User approval: directed by approved roadmap sequence.
- 2026-04-30 Decision: Complete public migration during cleanup/release validation
  - Trigger / new insight: User clarified that this step should leave the project fully migrated to the new architecture, and new implementation should be added if needed.
  - Plan delta: Changed the cleanup plan from transitional legacy retention to public graph/vector/embedder constructor/facade migration plus legacy flat path removal.
  - Tradeoffs considered: Removing the flat facade breaks old examples and tests, but avoids preserving incompatible hard update/delete and top-k flat search semantics as v0.1 compatibility surface.
  - User approval: explicit clarification in implementation thread.

## Notes
- Risks:
  - Qdrant candidates can outlive embedded in-memory Oxigraph graph state across process restarts until persistent Oxigraph configuration lands.
  - Production persistent Oxigraph configuration remains future work; the current graph authority is embedded/in-memory.
  - Hub entities such as primary user/assistant can create unbounded graph expansion without strict fanout/depth tests.
  - Live Qdrant smoke tests require local service configuration even when deterministic compile/unit checks pass.
- Deferred review findings:
  - Add persistent Oxigraph configuration and service/persistence validation when the project moves beyond embedded in-memory graph authority.
  - Split broad shared test support into narrower fake/fixture/embedder/raw-reference modules as future retrieval/lifecycle tests make stable ownership clearer.
- Edge cases:
  - Observations should be salient excerpts, not every turn by default.
  - Threads must remain optional, many-to-many, and confidence-scored.
  - Suppressed/deleted memories must be excluded from normal retrieval.
  - Superseded derived memories must not appear as current context unless policy explicitly includes them.
  - Raw references should be preserved without forcing raw transcript storage into Qdrant or Oxigraph.
