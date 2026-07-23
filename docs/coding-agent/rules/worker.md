---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
rule_file: "worker"
last_updated: "2026-07-23"
---

# Worker Repository Rules
## Repo-Specific Worker Notes

- When adding or extending a public options type, include at least one test per option toggled independently of its commonly paired option (cross-product spot checks on defaults), not only matched-pair combinations.
- When changing test skip-gating predicates, verify each branch against the value the producing site actually emits (trace the string from producer to matcher); waived or skipped suites do not exercise these branches.
- Qdrant client deadlines must be configured through `QdrantConfig::timeout`; per-request builder `.timeout()` may be used only for an intentionally server-side operation limit verified by a live probe (lesson 2026-07-03: 2.41s -> 0.048s).
- Diagnostics over generated plans must trace every compared field through the production-default constructor and include a negative regression using production-default options (lesson 2026-07-18).
- New validators and admission checks classify their failures with an owned structured error type AT INTRODUCTION (typed variants/fields per the design's error conventions), with tests asserting variants and fields; anyhow/prose belongs only at outer boundaries. Three same-phase recurrences of retrofitting prose validators forced this rule (2026-07-23).

## Repo CI / Checks Mapping

- None yet.

## Global Migration Candidates (Placeholder)

- Any compatibility or reader-behavior claim in a design or evidence document that names a concrete reader path must cite the file:line of that reader; tolerance verified for one artifact family must never be generalized to another by analogy (Tier A finding 2026-07-21: a "tolerant 1.0.0 result reader" was asserted that did not exist).
