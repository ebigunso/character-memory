# Plan: v0.1 Correction And Forget Lifecycle

- status: done
- generated: 2026-04-29
- last_updated: 2026-04-30
- work_type: mixed

## Goal
- Implement non-destructive correction and forget lifecycle behavior on top of the landed graph-authoritative remember/link/retrieve path.
- Preserve original source provenance, correction-origin provenance, and source references while marking replaced derived memories non-current, suppressing forgotten source/derived memories by default, and preventing source-provenanced derived memories from continuing to influence retrieval after their source is forgotten.
- Keep this chunk focused on lifecycle mutation and retrieval visibility; do not implement production raw storage, reflection scheduling, belief/evidence ontology, or broad legacy facade removal beyond necessary isolation.

## Definition of Done
- Backend-free correction and forget request/outcome DTOs exist without Qdrant/Oxigraph/RDF types.
- Correction creates replacement canonical derived-memory objects/links, records supersession through both `DerivedMemory.supersedes` and a typed `RelationType::Supersedes` link, marks replaced or source-affected derived memories non-current, and preserves both original source provenance and correction-origin provenance/source references.
- Supersession support is explicitly scoped by object type: this chunk supports derived-memory correction where `new DerivedMemory supersedes old DerivedMemory`; episode/observation correction may create correction derived memories and supersede affected derived memories discovered through provenance, without rewriting historical source objects.
- Forget defaults to soft lifecycle mutation rather than hard deletion. Derived memories, episodes, and observations can be suppressed; episode/observation suppression cascades by default to behavior-influencing derived memories provenanced to the forgotten source. Memory threads can be archived. Physical redaction/delete behavior is deferred from this chunk; the existing `RetentionState::Deleted` remains a soft lifecycle state that retrieval can filter.
- Graph authority remains the source of truth for supersession, suppression, lifecycle, provenance, and currentness; Qdrant payloads/candidates are updated or removed only to keep candidate recall from surfacing stale hints.
- Normal retrieve behavior excludes suppressed/deleted and non-current/superseded memories after correction/forget. Compact rationale summarizes omissions; detailed historical/included-by-policy inspection requires trace-enabled retrieval.
- Retrieval correctness still holds if stale vector candidates remain after graph mutation or vector maintenance partially fails; graph authority must exclude stale lifecycle state from final normal retrieval.
- Required Rust checks, deterministic lifecycle tests, `cargo test --lib`, embedded Oxigraph smoke, gated Qdrant candidate smoke, clippy, and Reviewer approval are complete, or blockers/required-check waivers are explicitly recorded.

## Current Blockers
- None. Prior Task_6 closeout blockers are resolved: clippy passes after `ForgetLifecyclePolicy` derived `Default`, and the local Qdrant live smoke passed with a prepared service environment.
- Final Reviewer approved and Orchestrator closeout confirmed evidence completeness before moving this plan to completed.

## Scope / Non-goals
- Scope:
  - Backend-free correction/forget request, policy, and outcome DTOs.
  - Correction-origin provenance fields that can identify the episode/observation/source reference explaining why a correction was made, without requiring production raw transcript storage.
  - Lifecycle target object-type boundary selected before implementation: derived-memory correction/supersession and derived-memory forget/suppression are in scope; episode/observation correction can supersede affected derived memories through provenance rather than rewriting the source object; episode/observation forget suppresses the source object and cascades to dependent behavior-influencing derived memories by default; `MemoryThread` forget archives the thread through `ThreadStatus::Archived`; `Entity` and `MemoryLink` do not currently have retention/currentness fields and are deferred.
  - Provider-neutral graph lifecycle mutation support needed for derived-memory currentness, typed supersession, episode/observation/derived-memory suppression, and memory-thread archival, without adding physical hard-delete/redaction graph contracts.
  - Vector candidate maintenance needed after correction/forget so Qdrant remains a candidate hint store rather than authority; the selected strategy is to delete affected old/suppressed/archived candidates and upsert replacement candidates after graph success.
  - Internal correction and forget pipelines over `GraphAuthorityStore`, `VectorCandidateStore`, and `MemoryEmbedder` where replacement objects need indexing.
  - Crate-visible injected `CharacterMemory::correct` / `CharacterMemory::forget` facade shape.
  - Deterministic fake-store tests plus embedded Oxigraph and gated Qdrant smoke evidence.
