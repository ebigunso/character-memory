# Orchestrator Repository Rules

last_updated: 2026-04-30

## Repo-Specific Orchestrator Policies

- When creating or updating a PR, follow the format specified in `.github/pull_request_template.md`.
- When requesting Copilot PR re-review, try the normal reviewer path first with PowerShell-safe quoting: `gh pr edit PR_NUMBER --add-reviewer '@copilot'`. If Copilot has already reviewed the PR and that command returns success without starting a review, use GitHub GraphQL `requestReviewsByLogin` with `userLogins: ["copilot-pull-request-reviewer"]`, then verify `reviewRequests` or a new `latestReviews` entry instead of trusting the `gh pr edit` exit code alone.

## Repo-Specific Integration / Git Policy

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
