# Plan: v0.1 Schema Migration Hooks And Tests

- status: done
- generated: 2026-05-01
- last_updated: 2026-05-01
- work_type: code

## Goal
- Add explicit schema-version compatibility and migration seams for current persisted surfaces without introducing a second schema or overbuilding real migrations before they exist.

## Definition of Done
- Current schema version is accepted consistently at storage/payload/graph mapping boundaries.
- Unsupported schema versions fail with clear, typed errors before silent persistence or payload mapping.
- A minimal migration hook exists for future schema versions and has tested no-op behavior for the current schema.
- Domain, Qdrant payload, and RDF graph mapping tests document the expected schema-version behavior.

## Scope / Non-goals
- Scope:
  - Internal schema compatibility helpers.
  - Validation at graph and vector persistence/mapping boundaries.
  - Tests for current-version no-op behavior and unsupported-version failure behavior.
- Non-goals:
  - A real migration from an older schema.
  - Backward compatibility with unmodeled historical schemas.
  - Changing `DEFAULT_SCHEMA_VERSION`.
  - Persistent Oxigraph or Qdrant/Oxigraph reconciliation.

## Context (workspace)
- Related files/areas:
  - `src/api/types/domain.rs`
  - `src/api/types/draft.rs`
  - `src/internal/infrastructures/graph/rdf_mapping.rs`
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `src/internal/infrastructures/external_services/qdrant_payload.rs`
  - `src/internal/models/vector/record.rs`
  - `src/errors/custom.rs`
