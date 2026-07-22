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
- When opening a PR (or taking ownership of one), arm a review-feedback monitor in the same action: new reviews, inline review comments, issue comments, and terminal merged/closed state (user-directed 2026-07-19).
- When requesting Copilot PR re-review, try the normal reviewer path first with PowerShell-safe quoting: `gh pr edit PR_NUMBER --add-reviewer '@copilot'`. If Copilot has already reviewed the PR, that command returns success WITHOUT starting a review (verify with `gh pr view PR_NUMBER --json reviewRequests` — empty means it no-opped). The working fallback (verified 2026-07-11) is REST: `gh api repos/OWNER/REPO/pulls/PR_NUMBER/requested_reviewers -f 'reviewers[]=copilot'` — the login is plain `copilot`; `copilot-pull-request-reviewer` is rejected as a non-collaborator, and the GraphQL `requestReviews` mutation no longer accepts `userLogins`. Confirm success by `requested_reviewers` containing `Copilot` in the response.

## Delegation Routing (model-strength aware platform recommendation; user-approved 2026-07-11)

- When both Claude and Codex delegation targets are available at runtime, prefer routing by failure mode: if a miss would be a subtle bug or overlooked line, prefer a Codex agent (detail scrutiny); if a miss would be building the wrong thing well, prefer a Claude agent (altitude and lateral judgment). If only one platform is available, any agent may take any role.
- Research: prefer Claude for exploratory research (design-space surveys, alternatives with tradeoffs, cross-repo implications); prefer Codex for forensic research (exhaustive inventories with file:line evidence, call-site censuses, computability tables).
- Review tiers: Tier D defect/compliance review (post-implementation diff correctness, dependency-direction and entity-neutrality audits, serde/schema verification, determinism sweeps, acceptance-evidence checking) prefers a Codex reviewer — never the same agent identity that authored the diff, on any platform. Tier A altitude review (design/plan soundness, goal-achievement and what-will-bite-later review) prefers a Claude reviewer. Routine impl diffs get Tier D only; design docs get Tier A only; milestone gates get both tiers in parallel.
- Implementation prefers Codex workers; give creative-design subtasks a Claude design pass first and hand the implementing worker a spec.
- ADRs and other design-decision records are drafted by the Orchestrator (or a Claude design agent) that holds the decision context; implementation workers may be asked to fact-check file:line claims in a draft, never to author the decision record (user-directed 2026-07-18).

## Push Sequencing (internal review before external review)

- A push that triggers external review (opening a PR, or pushing commits to a branch with an open PR) is the promotion step from internally-approved to externally-visible: it happens only AFTER the internal Tier D verdict covering those commits is APPROVED (user-directed 2026-07-22).
- Workers commit locally and do not push; internal reviewers pin worktrees from the local repository, so review never requires the remote. Dispatch prompts must not pair "push" with "reviewer bounce follows".
- Exceptions, each explicit per instance: docs-only commits with no review obligation; a CI-environment behavior that genuinely cannot be reproduced locally (orchestrator-ruled, with the reason recorded).
- Rationale: external reviewers (Copilot) should spend their rounds on internally-approved code, not re-discover defects the internal pass was already catching; overlapping the two layers wastes external rounds and creates thread churn.

## Value-Audit Triggers (design-value review scheduling)

- The design-value audit is a named review type (user-directed 2026-07-22): a Claude Tier A agent judges each structure against roadmap deliverables and the philosophy — does it serve a meaningful purpose NOW — with verdicts EARNS ITS PLACE / OVERSIZED / DELETE; over-engineering for what "might" happen marks structure as better deleted, per the deletion-first precedent.
- Trigger 1, design review: every structure a design doc proposes must name its concrete consumer — a current caller or a NAMED next-phase deliverable; "future callers" is not a consumer. Apply the same existence question to new structure that reviews apply to inherited code.
- Trigger 2, fix-chain depth: a third bounce on the same seam automatically raises the proportionality question — is the accumulated apparatus still cheaper than stepping back to the simpler contract? — answered explicitly in the Decision Log before round four proceeds (sunk-cost grows with each verified round; the question must be forced from outside the chain).
- Trigger 3, pre-merge after fix churn: a milestone-gate value audit runs alongside the detail-coherence audit before merge — the last moment deletion is cheap, and fix pressure adds structure nobody planned.
- Trigger 4, next-phase planning: audit which prior-phase structures the new plan actually consumes; structures no phase inherits are deletion candidates, and every deferral parked on a then-unwritten phase doc is re-confirmed when that doc is authored, or its "named consumer" is a label.
- Deliberate non-trigger: not at every bounce or ruling — the question needs altitude and accumulated context; asked continuously it degrades into ritual answered reflexively.

## Design-Consult Threshold (coordination/advice separation)

- Escalation rulings split into two tiers (user-directed 2026-07-22, from the project-manager/product-manager analysis): routine escalations (naming, single-variant additions, test shapes, mechanical sequencing) are ruled fast-path WITH the blast-radius checklist; contract-shape escalations — public API surfaces, serialization schemas, cross-repo obligations, deferral-boundary questions — additionally require a design consult BEFORE ruling.
- The design consult is a dispatched Claude design/Tier-A agent holding the design doc and its amendments as resident context, asked what the proposed shape implies for the whole contract; when genuine urgency forbids the round trip, the orchestrator runs the full blast-radius checklist itself and records in the ruling that the consult was skipped and why.
- Rationale: coordination runs at interrupt tempo and biases rulings toward the proposal's local elegance; the phase's defective rulings were all contract-shape decisions made at coordination tempo, while every altitude decision routed through a dedicated design agent held up.

## Workaround Tripwire Obligations

- Treat Worker or Reviewer tripwire escalations (see common.md) as replan triggers: record the ruling in the plan Decision Log before the affected chunk resumes (user-directed 2026-07-21).
- When framing dispatches, do not attach surface-minimizing constraints ("minimal diff", "no new public types", "keep the signature") to contract, diagnostics, or schema work without also stating that preserving existing structure outranks the constraint; a constraint that forces a workaround is the Orchestrator's framing defect, not the Worker's implementation choice.
- Before dispatching a fix for a reported finding, check the proposed fix shape against the owning types and design record; a fix that works around a type it could change is itself a tripwire.
- Ruling scope is the blast radius, not the patch (user-directed 2026-07-22): workers and reviewers legitimately see only the local code they are working on; the Orchestrator's assessment of every escalation is what the change implies for the entirety of what it affects — every consumer (both repos), serialization/schema surfaces, deferred or coordinated scopes, and existing owned contracts. When the Orchestrator's own verification cannot cover that radius quickly, dispatch a researcher subagent (Codex forensic for consumer/call-site censuses, Claude for design-implication surveys) BEFORE ruling, not after the break.

## Repo-Specific Integration / Git Policy

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
