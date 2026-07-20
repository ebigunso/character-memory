---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
rule_file: "worker"
last_updated: "2026-07-21"
---

# Worker Repository Rules
## Repo-Specific Worker Notes

- When adding or extending a public options type, include at least one test per option toggled independently of its commonly paired option (cross-product spot checks on defaults), not only matched-pair combinations.
- When local integration evidence matters for a specific test, rerun it in a targeted invocation and confirm no skip message was printed for it; aggregate green runs with skip messages are incomplete evidence.
- When changing test skip-gating predicates, verify each branch against the value the producing site actually emits (trace the string from producer to matcher, or run once with the service deliberately down); waived or skipped suites do not exercise these branches.
- Targeted test evidence must state the executed-test count, and zero-executed runs are invalid evidence: with `--exact`, always use the fully qualified module path, and treat "0 passed; 0 failed" as a filter bug, not a pass (recurred 2026-07-18 across CM worker and evals reviewer).
- Tests must not lock in low-value specifics: assert error/warning kinds plus load-bearing tokens (offending key, widths, ids), never full message phrasing, log formatting, or incidental parameter values; a pinned value is contractual only when drift in it means silent scope or behavior change (user-directed 2026-07-20).
- Workaround Tripwire (see common.md): when implementation is going around a type, signature, schema, boundary, or dispatch constraint where changing that thing would be the cleaner design, stop that chunk and escalate to the Orchestrator with the alternative and cost delta before implementing through it. Dispatch constraints such as "minimal diff", "no new public types", or "keep the signature" are instrumental, not terminal — when one forces a workaround, escalation outranks compliance (user-directed 2026-07-21).

## Repo CI / Checks Mapping

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
