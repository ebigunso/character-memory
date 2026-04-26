# Plan: v0.1 Domain Foundation

- status: done
- generated: 2026-04-27
- last_updated: 2026-04-27
- work_type: mixed

## Goal
- Establish the v0.1 domain foundation for Character Memory: typed memory objects, stable IDs and graph URIs, schema versioning, raw reference fields, lifecycle enums, validation rules, and deterministic tests.
- Make the breaking API direction explicit without implementing storage adapters, graph persistence, retrieval pipelines, correction, or forgetting yet.

## Definition of Done
- The v0.1 public/internal model boundary is selected and documented.
- Core enums, IDs, schema constants, graph URI helpers, and core object structs compile.
- Model validation covers required v0.1 invariants, including derived-memory provenance and score bounds.
- Raw input is represented by reference IDs/strings, with a deterministic file-backed raw-ref fixture for tests only.
- Required Rust checks and targeted model tests pass, or any blocker is recorded with evidence.
- Reviewer approves the domain foundation diff and validation evidence.

## Scope / Non-goals
- Scope:
  - `Episode`, `Observation`, `Entity`, `MemoryThread`, `DerivedMemory`, `MemoryLink`, and `MemoryObject` or equivalent.
  - Shared enums for object type, modality, entity type, derived type, relation type, retention state, stability, and thread status.
  - `Uuid`-backed `MemoryId` strategy and deterministic `urn:cmem:*` graph URI helper.
  - Schema version constants/defaults and model validation helpers.
  - Raw reference preservation and test fixture pattern.
- Non-goals:
  - Qdrant payload migration or vector adapter behavior.
  - Oxigraph dependency, RDF triples, SPARQL query builders, or graph store implementation.
  - Store contracts, fake stores, remember/retrieve/link/correct/forget pipelines.
  - Compatibility wrappers for the old flat API.
  - Production raw input storage.

## Context (workspace)
- Related files/areas:
  - `src/lib.rs`
  - `src/api/**`
  - `src/models.rs`
  - `src/internal/models/**`
  - `src/internal/mod.rs`
  - `tests/**`
  - `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap.md`
- Existing patterns or references:
  - Public DTOs currently live under `src/api/types`.
  - Internal model code currently lives under `src/internal/models/**`.
  - The current flat public DTOs are `Memory`, `MemoryInput`, `MemoryType`, and `ScoredMemory`.
  - The roadmap accepts breaking changes for v0.1 and consumer-owned raw input via reference IDs.