- Non-goals:
  - Full belief/evidence ontology or contradiction-resolution system.
  - Reflection scheduling, background summarization, external LLM extraction, or LLM-based correction generation.
  - Production raw transcript storage.
  - Public production constructor rewiring unless narrowly required to keep lifecycle behavior testable.
  - Generic cross-object supersession beyond source-provenanced derived-memory correction, or lifecycle mutation for object types without existing lifecycle/currentness fields.
  - Physical hard deletion/redaction contracts or policy paths; these require separate raw/source, graph, vector, and privacy semantics.
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
  - `RetrieveOutcome` and rationale/trace behavior now expose graph-authoritative omission and supersession evidence; compact rationale contains aggregate omission counts, while trace carries detailed candidate, lifecycle, relation, and section records.
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
- None.

## Required-Check Waivers

- None active.

## Resolved Required-Check Evidence

- 2026-04-30 Deterministic closeout validation passed after clippy remediation.
  - Check: `cargo fmt --check && cargo check && cargo test --no-run && cargo test --lib && cargo clippy --all-targets -- -D warnings`
  - Local result: passed; `cargo test --lib` reported 180 passed, 0 failed, 1 ignored, and final clippy finished the dev profile successfully.
  - Notes: The prior `clippy::derivable_impls` failure on `ForgetLifecyclePolicy` was resolved by deriving `Default`; a final policy-toggle remediation added two pipeline tests before the passing rerun.
- 2026-04-30 Live Qdrant candidate maintenance smoke passed locally.
  - Check: `docker compose -f docker-compose.qdrant.yml up -d && QDRANT_CONNECTION_STRING=http://localhost:6334 cargo test --lib qdrant_candidate_store_live_smoke_upserts_filters_searches_and_deletes -- --ignored`
  - Local result: passed, 1 passed, 0 failed.
  - Notes: This closes the prior missing-`QDRANT_CONNECTION_STRING` waiver for Task_6 lifecycle closeout evidence.

## Review Placeholders

- Reviewer-owned: approve no production raw-storage, reflection scheduling, external LLM extraction, physical hard-delete/redaction, or belief/evidence ontology scope creep was introduced by the lifecycle implementation.
- Reviewer-owned: confirm normal retrieval lifecycle behavior remains inspectable through compact rationale aggregates and trace-enabled lifecycle/candidate omission details.
- Orchestrator-owned: confirm Task_6 blocker resolution, waiver closure, evidence completeness, and independence of the next Documentation, Migration Cleanup, And Release Validation plan before moving this plan to completed.