- Existing patterns or references:
  - Domain objects already carry `schema_version`.
  - Qdrant payloads and RDF triples already include schema-version fields.
  - Draft defaults use `DEFAULT_SCHEMA_VERSION`.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`
  - `docs/decisions/implementation/ADR-I-0007-schema-versioning.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`

## Open Questions (max 3)
- Resolved: Unsupported schema rejection should live at persistence/import/mapping boundaries, not domain `validate()`.

## Assumptions
- A1: The first implementation should validate current compatibility and fail unsupported versions rather than transforming records.
- A2: Domain objects may still represent records with non-current versions if future import/migration code needs that, so persistence-boundary rejection is the safer default.
- A3: Error messages should use stable schema language, not roadmap version labels.

## Tasks

### Task_1: Define Internal Schema Compatibility Boundary
- type: impl
- owns:
  - `src/internal/**`
  - `src/errors/custom.rs`
- depends_on: []
- description: |
  Add an internal compatibility/migration seam that can validate schema versions, expose current no-op migration behavior, and return clear errors for unsupported versions.
- acceptance:
  - Current schema is accepted by a single reusable helper.
  - Unsupported schema produces a clear `CustomError` variant or message appropriate for storage/mapping failures.
  - The helper does not expose backend-specific types or public migration APIs.
  - Tests cover accepted current schema and rejected unsupported schema.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test api::types::domain --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal --lib"

### Task_2: Apply Schema Checks To Graph And Vector Mapping Boundaries
- type: impl
- owns:
  - `src/internal/infrastructures/graph/rdf_mapping.rs`
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `src/internal/infrastructures/external_services/qdrant_payload.rs`
  - `src/internal/models/vector/record.rs`
- depends_on: [Task_1]
- description: |
  Wire schema compatibility checks into persistence and mapping paths that serialize records to RDF or Qdrant payloads. Ensure failures happen before unsupported records are silently written or indexed.
- acceptance:
  - RDF mapping rejects unsupported schema versions for all durable memory object variants.
  - Oxigraph upsert paths reject unsupported schema versions before committing graph mutations.
  - Qdrant payload mapping rejects unsupported schema versions before point construction.
  - Existing current-schema fixtures continue to pass.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::graph --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::external_services::qdrant_payload --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"

### Task_3: Add Migration Hook Regression Tests
- type: test
- owns:
  - `src/internal/schema.rs`
  - `src/api/types/domain/tests.rs`
  - `src/internal/infrastructures/graph/rdf_mapping.rs`
  - `src/internal/infrastructures/external_services/qdrant_payload.rs`
  - `src/internal/repositories/test_support.rs`
- depends_on: [Task_2]
- description: |
  Add regression tests that document current-version no-op migration behavior, unsupported-version failure behavior, and payload/triple schema-version preservation.
- acceptance:
  - Tests verify current schema version remains pinned to `EPISODIC_MEMORY_SCHEMA_VERSION`.
  - Tests verify unsupported versions fail clearly at graph/vector mapping boundaries.
  - Tests verify schema-version fields are preserved in RDF and Qdrant payload surfaces.
  - Tests document that real forward migrations are intentionally absent until a second schema exists.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"

### Task_4: Reviewer Gate
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Review the schema boundary for appropriate scope, failure behavior, and future migration extensibility.
- acceptance:
  - Reviewer status is APPROVED.
  - Review confirms current-schema behavior is unchanged.
  - Review confirms the plan did not add fake historical migrations or persistent-backend behavior.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review against ADR-I-0007 and storage/backend migration-test expectations"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. Rust library/storage behavior only.

## Rollback / Safety
- Keep the helper internal and narrowly wired so rejection behavior can be adjusted without public API churn.
- Avoid changing schema constants in this plan.

## Progress Log (append-only)

- 2026-05-01 00:00 Draft created.
  - Summary: Split schema migration hooks/tests into their own v0.1 backend-contract plan.
  - Validation evidence: Not run; plan only.
  - Notes: Awaiting user approval before implementation.

- 2026-05-01 01:35 Wave 1 completed: [Task_1]
  - Summary: Added internal schema compatibility helpers and typed unsupported-schema error.
  - Validation evidence: Worker reported `cargo fmt --check`, `cargo test api::types::domain --lib`, and `cargo test internal --lib` passed after formatting.
  - Notes: Initial `cargo fmt --check` failed before formatting, then passed after `cargo fmt`.

- 2026-05-01 01:45 Wave 2 completed: [Task_2]
  - Summary: Wired current-schema checks into RDF mapping, Oxigraph upserts, and Qdrant payload mapping.
  - Validation evidence: Worker reported `cargo test internal::infrastructures::graph --lib`, `cargo test internal::infrastructures::external_services::qdrant_payload --lib`, and `cargo check` passed.
  - Notes: `cargo check` passed with a dead-code warning for the intentionally introduced migration hook; Task_3 owns was expanded to cover `src/internal/schema.rs` for regression documentation or an explicit temporary suppression.

- 2026-05-01 01:55 Wave 3 completed: [Task_3]
  - Summary: Added regression coverage for current-schema no-op migration, absent forward migrations, RDF schema-version preservation, and Qdrant schema-version preservation.
  - Validation evidence: Worker reported `cargo test --lib` and `cargo test --no-run` passed; optional `cargo fmt --check` and `cargo check` also passed.
  - Notes: The intentionally future-facing migration hook now has a narrow documented dead-code allowance.

- 2026-05-01 02:05 Wave 4 completed: [Task_4]
  - Summary: Reviewer approved the schema boundary implementation with no findings.
  - Validation evidence: Reviewer reran `cargo fmt --check`, `cargo check`, `cargo test internal::schema --lib`, `cargo test internal::infrastructures::graph --lib`, and `cargo test internal::infrastructures::external_services::qdrant_payload --lib`.
  - Notes: Reviewer inspected worker evidence for full `cargo test --lib` and `cargo test --no-run`; live Qdrant remains out of scope for this plan.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-01 00:00 Decision:
  - Trigger / new insight: Prior repository assessment found schema fields/constants exist, but migration hooks and unsupported-version tests are not implemented.
  - Plan delta (what changed): Created a standalone plan for schema compatibility and migration-test seams.
  - Tradeoffs considered: Persistence-boundary rejection avoids constraining future import/migration representation in domain objects.
  - User approval: no

- 2026-05-01 01:30 Decision:
  - Trigger / new insight: User approved implementation and asked to resolve open questions before work.
  - Plan delta (what changed): Marked the plan in progress and resolved schema-version rejection to persistence/import/mapping boundaries.
  - Tradeoffs considered: Keeping domain validation schema-version-neutral preserves future ability to represent old records before migration.
  - User approval: yes

- 2026-05-01 01:45 Decision:
  - Trigger / new insight: Task_2 validation passed with an unused-hook warning for `migrate_current_schema`.
  - Plan delta (what changed): Expanded Task_3 owns to include `src/internal/schema.rs` so the regression task can document or suppress the intentionally unused future migration hook within scope.
  - Tradeoffs considered: Keeping the hook preserves the migration seam required by the plan; adding a narrow owned path avoids out-of-scope follow-up edits.
  - User approval: yes

## Notes
- Risks:
  - Rejecting unsupported versions too deep may allow partial work before failure; the implementation should fail before graph/vector writes.
  - Rejecting unsupported versions in domain validation may prevent future migration tooling from representing old objects.
- Edge cases:
  - Mixed object batches where one object has unsupported schema.
  - Links with unsupported schema.
  - Vector records produced from otherwise valid domain objects but carrying unsupported schema metadata.
