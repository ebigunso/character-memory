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