## Resolved Decisions
- User approval for the revised lifecycle plan is recorded as of 2026-04-30. Task_1 selects the implementation boundary before Rust edits; later implementation tasks must treat these decisions as their dependency contract.
- Correction/forget facades remain crate-visible injected surfaces in this chunk, matching the current `remember`/`link`/`retrieve` boundary. Public lifecycle API stabilization belongs to Documentation, Migration Cleanup, And Release Validation after production construction is graph-authoritative.
- Supported lifecycle mutation targets are bounded to existing lifecycle fields: correction supersedes `DerivedMemory` records directly; episode/observation correction does not rewrite historical source objects but can create correction derived memories and supersede or mark non-current affected derived memories discovered through provenance; forget suppresses `DerivedMemory`, `Episode`, and `Observation` records; forget archives `MemoryThread` records through `ThreadStatus::Archived`; entity and link lifecycle are deferred.
- Source-target cascade rules are defaulted now: correcting an episode or observation discovers behavior-influencing derived memories provenanced to that source and supersedes or marks those derived memories non-current through replacement correction memories; forgetting an episode or observation suppresses the source and cascade-suppresses behavior-influencing derived memories whose provenance references the forgotten source, so forgotten source material does not continue influencing normal retrieval.
- Correction requests must carry or reference correction-origin provenance when available, such as a correction episode ID, correction observation ID, and/or raw/source reference. Replacement correction memories must preserve both the superseded memory's original source chain and the distinct correction event/source that explains the revision.
- Derived-memory supersession direction is `new DerivedMemory supersedes old DerivedMemory`. Persistence must include both the replacement object's `DerivedMemory.supersedes` field and a typed `MemoryLink` with `RelationType::Supersedes` from the new derived memory to the old derived memory; retrieval evidence should present old or source-affected derived memories as `superseded_by` the replacement where relation evidence exists.
- Default lifecycle behavior is non-destructive supersession for correction and suppression/archive for forget. Physical hard deletion/redaction is deferred; this chunk may recognize the existing `RetentionState::Deleted` as a retrieval-filtered soft lifecycle state, but it must not add physical deletion/redaction behavior.
- Graph authority is the source of truth for lifecycle mutation semantics: currentness, retention state, thread archival, provenance, and typed supersession links are graph-authoritative. After graph success, vector maintenance deletes candidates for old/superseded/suppressed/archived object IDs and upserts replacement candidates; vector failure after graph success returns explicit partial-success evidence and must not roll back graph truth or compromise final retrieval correctness.
- Public DTOs stay backend-free; lifecycle pipelines depend on provider-neutral `GraphAuthorityStore`, `VectorCandidateStore`, and `MemoryEmbedder` contracts; Qdrant/Oxigraph/RDF remain infrastructure details.
- Compact rationale should report aggregate lifecycle omissions for corrected, superseded, suppressed, and historical opt-in behavior without promising full history in always-on output. Trace-enabled retrieval remains the detailed inspection path for per-object correction, supersession, suppression, vector-candidate, and historical-policy evidence.

## Assumptions
- A1: Correction is non-destructive by default: create replacement memory, preserve old provenance, and mark old or source-affected derived memories non-current/superseded rather than overwriting them.
- A2: Forget means suppression by default; hard deletion/redaction requires a later dedicated policy and validation plan.
- A3: Retrieval defaults from the completed retrieve plan remain the acceptance oracle for lifecycle visibility.
- A4: Vector candidate maintenance should reduce stale recall noise and cost but must not become lifecycle authority or a correctness requirement.

## Tasks

### Task_1: Select lifecycle API and policy boundary
- type: design
- owns:
  - `docs/coding-agent/plans/active/v0-1-correction-forget-lifecycle-plan.md`
- depends_on: []
- description: |
  Decide correction/forget request names, facade visibility, suppression versus hard-delete policy, source-object cascade boundaries, thread archival behavior, vector maintenance strategy, and failure semantics before implementation edits.
