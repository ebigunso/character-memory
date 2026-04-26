# Plan: Rust Module File Layout Migration

- status: done
- generated: 2026-04-27
- last_updated: 2026-04-27
- work_type: code

## Goal
- Move Rust module sources away from `mod.rs` files to Rust 2018-style direct module filenames.
- Relocate the newly added pure domain tests from integration-test targets into the production domain module as unit tests.
- Preserve public API paths, visibility, behavior, and legacy flat DTO compatibility.

## Definition of Done
- No `src/**/mod.rs` files remain.
- No duplicate module-source pairs such as `foo.rs` plus `foo/mod.rs` remain.
- Existing `mod`, `pub mod`, and `pub(crate) mod` declarations continue to resolve the same module names.
- The three newly added domain test files under `tests/` are removed as integration test targets and their equivalent tests run as unit tests under the domain module.
- Required non-service Rust validation passes.
- Reviewer approves the migration diff and validation evidence.

## Scope / Non-goals
- Scope:
  - Mechanical moves for existing `src/**/mod.rs` files.
  - Test relocation for `tests/domain_foundation_tests.rs`, `tests/domain_object_tests.rs`, and `tests/domain_validation_tests.rs`.
  - Minimal planning/docs reference updates from `domain/mod.rs` to `domain.rs` where they would otherwise be stale.
- Non-goals:
  - Changing public API names, re-export behavior, or visibility.
  - Refactoring domain model behavior.
  - Migrating legacy flat DTOs or repository contracts.
  - Running service-dependent tests.

## Context
- Current direct-module precedent already exists at `src/config.rs`, `src/config/settings.rs`, `src/errors.rs`, `src/models.rs`, and similar files.
- The domain foundation code now lives at `src/api/types/domain.rs` after the Task_1 module-layout move.
- The newly added domain tests are pure unit-level tests of domain serialization, validation, and helper behavior, so they belong under the domain module rather than `tests/`.

## Open Questions
- Q1: None.

## Assumptions
- A1: This migration should preserve module declarations wherever possible; file moves are preferred over declaration rewrites.
- A2: Domain unit tests should live in `src/api/types/domain/tests.rs` and be included with `#[cfg(test)] mod tests;` from `src/api/types/domain.rs`.
- A3: Existing older integration tests under `tests/` remain integration tests and are not moved.

## Tasks

### Task_1: Move source modules off `mod.rs`
- type: impl
- owns:
  - `src/**/mod.rs`
  - `src/**/*.rs`
  - `docs/coding-agent/plans/active/rust-module-file-layout-migration-plan.md`
  - `docs/coding-agent/plans/active/v0-1-store-contracts-test-harness-plan.md`
  - `docs/coding-agent/plans/completed/v0-1-domain-foundation-plan.md`
- depends_on: []
- description: |
  Move all existing `src/**/mod.rs` files to direct Rust module filenames while preserving module declarations and public paths.
- acceptance:
  - `src/api/mod.rs` is moved to `src/api.rs`.
  - `src/api/types/mod.rs` is moved to `src/api/types.rs`.
  - `src/api/types/domain/mod.rs` is moved to `src/api/types/domain.rs`.
  - All internal `mod.rs` files are moved to their direct module filename equivalents.
  - No `src/**/mod.rs` files remain after the move.
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
  - kind: search
    required: true
    owner: worker
    detail: "Confirm no `src/**/mod.rs` files remain."

### Task_2: Move domain tests into module unit tests
- type: test
- owns:
  - `src/api/types/domain.rs`
  - `src/api/types/domain/tests.rs`
  - `tests/domain_foundation_tests.rs`
  - `tests/domain_object_tests.rs`
  - `tests/domain_validation_tests.rs`
  - `docs/coding-agent/plans/active/rust-module-file-layout-migration-plan.md`
  - `docs/coding-agent/plans/completed/v0-1-domain-foundation-plan.md`
- depends_on: [Task_1]
- description: |
  Move the newly added pure domain tests from integration-test files into the domain module's unit-test module.
- acceptance:
  - Equivalent enum/schema/URI tests exist under the domain module.
  - Equivalent object serde and `MemoryObject` tests exist under the domain module.
  - Equivalent validation and file-backed raw-ref fixture tests exist under the domain module.
  - `tests/domain_foundation_tests.rs`, `tests/domain_object_tests.rs`, and `tests/domain_validation_tests.rs` are deleted.
  - Unit tests use module-local paths where practical.
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
    detail: "cargo test --lib -- --nocapture"
  - kind: search
    required: true
    owner: worker
    detail: "Confirm no `tests/domain_*_tests.rs` files remain."

### Task_3: Review module-layout migration
- type: review
- owns: []
- depends_on: [Task_1, Task_2]
- description: |
  Review the migration for API preservation, absence of `mod.rs` remnants, correct unit-test relocation, and required validation evidence.
