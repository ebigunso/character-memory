# Orchestrator Repository Rules

last_updated: 2026-07-11

## Repo-Specific Orchestrator Policies

- When creating or updating a PR, follow the format specified in `.github/pull_request_template.md`.
- Layer-boundary reorganizations must include a `use crate::` dependency-direction audit as required Reviewer evidence (errors/domain/ports/policy must not import api or usecases); file-placement conformance alone does not catch inverted edges hidden behind re-export shims.
- Scope the ADR-I-0018 dependency-direction audit to the diff under review (e.g. `git diff | grep '^+.*use crate::'`) when reviewing incremental changes: pre-existing ports/policy imports of domain types via `crate::api::types` are grandfathered debt awaiting a one-time sweep to `crate::domain`, and a blanket grep forces per-line disambiguation between old and newly introduced edges.
- When requesting Copilot PR re-review, try the normal reviewer path first with PowerShell-safe quoting: `gh pr edit PR_NUMBER --add-reviewer '@copilot'`. If Copilot has already reviewed the PR and that command returns success without starting a review, use GitHub GraphQL `requestReviewsByLogin` with `userLogins: ["copilot-pull-request-reviewer"]`, then verify `reviewRequests` or a new `latestReviews` entry instead of trusting the `gh pr edit` exit code alone.

## Repo-Specific Integration / Git Policy

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
