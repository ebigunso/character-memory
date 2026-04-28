# Lessons Log (Coding Agent)

Purpose:
- capture recurring mistakes and the prevention mechanism
- enable "read once, don't repeat" improvements

## How to use
- Append a new entry after any user correction or significant miss.
- Keep entries short and actionable.
- Promote repeated/high-severity lessons into repo rules, first-party skills/references, or troubleshooting knowledge.

## Tags (recommended)
- planning
- validation
- delegation
- review
- ui-e2e
- tooling
- ci
- scope-owns

## Entries

## 2026-04-28 - Distinguish Temporary And Durable Code Comments  [tags: code-quality, communication, architecture]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-remember-and-link-pipelines-plan.md`
- Task/Wave: plan decision refinement before implementation
- Roles involved: Orchestrator

Symptom:
- The user clarified that comments should communicate whether a structure is temporary migration scaffolding or durable production API/design surface.

Root cause:
- Planning could otherwise treat all comments as generic explanation, leaving future Workers/Reviewers unsure which code should be removed later and which code is intended to survive the complete v0.1 refactor.

Fix applied:
- Updated the remember/link plan to require removal-condition comments for temporary scaffolding and stable production-ready comments for durable injectable constructor/API structures.

Prevention:
- When adding comments during v0.1 refactor work, explicitly choose the comment category: temporary comments name when to remove/change the code; durable comments describe stable intent without implying future cleanup.
- Reviewers should flag transitional comments without removal conditions and durable API comments that read like temporary scaffolding.

Evidence:
- Active remember/link plan now includes resolved decision and Task_1/Task_5 acceptance coverage for temporary-vs-durable comment guidance.

## 2026-04-28 - Parallelize Review Loops And Avoid Token-Burning Waits  [tags: delegation, review, tooling]

Context:
- Plan: PR #31 Copilot review remediation
- Task/Wave: PR comment triage and re-review loop
- Roles involved: Orchestrator

Symptom:
- The user clarified that review/remediation loops should use subagents as much as possible and should wait in ways that do not burn inference tokens.

Root cause:
- The main thread was carrying too much review/verification work directly and risked treating periodic Copilot polling as an active waiting loop.

Fix applied:
- Delegated focused remediation review to Reviewer subagents, kept the main thread to orchestration and decisions, and avoided sleep/poll loops.

Prevention:
- For PR review remediation, split independent review aspects into Reviewer subagents and use main-thread checks only for state transitions, validation evidence, or user/terminal notifications.
- Do not run token-burning polling loops while waiting for external review; use non-interactive status checks only when prompted by a state change or after returning control.

Evidence:
- PR #31 Copilot remediation used focused Reviewer subagents for scoped patch review and validation confirmation.

## 2026-04-28 - Avoid Separate Skipped Checks For CI Rationale  [tags: ci, review, communication]

Context:
- Plan: PR #29 CI trust-gated integration test follow-up
- Task/Wave: PR review follow-up
- Roles involved: Orchestrator

Symptom:
- Added a separate `integration_tests_skipped` job to explain why live integration tests do not run for fork/Dependabot PRs.
- User clarified that surfacing the explanation as its own skipped check is confusing.

Root cause:
- Treated visible CI explanation as equivalent to a dedicated check, without considering how that extra check appears in the PR status UI.

Fix applied:
- Removed the separate skipped-check job and moved the rationale into comments on the actual live integration-test job.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: Prefer inline workflow comments or existing job/step logs for CI rationale; do not add separate skipped check jobs solely for explanation unless the user wants that PR checks UI.
- Dispatch/plan guardrail:
  - When adding skipped CI jobs, explicitly consider whether the extra check improves or clutters the PR status surface.

Evidence:
- PR #29 follow-up removed `integration_tests_skipped` and kept the trust-gating rationale near the `integration_tests` job condition.

## 2026-04-28 - Quote PR Bodies As Literal Input  [tags: tooling, git, output-contract]

Context:
- Plan: none
- Task/Wave: PR creation
- Roles involved: Orchestrator

Symptom:
- Initial `gh pr create --body "..."` attempt treated markdown backticks in the PR body as shell command substitutions.
- The shell tried to execute commands and markdown file paths before the PR was created.

Root cause:
- Passed a markdown PR body containing backticks through a double-quoted shell argument.

