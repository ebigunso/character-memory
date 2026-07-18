---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
lifecycle_manifest: "docs/coding-agent/rules/_lifecycle.json"
required_files:
  - "common.md"
  - "worker.md"
  - "orchestrator.md"
  - "reviewer.md"
---

# Coding Agent Rules Index

Read these files by role:

- `common.md`: shared repository facts, validation contract, safety boundaries.
- `worker.md`: Worker execution and validation mapping.
- `orchestrator.md`: planning, dispatch, integration, git, and rule maintenance.
- `reviewer.md`: review-specific repository policy and recurring risk hotspots.

## Rule Freshness

Do not read `_lifecycle.json` during normal work.

Use `_lifecycle.json` only when:
- required rule files are missing or suite IDs do not match;
- schema migration or rule-suite repair is needed;
- the task changes CI, validation, build, agent-instruction, or rule-source files;
- repository facts contradict the rules;
- targeted rule refresh is needed.
