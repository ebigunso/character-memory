# Orchestrator Repository Rules

last_updated: 2026-07-11

## Repo-Specific Orchestrator Policies

- When creating or updating a PR, follow the format specified in `.github/pull_request_template.md`.
- Layer-boundary reorganizations must include a `use crate::` dependency-direction audit as required Reviewer evidence (errors/domain/ports/policy must not import api or usecases); file-placement conformance alone does not catch inverted edges hidden behind re-export shims.
- Scope the ADR-I-0018 dependency-direction audit to the diff under review (e.g. `git diff | grep '^+.*use crate::'`) when reviewing incremental changes: pre-existing ports/policy imports of domain types via `crate::api::types` are grandfathered debt awaiting a one-time sweep to `crate::domain`, and a blanket grep forces per-line disambiguation between old and newly introduced edges.
- When requesting Copilot PR re-review, try the normal reviewer path first with PowerShell-safe quoting: `gh pr edit PR_NUMBER --add-reviewer '@copilot'`. If Copilot has already reviewed the PR and that command returns success without starting a review, use GitHub GraphQL `requestReviewsByLogin` with `userLogins: ["copilot-pull-request-reviewer"]`, then verify `reviewRequests` or a new `latestReviews` entry instead of trusting the `gh pr edit` exit code alone.

## Delegation Routing (model-strength aware; user-approved 2026-07-11)

- Route by failure mode: if a miss would be a subtle bug or overlooked line, delegate to a Codex agent (detail scrutiny); if a miss would be building the wrong thing well, delegate to a Claude agent (altitude and lateral judgment).
- Research: exploratory research (design-space surveys, alternatives with tradeoffs, cross-repo implications) goes to Claude researcher subagents; forensic research (exhaustive inventories with file:line evidence, call-site censuses, computability tables) goes to Codex agents via agmsg.
- Review tiers: Tier D defect/compliance review (post-implementation diff correctness, dependency-direction and entity-neutrality audits, serde/schema verification, determinism sweeps, acceptance-evidence checking) goes to the Codex `cm-reviewer` agent via agmsg — never to the Codex identity that authored the diff. Tier A altitude review (design/plan soundness, goal-achievement and what-will-bite-later review) goes to Claude reviewer subagents. Routine impl diffs get Tier D only; design docs get Tier A only; milestone gates get both tiers in parallel.
- Workers stay Codex; give creative-design subtasks a Claude design pass first and hand the Codex worker a spec.

## Repo-Specific Integration / Git Policy

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
