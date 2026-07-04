# Worker Repository Rules

last_updated: 2026-07-04

## Repo-Specific Worker Notes

- When adding or extending a public options type, include at least one test per option toggled independently of its commonly paired option (cross-product spot checks on defaults), not only matched-pair combinations.
- When local integration evidence matters for a specific test, rerun it in a targeted invocation and confirm no skip message was printed for it; aggregate green runs with skip messages are incomplete evidence.
- When changing test skip-gating predicates, verify each branch against the value the producing site actually emits (trace the string from producer to matcher, or run once with the service deliberately down); waived or skipped suites do not exercise these branches.

## Repo CI / Checks Mapping

- None yet.

## Global Migration Candidates (Placeholder)

- None yet.