- acceptance:
  - Decision records whether correction/forget facades are crate-visible injected surfaces or public API in this chunk.
  - Decision records supported lifecycle target object types and defers unsupported object types explicitly.
  - Decision records source-target cascade rules for correcting or forgetting episodes/observations, including default cascade suppression for behavior-influencing derived memories provenanced to forgotten sources.
  - Decision records how correction-origin provenance is represented separately from the superseded memory's original source provenance.
  - Decision records that derived-memory supersession direction is `new DerivedMemory supersedes old DerivedMemory`, that persistence includes both `DerivedMemory.supersedes` and a typed `RelationType::Supersedes` link, and that old or source-affected derived memories appear as `superseded_by` in retrieval evidence.
  - Decision records default suppression and supersession behavior, and records that physical hard deletion/redaction is deferred from this chunk.
  - Decision records graph-authority lifecycle mutation semantics and Qdrant candidate maintenance strategy, including delete-old/delete-suppressed/delete-archived/upsert-replacement vector hygiene and graph-authoritative correctness when vector maintenance fails.
  - Decision records dependency direction: public DTOs stay backend-free; pipelines depend on provider-neutral graph/vector/embedder contracts; Qdrant/Oxigraph/RDF remain infrastructure details.
  - Decision records how compact rationale and optional trace should expose corrected, superseded, suppressed, and historical opt-in behavior without overpromising detailed history in always-on rationale.
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
  - DTOs can express typed lifecycle targets for `DerivedMemory`, `Episode`, `Observation`, and `MemoryThread`; replacement derived-memory data; superseded derived-memory IDs; source-object correction targets; correction-origin episode/observation/source references; rationale; lifecycle policy; cascade policy; suppression/archive policy; and optional trace flags where selected.
  - DTOs preserve `supersedes` direction and retrieval-facing `superseded_by` evidence semantics without exposing backend graph/vector details.
  - DTOs preserve original source/raw references and correction-origin source/raw references without introducing raw transcript storage.
  - Outcomes report graph-mutated object IDs, link IDs, vector-maintained object IDs, and partial vector maintenance failures where graph mutation succeeded.
  - Pure DTO tests cover serialization, defaults, validation, typed target boundaries, non-destructive correction semantics, source-target cascade defaults, original-source versus correction-origin provenance, suppression/archive defaults, and hard-delete/redaction deferral.
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
  Add provider-neutral graph authority behavior needed to mark supported lifecycle/currentness fields, record derived-memory supersession, support source-object cascade discovery, and support suppression/archive forget semantics.
- acceptance:
  - Graph authority can persist derived-memory correction supersession links, episode/observation/derived-memory retention-state updates, memory-thread archive updates, and supported lifecycle/currentness updates without hard-deleting by default.
  - Graph lifecycle mutation may reuse whole-object `upsert_objects` and `upsert_links` where sufficient; new special mutation methods are optional and must be justified by adapter needs.
  - Supersession persistence includes both replacement-object `supersedes` data and fake/Oxigraph parity for typed `RelationType::Supersedes` link evidence.
  - Fake graph and embedded Oxigraph behavior can discover derived memories affected by an episode or observation through provenance fields and/or typed provenance links under bounded query rules.
  - Fake graph and embedded Oxigraph behavior keep old and replacement objects inspectable under explicit historical policy.
  - Normal retrieval after lifecycle mutation excludes suppressed/deleted and non-current/superseded records by default.
  - Tests cover supersession directionality, episode/observation/derived-memory suppression, memory-thread archival, source-provenance cascade discovery, optional historical inclusion, and no accidental traversal through suppressed/non-current records.
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
  - Correction validates all inputs before graph mutation, writes replacement derived-memory objects/links, records typed supersession, marks directly replaced or source-affected derived memories non-current, and indexes replacement vector records when selected.
  - Correction targeting an episode or observation discovers affected derived memories through provenance, creates a correction derived memory with both original-source and correction-origin provenance, and supersedes or marks non-current the affected derived memories without rewriting the historical episode/observation.
  - Forget applies suppression to derived-memory, episode, and observation targets by default; episode/observation forget cascade-suppresses behavior-influencing derived memories provenanced to the forgotten source; memory-thread forget archives the thread.
  - Graph mutation failure prevents vector maintenance; vector maintenance failure after graph success returns explicit partial-success outcome.
  - After graph success, correction deletes vector candidates for old/superseded object IDs and upserts replacement candidates; forget deletes candidates for suppressed/archived object IDs; vector cleanup is hygiene and never lifecycle authority.
  - Tests cover ordering, graph-authoritative lifecycle changes, correction-origin provenance preservation, vector maintenance, partial failure, and retrieval-after-mutation defaults.
  - Tests prove retrieval still excludes stale lifecycle state through graph authority when vector maintenance fails or stale candidates remain, including source-object stale candidates and dependent derived-memory stale candidates.
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
  - Facade tests prove deprecated `update_memory` / `delete_memory` do not participate in lifecycle correction/forget behavior.
  - Deterministic facade tests cover derived-memory correction, episode/observation correction of affected derived memories, correction-origin provenance inspectability, derived-memory forget/suppression, episode/observation forget cascade, memory-thread archive behavior, retrieval exclusion, historical opt-in visibility if selected, and legacy isolation.
  - README examples are updated only if the public runnable surface is production-usable enough to document without misleading users.
  - If correction/forget remains crate-visible like `remember`/`link`/`retrieve`, README updates avoid implying a public production lifecycle API exists.
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
    detail: "Run live Qdrant candidate maintenance smoke locally before lifecycle PR merge, or provide CI job evidence before merge."
  - kind: command
    required: true
    owner: worker
    detail: "cargo clippy --all-targets -- -D warnings"
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
- Preserve non-destructive correction, suppression-by-default forget behavior, and source-provenance cascade behavior for forgotten episodes/observations.
- Preserve both provenance chains for corrections: what the old memory came from and what correction event/source explains the revision.
- Keep physical hard deletion/redaction deferred; do not add graph/vector/raw redaction contracts in this chunk.
- Treat Qdrant lifecycle/supersession fields as hints; graph authority remains final truth.
- Normal retrieval must exclude suppressed/deleted and non-current/superseded memories by default after lifecycle mutation.
- Keep production raw storage, reflection scheduling, external LLM behavior, and belief/evidence ontology out of this plan.

