---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
rule_file: "reviewer"
last_updated: "2026-07-19"
---

# Reviewer Repository Rules

## Repo-Specific Reviewer Notes

- Layer-boundary changes require a `use crate::` dependency-direction audit (errors/domain/ports/policy must not import api or usecases); scope the audit to the diff under review for incremental changes because pre-existing ports/policy imports of domain types via `crate::api::types` are grandfathered debt.
- Review in an isolated pinned worktree at the exact commit under review, never in a shared working checkout; state the pinned commit and any sibling-repo provenance in the verdict.
- Live-service evidence must state the endpoint used and a pass/skip census; zero-executed targeted test runs are invalid evidence.

## Review Risk Hotspots

Suggested durable review categories:
- public_api_compatibility
- public_surface_completeness
- diagnostic_fidelity
- build_config_parity
- strict_ci_hygiene
- entrypoint_intent
- admission_before_side_effect
- collection_semantics
- runtime_model_compatibility
- abstraction_value_searchability
- canonical_policy_path
- authority_vs_derived_data
- failure_mode_completeness
- semantic_consistency
- validation_boundary_correctness
- risk_based_test_coverage

## Required Reviewer-Owned Evidence

| Trigger | Evidence Required | Source |
|---|---|---|
| Layer-boundary or module reorganization diffs | Diff-scoped dependency-direction audit result | orchestrator.md audit policy |
| Retrieval or entity-policy changes | Entity-neutrality check (no name/role special-casing) | roadmap invariant 2.7 |
| Live integration evidence | Endpoint + pass/skip census; no silent skips | worker.md skip-gating notes |

## Review Heuristics

- Verify factual claims in documentation against the canonical artifact bytes (counts, hashes, equalities) rather than trusting supplied wording.
- For additive-telemetry or additive-diagnostic diffs, independently confirm the pre-existing outcome fields are byte-identical when the new feature is inactive.

## Recurring Misses And Prevention

- Acceptance claims must be reconciled to the pinned commit's timeline before assigning severity (state at the reviewed commit can legitimately predate later work).

## Mechanical Gate Candidates

- None.
