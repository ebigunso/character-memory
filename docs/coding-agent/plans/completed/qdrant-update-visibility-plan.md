# Plan: Qdrant Update Visibility Fix

- status: done
- generated: 2026-04-27
- last_updated: 2026-04-27
- work_type: code

## Goal
- Make memory updates immediately observable by the repository read path so `test_update_memory` no longer reads stale payload data after a successful update.

## Definition of Done
- Qdrant write operations used by memory create/update wait for write application before returning.
- The reported `memory_modification_tests` failure passes against the local test environment.
- Repository-required Rust validation commands pass or are explicitly reported if blocked by the environment.
- Reviewer approves the diff for correctness and scope.

## Scope / Non-goals
- Scope:
  - Qdrant vector repository write visibility for upsert-based memory storage.
  - Targeted validation for memory modification behavior.
- Non-goals:
  - Qdrant client/server version changes.
  - Broader update semantics refactors beyond the reported stale-read failure.
  - Public API changes.

## Context (workspace)
- Related files/areas:
  - `src/internal/infrastructures/external_services/qdrant_vector_memory_repository.rs`
  - `tests/memory_modification_tests.rs`
- Existing patterns or references:
  - `tests/test_utils.rs` waits after create because Qdrant visibility can be asynchronous.
  - Qdrant `UpsertPointsBuilder` supports `wait(true)` to wait until changes are applied.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`

## Open Questions
- None.

## Assumptions
- The Qdrant client/server compatibility warning is incidental to the stale-read symptom, though it can make timing races easier to observe.
- A small write-latency increase is acceptable for correctness in this library path.

## Tasks

### Task_1: Wait for Qdrant upserts
- type: impl
- owns:
  - `src/internal/infrastructures/external_services/qdrant_vector_memory_repository.rs`
- depends_on: []
- description: |
  Add Qdrant write wait semantics to upsert requests used for storing memory points.
- acceptance:
  - `store_memory` upserts wait until changes are applied before returning.
  - `bulk_insert` upserts use the same visibility behavior for consistency.
  - No public API or model shape changes are introduced.
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
    detail: "cargo test --test memory_modification_tests -- --nocapture"

### Task_2: Review Qdrant visibility fix
- type: review
- owns: []
- depends_on: [Task_1]
- description: |
  Review the diff and validation evidence against the stale-read failure.
- acceptance:
  - Reviewer status is APPROVED or any blocking issues are resolved.
  - Required validation evidence from Task_1 is present.
  - Residual risk is documented.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review vs acceptance criteria and validation evidence"

## Task Waves

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]

## Rollback / Safety
- Revert the Qdrant builder option changes if they cause unacceptable write latency or incompatibility.

## Progress Log

- 2026-04-27 Research completed:
  - Summary: Read-only investigation found Qdrant upserts did not wait for write application before immediate retrieval.
  - Validation evidence: Researcher reported targeted local `memory_modification_tests` passed, but user failure and code path indicate a timing race.
  - Notes: Plan targets the persistence boundary rather than adding sleeps to tests.
- 2026-04-27 Wave 1 completed: [Task_1]
  - Summary: Added Qdrant `wait(true)` to upsert requests in `store_memory` and `bulk_insert`.
  - Validation evidence: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and `cargo test --test memory_modification_tests -- --nocapture` passed.
  - Notes: No public API or model changes were introduced.
- 2026-04-27 Wave 2 completed: [Task_2]
  - Summary: Reviewer approved the diff with no findings.
  - Validation evidence: Reviewer confirmed Worker evidence was sufficient and the diff was limited to Qdrant upsert visibility behavior.
  - Notes: Residual risk is low; delete visibility and Qdrant client/server version mismatch remain outside scope.

## Decision Log

- 2026-04-27 Decision:
  - Trigger / new insight: `update_memory` delegates to `store_memory`, whose Qdrant upsert request is built without `wait(true)`.
  - Plan delta: Fix the repository write request and validate with targeted memory modification tests plus repo-required commands.
  - Tradeoffs considered: Waiting for applied writes may add latency but prevents stale immediate reads.
  - User approval: yes.

## Notes
- Risks:
  - Qdrant client/server version mismatch remains outside this fix.
  - `update_nonexistent_memory` semantics may warrant a separate test-strengthening pass, but it is not necessary for the reported stale read.
- Edge cases:
  - Immediate retrieval after create, update, bulk insert, and delete can depend on Qdrant write visibility timing.
