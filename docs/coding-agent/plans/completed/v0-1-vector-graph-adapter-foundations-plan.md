# Plan: v0.1 Vector And Graph Adapter Foundations

- status: done
- generated: 2026-04-28
- last_updated: 2026-04-28
- work_type: mixed

## Goal
- Build the first live-adapter foundation on top of the v0.1 domain model, store contracts, and deterministic test harness.
- Move vector persistence toward v0.1 `VectorRecord` payloads and natural-language embedding surfaces.
- Introduce RDF/Oxigraph graph-authority mapping for canonical memory objects and links while keeping public domain types backend-free.
- Preserve the roadmap split: Qdrant is candidate/filter infrastructure; Oxigraph is authoritative for relationships, provenance, lifecycle, currentness, supersession, and bounded expansion.

## Definition of Done
- Provider-neutral vector model code can represent indexed v0.1 records for `Episode`, `Observation`, `DerivedMemory`, `MemoryThread`, and `Entity`.
- Embedding surface builders produce concise natural-language text and keep IDs, schema versions, lifecycle flags, scores, and backend metadata out of `embedding_text` by default.
- Qdrant-facing payload mapping exists for v0.1 vector records, including stable object IDs, graph URIs, schema version, lifecycle/currentness filter hints, and relevant relationship hint IDs without making Qdrant the graph authority.
- The Oxigraph crate is added, and an embedded/in-memory Oxigraph-backed `GraphAuthorityStore` foundation exists for canonical memory objects and `MemoryLink` relationships, including provenance, entity/thread links, retention state, currentness, and supersession where represented by the domain model.
- Bounded graph expansion behavior is represented and tested at the adapter-foundation level without implementing `retrieve` or `ContinuityContextPack` assembly.
- Required Rust checks, targeted deterministic adapter tests, required live-service smoke evidence, and Reviewer approval are complete, or blockers/waivers are explicitly recorded.

## Scope / Non-goals
- Scope:
  - Vector record and payload-neutral model shape under the internal vector model boundary.
  - Natural-language embedding surface builders for vector-indexed v0.1 objects.
  - Qdrant payload serialization/index-field mapping for v0.1 candidate records.
  - RDF/Oxigraph vocabulary, mapping, and embedded/in-memory Oxigraph-backed graph store foundation for canonical objects and typed links.
  - Adapter-level bounded expansion behavior or query translation that honors explicit limits.
  - Deterministic unit tests plus live Qdrant/Oxigraph smoke checks that must run in CI before merge or locally before PR creation.
- Non-goals:
  - Public `remember`, `retrieve`, `link`, `correct`, `forget`, reranking, retrieval rationale, or `ContinuityContextPack` pipelines.
  - Production raw input storage.
  - Broad removal of the old flat `CharacterMemory` facade unless directly required for compilation.
  - Treating Qdrant payload relationship hints as authoritative relationship data.
  - Service-backed tests as the only validation path.

