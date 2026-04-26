# Plan: Roadmap Docs Rust And Organization Cleanup

- status: done
- generated: 2026-04-27
- last_updated: 2026-04-27
- work_type: docs

## Goal
- Update roadmap documentation so it describes the Rust crate rather than assuming Python.
- Move phase-level detailed design drafts into a recognizable design directory.
- Remove numeric filename prefixes from the phase docs while preserving clear organization through names and links.

## Definition of Done
- Python-specific examples and structure references in roadmap phase docs are rewritten as Rust-oriented examples or neutral design language.
- Phase-level detailed drafts live under `docs/design/roadmap-phases/` with filenames that do not start with `02_` through `07_`.
- `docs/roadmap/development_roadmap.md` links to the new design-doc locations.
- Targeted checks show no stale numbered roadmap links or obvious Python residue in the affected docs.

## Scope / Non-goals
- Scope:
  - `docs/roadmap/development_roadmap.md`
  - `docs/roadmap/02_v0_1_starter_episodic_memory.md`
  - `docs/roadmap/03_v0_1_storage_and_backend_contracts.md`
  - `docs/roadmap/04_v0_2_continuity_reflection.md`
  - `docs/roadmap/05_v0_3_factual_rigor_belief_tracking.md`
  - `docs/roadmap/06_v0_4_advanced_recall_governance.md`
  - `docs/roadmap/07_v1_0_multimodal_embodied_expansion.md`
  - `docs/design/roadmap-phases/`
- Non-goals:
  - Changing Rust source code or tests.
  - Changing the database design document unless a link needs to reference it.
  - Redesigning the roadmap content beyond Python-to-Rust wording and organization.

## Context (workspace)
- Related files/areas:
  - `docs/roadmap/`
  - `docs/design/`
- Existing patterns or references:
  - `docs/design/database/` already groups design specifications by subject.
  - Repo crate name is `character_memory`.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`

## Open Questions (max 3)
- Q1: None.

## Assumptions
- A1: Phase-level detailed roadmap drafts are better treated as design docs than as top-level roadmap index files.
- A2: Rust-oriented examples can be illustrative and do not need to exactly match the current public API surface unless existing names are already clear.

## Tasks

### Task_1: Create Phase Design Directory And Move Files
- type: docs
- owns:
  - `docs/roadmap/development_roadmap.md`
  - `docs/roadmap/02_v0_1_starter_episodic_memory.md`
  - `docs/roadmap/03_v0_1_storage_and_backend_contracts.md`
  - `docs/roadmap/04_v0_2_continuity_reflection.md`
  - `docs/roadmap/05_v0_3_factual_rigor_belief_tracking.md`
  - `docs/roadmap/06_v0_4_advanced_recall_governance.md`
  - `docs/roadmap/07_v1_0_multimodal_embodied_expansion.md`
  - `docs/design/roadmap-phases/`
- depends_on: []
- description: |
  Create `docs/design/roadmap-phases/` and move the numbered detailed phase docs there with unnumbered names. Do not rewrite content in this task except for path-level move/rename effects.
- acceptance:
  - Detailed phase docs are moved to `docs/design/roadmap-phases/` with no `02_` through `07_` filename prefixes.
  - The original numbered phase doc files no longer exist in `docs/roadmap/`.
  - The new directory name clearly identifies the files as roadmap phase design documents.
- validation:
  - kind: manual
    required: true
    owner: worker
    detail: "List `docs/roadmap/` and `docs/design/roadmap-phases/` to confirm moved paths"

### Task_2: Update Roadmap Index Links And Structure Notes
- type: docs
- owns:
  - `docs/roadmap/development_roadmap.md`
- depends_on: [Task_1]
- description: |
  Update the roadmap index so detailed-draft links point to the moved phase design docs, and rewrite the Python-oriented suggested implementation structure as Rust crate/module structure.
- acceptance:
  - All detailed-draft links target `../design/roadmap-phases/...` paths.
  - The suggested implementation structure uses Rust crate paths and `.rs` modules instead of Python package files.
  - Public API examples in the roadmap index use Rust fences or neutral text.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Inspect updated roadmap index links and Rust wording"

### Task_3: Rewrite Phase Docs For Rust Assumptions
- type: docs
- owns:
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/design/roadmap-phases/v0_2_continuity_reflection.md`
  - `docs/design/roadmap-phases/v0_3_factual_rigor_belief_tracking.md`
  - `docs/design/roadmap-phases/v0_4_advanced_recall_governance.md`
  - `docs/design/roadmap-phases/v1_0_multimodal_embodied_expansion.md`
- depends_on: [Task_1]
- description: |
  Rewrite Python-specific code fences, signatures, and optional-value language in moved phase docs as Rust-oriented examples or neutral design wording.
- acceptance:
  - Phase docs no longer use Python code fences for API examples.
  - Python-specific signatures such as `-> str` and `None` are replaced with Rust-style signatures or neutral wording.
  - Rust examples are illustrative and consistent with the crate direction.
  - Existing roadmap intent and phase scope remain intact.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Inspect moved phase docs for coherent Rust-oriented API examples"

### Task_4: Run Targeted Docs Validation
- type: test
- owns: []
- depends_on: [Task_2, Task_3]
- description: |
  Run targeted searches to catch stale numbered filenames, broken move references, and obvious Python residue in affected documentation.
- acceptance:
  - No stale references to old numbered filenames remain in docs.
  - No obvious Python residue remains in affected roadmap/design phase docs.
  - Any benign matches are documented with rationale.
