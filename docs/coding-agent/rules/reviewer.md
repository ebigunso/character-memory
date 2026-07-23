---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
rule_file: "reviewer"
last_updated: "2026-07-23"
---

# Reviewer Repository Rules

## Repo-Specific Reviewer Notes

- Layer-boundary changes require a `use crate::` dependency-direction audit per ADR-I-0018: `ports`/`policy`/`models` never import `usecases`, and import `api` only for the one named exception (the `api::types::retrieval` trace/telemetry vocabulary in their result contracts — a valid edge, not a violation); domain types must come from `crate::domain`. Scope the audit to the diff under review for incremental changes; pre-existing ports/policy/models imports of DOMAIN types via `crate::api::types` are grandfathered debt awaiting a one-time sweep.
- Review in an isolated pinned worktree at the exact commit under review, never in a shared working checkout; state the pinned commit and any sibling-repo provenance in the verdict.
- Live-service evidence must state the endpoint used and a pass/skip census.

## Review Risk Hotspots

- Generic risk categories are harness-owned: route via engineering-quality-baselines `review-latent-risk.md` and its shards (state, failure, contract-scope, performance, future-surface, validation-tests, public-api, entrypoints-admission, diagnostics, build-ci, conservation).

## Required Reviewer-Owned Evidence

| Trigger | Evidence Required | Source |
|---|---|---|
| Layer-boundary or module reorganization diffs | Diff-scoped dependency-direction audit result | ADR-I-0018 (canonical rule incl. the retrieval-telemetry exception) |
| Retrieval or entity-policy changes | Entity-neutrality check (no name/role special-casing) | roadmap invariant 2.7 |
| Live integration evidence | Endpoint + pass/skip census; no silent skips | worker.md skip-gating notes |
| Pruning or closed-contract changes | Touched-file suppression census; bidirectional totality evidence for each claimed single source; empty/non-empty parity tests for every adapter implementing the port | lesson 2026-07-21 (Task_3 pruning wave) |

## Review Heuristics

- Verify factual claims in documentation against the canonical artifact bytes (counts, hashes, equalities) rather than trusting supplied wording.
- For additive-telemetry or additive-diagnostic diffs, independently confirm the pre-existing outcome fields are byte-identical when the new feature is inactive.

## Recurring Misses And Prevention

- Acceptance claims must be reconciled to the pinned commit's timeline before assigning severity (state at the reviewed commit can legitimately predate later work).

## Mechanical Gate Candidates


- When reviewing compatibility claims (schema bumps, sealed-artifact readability, legacy tolerance), verify the named reader at its cited file:line yourself; a claim without a reader citation is an automatic finding (Tier A catch 2026-07-21: asserted result-reader tolerance belonged to a different artifact family).