## Context (workspace)
- Related files/areas:
  - `src/api/types/domain.rs`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/infrastructures.rs`
  - `src/internal/infrastructures/external_services.rs`
  - `src/internal/infrastructures/external_services/**`
  - `src/config/**`
  - `Cargo.toml`
  - `tests/**`
  - `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap.md`
  - `docs/coding-agent/plans/completed/v0-1-domain-foundation-plan.md`
  - `docs/coding-agent/plans/completed/v0-1-store-contracts-test-harness-plan.md`
- Existing patterns or references:
  - Canonical v0.1 domain types live in `src/api/types/domain.rs` and are re-exported through `src/api/types.rs` and `src/lib.rs`.
  - v0.1 contracts live in `src/internal/repositories/` and use canonical domain IDs/types rather than legacy flat DTOs.
  - Deterministic fakes and fixtures live in `src/internal/repositories/test_support.rs` behind `cfg(test)`.
  - At plan start, the vector contract model was the thin `VectorCandidateRecord` in `src/internal/models/vector/candidate_record.rs`, the live Qdrant adapter was legacy flat-memory-shaped, and Oxigraph/RDF support had not landed.
  - Final state: this completed plan landed provider-neutral `VectorRecord`/`VectorRecordEmbedding`, natural-language embedding surfaces, v0.1 Qdrant payload/candidate-store mapping, RDF/Oxigraph graph mapping, an embedded/in-memory `OxigraphGraphAuthorityStore`, and bounded graph expansion support.
  - The stale completed-artifact concern was a planning-time lifecycle risk; Task_6 rechecked the completed-plan location before lifecycle move and recorded the reconciliation as a no-op.
- Repo reference docs consulted:
  - `docs/roadmap/development_roadmap.md`
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/design/database/vector_payload_design.md`
  - `docs/decisions/design/ADR-D-0002-derived-memory-provenance.md`
  - `docs/decisions/design/ADR-D-0006-supersession-and-suppression.md`
  - `docs/decisions/design/ADR-D-0008-preserve-source-references.md`
  - `docs/decisions/implementation/ADR-I-0001-stable-cross-store-ids.md`
  - `docs/decisions/implementation/ADR-I-0002-natural-language-embedding-surfaces.md`
  - `docs/decisions/implementation/ADR-I-0003-qdrant-oxigraph-defaults.md`
  - `docs/decisions/implementation/ADR-I-0004-typed-memory-links.md`
  - `docs/decisions/implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md`
  - `docs/decisions/implementation/ADR-I-0006-bounded-graph-expansion.md`
  - `docs/decisions/implementation/ADR-I-0007-schema-versioning.md`

## Open Questions
- None. Oxigraph is in scope for this chunk, and live Qdrant/Oxigraph smoke checks are required before PR merge through CI or locally before PR creation.

## Resolved Decisions
- Add the Oxigraph crate in this chunk and implement an embedded/in-memory Oxigraph-backed `GraphAuthorityStore` foundation alongside RDF mapping and deterministic tests.
- Live Qdrant/Oxigraph smoke checks are required release/PR evidence: run them in CI before merge or locally before creating the PR, with prerequisites and evidence recorded.

## Review Mode
- mode: remediation
- scope: final-plan-review
- max_iterations: 2
- status: completed

## Assumptions
- A1: New adapter code should use the v0.1 `VectorCandidateStore`, `GraphAuthorityStore`, `MemoryEmbedder`, and raw-reference contracts rather than extending the old flat `VectorMemoryRepository` path.
- A2: It is acceptable for legacy flat adapters to coexist during this chunk if they are isolated and not treated as canonical v0.1 behavior.
- A3: Raw inputs remain consumer-owned; payloads/triples may preserve `raw_ref` pointers or source identifiers but must not store full raw transcripts.
- A4: Deterministic mapping/unit tests are required before relying on live Qdrant or Oxigraph smoke checks, but those smoke checks are still required before PR creation/merge evidence is complete.

## Tasks

### Task_1: Select adapter boundary and dependency strategy
- type: design
- owns:
  - `Cargo.toml`
  - `src/internal/infrastructures.rs`
  - `src/internal/infrastructures/**`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `docs/coding-agent/plans/active/v0-1-vector-graph-adapter-foundations-plan.md`
- depends_on: []
- description: |
  Inspect the current internal module layout, legacy Qdrant adapter, v0.1 store contracts, and dependency set. Record where vector payload mapping, Qdrant candidate-store code, RDF mapping, and any Oxigraph-backed implementation will live before implementation edits.
- acceptance:
  - Decision records the Oxigraph crate/version choice and embedded/in-memory store module placement.
  - Decision records module placement for vector record builders, Qdrant payload mapping, graph/RDF mapping, and bounded expansion behavior.
  - Decision keeps Qdrant/Oxigraph/RDF client types out of canonical domain types and provider-neutral repository contracts.
  - Decision identifies how to handle the stale completed adapter-foundation plan artifact before this plan is completed.
  - Decision records how required Qdrant/Oxigraph smoke checks will run before PR creation or merge, including local/CI prerequisites.
  - Decision identifies legacy flat Qdrant pieces that remain isolated versus pieces that can be replaced in this chunk.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Record adapter boundary, Oxigraph dependency strategy, live-smoke validation route, and stale-plan reconciliation in this plan's Decision Log before implementation tasks begin."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review boundary decision against store contracts, roadmap constraints, and storage ADRs."

### Task_2: Add VectorRecord and embedding surface builders
- type: impl
- owns:
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal/repositories/test_support.rs`
- depends_on: [Task_1]
- description: |
  Add the provider-neutral v0.1 vector record shape and builder functions from canonical domain objects. Keep this layer independent of Qdrant client types.
- acceptance:
  - `VectorRecord` or equivalent captures object ID/type, graph URI, vector surface, `embedding_text`, `content_text`, schema version, lifecycle/currentness hints where applicable, and relationship hint IDs needed for filtering.
  - Builders cover `Episode`, `Observation`, `DerivedMemory`, `MemoryThread`, and `Entity` using canonical domain fields.
  - `embedding_text` is concise natural language and excludes IDs, graph URIs, schema versions, retention states, booleans, numeric scores, and serialized metadata boilerplate by default.
  - Raw source material is not copied into vector records; only summaries, excerpts, derived text, names, and `raw_ref` pointers where appropriate are preserved.
  - Existing `VectorCandidateRecord` contract compatibility is either maintained through conversion or updated consistently within this task's owns.
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
    detail: "Run targeted vector record and embedding-surface unit tests added by this task."

### Task_3: Add Qdrant v0.1 payload mapping foundation
- type: impl
- owns:
  - `src/internal/infrastructures.rs`
  - `src/internal/infrastructures/external_services.rs`
  - `src/internal/infrastructures/external_services/**`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
- depends_on: [Task_2]
- description: |
  Add Qdrant-facing payload serialization, filter/index field naming, and v0.1 candidate-store adapter scaffolding for vector records. Avoid implementing retrieval ranking or graph expansion here.
- acceptance:
  - Qdrant payload mapping includes `object_id`, `graph_uri`, `object_type`, `record_type`, `schema_version`, `embedding_text`, `content_text`, lifecycle/currentness hints, time fields, salience/confidence where applicable, and episode/observation/thread/entity hint IDs.
  - Payload mapping treats relationship IDs as filter hints and documents/tests that graph relationships remain authoritative in `GraphAuthorityStore`.
  - Payload mapping excludes full raw transcripts and preserves at most `raw_ref`/source pointers if included by the vector record model.
  - Qdrant field/index helper coverage includes high-value filter fields from ADR-I-0005 and the storage contract draft.
  - Legacy `QdrantVectorMemoryRepository` remains isolated from new v0.1 adapter code or is replaced only where this task's acceptance is still fully satisfiable.
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
    detail: "Run targeted Qdrant payload mapping/index-field tests added by this task."
  - kind: command
    required: true
    owner: worker
    detail: "Run Qdrant live smoke checks locally before PR creation or provide CI job evidence before merge; record prerequisites, command/job name, and result."

### Task_4: Add RDF/Oxigraph graph mapping foundation
- type: impl
- owns:
  - `Cargo.toml`
  - `Cargo.lock`
  - `src/internal/infrastructures.rs`
  - `src/internal/infrastructures/**`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
- depends_on: [Task_1]
- description: |
  Add the Oxigraph crate, RDF/Oxigraph graph-authority mapping, and an embedded/in-memory Oxigraph-backed `GraphAuthorityStore` foundation. Prefer deterministic mapping tests first, then validate the concrete store path.
- acceptance:
  - Mapping covers canonical `Episode`, `Observation`, `Entity`, `MemoryThread`, `DerivedMemory`, and `MemoryLink` records.
  - Stable RDF resources use canonical `graph_uri` output from the domain helper and do not derive IRIs from mutable text.
  - Mapping includes object classes, schema version metadata, provenance, typed links, entity/thread associations, lifecycle/currentness, and supersession where represented by current domain fields.
  - Public domain structs remain free of RDF/Oxigraph-specific types.
  - A concrete embedded/in-memory Oxigraph-backed store implements the existing `GraphAuthorityStore` contract without changing the public domain model.
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
    detail: "Run targeted RDF/Oxigraph mapping tests added by this task."
  - kind: command
    required: true
    owner: worker
    detail: "Run embedded/in-memory Oxigraph smoke checks locally before PR creation or provide CI job evidence before merge; record prerequisites, command/job name, and result."

### Task_5: Add bounded graph expansion adapter behavior
- type: impl
- owns:
  - `src/internal/infrastructures.rs`
  - `src/internal/infrastructures/**`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/repositories/test_support.rs`
- depends_on: [Task_4]
- description: |
  Implement or prepare adapter-level bounded expansion query behavior that honors the existing `GraphExpansionQuery` limits. Keep this at graph-store expansion level, not retrieval pack assembly.
- acceptance:
  - Expansion behavior preserves `max_depth`, `max_nodes`, and allowed object-type constraints from `GraphExpansionQuery`.
  - Tests cover hub-entity or high-fanout fixture behavior to prove expansion remains bounded and deterministic.
  - Expansion returns canonical `MemoryObject` and `MemoryLink` values through `GraphExpansion` without introducing retrieval-specific ranking or sectioning.
  - Failure or unsupported-query behavior is explicit and maps into `CustomError` rather than hidden panics.
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
    detail: "Run targeted bounded graph expansion adapter tests added by this task."

### Task_6: Final adapter review and next-plan draft
- type: review
- owns:
  - `docs/coding-agent/plans/active/**`
  - `docs/coding-agent/plans/completed/**`
- depends_on: [Task_3, Task_5]
- description: |
  Review the adapter-foundation diff and validation evidence using the review-remediation-loop structured appendix. If approved, reconcile plan lifecycle artifacts and draft the next concrete remember/link pipeline plan from the landed adapter shape. If final review reports kept MAJOR/CRITICAL findings, run up to two remediation iterations before completion.
- acceptance:
  - Reviewer approves the adapter-foundation implementation or all blocking findings are resolved/waived.
  - Final Reviewer output includes the review-remediation-loop structured appendix.
  - Any kept MAJOR/CRITICAL findings are triaged and remediated through bounded Worker follow-up tasks, or the plan is left blocked with evidence.
  - Required validation evidence from Tasks 1-5 is present.
  - This plan's Progress Log and Decision Log are updated with outcomes.
  - The stale completed adapter-foundation plan artifact is reconciled so plan lifecycle state is not misleading.
  - A separate active plan for remember/link pipelines is drafted from the landed code shape.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Review adapter-foundation implementation against storage contracts, ADRs, validation evidence, and stale-plan reconciliation; include review-remediation-loop structured appendix."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Confirm plan completion criteria, required evidence completeness, lifecycle artifact reconciliation, and next-plan independence."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Confirm required Qdrant/Oxigraph smoke-check evidence exists from local pre-PR execution or CI before merge; otherwise keep the plan blocked or record an explicit waiver."

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (design gate): [Task_1]
- Wave 2 (vector foundation): [Task_2]
- Wave 3 (Qdrant mapping): [Task_3]
- Wave 4 (RDF/Oxigraph mapping): [Task_4]
- Wave 5 (graph expansion): [Task_5]
- Wave 6 (review and next-plan draft): [Task_6]

## E2E / Visual Validation Spec

- Not applicable. This is Rust storage adapter infrastructure with no UI/user-flow surface.

## Rollback / Safety
- Keep new adapter modules behind internal boundaries until public pipeline behavior needs them.
- Keep vendor-specific Qdrant/Oxigraph/RDF types out of canonical domain modules and provider-neutral repository contracts.
- Treat Qdrant payload relationship fields as denormalized hints only; graph mappings remain authoritative for relationships and provenance.
- Keep raw input storage consumer-owned and avoid copying full raw transcripts into vector payloads or graph triples.
- Keep live-service checks prerequisite-gated and documented, but required before PR creation/merge evidence is complete; deterministic mapping/unit tests remain the fast required validation path before service smoke checks.
- Do not remove old flat API surfaces unless the replacement surface is in place and all required checks remain passable.

## Quality Routing Note
- Routing level: L2
- In-scope docs: Rust adapter architecture, internal storage boundaries, deterministic adapter/mapping tests, schema/persistence boundaries, data-integrity risks.
- Out-of-scope docs: UI/E2E, frontend/browser checks, auth/security, public pipeline behavior, production raw storage.
- Top risks: data-integrity, external dependency/integration, contract/schema compatibility, migration drift from legacy flat adapters, Qdrant/graph consistency drift.
- Risk profile: medium-high because this chunk introduces backend adapter foundations and may add Oxigraph/RDF dependencies, but behavior is bounded to mappings/adapters and deterministic tests.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted vector/Qdrant/RDF/Oxigraph/graph-expansion tests, Qdrant/Oxigraph smoke-check evidence before PR creation or merge, Reviewer gate.
- Optional recommended checks: none for this chunk.
- At Risk items: Oxigraph dependency/version choice, stale completed plan artifact reconciliation, live-service prerequisites, and avoiding duplicated graph authority in Qdrant payloads.

## Progress Log

- 2026-04-28 Plan drafted.
  - Summary: Created a fresh active concrete plan for vector and graph adapter foundations from the current source tree, completed domain foundation, completed store contracts, roadmap, design docs, and ADRs.
  - Validation evidence: Researcher report plus direct source inspection confirmed the current tree has v0.1 domain/store contracts but does not yet contain the richer `VectorRecord`, v0.1 Qdrant candidate adapter, RDF/Oxigraph mapping, or Oxigraph dependency.
  - Notes: A stale completed adapter-foundation plan artifact exists and must be reconciled during Task_1/Task_6 before this work is considered complete.
- 2026-04-28 Open questions resolved.
  - Summary: User directed that the Oxigraph crate and embedded/in-memory Oxigraph-backed graph store foundation should be included in this chunk, and that live Qdrant/Oxigraph smoke checks should run in CI before merge or locally before PR creation.
  - Validation evidence: Documentation-only plan update from user guidance.
  - Notes: The plan now treats service smoke evidence as required pre-PR/pre-merge evidence rather than optional validation.
- 2026-04-28 Task_1 completed: selected adapter boundary and dependency strategy.
  - Summary: Recorded the v0.1 adapter module placement, Oxigraph dependency/version strategy, Qdrant/Oxigraph smoke-check route, stale completed-plan reconciliation, and legacy flat-Qdrant isolation rules before implementation edits.
  - Validation evidence: Manual review of this plan, the v0.1 roadmap, completed domain/store-contract plans, storage ADRs, `Cargo.toml`, `src/api/types/domain.rs`, `src/internal/models/vector/**`, `src/internal/repositories/**`, `src/internal/infrastructures/**`, `.github/workflows/pr_validation.yaml`, `docker-compose.qdrant.yml`, and current plan artifact search results.
  - Notes: No Rust implementation files were changed and no cargo commands were required for this docs/design gate.
- 2026-04-28 Task_1 reviewer gate approved.
  - Summary: Reviewer approved the recorded adapter boundary and dependency strategy with no findings.
  - Validation evidence: Reviewer confirmed Task_1 acceptance coverage, boundary consistency, validation route, and implementation risk are acceptable.
  - Notes: Proceed to Task_2. Final review remediation mode is active at user request.
- 2026-04-28 Task_2 complete: added provider-neutral vector records and embedding surfaces.
  - Summary: Added `VectorRecord`, `VectorRelationshipHints`, conversions to `EmbeddingInput`/`VectorCandidateRecord`, and natural-language surface builders for `Episode`, `Observation`, `DerivedMemory`, `MemoryThread`, and `Entity`. The generic `MemoryObject` helper skips `MemoryLink` so links remain graph-authority objects rather than vector-indexed records.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and `cargo test internal::models::vector -- --nocapture` passed. Targeted vector tests reported 20 passed, 0 failed.
  - Notes: No Qdrant, RDF, or Oxigraph types were introduced into provider-neutral vector models or repository contracts.
- 2026-04-28 Task_3 complete: added Qdrant v0.1 payload mapping and candidate-store foundation.
  - Summary: Added v0.1 Qdrant payload mapping/index helpers, a Qdrant candidate-store scaffold, and provider-neutral `VectorRecordEmbedding` upsert support so the store contract carries full `VectorRecord` payloads plus embeddings instead of lossy candidate-only records.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted Qdrant payload/index and candidate-store tests, fake-store contract tests, and local Qdrant live smoke passed. Local smoke prerequisites were Docker 29.4.0, `docker compose -f docker-compose.qdrant.yml up -d`, and `QDRANT_CONNECTION_STRING=http://127.0.0.1:6334`.
  - Notes: Initial Task_3 review found that the Qdrant store implemented `VectorCandidateStore` while rejecting the required upsert method. Follow-up changed the provider-neutral trait to `upsert_vector_records`, updated fake support, and re-ran validation. Legacy flat `QdrantVectorMemoryRepository` remains isolated.
- 2026-04-28 Task_3 reviewer follow-up approved.
  - Summary: Reviewer approved the contract-risk follow-up with no remaining findings.
  - Validation evidence: Reviewer confirmed the previous substitutability risk is resolved, Qdrant upsert uses full v0.1 payload mapping, fake behavior remains deterministic, and backend-specific types remain confined to infrastructure modules.
  - Notes: Proceed to Task_4.
- 2026-04-28 Task_4 complete: added RDF/Oxigraph mapping and embedded graph store foundation.
  - Summary: Added `oxigraph = { version = "0.4.11", default-features = false }`, internal RDF vocabulary/mapping, and an embedded/in-memory `OxigraphGraphAuthorityStore` implementing `GraphAuthorityStore` for canonical objects, links, query, and bounded expansion foundation.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted `cargo test internal::infrastructures::graph -- --nocapture`, and embedded in-memory Oxigraph smoke passed. Orchestrator re-ran the targeted graph test and observed 8 passed, 0 failed.
  - Notes: Initial Task_4 review found repeated upserts left stale RDF quads. Follow-up now tracks inserted quads by owning graph URI and replaces prior object/link materialization, including direct relation triples owned by links. `Cargo.lock` was added to Task_4 owns because adding a Rust dependency necessarily updates the lockfile.
- 2026-04-28 Task_4 reviewer follow-up approved.
  - Summary: Reviewer approved the Oxigraph upsert remediation with no remaining findings.
  - Validation evidence: Reviewer confirmed stale object/link RDF facts are replaced, provider types remain confined to infrastructure modules, and targeted regressions cover object, link, direct relation, and embedded smoke behavior.
  - Notes: Proceed to Task_5. Residual non-blocking design risk: duplicate identical direct relation triples owned by multiple distinct links may need revisiting when link uniqueness/upsert semantics are formalized.
- 2026-04-28 Task_5 complete: added shared bounded graph expansion semantics.
  - Summary: Added a provider-neutral bounded expansion helper used by both fake graph support and the embedded Oxigraph adapter. Expansion preserves `max_depth`, `max_nodes`, allowed object-type constraints, deterministic traversal, canonical `MemoryObject`/`MemoryLink` return values, and explicit `CustomError` behavior for unsupported or missing roots.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo test internal::repositories::test_support -- --nocapture`, and `cargo test internal::infrastructures::graph -- --nocapture` passed. Targeted graph tests reported 11 passed, 0 failed after Task_5.
  - Notes: No retrieval ranking, context-pack assembly, or public retrieve behavior was introduced.
- 2026-04-28 Task_5 review result.
  - Summary: Reviewer found the Task_5 code acceptable and requested only that Task_5 validation evidence be recorded before Task_6.
  - Validation evidence: Reviewer confirmed the helper is provider-neutral, expansion constraints are preserved, high-fanout deterministic behavior is covered, and diagnostics were clean.
  - Notes: Evidence recorded; proceed to Task_6 final review with remediation mode active.
- 2026-04-28 Task_6 final review approved with remediation mode.
  - Summary: Final Reviewer approved the full adapter-foundation diff with no findings. Review-remediation-loop structured appendix reported `structured_findings: []`, `status: APPROVED`, and `highest_severity: NONE`; no remediation iteration was required.
  - Validation evidence: Final Orchestrator gates passed: `cargo fmt --check`, `cargo check`, and `cargo test --no-run`. Recorded targeted evidence includes vector model tests, Qdrant payload/candidate/fake-store tests, local Docker Qdrant smoke, RDF/Oxigraph tests, embedded Oxigraph smoke, and bounded graph expansion tests.
  - Notes: Qdrant smoke service was stopped with `docker compose -f docker-compose.qdrant.yml down` after successful smoke evidence.
- 2026-04-28 Task_6 lifecycle closeout complete.
  - Summary: Rechecked stale completed adapter-foundation artifact path and found no existing completed artifact. Drafted the next active plan for v0.1 remember and typed link pipelines, and updated the roadmap link.
  - Validation evidence: `docs/coding-agent/plans/**/v0-1-vector-graph-adapter-foundations-plan.md` search returned only this active plan before lifecycle move; completed plans listing did not contain a stale adapter-foundation plan.
  - Notes: This plan is ready to move to completed.

## Decision Log

- 2026-04-28 Decision: Draft a fresh active adapter-foundation plan despite stale completed artifact
  - Trigger / new insight: Current source inspection found no `VectorRecord` model, no v0.1 Qdrant candidate-store adapter, no RDF/Oxigraph mapping module, and no Oxigraph dependency, while a completed adapter-foundation plan artifact already exists.
  - Plan delta: Added a new active plan for the actual remaining adapter-foundation work and included explicit stale-plan reconciliation in Task_1 and Task_6.
  - Tradeoffs considered: Reusing the completed artifact as-is would hide incomplete implementation state; deleting or moving it immediately would be a lifecycle mutation beyond plan drafting. A fresh active plan preserves current execution clarity and leaves reconciliation to the approved implementation wave.
  - User approval: yes.
- 2026-04-28 Decision: Include Oxigraph crate and require live smoke evidence
  - Trigger / new insight: User answered Q1 and Q2 for the adapter-foundation plan.
  - Plan delta: Oxigraph is now explicitly in scope for this chunk, including an embedded/in-memory Oxigraph-backed `GraphAuthorityStore` foundation. Qdrant/Oxigraph smoke checks are required before PR creation or merge, using local execution or CI evidence.
  - Tradeoffs considered: Adding Oxigraph now increases dependency/build/test surface, but prevents the graph-authority adapter from remaining only a mapping abstraction. Requiring smoke evidence adds setup cost, but better matches the intended merge bar for live storage backends.
  - User approval: yes.
- 2026-04-28 Decision: Use review remediation loop for final review
  - Trigger / new insight: User explicitly requested the review remediation loop for final review.
  - Plan delta: Added Review Mode with remediation active, max two iterations, and updated Task_6 to require the structured appendix and bounded remediation of kept MAJOR/CRITICAL findings.
  - Tradeoffs considered: The remediation loop adds final-gate overhead, but this chunk is code-focused and storage-boundary-sensitive.
  - User approval: yes.
- 2026-04-28 Decision: Keep v0.1 adapters internal, provider-neutral at contracts, and backend-specific only at infrastructure edges
  - Trigger / new insight: Task_1 source review confirmed the current v0.1 store contracts already use canonical `MemoryId`, `ObjectType`, `MemoryObject`, `MemoryLink`, `VectorCandidateRecord`, and `GraphExpansionQuery` shapes, while the live Qdrant adapter remains legacy flat-memory-shaped and there is no current Oxigraph/RDF module or dependency.
  - Plan delta: Task_2 should add provider-neutral vector record and natural-language surface builders under `src/internal/models/vector/` with direct module files such as `record.rs` and `embedding_surface.rs`, re-exported through `src/internal/models/vector.rs`. Task_3 should place Qdrant v0.1 payload mapping, index-field helpers, and the `VectorCandidateStore` adapter under `src/internal/infrastructures/external_services/` in new v0.1-named files such as `qdrant_payload.rs` and `qdrant_vector_candidate_store.rs`, re-exported through `src/internal/infrastructures/external_services.rs`. Task_4 should add a new graph infrastructure boundary from `src/internal/infrastructures.rs` such as `pub(crate) mod graph;`, with RDF vocabulary/mapping in `src/internal/infrastructures/graph/rdf_mapping.rs` and `src/internal/infrastructures/graph/vocabulary.rs`, and the embedded/in-memory Oxigraph implementation in `src/internal/infrastructures/graph/oxigraph_authority_store.rs`. Task_5 should keep bounded graph expansion translation/limits at the graph adapter layer, either in `src/internal/infrastructures/graph/bounded_expansion.rs` or as a narrowly factored helper used by `oxigraph_authority_store.rs`, while the provider-neutral query/result contracts remain in `src/internal/repositories/graph_authority_store.rs`.
  - Dependency strategy: Add `oxigraph = { version = "0.4.11", default-features = false }` in `Cargo.toml` during Task_4 for the embedded/in-memory graph store path. Do not add separate RDF crates unless the implementation proves Oxigraph's exported model/query APIs are insufficient; if the selected crate version is unavailable or incompatible with the pinned Rust toolchain, Task_4 must update this Decision Log with the resolved version before continuing adapter implementation.
  - Boundary rule: Canonical domain types in `src/api/types/domain.rs` and provider-neutral repository contracts in `src/internal/repositories/**` must not expose `qdrant-client`, `oxigraph`, RDF term, SPARQL query, or backend payload types. Those types are allowed only inside infrastructure mapping/adapter modules and their module-local tests. Cross-store joins continue to use stable domain IDs and `graph_uri(...)` strings.
  - Live smoke route: Deterministic unit/mapping tests remain the fast required checks before service smoke checks. Before PR creation or merge, Task_3/Task_4 must add and run narrowly named smoke tests for the v0.1 Qdrant candidate adapter and embedded/in-memory Oxigraph graph store, then record command/job evidence. The local route is: start Qdrant with `docker compose -f docker-compose.qdrant.yml up -d`, provide the existing live-test environment required by settings (`QDRANT_CONNECTION_STRING`, `EMBEDDING_MODEL`, and any current settings preflight values), and run the targeted smoke commands that Task_3/Task_4 introduce. The CI route is: extend or use the existing same-repository, non-Dependabot `Live service integration tests` job so it starts Qdrant, runs the targeted Qdrant smoke, runs the embedded/in-memory Oxigraph smoke in process, and updates the current preflight message/variables if Oxigraph no longer needs a network endpoint.
  - Stale-plan reconciliation: Task_6 must re-check `docs/coding-agent/plans/completed/v0-1-vector-graph-adapter-foundations-plan.md` before lifecycle completion. If a stale completed artifact exists, do not leave two authoritative completed adapter-foundation plans; replace it with this plan's final completed artifact or explicitly mark/remove the stale artifact in the same lifecycle update. If the artifact is absent, as Task_1 filesystem search found, record the reconciliation as a no-op and update any remaining stale references before completion. Final outcome: Task_6 found no stale completed artifact and moved this plan to completed.
  - Legacy Qdrant isolation: Keep `src/internal/infrastructures/external_services/qdrant_vector_memory_repository.rs`, `src/internal/repositories/vector_memory_repository.rs`, `src/internal/repositories/memory_repository.rs`, `src/internal/models/memory/**`, and `src/internal/models/vector/vector_metadata.rs` isolated as the old flat API path unless a later task replaces them while preserving required checks. New v0.1 code should implement `VectorCandidateStore` and consume the new vector record/payload mapping instead of extending `VectorMemoryRepository`; reusable Qdrant client setup, collection/index creation patterns, and live-test readiness ideas may be copied/adapted, but legacy flat payload structs and flat `MemoryType`/`MemoryEntry` semantics must not become part of the v0.1 adapter surface.
  - Tradeoffs considered: A separate graph infrastructure module makes the Oxigraph/RDF authority boundary clearer than adding it under `external_services`, while keeping Qdrant under `external_services` matches the existing live-client organization. Pinning Oxigraph with default features disabled keeps the first graph store embedded/in-memory and avoids committing to persistent RocksDB/service configuration in this chunk. Leaving legacy flat Qdrant code in place minimizes unrelated churn, but requires explicit naming so future tasks do not confuse it with canonical v0.1 storage behavior.
  - User approval: derived from the approved plan plus user direction that Oxigraph belongs in this chunk and live smoke evidence is required before PR creation or merge.

## Notes
- Risks:
  - Qdrant payload hints can drift from graph truth unless later remember/link/correction writes update both sides predictably.
  - Oxigraph dependency choice can affect build time, test strategy, and local setup friction.
  - Hub entities and active threads can create unbounded graph expansion without strict adapter-level bounds.
  - Legacy flat Qdrant code can confuse future implementers if new v0.1 adapter modules are not clearly named and isolated.
- Edge cases:
  - Suppressed/deleted and non-current/superseded state must be represented for later filtering even though retrieval filtering is not implemented here.
  - `DerivedMemory` vector and graph mapping must preserve provenance through episode/observation IDs and typed links.
  - Natural-language embedding surfaces should include meaningful recall cues such as entity names or thread titles only when they read like natural context, not metadata templates.
  - `raw_ref` pointers should survive mapping where relevant without storing full raw source material in Qdrant or Oxigraph.