- acceptance:
  - Reviewer status is APPROVED or blocking issues are resolved/waived.
  - Required validation evidence from Tasks 1 and 2 is present.
  - No unrelated source behavior changes are introduced.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Review diff and validation evidence for module layout migration and unit-test relocation."

## Task Waves

- Wave 1 (module moves): [Task_1]
- Wave 2 (unit test relocation): [Task_2]
- Wave 3 (review): [Task_3]

## E2E / Visual Validation Spec

- Not applicable. This is a Rust library module-layout refactor with no UI/user-flow surface.

## Rollback / Safety
- Keep module contents and declarations unchanged during the mechanical move except for path reference updates.
- Move tests only after the source module layout compiles.
- Do not touch existing integration tests unrelated to the new domain unit tests.

## Quality Routing Note
- Routing level: L1
- In-scope docs: Rust module layout, deterministic unit-test placement, validation evidence.
- Out-of-scope docs: live service integration, UI/E2E, security/auth, storage adapter behavior.
- Top risks: module resolution, test target relocation, accidental public API change.
- Risk profile: medium-low; many files move, but behavior should be preserved.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, `cargo test --lib -- --nocapture`, targeted file searches, Reviewer gate.
- Optional recommended checks: none.
- At Risk items: []

## Progress Log

- 2026-04-27 Plan drafted.
  - Summary: Created a focused plan for moving `mod.rs` files to direct module filenames and relocating newly added domain tests into module unit tests.
  - Validation evidence: Pending approval and execution.
  - Notes: Existing integration tests remain in `tests/`.
- 2026-04-27 Task_1 complete: Moved source modules off `mod.rs`.
  - Summary: Moved all 12 source `mod.rs` files to Rust 2018-style direct module filenames while preserving module declarations and public paths. Task_2 test relocation remains pending and untouched.
  - Validation evidence: `cargo fmt --check`, `cargo check`, and `cargo test --no-run` passed locally. A targeted `src/**/mod.rs` search found no remaining files.
  - Notes: Updated stale plan references to the domain module's new `src/api/types/domain.rs` filename and recorded user approval in the Decision Log.
- 2026-04-27 Task_2 complete: Moved pure domain integration tests into module unit tests.
  - Summary: Added `#[cfg(test)] mod tests;` to `src/api/types/domain.rs`, created `src/api/types/domain/tests.rs` with equivalent domain foundation/object/validation/raw-ref tests, and removed the three `tests/domain_*_tests.rs` integration targets.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and `cargo test --lib -- --nocapture` passed locally. A targeted `tests/domain_*_tests.rs` search found no remaining files.
  - Notes: Existing older integration tests under `tests/` remain integration tests.
- 2026-04-27 Task_3 complete: Reviewer approved module-layout migration.
  - Summary: Reviewer approved the direct module file migration and domain unit-test relocation with no blocking findings.
  - Validation evidence: Reviewer confirmed no `src/**/mod.rs` or `tests/domain_*_tests.rs` files remain, public paths/re-exports are preserved, and required validation evidence is sufficient.
  - Notes: One minor stale historical path in the completed domain foundation plan was cleaned up before closeout.

## Decision Log

- 2026-04-27 Decision: Use direct Rust module files and domain unit tests
  - Trigger / new insight: User requested moving away from `mod.rs` files and reserving `tests/` for integration tests.
  - Plan delta: Migrate all existing `src/**/mod.rs` files, and move new pure domain tests under `src/api/types/domain/tests.rs`.
  - Tradeoffs considered: Moving only newly added files would leave inconsistent module layout; moving all source `mod.rs` files in one focused plan keeps the convention coherent.
  - User approval: yes.

## Notes
- Migration map:
  - `src/api/mod.rs` -> `src/api.rs`
  - `src/api/types/mod.rs` -> `src/api/types.rs`
  - `src/api/types/domain/mod.rs` -> `src/api/types/domain.rs`
  - `src/internal/mod.rs` -> `src/internal.rs`
  - `src/internal/config/mod.rs` -> `src/internal/config.rs`
  - `src/internal/config/settings/mod.rs` -> `src/internal/config/settings.rs`
  - `src/internal/infrastructures/mod.rs` -> `src/internal/infrastructures.rs`
  - `src/internal/infrastructures/external_services/mod.rs` -> `src/internal/infrastructures/external_services.rs`
  - `src/internal/models/mod.rs` -> `src/internal/models.rs`
  - `src/internal/models/memory/mod.rs` -> `src/internal/models/memory.rs`
  - `src/internal/models/vector/mod.rs` -> `src/internal/models/vector.rs`
  - `src/internal/repositories/mod.rs` -> `src/internal/repositories.rs`