## Quality Routing Note
- Routing level: L2
- In-scope docs: Rust API/internal pipeline architecture, graph-authoritative lifecycle mutation, vector candidate maintenance, data-integrity filtering, provenance/source-reference preservation, deterministic fake-store validation, smoke evidence.
- Out-of-scope docs: UI/E2E, frontend/browser checks, production raw storage, reflection scheduling, external LLM extraction/reranking, full belief/evidence ontology.
- Top risks: data-integrity lifecycle mutation, correction provenance, graph/vector drift, legacy update/delete drift, public API shape.
- Risk profile: medium-high because this chunk mutates lifecycle/currentness state and must keep retrieval visibility correct across graph and vector stores.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo test --lib`, `cargo clippy --all-targets -- -D warnings`, targeted DTO/graph/pipeline/facade/retrieval regression tests, embedded Oxigraph lifecycle smoke, live Qdrant candidate maintenance smoke or CI evidence/required-check waiver, Reviewer gate.

## Progress Log

- 2026-04-29 Plan drafted.
  - Summary: Created the next concrete plan for correction and forget lifecycle behavior from the landed retrieve/context-pack implementation.
  - Validation evidence: Documentation-only draft after completed retrieve plan validation and final Reviewer approval.
  - Notes: Initial draft state; user approval was later recorded by Task_1.
- 2026-04-30 Final PR #33 closeout review tightened plan boundaries.
  - Summary: Updated lifecycle plan to reflect the final retrieval shape: compact omission rationale plus detailed trace, graph authority over vector maintenance, derived-memory supersession direction, object-type lifecycle boundaries, and required clippy validation.
  - Validation evidence: Documentation review against completed retrieve plan, project philosophy, ADR-D-0006, ADR-I-0005, and current retrieval DTO/pipeline behavior.
  - Notes: No lifecycle implementation performed; user approval was later recorded by Task_1.
- 2026-04-30 Pre-implementation plan review tightened lifecycle target scope.
  - Summary: Reviewed the draft against the overarching roadmap, v0.1 roadmap, ADR-D-0002, ADR-D-0006, ADR-D-0008, ADR-I-0004, ADR-I-0005, ADR-I-0006, and current domain/retrieval/repository code. Initially narrowed this chunk to derived-memory correction/supersession and derived-memory forget/suppression, deferred source-object cascades and physical redaction/delete, required typed supersession links plus object supersedes fields, selected delete-old/upsert-replacement vector hygiene, and added `cargo test --lib` as required evidence.
  - Validation evidence: Researcher plan review plus direct source/doc inspection; no implementation performed.
  - Notes: User approval was later recorded by Task_1.
- 2026-04-30 Roadmap scope recheck broadened lifecycle targets.
  - Summary: Rechecked the development roadmap and v0.1 roadmap after user feedback. Broadened this plan so completion covers derived-memory correction, episode/observation correction of affected derived memories, derived-memory/episode/observation suppression, default source-provenance cascade for forgotten episodes/observations, memory-thread archival, and graph-authoritative retrieval exclusion.
  - Validation evidence: Researcher scope reassessment plus direct roadmap/design/ADR review; no implementation performed.
  - Notes: User approval was later recorded by Task_1.
- 2026-04-30 Reviewer requested explicit correction-origin provenance.
  - Summary: Added requirements that correction requests and outcomes preserve correction-origin episode/observation/source references separately from original source provenance, and that tests prove correction-origin provenance remains inspectable.
  - Validation evidence: Reviewer plan review requested changes; no implementation performed.
  - Notes: User approval was later recorded by Task_1.
- 2026-04-30 Task_1 completed: lifecycle API and policy boundary selected.
  - Summary: Recorded user approval of the revised lifecycle plan and selected crate-visible injected correction/forget facades, supported lifecycle targets, source-object correction/forget cascade rules, separate correction-origin provenance, derived-memory supersession direction and persistence shape, non-destructive suppression/default supersession behavior, graph-authoritative mutation semantics, vector hygiene/failure policy, backend-free dependency direction, and rationale/trace exposure boundaries.
  - Validation evidence: Manual Worker review of the plan's lifecycle API, policy, vector-maintenance, failure-policy, provenance, graph-authority, and retrieval-evidence decisions before implementation edits; documentation-only Task_1 edit, no Rust checks required.
  - Notes: No source files, lessons, or git state were changed by this task. Task_2 and later implementation tasks remain bound by this design gate and their own validation/reviewer requirements.
- 2026-04-30 Tasks_2 through Task_5 implementation evidence consolidated by Task_6.
  - Summary: Working tree contains backend-free lifecycle DTOs, provider-neutral correction/forget pipeline behavior, graph-authoritative lifecycle mutation support, crate-visible injected `CharacterMemory::correct` / `CharacterMemory::forget`, and deterministic retrieval regression coverage for supersession, suppression, archival, vector-maintenance failure, legacy isolation, and trace inspectability.
  - Validation evidence: Final deterministic validation reran `cargo fmt --check` (pass), `cargo check` (pass), `cargo test --no-run` (pass), `cargo test --lib` (180 passed, 0 failed, 1 ignored), and `cargo clippy --all-targets -- -D warnings` (pass). The library suite includes lifecycle DTO tests, fake graph/vector pipeline tests, embedded Oxigraph graph lifecycle tests, injected facade lifecycle tests, retrieval stale-candidate/lifecycle omission tests, and policy-toggle regression tests.
  - Notes: This is evidence consolidation only; Task_6 did not edit source, README, tests, rules, lessons, or git state.
- 2026-04-30 Task_6 blocked during final validation.
  - Summary: Embedded Oxigraph lifecycle/retrieve smoke passed and the next Documentation, Migration Cleanup, And Release Validation plan was drafted, but lifecycle closeout was paused at that point because clippy failed in source outside Task_6 owns.
  - Validation evidence: Historical result: `cargo test --lib oxigraph_retrieval_after_lifecycle_mutation_excludes_stale_records_by_default` passed (1 passed, 0 failed); `cargo clippy --all-targets -- -D warnings` failed with `clippy::derivable_impls` on `ForgetLifecyclePolicy`; live Qdrant smoke failed before service exercise because `QDRANT_CONNECTION_STRING` was missing.
  - Notes: This entry is superseded by the follow-up evidence below, which records clippy remediation, successful deterministic validation, successful live Qdrant smoke, and final gates pending.
- 2026-04-30 Task_6 follow-up cleared validation blockers.
  - Summary: Orchestrator remediated the clippy source issue by deriving `Default` for `ForgetLifecyclePolicy`, reran the deterministic validation suite successfully, started local Qdrant, and reran the live Qdrant candidate smoke successfully. The lifecycle plan remains active in pending review state rather than being moved to completed.
  - Validation evidence: `cargo fmt --check && cargo test correction_forget_pipeline --lib && cargo test --lib && cargo check && cargo test --no-run && cargo clippy --all-targets -- -D warnings` passed; `cargo test correction_forget_pipeline --lib` reported 11 passed, 0 failed; `cargo test --lib` reported 180 passed, 0 failed, 1 ignored; `docker compose -f docker-compose.qdrant.yml up -d && QDRANT_CONNECTION_STRING=http://localhost:6334 cargo test --lib qdrant_candidate_store_live_smoke_upserts_filters_searches_and_deletes -- --ignored` passed with 1 passed, 0 failed.
  - Notes: Reviewer and Orchestrator final gates remain pending before moving this plan to completed. The next Documentation, Migration Cleanup, And Release Validation plan remains drafted and preserved.
