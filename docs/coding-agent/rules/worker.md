---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
rule_file: "worker"
last_updated: "2026-07-18"
---

# Worker Repository Rules
## Repo-Specific Worker Notes

- When adding or extending a public options type, include at least one test per option toggled independently of its commonly paired option (cross-product spot checks on defaults), not only matched-pair combinations.
- When local integration evidence matters for a specific test, rerun it in a targeted invocation and confirm no skip message was printed for it; aggregate green runs with skip messages are incomplete evidence.
- When changing test skip-gating predicates, verify each branch against the value the producing site actually emits (trace the string from producer to matcher, or run once with the service deliberately down); waived or skipped suites do not exercise these branches.
- Targeted test evidence must state the executed-test count, and zero-executed runs are invalid evidence: with `--exact`, always use the fully qualified module path, and treat "0 passed; 0 failed" as a filter bug, not a pass (recurred 2026-07-18 across CM worker and evals reviewer).

## Repo CI / Checks Mapping

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
