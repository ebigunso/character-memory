# Plan: Split PR CI Validation

- status: done
- generated: 2026-04-28
- last_updated: 2026-04-28
- work_type: code

## Goal
- Refactor PR CI into smaller, meaningful validation chunks so fast checks complete independently and service-backed tests no longer block unrelated validation feedback.

## Definition of Done
- Formatting, compile check, test compilation, clippy, and service-backed integration tests are represented as separate CI jobs or workflows.
- Repo-required checks from `docs/coding-agent/rules/common.md` are preserved: `cargo fmt --check`, `cargo check`, and `cargo test --no-run`.
- Qdrant service setup and service-backed environment values are scoped only to the integration-test chunk.
- PR checklist text matches the resulting validation model.
- Required local validation evidence is recorded before completion.

## Scope / Non-goals
- Scope:
  - `.github/workflows/check_formatting.yaml`
  - `.github/workflows/pr_validation.yaml`
  - `.github/pull_request_template.md`
- Non-goals:
  - Changing Rust production or test code.
  - Removing integration-test coverage from CI.
  - Introducing new CI actions or caching behavior beyond the existing Rust setup action.

## Context (workspace)
- Related files/areas:
  - `.github/workflows/pr_validation.yaml`
  - `.github/workflows/check_formatting.yaml`
  - `.github/pull_request_template.md`
  - `docs/coding-agent/rules/common.md`
- Existing patterns or references:
  - Current formatting workflow is already separate but only targets PRs to `main`.
  - Current PR validation workflow runs Qdrant startup, `cargo check --all-targets`, `cargo test --verbose`, and clippy serially in one job.
  - Repo rules define service-free required validation commands.
- Repo reference docs consulted:
  - None listed in repo rules.

## Open Questions (max 3)
- Q1: Should full service-backed integration tests remain required on every PR, or be moved to manual/scheduled CI later? This plan keeps them required on PRs unless the user requests otherwise.

## Assumptions
- A1: The user wants faster independent CI chunks, not less validation coverage.
- A2: Keeping `cargo clippy --all-targets -- -D warnings` in PR CI is desired because it already exists in the current workflow and PR checklist.
- A3: `cargo test --no-run` should be the fast test-validation chunk because repo rules explicitly identify it as service-free test target compilation.

## Tasks

### Task_1: Split workflow jobs
- type: impl
- owns:
  - `.github/workflows/check_formatting.yaml`
  - `.github/workflows/pr_validation.yaml`
- depends_on: []
- description: |
  Update workflow definitions so each meaningful validation chunk runs in its own job. Keep service-free jobs free of Qdrant/secrets and scope service setup to the integration-test job.
- acceptance:
  - Formatting workflow checks PRs to both `main` and `develop` and uses `cargo fmt --check`.
  - PR validation contains separate jobs for compile check, test compilation, clippy, and integration tests.
  - Only the integration-test job declares Qdrant service readiness and service/env values.
  - Job names and step names clearly describe their validation chunk.
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
    detail: "cargo clippy --all-targets -- -D warnings"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review workflow diff for trigger consistency, scoped services/secrets, and preserved validation coverage"

### Task_2: Align PR checklist
- type: docs
- owns:
  - `.github/pull_request_template.md`
- depends_on: [Task_1]
- description: |
  Update the PR checklist so contributor-facing validation commands match the split CI model and repo-required service-free checks.
- acceptance:
  - Checklist names `cargo check`, `cargo test --no-run`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` consistently.
  - Checklist distinguishes service-free validation from full integration-test execution.
  - No unrelated PR-template sections are rewritten.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Read the updated PR checklist against workflow commands for consistency"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review PR template diff for command consistency and scope"

### Task_3: Final review gate
- type: review
- owns: []
- depends_on: [Task_1, Task_2]
- description: |
  Perform final review of the full CI refactor and validation evidence.
- acceptance:
  - Reviewer status is APPROVED or required fixes are dispatched before completion.
  - Required validation evidence from Task_1 and Task_2 is present.
  - No required validation item remains implicit or pending.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Final diff review and validation evidence check"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]

## E2E / Visual Validation Spec

- Not applicable; no UI or browser user flow is impacted.

## Rollback / Safety
- Revert the workflow and PR-template diffs to restore the current single PR validation job and existing formatting workflow.

## Progress Log (append-only)

- 2026-04-28 Final trigger cleanup completed.
  - Summary: Kept formatting workflow `push` behavior scoped to `main` while preserving PR checks for both `main` and `develop`.
  - Validation evidence: Scoped diagnostics found no errors in the formatting workflow or completed plan; final Reviewer re-check approved the trigger cleanup with no findings.
  - Notes: This avoids adding a new push-to-`develop` formatting trigger outside the approved PR-focused split.

- 2026-04-28 Wave 3 completed: [Task_3]
  - Summary: Final Reviewer approved the workflow split, PR checklist alignment, and validation evidence with no findings.
  - Validation evidence: Reviewer verified scoped workflow/template acceptance and confirmed required Task_1 and Task_2 evidence exists.
  - Notes: Residual risk is unchanged from the plan: the Qdrant-backed integration job still depends on repository vars/secrets, matching the preserved coverage model.

- 2026-04-28 Wave 2 completed: [Task_2]
  - Summary: PR checklist now matches the split CI model and separates service-free validation from Qdrant-backed integration tests.
  - Validation evidence: Worker reviewed checklist commands against workflows: `cargo check`, `cargo test --no-run`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and Qdrant-backed `cargo test --verbose`.
  - Notes: No unrelated PR-template sections were rewritten.

- 2026-04-28 Wave 1 completed: [Task_1]
  - Summary: Formatting and PR validation workflows were split into explicit Rust formatting, compile check, test compilation, clippy, and Qdrant-backed integration-test chunks.
  - Validation evidence: `cargo fmt --check` passed; `cargo check` passed; `cargo test --no-run` passed; `cargo clippy --all-targets -- -D warnings` passed.
  - Notes: Only the integration-test job declares Qdrant service setup and service-backed env values.

- 2026-04-28 Plan approved; execution started.
  - Summary: User approved the draft plan for implementation.
  - Validation evidence: Pending Task_1 workflow implementation.
  - Notes: Pre-dispatch checks passed: Task_1 acceptance fits its owns scope, and required validation owners are explicit.

- 2026-04-28 Draft created.
  - Summary: Plan drafted from Researcher findings and repo validation rules.
  - Validation evidence: Pending user approval before implementation.
  - Notes: Full integration-test coverage is preserved as a separate PR job by default.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-04-28 Decision: Preserve integration-test coverage while splitting fast checks.
  - Trigger / new insight: Current CI serializes service startup, full tests, compile check, and clippy in one job.
  - Plan delta (what changed): Proposed separate jobs for service-free gates and a dedicated Qdrant-backed integration-test job.
  - Tradeoffs considered: Dropping runtime tests would be faster but would reduce coverage; this draft avoids that behavior loss.
  - User approval: yes

## Notes
- Risks:
  - Replacing full runtime tests with only `cargo test --no-run` would lose integration behavior coverage, so this plan does not remove the runtime test job.
  - The integration-test job will still take as long as current service-backed execution, but it will no longer block early feedback from independent jobs.
- Edge cases:
  - If repository variables are unset, the integration-test job may still fail exactly as the current combined workflow would fail.