- 2026-04-30 Final Reviewer and Orchestrator closeout approved lifecycle completion.
  - Summary: Reviewer approved the completed lifecycle implementation after policy-toggle remediation, with no blocking findings. Orchestrator confirmed required evidence completeness, live Qdrant smoke evidence, retrieval lifecycle behavior, and next-plan independence.
  - Validation evidence: Final Reviewer approval plus deterministic validation, embedded Oxigraph smoke, live Qdrant smoke, and clippy evidence recorded above.
  - Notes: This plan is ready to move from active to completed. The Documentation, Migration Cleanup, And Release Validation plan remains active as the next independent concrete plan.

## Decision Log

- 2026-04-29 Decision: Draft correction/forget lifecycle plan after retrieval implementation
  - Trigger / new insight: Retrieval now exposes graph-authoritative lifecycle/currentness omission behavior, rationale, trace, and fail-closed bounded expansion policy, making correction and forget lifecycle mutation the next roadmap chunk.
  - Plan delta: Added this active concrete plan for non-destructive correction, suppression-by-default forget, vector candidate maintenance, retrieval regression tests, and final lifecycle review.
  - Tradeoffs considered: Implementing lifecycle mutation before documentation cleanup keeps retrieval behavior grounded in actual graph state before retiring or rewriting legacy flat update/delete examples.
  - User approval: approved on 2026-04-30 via revised lifecycle plan approval.
