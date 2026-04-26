# Plan: CharacterMemory Rename

- status: done
- generated: 2026-04-26
- last_updated: 2026-04-26
- work_type: mixed

## Goal
- Replace source-controlled references to the old AgentMemory terminology with CharacterMemory terminology.
- Keep crate/package names, public Rust symbols, tests, docs, and local development metadata consistent.

## Definition of Done
- No source-controlled old-name references remain for `AgentMemory`, `AgentMemoryBuilder`, `agent_memory`, `agent-memory`, `agentmemory`, `Agent Memory`, or `agent memory`, excluding generated build output.
- Rust package name is `character_memory`.
- Public primary type is `CharacterMemory`.
- Tests and docs use the new terminology.
- Required validation passes or is explicitly waived.

## Scope / Non-goals
- Scope:
  - `Cargo.toml`, `Cargo.lock`, `.vscode/launch.json`, `README.md`, `src/lib.rs`, `docs/roadmap/agent_thread_kickoff.md`, and `tests/**`.
  - Regenerate/update lockfile effects caused by the package rename.
- Non-goals:
  - Preserve old `AgentMemory` API compatibility aliases.
  - Rename `docs/roadmap/agent_thread_kickoff.md`, because `agent_thread` appears to describe coding-agent workflow context, not the old project name.
  - Edit generated build artifacts under `target/**`.

## Context (workspace)
- Related files/areas:
  - Package metadata: `Cargo.toml`, `Cargo.lock`.
  - Public API docs/type: `src/lib.rs`.
  - Integration tests and helpers: `tests/**`.
  - User docs and roadmap examples: `README.md`, `docs/roadmap/agent_thread_kickoff.md`.
  - Local debug configuration: `.vscode/launch.json`.
- Existing patterns or references:
  - Researcher found old-name references in manifest, docs, public type names, test imports/helpers, and VS Code debug launch config.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`

## Open Questions
- None.

## Assumptions
- `CharacterMemory` is the desired public Rust type name, not only the repository display name.
- `character_memory` is the desired Rust crate/package name.
- Breaking downstream imports from `agent_memory::...` is acceptable for this repository rename.

## Tasks

### Task_1: Rename package and public API
- type: impl
- owns:
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
- depends_on: []
- description: |
  Rename the Rust package/crate metadata and primary public type/docs from AgentMemory terminology to CharacterMemory terminology.
- acceptance:
  - `Cargo.toml` package name is `character_memory`.
  - Lockfile package entry is updated consistently by Cargo or equivalent manifest-aware update.
  - `src/lib.rs` exposes `CharacterMemory` and contains no old project terminology.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"

### Task_2: Rename tests and local helper terminology
- type: impl
- owns:
  - tests/**
- depends_on: [Task_1]
- description: |
  Update crate imports, helper names, test names, local variables, comments, and error text in tests to CharacterMemory terminology.
- acceptance:
  - Tests import `character_memory::...`.
  - Test helper names and local variables use `character_memory` terminology.
  - No old project terminology remains in `tests/**`.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"

### Task_3: Rename docs and editor metadata
- type: docs
- owns:
  - README.md
  - docs/roadmap/agent_thread_kickoff.md
  - .vscode/launch.json
- depends_on: [Task_1]
- description: |
  Update user-facing docs, roadmap code examples, Qdrant container naming, and local debug metadata to CharacterMemory terminology.
- acceptance:
  - README title and examples use CharacterMemory terminology.
  - Roadmap examples use CharacterMemory terminology where referring to the library API.
  - VS Code debug config references `character_memory` package metadata.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "git grep -n -E 'AgentMemory|AgentMemoryBuilder|agent-memory|agent_memory|Agent Memory|agent memory|agentmemory|Agentmemory' -- . ':(exclude)target/**' returns no matches except intentionally retained plan/research notes under docs/coding-agent if any"

### Task_4: Review rename completeness
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Review the final diff for rename completeness, accidental overreach, public API consistency, and validation evidence.
- acceptance:
  - Reviewer status is APPROVED or any issues are resolved/waived.
  - Required validation evidence is present for all implementation tasks.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Review final diff, old-term search evidence, and validation outputs."

## Task Waves

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2, Task_3]
- Wave 3 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable; no UI/user flow changes.

## Rollback / Safety
- Revert the focused rename diff if needed.
- Package/type rename is an intentional public API break; downstream callers must update imports and type names.

## Quality Routing Note
- Routing level: L1
- In-scope docs: engineering-quality-baselines, plan-format, rulebook, subagent-strategy
- Out-of-scope docs: UI/E2E, security-boundaries, database/migration guidance (no UI, auth, secrets, schema, or data migration changes)
- Top risks: contract/API compatibility
- Risk profile: medium because crate name and primary public type are public contract names.
- Required checks:
  - cargo fmt --check
  - cargo check
  - cargo test --no-run
  - old-term grep search
  - reviewer diff review
- Optional recommended checks:
  - cargo test with Qdrant running
- At Risk items: []
- Residual risk / follow-up: downstream users must update crate imports and type names.

## Progress Log

- 2026-04-26 Plan drafted from Researcher rename inventory.
- 2026-04-26 Wave 1 completed: [Task_1]
  - Summary: Renamed Cargo package metadata, lockfile entry, and primary public type/docs.
  - Validation evidence: `cargo check` pass; `cargo fmt --check` pass.
  - Notes: No compatibility alias retained.
- 2026-04-26 Wave 2 completed: [Task_2, Task_3]
  - Summary: Updated tests, docs, roadmap API examples, and VS Code launch metadata.
  - Validation evidence: `cargo test --no-run` pass; old-term grep pass after follow-up validation.
  - Notes: Initial Task_3 grep raced Task_2 during parallel execution, then passed after Task_2 completed.
- 2026-04-26 Wave 3 completed: [Task_4]
  - Summary: Reviewer approved the final rename diff with no findings.
  - Validation evidence: Reviewer old-term searches excluding `target/**`, `docs/coding-agent/**`, and `.git/**` produced no output; workspace diagnostics had no errors.
  - Notes: Full `cargo test` was not run because Qdrant-backed execution is outside the required compile-only validation.

## Decision Log

- 2026-04-26 Decision: strict rename without compatibility alias.
  - Trigger / new insight: old project name is embedded in public crate and type names.
  - Plan delta: include public API rename and contract risk note.
  - Tradeoffs considered: compatibility alias would preserve old terminology, which conflicts with the rename goal.
  - User approval: yes

## Notes
- Generated output under `target/**` is intentionally excluded from rename edits and final search checks.
