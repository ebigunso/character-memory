---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
rule_file: "orchestrator"
last_updated: "2026-07-23"
---

# Orchestrator Repository Rules
## Repo-Specific Orchestrator Policies

- When creating or updating a PR, follow the format specified in `.github/pull_request_template.md`.
- Layer-boundary reorganizations must include a `use crate::` dependency-direction audit as required Reviewer evidence per ADR-I-0018 (ports/policy/models never import usecases, and import api only for the ADR's one named exception: the `api::types::retrieval` trace/telemetry vocabulary; errors/domain import no upper layer); file-placement conformance alone does not catch inverted edges hidden behind re-export shims.
- Scope the ADR-I-0018 dependency-direction audit to the diff under review (e.g. `git diff | grep '^+.*use crate::'`) when reviewing incremental changes: pre-existing ports/policy/models imports of domain types via `crate::api::types` are grandfathered debt awaiting a one-time sweep to `crate::domain`, and a blanket grep forces per-line disambiguation between old and newly introduced edges.
- PR feedback monitors must include terminal merged/closed state (harness pr-review-monitoring owns arming for reviews/comments but does not cover terminal-state watch) (user-directed 2026-07-19).
- When requesting Copilot PR re-review, try the normal reviewer path first with PowerShell-safe quoting: `gh pr edit PR_NUMBER --add-reviewer '@copilot'`. If Copilot has already reviewed the PR, that command returns success WITHOUT starting a review (verify with `gh pr view PR_NUMBER --json reviewRequests` — empty means it no-opped). The working fallback (verified 2026-07-11) is REST: `gh api repos/OWNER/REPO/pulls/PR_NUMBER/requested_reviewers -f 'reviewers[]=copilot'` — the login is plain `copilot`; `copilot-pull-request-reviewer` is rejected as a non-collaborator, and the GraphQL `requestReviews` mutation no longer accepts `userLogins`. Confirm success by `requested_reviewers` containing `Copilot` in the response.

## Delegation Routing (repo scheduling; model-strength routing is harness-owned)

- Model-strength/platform routing is harness-owned: subagent-strategy `model-routing.md`. Repo policy retained: a reviewer is never the same agent identity that authored the diff, on any platform; routine impl diffs get Tier D only; design docs get Tier A only; milestone gates get both tiers in parallel.

## Push Sequencing (internal review before external review)

- A push that triggers external review (opening a PR, or pushing commits to a branch with an open PR) is the promotion step from internally-approved to externally-visible: it happens only AFTER the internal Tier D verdict covering those commits is APPROVED (user-directed 2026-07-22).
- Workers commit locally and do not push; internal reviewers pin worktrees from the local repository, so review never requires the remote. Dispatch prompts must not pair "push" with "reviewer bounce follows".
- Exceptions, each explicit per instance: docs-only commits with no review obligation; a CI-environment behavior that genuinely cannot be reproduced locally (orchestrator-ruled, with the reason recorded).
- Rationale: external reviewers (Copilot) should spend their rounds on internally-approved code, not re-discover defects the internal pass was already catching; overlapping the two layers wastes external rounds and creates thread churn.

## Value-Audit Triggers (design-value review scheduling)

- Design-value audit verdict mechanics and triggers are harness-owned (long-horizon-audit appendix; third-bounce and pre-merge-churn triggers). Repo policy retained: the audit is judged against this repo's roadmap deliverables and philosophy — does it serve a meaningful purpose NOW — and is assigned to a Claude Tier A agent.

## Design-Consult Threshold (coordination/advice separation)

- Escalation-ruling tiers and the blast-radius obligation are harness-owned (lifecycle-gates Escalation Ruling). Repo policy retained, stronger than the harness default: contract-shape escalations — public API surfaces, serialization schemas, cross-repo obligations, deferral-boundary questions — REQUIRE a design consult before ruling.
- The design consult is a dispatched Claude design/Tier-A agent holding the design doc and its amendments as resident context, asked what the proposed shape implies for the whole contract; when genuine urgency forbids the round trip, the orchestrator runs the full blast-radius checklist itself and records in the ruling that the consult was skipped and why.
- Rationale: coordination runs at interrupt tempo and biases rulings toward the proposal's local elegance; the phase's defective rulings were all contract-shape decisions made at coordination tempo, while every altitude decision routed through a dedicated design agent held up.

## Repo-Specific Integration / Git Policy

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