- 2026-04-30 Decision: Lifecycle plan must not overgeneralize supersession or vector maintenance authority
  - Trigger / new insight: Final retrieve PR review added compact omission rationale, relation-derived supersession evidence, and stronger Qdrant/fake hint parity; next-phase lifecycle work must align with that landed behavior.
  - Plan delta: Clarified derived-memory supersession direction, object-type lifecycle boundaries, vector maintenance as non-authoritative hygiene, trace versus compact rationale expectations, and required clippy validation.
  - Tradeoffs considered: Keeping generic correction/forget wording would be shorter, but risks workers implementing lifecycle mutation for object types that lack canonical lifecycle fields or treating vector maintenance as correctness authority.
  - User approval: approved on 2026-04-30 via revised lifecycle plan approval.
- 2026-04-30 Decision: Constrain lifecycle implementation to derived-memory mutation first
  - Trigger / new insight: Current code supports retention filtering for episodes/observations but has no source-object forget cascade semantics, no physical delete/redact graph contract, and retrieval supersession evidence is relation-derived from typed links.
  - Plan delta: Resolved open questions toward crate-visible injected facades, derived-memory-only correction/forget mutation, hard-delete/redaction deferral, typed supersession link plus object field persistence, and vector deletion of stale candidates after graph success.
  - Tradeoffs considered: Supporting episode/observation forget immediately would appear broader, but could leave behavior-influencing derived memories active unless cascade rules are designed and tested. Deferring physical redaction/delete avoids inventing incomplete graph/raw/vector privacy semantics in a lifecycle chunk.
  - User approval: superseded before implementation by the source-object suppression and provenance cascade decision below.