- validation:
  - kind: search
    required: true
    owner: worker
    detail: "Search affected docs for old numeric filenames: 02_v0_1|03_v0_1|04_v0_2|05_v0_3|06_v0_4|07_v1_0"
  - kind: search
    required: true
    owner: worker
    detail: "Search affected docs for obvious Python residue: python|\.py|None|-> str|pip|pytest|venv|poetry|pydantic|fastapi"
  - kind: manual
    required: true
    owner: worker
    detail: "Inspect `docs/roadmap/development_roadmap.md` detailed-draft links for correct relative paths"

### Task_5: Review Docs Cleanup
- type: review
- owns: []
- depends_on: [Task_4]
- description: |
  Review the docs-only diff against the user request and required validation evidence.
- acceptance:
  - Reviewer confirms the moved docs, renamed paths, and link updates satisfy the request.
  - Reviewer confirms no required validation evidence is missing.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review against acceptance criteria and validation evidence"

## Task Waves (explicit parallel dispatch sets)

Interpretation:
- Tasks listed in the same wave are intended to be dispatched in parallel by default,
  when `owns` are disjoint and dependencies are met.
- Waves are executed sequentially.

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2, Task_3]
- Wave 3 (parallel): [Task_4]
- Wave 4 (parallel): [Task_5]

## E2E / Visual Validation Spec

- Not applicable; docs-only change with no UI/user-flow impact.

## Rollback / Safety
- Move the phase docs back to `docs/roadmap/` and restore the previous links/content from git if needed.

## Progress Log (append-only)

- 2026-04-27 Draft created.
  - Summary: Captured docs-only move, rewrite, and review tasks.
  - Validation evidence: Pending approval and execution.
  - Notes: Researcher identified phase docs and Python-specific residue.
- 2026-04-27 User approved implementation after smaller breakdown.
  - Summary: Split the original large docs task into move, roadmap-index update, phase-doc rewrite, targeted validation, and review tasks.
  - Validation evidence: Pending execution.
  - Notes: Smaller tasks reduce path-move and content-rewrite risk.
- 2026-04-27 Wave 1 completed: [Task_1]
  - Summary: Created `docs/design/roadmap-phases/` and moved six detailed phase docs there with unnumbered filenames.
  - Validation evidence: Worker listed `docs/roadmap/` and `docs/design/roadmap-phases/`; `docs/roadmap/` contains only `development_roadmap.md`, and the target directory contains all six moved phase docs.
  - Notes: No content rewrites were performed in Task_1.
- 2026-04-27 Wave 2 completed: [Task_2, Task_3]
  - Summary: Updated the roadmap index links and Rust crate structure notes; rewrote moved phase-doc API examples from Python-shaped examples to Rust-oriented examples.
  - Validation evidence: Workers inspected updated links, Rust structure wording, and moved phase docs; targeted searches found no Python code fences, `.py` module references, `-> str`, old numbered phase filenames, or Python API residue in their owned docs.
  - Notes: `None` may still appear only if part of Rust `Option<T> = None` examples; Task_4 will run the plan-level targeted checks.
- 2026-04-27 Wave 3 completed: [Task_4]
  - Summary: Ran targeted validation across the roadmap index and six moved phase design docs.
  - Validation evidence: No stale old numbered filename references were found; detailed-draft links use `../design/roadmap-phases/...` and all six targets exist.
  - Notes: Broad Python-residue search found only benign matches: `pip` inside `pipeline` and Rust `Option<T> = None` examples.
- 2026-04-27 Wave 4 review requested changes: [Task_5]
  - Summary: Reviewer found an out-of-scope working-tree deletion of `docs/roadmap/agent_thread_kickoff.md`.
  - Validation evidence: The deletion is outside this plan's owns scope and the file was already absent from the workspace roadmap listing before this implementation request.
  - Notes: Per repository safety rules, the deletion is treated as a pre-existing/user working-tree change and is not restored by this plan.
- 2026-04-27 Wave 4 completed: [Task_5]
  - Summary: Follow-up Reviewer approved the scoped roadmap docs cleanup.
  - Validation evidence: Reviewer confirmed the user request is satisfied, roadmap links target moved docs, Task_4 evidence is adequate, and no blocking scoped issues remain.
  - Notes: `docs/roadmap/agent_thread_kickoff.md` remains an out-of-scope residual working-tree deletion.

## Decision Log (append-only)

- 2026-04-27 Decision:
  - Trigger / new insight: User requested roadmap docs organization and language assumption cleanup.
  - Plan delta (what changed): Proposed moving detailed phase drafts from `docs/roadmap/` to `docs/design/roadmap-phases/` while keeping the roadmap index in place.
  - Tradeoffs considered: Keeping all docs in `docs/roadmap/` would satisfy filename cleanup but would not make associated phase design documents as recognizable within the existing `docs/design/` structure.
  - User approval: yes.
- 2026-04-27 Decision:
  - Trigger / new insight: Reviewer reported `docs/roadmap/agent_thread_kickoff.md` deleted in the broader working tree.
  - Plan delta (what changed): No docs implementation scope change; final review should evaluate the approved roadmap/docs phase changes and treat this deletion as out-of-scope.
  - Tradeoffs considered: Restoring the file would revert a change not made by this plan; leaving it alone respects the dirty-worktree boundary.
  - User approval: not requested because no action is taken on the out-of-scope file.

## Notes
- Risks: Low; docs-only path moves and wording updates.
- Edge cases: Search terms such as `None` or `python` may appear in explanatory prose only if intentionally retained, but the goal is to remove Python assumptions from affected docs.