- Repo reference docs consulted:
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/decisions/design/ADR-D-0001-episode-backed-object-model.md`
  - `docs/decisions/design/ADR-D-0002-derived-memory-provenance.md`
  - `docs/decisions/design/ADR-D-0007-chat-native-transcript-compatible-start.md`
  - `docs/decisions/design/ADR-D-0008-preserve-source-references.md`
  - `docs/decisions/implementation/ADR-I-0001-stable-cross-store-ids.md`
  - `docs/decisions/implementation/ADR-I-0007-schema-versioning.md`

## Open Questions
- Q1: None for this chunk. Graph validation phasing is accepted but belongs to later graph/store chunks.

## Review Mode
- mode: remediation
- scope: final-plan-review
- max_iterations: 2
- status: completed

## Assumptions
- A1: `MemoryId` is either `pub type MemoryId = uuid::Uuid` or an equivalent UUID-backed newtype if local conventions make a newtype worthwhile.
- A2: Existing `chrono` usage remains the default time choice for this chunk unless source inspection reveals a stronger local convention.
- A3: The old flat API can temporarily coexist during this chunk if removal would broaden the change beyond the model foundation.
- A4: Validation should reject invalid scores instead of silently clamping unless existing repo patterns strongly prefer normalization.

## Tasks

### Task_1: Select model module boundary
- type: design
- owns:
  - `src/api/**`
  - `src/models.rs`
  - `src/internal/models/**`
  - `src/internal/mod.rs`
  - `docs/coding-agent/plans/active/v0-1-domain-foundation-plan.md`
- depends_on: []
- description: |
  Inspect the current public and internal model layout, then choose where v0.1 model types should live for this chunk. Record the decision before broad implementation edits.
- acceptance:
  - Public vs internal model boundary is recorded in this plan's Decision Log or Progress Log.
  - The selected boundary supports exporting v0.1 model types without making old flat `MemoryType` canonical.
  - Temporary coexistence with old flat DTOs is explicitly scoped if needed for compilation.
  - The decision respects existing repo conventions unless there is a clear reason to introduce a new layout.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Read current model/API modules and record the selected module boundary before implementation edits."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review the selected boundary for consistency with repo conventions and v0.1 handoff goals."

### Task_2: Add shared enums, IDs, schema, and graph URI helpers
- type: impl
- owns:
  - `src/api/**`
  - `src/models.rs`
  - `src/internal/models/**`
  - `src/internal/mod.rs`
  - `tests/**`
- depends_on: [Task_1]
- description: |
  Add the shared v0.1 enum and identifier foundation without implementing storage adapters or pipelines.
- acceptance:
  - Enums exist and serialize as snake_case: object type, modality, entity type, derived type, relation type, retention state, stability, and thread status.
  - `MemoryId` is UUID-backed.
  - Graph URI helper maps object type plus ID to stable `urn:cmem:*` IRIs.
  - Schema version constants/defaults are available to v0.1 model constructors or helpers.
  - Tests cover enum serialization and graph URI determinism.
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
    detail: "Run targeted enum serialization and graph URI tests added by this task."

### Task_3: Add core object structs
- type: impl
- owns:
  - `src/api/**`
  - `src/models.rs`
  - `src/internal/models/**`
  - `tests/**`
- depends_on: [Task_2]
- description: |
  Add the v0.1 core object structs and a sum type such as `MemoryObject`, including raw reference fields but no production raw storage.
- acceptance:
  - Structs exist for `Episode`, `Observation`, `Entity`, `MemoryThread`, `DerivedMemory`, and `MemoryLink`.
  - Objects carry expected v0.1 fields for IDs, object types, timestamps, schema version, lifecycle where applicable, and raw references where applicable.
  - `DerivedMemory` includes source episode and observation ID vectors.
  - `MemoryLink` supports typed from/to object IDs, relation type, confidence, rationale, timestamp, and schema version.
  - Serde round-trip tests cover representative objects.
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
    detail: "Run targeted object serde round-trip tests added by this task."

### Task_4: Add validation helpers and raw-ref fixture pattern
- type: impl
- owns:
  - `src/api/**`
  - `src/models.rs`
  - `src/internal/models/**`
  - `tests/**`
- depends_on: [Task_3]
- description: |
  Add deterministic validation helpers for model invariants and introduce a test-only raw-ref fixture pattern using file-backed raw text referenced by ID/string.
- acceptance:
  - Episode validation rejects empty summaries.
  - Observation validation requires an episode reference.
  - Derived memory validation requires at least one source episode or observation.
  - Score validation rejects or normalizes out-of-range values consistently with the recorded decision.
  - Test fixture stores raw text outside memory objects and verifies `raw_ref` preservation by reference.
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
    detail: "Run targeted validation and raw-ref fixture tests added by this task."

### Task_5: Review domain foundation and prepare next plan
- type: review
- owns:
  - `docs/coding-agent/plans/active/**`
- depends_on: [Task_4]
- description: |
  Review the domain foundation diff and validation evidence. If approved, draft the next concrete plan for store contracts and deterministic test harness using the landed model shape.
- acceptance:
  - Reviewer approves the domain foundation diff or blocking issues are resolved/waived.
  - Required validation evidence from Tasks 1-4 is present.
  - This plan's Progress Log and Decision Log are updated with outcomes.
  - A separate active plan for store contracts and deterministic test harness is drafted from the landed code shape.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Review domain foundation implementation against this plan and v0.1 handoff model requirements; include review-remediation-loop structured appendix for MAJOR/CRITICAL findings."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Apply review-remediation-loop triage if final review surfaces kept MAJOR/CRITICAL findings, then confirm the next concrete plan is independent and based on landed model code."

## Task Waves (explicit parallel dispatch sets)

Interpretation:
- Tasks listed in the same wave are intended to be dispatched in parallel by default when owns are disjoint and dependencies are met.
- Waves are executed sequentially.

- Wave 1 (design gate): [Task_1]
- Wave 2 (enum/id/schema foundation): [Task_2]
- Wave 3 (core objects): [Task_3]
- Wave 4 (validation and raw refs): [Task_4]
- Wave 5 (review and next-plan draft): [Task_5]

## E2E / Visual Validation Spec

- Not applicable. This is a Rust library model foundation with no UI/user-flow surface.

## Rollback / Safety
- Keep this chunk focused on model foundation; do not add Qdrant, Oxigraph, graph store, or pipeline behavior here.
- Preserve old flat DTOs temporarily if removing them would broaden the chunk beyond model foundation.
- Keep raw input storage consumer-owned; file-backed raw text is only a test fixture pattern.
- Treat broad API cleanup as later migration cleanup unless required for compilation.

## Quality Routing Note
- Routing level: L2
- In-scope docs: Rust model/API architecture, schema/versioning, validation/test evidence, API compatibility.
- Out-of-scope docs: Qdrant/Oxigraph adapter details beyond ID/URI shape, UI/E2E, auth/security, production raw storage.
- Top risks: contract/API compatibility, data-integrity, migration/schema.
- Risk profile: medium for this chunk because it introduces new public model types and validation rules but avoids storage and retrieval behavior.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted deterministic model tests, Reviewer gate.
- Optional recommended checks: none for this chunk.
- At Risk items: final module placement and old flat DTO coexistence.

## Review Remediation Log

- Final review approved with no findings. No remediation iterations were needed.

## Progress Log (append-only)

- 2026-04-27 Plan drafted.
  - Summary: Created a concrete execution plan for the v0.1 domain foundation chunk.
  - Validation evidence: Reviewer approved the plan structure after confirming Task_X format, owns, dependencies, validation ownership, and outcome-based plan naming.
  - Notes: This plan is separate from the full implementation roadmap.
- 2026-04-27 Task_1 complete: Selected v0.1 model module boundary.
  - Summary: Canonical v0.1 domain model types will live in a new public submodule under `src/api/types`, be re-exported through `src/api/types/mod.rs` and `src/lib.rs`, and remain independent of internal storage/vector implementation shapes.
  - Validation evidence: Worker review confirmed the decision satisfies Task_1 acceptance by recording the public/internal boundary, preserving temporary old flat DTO coexistence as legacy-only, and preventing internal repository/Qdrant/embedding dependencies from entering canonical domain types.
  - Notes: No implementation files were edited for this design gate.
- 2026-04-27 Task_2 complete: Added v0.1 enum/id/schema/graph foundation.
  - Summary: Implemented canonical foundation types under `src/api/types/domain`, re-exported them from `src/api/types/mod.rs` and `src/lib.rs`, and added deterministic service-free tests for enum serialization, schema aliases, and graph URI generation.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and the domain foundation unit tests under `src/api/types/domain/tests.rs` passed locally. Historical target at time of original execution: `cargo test --test domain_foundation_tests`.
  - Notes: The old flat DTOs remain in place as legacy compatibility surface.
- 2026-04-27 Task_3 complete: Added v0.1 core object structs.
  - Summary: Added `Episode`, `Observation`, `Entity`, `MemoryThread`, `DerivedMemory`, `MemoryLink`, and tagged `MemoryObject` under `src/api/types/domain`, re-exported through the public API, and added deterministic serde round-trip tests including raw reference preservation.
  - Validation evidence: See the Task_3 validation entry below.
  - Notes: This task adds raw reference fields only and does not add production raw storage, validation helpers, storage adapters, graph persistence, or pipeline behavior.
- 2026-04-27 Task_3 validation complete.
  - Summary: Verified the core object structs and `MemoryObject` compile and round-trip deterministically through serde.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and the domain object unit tests under `src/api/types/domain/tests.rs` passed locally. Historical target at time of original execution: `cargo test --test domain_object_tests`.
  - Notes: Targeted tests cover representative objects, raw reference preservation, and tagged `MemoryObject` serialization/deserialization.
- 2026-04-27 Task_3 follow-up: Aligned v0.1 core object optionality with the handoff.
  - Summary: Episode start and observation observed times are optional, while entity/thread/derived update times and thread summary are required.
  - Validation evidence: Serde round-trip fixtures were updated to match before Task_4 validation work; follow-up validation evidence is recorded in the Worker report.
  - Notes: No validation helpers, storage behavior, raw storage, Qdrant/Oxigraph behavior, or pipeline behavior were added.
- 2026-04-27 Task_4 complete: Added validation helpers and raw-ref fixture pattern.
  - Summary: Added `DomainValidationError`, `validate()` helpers on v0.1 domain objects, and deterministic tests for required invariants plus file-backed raw reference preservation.
  - Validation evidence: Worker and Orchestrator both ran required checks. Current post-refactor evidence: domain validation and raw-ref fixture coverage now lives in `src/api/types/domain/tests.rs` and is validated by `cargo test --lib -- --nocapture`. Historical target names at time of original execution were `cargo test --test domain_foundation_tests`, `cargo test --test domain_object_tests`, and `cargo test --test domain_validation_tests`.
  - Notes: Invalid scores are rejected rather than clamped; no production raw storage was added.
- 2026-04-27 Task_5 complete: Final review approved and next plan drafted.
  - Summary: Reviewer approved the domain foundation implementation with no findings; remediation mode completed with zero iterations. Drafted the next active plan for store contracts and deterministic test harness.
  - Validation evidence: Final Reviewer status APPROVED; structured remediation appendix had `structured_findings: []` and `highest_severity: NONE`.
  - Notes: Plan moved to completed after all required evidence was recorded.
- 2026-04-27 Post-completion clarification: legacy compatibility is not a v0.1 goal.
  - Summary: User clarified after this plan completed that compatibility is not a concern for v0.1.
  - Validation evidence: Documentation-only clarification.
  - Notes: Historical entries in this plan may mention temporary coexistence with old flat DTOs as a scope-control choice for the domain-foundation chunk. Future chunks should remove legacy implementations that do not contribute to the new architecture.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-04-27 Decision: Separate first implementation plan by achieved outcome
  - Trigger / new insight: User asked for concrete plans to be independent from the roadmap and named by what they achieve rather than by chunk number.
  - Plan delta: Created `v0-1-domain-foundation-plan.md` for the first concrete implementation chunk.
  - Tradeoffs considered: A numbered chunk plan would match roadmap order but carry less context in active/completed plan listings.
  - User approval: yes.
- 2026-04-27 Decision: Activate review remediation loop for final gate
  - Trigger / new insight: User requested using the review-remediation-loop skill for the final review gate if appropriate.
  - Plan delta: Added Review Mode and remediation-log placeholders, and updated Task_5 validation to use remediation triage for kept MAJOR/CRITICAL findings.
  - Tradeoffs considered: This chunk is code-focused and non-trivial, so a bounded final remediation loop is appropriate; early design reviews remain normal review gates.
  - User approval: yes.
- 2026-04-27 Decision: Use `src/api/types` public submodule for canonical v0.1 domain types
  - Trigger / new insight: v0.1 needs typed public domain objects without treating the current flat `Memory`, `MemoryInput`, `MemoryType`, and `ScoredMemory` DTOs as canonical.
  - Plan delta: Add canonical v0.1 domain model types in a new public submodule under `src/api/types`; re-export them from `src/api/types/mod.rs` and `src/lib.rs` for the public crate API.
  - Boundary: Storage/vector-only shapes stay under `src/internal/models`; canonical domain types must not depend on `crate::internal`, repositories, Qdrant payloads, or embedding services.
  - Compatibility scope: Old flat DTOs may temporarily coexist for the existing facade, repository adapters, and tests, but they are legacy compatibility surface during migration and are not the canonical v0.1 model.
  - Tradeoffs considered: Placing canonical types under `src/internal/models` would obscure the public API contract; replacing old flat DTOs immediately would broaden this design gate into migration work. A public `src/api/types` submodule follows existing conventions while isolating v0.1 domain models from storage adapters.
  - User approval: yes.
- 2026-04-27 Decision: Do not preserve old flat API compatibility going forward
  - Trigger / new insight: User clarified after the domain foundation PR that compatibility is not a concern for v0.1.
  - Plan delta: Treat old flat DTOs, repositories, and facade pieces as removable legacy once replacement architecture exists.
  - Tradeoffs considered: Keeping shims could reduce short-term churn but would increase architectural drag during the v0.1 rewrite.
  - User approval: yes.
- 2026-04-27 Decision: Name the canonical foundation module `domain` and schema constant `SCHEMA_VERSION_V0_1`
  - Trigger / new insight: Task_2 needed a concrete module name that does not make legacy `MemoryType` look canonical, plus one pinned schema spelling for tests and later constructors.
  - Plan delta: Added `src/api/types/domain` with `SCHEMA_VERSION_V0_1`, `CURRENT_SCHEMA_VERSION`, and `DEFAULT_SCHEMA_VERSION` aliases using the string value `v0.1`.
  - Boundary: The module contains only public domain enums, the UUID-backed `MemoryId` alias, schema constants, and deterministic URI helpers; it does not depend on internal storage, repositories, Qdrant payloads, or embedding services.
  - Tradeoffs considered: A more specific module name such as `v0_1` would encode versioning in the path but make imports noisier for the canonical model surface. A `domain` submodule keeps the public API cohesive while schema constants carry the version pin.
  - User approval: implied by Task_2 direction.
- 2026-04-27 Decision: Represent raw material with optional external raw reference strings on core objects
  - Trigger / new insight: Task_3 requires raw reference fields while explicitly excluding production raw storage.
  - Plan delta: `Episode` and `Observation` use `raw_ref: Option<String>` so consumers can preserve an external raw input locator without the library owning the raw payload.
  - Boundary: No file-backed fixture pattern, raw persistence, Qdrant payload storage, or graph storage was added; those remain out of scope for Task_3 or later tasks.
  - Tradeoffs considered: A dedicated raw-reference newtype could be introduced later if validation or URI semantics become stricter, but a string keeps this object-shape chunk minimal and serde-compatible.
  - User approval: implied by Task_3 direction.
- 2026-04-27 Decision: Align core object optionality with the v0.1 handoff
  - Trigger / new insight: Follow-up review found some timestamp and summary optionality had drifted from the handoff shape.
  - Plan delta: `Episode.started_at` and `Observation.observed_at` are optional; `Entity.updated_at`, `MemoryThread.summary`, `MemoryThread.updated_at`, and `DerivedMemory.updated_at` are required. External references and keys such as `source_conversation_id`, `canonical_key`, and `rationale` remain optional.
  - Boundary: No validation helpers were added to enforce presence or score ranges; Task_4 owns invariant validation.
  - Tradeoffs considered: Matching the handoff shape now reduces downstream adapter and validation churn; stricter semantic validation remains separate from struct field optionality.
  - User approval: implied by Task_3 direction.
- 2026-04-27 Decision: Reject invalid domain scores instead of clamping
  - Trigger / new insight: Task_4 needed one consistent score policy for salience and confidence validation.
  - Plan delta: Validation rejects non-finite values and values outside `0.0..=1.0` through `DomainValidationError::InvalidScore`.
  - Tradeoffs considered: Rejecting invalid input surfaces caller errors early and avoids hidden score mutation before store/ranking policy exists.
  - User approval: implied by Task_4 direction.
- 2026-04-27 Decision: Keep raw-ref fixture test-only
  - Trigger / new insight: Task_4 needed to prove file-backed raw text can be referenced by ID/string without adding production raw storage.
  - Plan delta: Added a test-local file-backed fixture for raw-reference behavior; after the module-layout cleanup this lives under `src/api/types/domain/tests.rs`. Production domain objects still store only `raw_ref: Option<String>`.
  - Tradeoffs considered: A reusable raw store fixture may be useful later, but adding it now would blur the boundary between raw references and raw persistence.
  - User approval: implied by Task_4 direction.

## Notes
- Risks:
  - The selected public/internal model boundary may affect later store and pipeline plans.
  - Removing old flat API types too early could create broad churn unrelated to the model foundation.
  - Score validation must be consistent across later draft inputs and store payloads.
- Edge cases:
  - Voice transcript support should remain chat-like in v0.1; do not introduce raw audio/video concepts.
  - `DerivedMemory` must never validate as behavior-influencing without episode or observation provenance.
  - Raw refs should identify external raw material without making the library responsible for storing it.
