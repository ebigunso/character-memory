# Plan: PR 29 CI Trust-Gated Integration Tests

- status: done
- generated: 2026-04-28
- last_updated: 2026-04-28
- work_type: mixed

## Goal
- Address PR #29 review comments by keeping service-free validation on every PR while running live Qdrant-backed integration tests only in trusted same-repository contexts.

## Definition of Done
- Service-free PR jobs remain ungated.
- Live integration tests are gated to trusted same-repository non-Dependabot PRs without using secrets directly in job-level `if:` expressions.
- Untrusted PR contexts show a clear skip explanation instead of failing from missing secrets.
- Same-repository missing live integration configuration fails clearly before service wait/test execution.
- Rust formatting workflow pins the stable toolchain consistently with PR validation.
- PR checklist wording matches the trusted-context live integration model.

## Scope / Non-goals
- Scope:
  - `.github/workflows/pr_validation.yaml`
  - `.github/workflows/check_formatting.yaml`
  - `.github/pull_request_template.md`
  - `docs/coding-agent/plans/active/pr-29-ci-trust-gated-integration-tests-plan.md`
- Non-goals:
  - Changing Rust production or test code.
  - Adding the future service-free fake-backed integration suite in this PR comment follow-up.
  - Using `pull_request_target` or exposing secrets to untrusted PR code.

## Context (workspace)
- Related files/areas:
  - `.github/workflows/pr_validation.yaml`
  - `.github/workflows/check_formatting.yaml`
  - `.github/pull_request_template.md`
  - PR #29 review comments
- Existing patterns or references:
  - PR validation already has always-on service-free jobs for `cargo check`, `cargo test --no-run`, and clippy.
  - The settings loader fails when live integration environment values are missing.
  - Formatting workflow currently installs rustfmt without explicitly selecting `stable`.
- Repo reference docs consulted:
  - None listed in repo rules.

## Open Questions (max 3)
- None. User approved the trusted-context live integration approach.

## Assumptions
- A1: Fork and Dependabot PRs should still run service-free validation, but should not run privileged live integration tests.
- A2: Missing live integration config in same-repository PRs is a repository misconfiguration and should fail clearly.
- A3: The future service-free fake-backed integration suite will be handled outside this PR follow-up.

## Tasks

### Task_1: Apply CI trust gates and template wording
- type: impl
- owns:
  - `.github/workflows/pr_validation.yaml`
  - `.github/workflows/check_formatting.yaml`
  - `.github/pull_request_template.md`
- depends_on: []
- description: |
  Add trusted-context gating and skip explanation for live integration tests, add same-repo live config preflight, pin rustfmt workflow to stable, and align PR checklist wording.
- acceptance:
  - Service-free jobs remain unchanged and ungated.
  - Live Qdrant-backed integration job runs only when the PR head repo is the same repository and the actor is not Dependabot.
  - Untrusted/fork/Dependabot contexts get an explanatory skip job.
  - Same-repo live integration job validates required env values before waiting on Qdrant.
  - Formatting workflow uses `toolchain: stable`, `override: true`, and `components: rustfmt`.
  - PR checklist describes live integration tests as trusted same-repo CI or local service-configured validation.
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
    detail: "Review workflow/template diff against PR comments and approved trust model"

### Task_2: Final review and PR update
- type: review
- owns: []
- depends_on: [Task_1]
- description: |
  Review evidence, push the branch update, and verify PR checks/comments state.
- acceptance:
  - Reviewer approves the remediation diff or issues are fixed/waived.
  - Branch update is pushed to PR #29.
  - Final status clearly reports validation and any remaining CI state.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Final remediation review"
  - kind: command
    required: true
    owner: orchestrator
    detail: "git status --short --branch"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]

## E2E / Visual Validation Spec

- Not applicable; no UI or browser user flow is impacted.

## Rollback / Safety
- Revert the workflow/template diff to return PR #29 to its previous CI behavior.

## Progress Log (append-only)

- 2026-04-28 Wave 2 completed: [Task_2]
  - Summary: Final Reviewer approved the CI trust-gate remediation with no findings.
  - Validation evidence: Reviewer confirmed service-free jobs remain ungated, live integration is same-repo/non-Dependabot gated, no job-level `if:` references secrets, skip/preflight behavior is clear, rustfmt is pinned to stable, and PR template wording matches the trust model.
  - Notes: Optional `actionlint` was unavailable locally (`command -v actionlint` exited 1 with no output), so it was not treated as required evidence.

- 2026-04-28 Wave 1 completed: [Task_1]
  - Summary: Added trusted-context gating for Qdrant-backed integration tests, an explanatory skip job for fork/Dependabot PRs, live config preflight, stable rustfmt setup, and matching PR checklist wording.
  - Validation evidence: `cargo fmt --check` passed; `cargo check` passed; `cargo test --no-run` passed; `cargo clippy --all-targets -- -D warnings` passed.
  - Notes: Service-free jobs remain ungated and no job-level condition references secrets directly.

- 2026-04-28 Plan approved and execution started.
  - Summary: User approved trusted-context live integration tests plus service-free coverage for all PRs.
  - Validation evidence: Pending Task_1 implementation.
  - Notes: Pre-dispatch checks passed: Task_1 acceptance fits its owns scope, and required validation owners are explicit.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-04-28 Decision: Trust-gate live integration tests instead of skipping all integration coverage for outside contributors.
  - Trigger / new insight: PR review noted missing secrets/vars can fail fork PRs before live integration tests can run meaningfully.
  - Plan delta (what changed): Keep service-free jobs always-on; gate live service-backed job to trusted same-repository non-Dependabot PRs; add explanatory skip job for untrusted contexts.
  - Tradeoffs considered: Direct secret/var checks in job-level `if:` are brittle and can hide same-repo misconfiguration. A trusted-context gate plus preflight keeps failures meaningful.
  - User approval: yes

## Notes
- Risks:
  - Branch protection that requires a specific live integration check name may need review if skip contexts should be mergeable.
  - Service-free fake-backed integration tests are still future work and are not added in this follow-up.
- Edge cases:
  - Dependabot is treated as untrusted for live secret-backed execution.