Fix applied:
- Retried PR creation with `gh pr create --body-file -` and a single-quoted heredoc delimiter so the body was passed literally.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: When creating or updating PR bodies from shell, pass markdown through a literal file/stdin path such as `--body-file - <<'EOF'` instead of a double-quoted `--body` argument.
- Dispatch/plan guardrail:
  - For PR bodies containing backticks, checkboxes, or command snippets, use literal stdin/file input before running `gh pr create` or `gh pr edit`.

Evidence:
- Retry created PR #29 successfully: https://github.com/ebigunso/CharacterMemory/pull/29

## 2026-04-28 - Explain Temporary Suppressions  [tags: review, code-quality, communication]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-store-contracts-test-harness-plan.md`
- Task/Wave: PR review follow-up
- Roles involved: Orchestrator

Symptom:
- Added `allow` suppressions for transitional v0.1 contract/test-support code without explaining why they exist or when they can be removed.
- User clarified that code should not just work without context when the safe revision point is not obvious.

Root cause:
- Treated the suppression as self-explanatory implementation scaffolding instead of documenting the temporary boundary it protects.

Fix applied:
- Added concise rationale and removal-condition comments for the transitional `dead_code` and `unused_imports` suppressions.

Prevention:
- Repo rule candidate:
  - audience: common
  - proposed rule: Temporary lint suppressions should explain why they exist and when they can be safely removed, unless the reason is obvious from nearby code.
- Dispatch/plan guardrail:
  - During review follow-up, inspect newly added `allow` attributes for rationale and removal conditions before marking comments resolved.

Evidence:
- Suppression comments added to v0.1 store contract and test-support modules.

## 2026-04-27 - Hidden PR Template Discovery  [tags: tooling, output-contract]

Context:
- Plan: none
- Task/Wave: PR creation
- Roles involved: Orchestrator

Symptom:
- Created PR #22 with a generic Summary/Testing body after failing to find the repository PR template.
- User later provided `.github/pull_request_template.md`, which should have been used.

Root cause:
- The template search used `rg --files` without `--hidden`, so the hidden `.github/` directory was missed.

Fix applied:
- Read `.github/pull_request_template.md`, ran the template checklist commands, and updated the PR body to follow the template.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: When creating or updating a PR, search hidden template locations such as `.github/pull_request_template.md` before assuming no template exists.
- Dispatch/plan guardrail:
  - Include hidden `.github/**` in PR-template discovery commands.

Evidence:
- `cargo check --all-targets` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test --verbose` passed.

## 2026-04-27 - PR Description Final-State Framing  [tags: output-contract, communication]

Context:
- Plan: none
- Task/Wave: PR description update
- Roles involved: Orchestrator

Symptom:
- PR description described additions as a sequence of branch edits and follow-up metadata changes.
- This exposed intermediary states that reviewers do not see when evaluating the final PR diff.

Root cause:
- The PR body was written from the agent's work history rather than from the reviewer's final-state perspective.

Fix applied:
- Rewrote PR #25 description to describe the documentation set as it exists in the final branch state.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: PR descriptions should describe the final branch state and reviewer-facing intent, not intermediary implementation states or correction history.
- Dispatch/plan guardrail:
  - Before updating a PR body, review it for phrases that imply hidden intermediate states when a final-state description would be clearer.

Evidence:
- PR #25 body now describes the philosophy doc, roadmap, roadmap phase docs, and ADR set as final additions.

## 2026-04-27 - Roadmap Granularity For Large Implementation Phases  [tags: planning, scope-owns, output-contract]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap-plan.md`
- Task/Wave: roadmap drafting
- Roles involved: Orchestrator

Symptom:
- Drafted concrete Task_X implementation plans for many downstream v0.1 chunks before the first chunk had resolved model, store, and validation shape.
- User clarified that the roadmap should contain high-level chunks for the whole implementation phase, with concrete plans drafted chunk-by-chunk as work reaches them.

Root cause:
- Treated the implementation-phase roadmap as a full execution plan for every future chunk, creating false precision for work that depends on earlier discoveries.

Fix applied:
- Reworked the roadmap into high-level implementation chunks and kept concrete Task_X detail only for Chunk 1.

Prevention:
- For large multi-phase implementation roadmaps, keep future phases/chunks at roadmap granularity unless the user asks for full execution detail.
- Add concrete Task_X plans only for the next executable chunk, then draft the next chunk plan from the code and decisions that actually landed.

Evidence:
- Updated active roadmap now separates `High-Level Implementation Roadmap` from `Concrete Plan For Chunk 1`.

## 2026-04-27 - Separate Roadmaps From Concrete Plan Lifecycle  [tags: planning, scope-owns, output-contract]