- 2026-04-30 Decision: Include source-object suppression with provenance cascade
  - Trigger / new insight: The development roadmap and v0.1 design expect `forget` and suppression to prevent memories from being used for generation, and ADR-D-0002 says corrections must be able to find derived memories affected by a source episode or observation.
  - Plan delta: Brought episode/observation suppression and correction-target provenance cascade into scope, while keeping entity/link lifecycle and physical redaction/delete deferred.
  - Tradeoffs considered: Derived-memory-only lifecycle mutation is simpler but can leave forgotten source material influencing generation through derived memories. Cascade suppression increases implementation and test scope but better satisfies the roadmap's suppression and provenance goals.
  - User approval: approved on 2026-04-30 via revised lifecycle plan approval.
- 2026-04-30 Decision: Preserve correction-origin provenance separately
  - Trigger / new insight: The roadmap says a correction episode explains why and v0.1 acceptance requires provenance to the correction episode, while ADR-D-0008 requires source-reference auditability.
  - Plan delta: Required correction DTOs, pipelines, and facade tests to represent and preserve correction-origin episode/observation/source references separately from the superseded memory's original provenance chain.
  - Tradeoffs considered: Only carrying the old source chain would preserve where the outdated memory came from, but not why it changed. A separate correction-origin chain keeps correction history auditable without forcing raw transcript storage into core.
  - User approval: approved on 2026-04-30 via revised lifecycle plan approval.
- 2026-04-30 Decision: Honor non-default lifecycle policy toggles consistently
  - Trigger / new insight: Final Reviewer noted that non-default DTO policy knobs were expressible but not fully honored by the pipeline.
  - Plan delta: Correction now suppresses typed supersession links and `superseded_by` trace evidence when `supersede_replaced_derived_memories` is disabled; source-object forget cascade now requires both the cascade policy and suppression policy to allow derived-memory suppression.
  - Tradeoffs considered: Defaults already satisfied the roadmap behavior, but honoring non-default policy toggles keeps the backend-free DTO contract honest before later public API stabilization.
  - User approval: implementation refinement within approved lifecycle scope.
- 2026-04-30 Decision: Approve Task_1 lifecycle boundary for implementation dependencies
  - Trigger / new insight: User approved the revised lifecycle plan after roadmap-scope corrections and requested Task_1 record the lifecycle API and policy boundary before code implementation.
  - Plan delta: Moved the plan to `in_progress`, renamed resolved decisions from pending approval to approved decisions, expanded the design-gate decisions to cover supported object types, cascade defaults, correction-origin provenance, supersession persistence/evidence, non-destructive lifecycle defaults, graph/vector failure semantics, backend-free dependency direction, and rationale/trace behavior, and recorded Task_1 completion in the progress log.
  - Tradeoffs considered: Keeping these details only in task acceptance would make later Workers re-derive policy from scattered bullets; recording them as resolved decisions gives Task_2+ a single dependency contract without changing Rust code.
  - User approval: approved on 2026-04-30; reviewer gates for later implementation tasks remain as listed in task validation.

## Notes
- Risks:
  - Graph/vector drift can make stale candidates noisy even when graph authority excludes them correctly.
  - Episode/observation forget cascades can accidentally leave behavior-influencing derived memories active unless dependency traversal and mutation rules are explicitly designed.
  - Hard deletion/redaction semantics can accidentally bypass provenance preservation if implemented without dedicated raw/source, graph, vector, and privacy contracts.
  - Legacy flat update/delete methods may confuse users until the public replacement lifecycle surface and documentation cleanup land.
- Edge cases:
  - Correcting a derived memory should not erase the old source-reference chain.
  - Correcting a memory should preserve the correction event/source that explains why the replacement exists.
  - Forgetting an episode or observation should cascade to behavior-influencing derived memories provenanced to that source by default.
  - Forget should suppress normal retrieval but preserve historical/audit visibility when policy explicitly includes it.
  - Supersession direction must remain clear: new memory supersedes old memory; old memory may be described as superseded_by the new memory in rationale/trace.
  - Vector candidates for suppressed/superseded objects must not become lifecycle authority even if they remain in Qdrant temporarily.
