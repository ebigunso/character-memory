---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
rule_file: "orchestrator"
last_updated: "2026-07-19"
---

# Orchestrator Repository Rules
## Repo-Specific Orchestrator Policies

- When creating or updating a PR, follow the format specified in `.github/pull_request_template.md`.
- Layer-boundary reorganizations must include a `use crate::` dependency-direction audit as required Reviewer evidence per ADR-I-0018 (ports/policy/models never import usecases, and import api only for the ADR's one named exception: the `api::types::retrieval` trace/telemetry vocabulary; errors/domain import no upper layer); file-placement conformance alone does not catch inverted edges hidden behind re-export shims.
- Scope the ADR-I-0018 dependency-direction audit to the diff under review (e.g. `git diff | grep '^+.*use crate::'`) when reviewing incremental changes: pre-existing ports/policy/models imports of domain types via `crate::api::types` are grandfathered debt awaiting a one-time sweep to `crate::domain`, and a blanket grep forces per-line disambiguation between old and newly introduced edges.
- When requesting Copilot PR re-review, try the normal reviewer path first with PowerShell-safe quoting: `gh pr edit PR_NUMBER --add-reviewer '@copilot'`. If Copilot has already reviewed the PR, that command returns success WITHOUT starting a review (verify with `gh pr view PR_NUMBER --json reviewRequests` — empty means it no-opped). The working fallback (verified 2026-07-11) is REST: `gh api repos/OWNER/REPO/pulls/PR_NUMBER/requested_reviewers -f 'reviewers[]=copilot'` — the login is plain `copilot`; `copilot-pull-request-reviewer` is rejected as a non-collaborator, and the GraphQL `requestReviews` mutation no longer accepts `userLogins`. Confirm success by `requested_reviewers` containing `Copilot` in the response.

## Delegation Routing (model-strength aware platform recommendation; user-approved 2026-07-11)

- When both Claude and Codex delegation targets are available at runtime, prefer routing by failure mode: if a miss would be a subtle bug or overlooked line, prefer a Codex agent (detail scrutiny); if a miss would be building the wrong thing well, prefer a Claude agent (altitude and lateral judgment). If only one platform is available, any agent may take any role.
- Research: prefer Claude for exploratory research (design-space surveys, alternatives with tradeoffs, cross-repo implications); prefer Codex for forensic research (exhaustive inventories with file:line evidence, call-site censuses, computability tables).
- Review tiers: Tier D defect/compliance review (post-implementation diff correctness, dependency-direction and entity-neutrality audits, serde/schema verification, determinism sweeps, acceptance-evidence checking) prefers a Codex reviewer — never the same agent identity that authored the diff, on any platform. Tier A altitude review (design/plan soundness, goal-achievement and what-will-bite-later review) prefers a Claude reviewer. Routine impl diffs get Tier D only; design docs get Tier A only; milestone gates get both tiers in parallel.
- Implementation prefers Codex workers; give creative-design subtasks a Claude design pass first and hand the implementing worker a spec.

## Repo-Specific Integration / Git Policy

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