Context:
- Plan: v0.1 starter episodic memory implementation planning
- Task/Wave: roadmap and first concrete plan drafting
- Roles involved: Orchestrator

Symptom:
- The roadmap still embedded the concrete first-chunk execution plan, making independent plan approval/completion tracking awkward.
- User asked for the roadmap to be separate from concrete implementation plans and for plans to be named by achieved outcome rather than chunk number.

Root cause:
- Treated a roadmap file as a convenient container for the first executable plan instead of giving roadmap and plan artifacts separate lifecycles.

Fix applied:
- Split the combined artifact into `v0-1-starter-episodic-memory-roadmap.md` and `v0-1-domain-foundation-plan.md`.
- Named the concrete plan by the capability it establishes rather than by chunk ordinal.

Prevention:
- For multi-chunk work, keep phase roadmaps and concrete execution plans in separate files when plans need independent completion tracking.
- Name concrete plans by the outcome they achieve, not by sequence number alone.

Evidence:
- Active plans directory now contains a roadmap-only file plus a separate domain-foundation plan file.

## 2026-04-27 - Rust Module Layout And Unit Test Placement  [tags: planning, output-contract, validation]

Context:
- Plan: Rust module file layout migration
- Task/Wave: post-domain-foundation cleanup
- Roles involved: Orchestrator

Symptom:
- Newly added domain code used `src/api/types/domain/mod.rs`, and pure domain tests were added under `tests/` as integration-test targets.
- User clarified the repo should use direct Rust module filenames and reserve `tests/` for integration tests.

Root cause:
- Followed the existing mixed module layout and placed pure domain tests in integration-test files instead of applying the desired Rust 2018-style module and unit-test convention.

Fix applied:
- Migrated source modules away from `mod.rs` files and moved pure domain tests into `src/api/types/domain/tests.rs`.

Prevention:
- Prefer direct module files such as `foo.rs` over `foo/mod.rs` for Rust modules.
- Put unit tests in the same source module tree as the production code they test; use `tests/` only for integration tests.

Evidence:
- Repo rules now record the module layout and test placement convention.

## 2026-04-27 - No Legacy Compatibility Goal For v0.1  [tags: planning, scope-owns, architecture]

Context:
- Plan: v0.1 starter episodic memory roadmap and store contracts planning
- Task/Wave: roadmap correction before next implementation chunk
- Roles involved: Orchestrator

Symptom:
- Roadmap and store-contracts planning still implied that old flat API compatibility or legacy repository paths might be preserved if cheap.
- User clarified that compatibility is not a concern and legacy implementations that do not contribute to the new architecture should be removed.

Root cause:
- Treated the old flat API as a temporary compatibility surface rather than as removable migration residue for the v0.1 rewrite.

Fix applied:
- Updated the roadmap and store-contracts plan context to make legacy compatibility a non-goal for v0.1 work.
- Removed the bounded v0.1 compatibility guidance from repo-wide common rules after user correction.

Prevention:
- Future v0.1 plans should identify legacy pieces that can be removed or replaced, not preserve them for compatibility alone.
- Do not add compatibility wrappers for old flat APIs unless they directly serve the new v0.1 architecture.

Evidence:
- Roadmap resolved decisions now state that legacy implementations which do not contribute to v0.1 should be removed as replacement chunks land.

## 2026-04-27 - Keep Bounded Guidance Out Of Common Rules  [tags: rulebook, scope-owns, planning]

Context:
- Plan: v0.1 roadmap and store-contracts planning correction
- Task/Wave: repo rule cleanup
- Roles involved: Orchestrator

Symptom:
- A v0.1-specific compatibility direction was added to `docs/coding-agent/rules/common.md`.
- User clarified common rules should contain repo-wide rules that always apply, not bounded task or phase guidance.

Root cause:
- Promoted a useful but phase-scoped planning constraint into the repo-wide rulebook instead of keeping it in the roadmap and relevant plans.

Fix applied:
- Removed the v0.1 compatibility bullet from `common.md`.
- Left the bounded guidance in the roadmap and store-contracts plan where it belongs.

Prevention:
- Before editing common rules, check whether the guidance is always repo-wide or only applies to a bounded plan, task, phase, or migration.
- Keep bounded guidance in plans/roadmaps/lessons unless it truly applies across the repository indefinitely.

Evidence:
- `common.md` now contains only repo-wide validation, naming, module-layout, and test-placement rules.
