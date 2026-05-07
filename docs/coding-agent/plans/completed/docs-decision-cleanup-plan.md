# Plan: Docs Decision Cleanup

- status: completed
- generated: 2026-05-07
- last_updated: 2026-05-07
- work_type: docs

## Goal
- Treat repository decision records as decided records rather than proposed ADR drafts, keep the live decision directory under `docs/decisions`, and move the project philosophy document to the top-level docs directory.

## Definition of Done
- Decision documentation points at `docs/decisions`.
- Decision README and template default to accepted/decided records rather than proposed records.
- Project philosophy lives at `docs/project_philosophy.md`.
- Live references to the old project philosophy path are updated.

## Scope / Non-goals
- Scope:
  - `docs/decisions/README.md`
  - `docs/decisions/template.md`
  - `docs/project_philosophy.md`
  - live references to the moved philosophy document
- Non-goals:
  - Rename existing `ADR-*` filenames.
  - Rewrite historical completed plan narrative unless it points to a moved live document.
  - Change Rust behavior.

## Tasks

### Task_1: Move Project Philosophy
- type: docs
- owns:
  - previous project philosophy path under `docs/design/`
  - `docs/project_philosophy.md`
  - live references to the moved path
- depends_on: []
- acceptance:
  - Philosophy document lives at `docs/project_philosophy.md`.
  - Active-plan references to the moved live document use the new path.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "Search docs for old philosophy-path references and confirm only intentional plan-history references remain."

### Task_2: Clean Decision Record Status And Directory Wording
- type: docs
- owns:
  - `docs/decisions/README.md`
  - `docs/decisions/template.md`
- depends_on: []
- acceptance:
  - Decision README uses `docs/decisions`.
  - Decision README describes records as accepted/current decisions rather than proposed implementation drafts.
  - Template defaults to `status: accepted`.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "Search docs for stale ADR-directory paths and proposed-status wording."

### Task_3: Final Reference Audit
- type: review
- owns: []
- depends_on: [Task_1, Task_2]
- acceptance:
  - Stale live paths and proposed-status wording are removed or confirmed historical.
  - Git diff contains only docs cleanup.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: `git diff --check`
  - kind: review
    required: true
    owner: reviewer
    detail: "Review docs-only diff for stale references and unintended history rewrites."

## Task Waves

- Wave 1 (parallel): [Task_1, Task_2]
- Wave 2 (parallel): [Task_3]

## E2E / Visual Validation Spec

- Not applicable. This plan does not touch UI or browser-facing flows.

## Progress Log

- 2026-05-07 00:00 Wave 1 completed: [Task_1, Task_2]
  - Summary: Moved the philosophy document to top-level docs, updated live references, corrected the decisions directory example, and changed the decision template default from proposed to accepted.
  - Validation evidence: Final reference audit recorded in Wave 2.

- 2026-05-07 00:00 Wave 2 completed: [Task_3]
  - Summary: Completed docs-only reference audit and reviewer gate.
  - Validation evidence: `rg` audits and `git diff --check` completed during closeout.

## Decision Log

- 2026-05-07 00:00 Decision:
  - Trigger / new insight: The repository already uses `docs/decisions`; cleanup should remove stale legacy ADR-directory wording rather than move an absent directory.
  - Plan delta (what changed): Kept existing `docs/decisions` directory and updated wording/status defaults around decided records.
  - Tradeoffs considered: Existing `ADR-*` filenames remain because the user asked to treat ADRs as decided, not to rename every record identifier.
  - User approval: yes; user requested this docs cleanup and a dedicated branch.

## Notes
- Researcher found all existing design and implementation ADR front matter already set to `status: accepted`; only template and README lifecycle wording still treated proposed records as normal repository state.
