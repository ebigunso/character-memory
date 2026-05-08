# Plan: v0.1.3 Remember Intake And Assisted Remember Roadmap Docs

- status: done
- generated: 2026-05-09
- last_updated: 2026-05-09
- work_type: docs

## Goal
- Add roadmap and ADR documentation for v0.1.3 remember intake interfaces and deterministic write planning, plus a future v0.6 assisted remember workflow phase, without changing code.

## Definition of Done
- Main roadmap table and sections include v0.1.3 and v0.6 in the correct order.
- New v0.1.3 and v0.6 phase docs exist beside existing roadmap phase docs and are linked from the main roadmap.
- Three new ADRs use the next correct per-track numbers and are indexed in `docs/decisions/README.md`.
- API evolution, YAGNI, and cross-version invariants are updated for the new write-planning scope.
- Link, numbering, and docs consistency checks pass.
- Reviewer approves the final documentation set.

## Scope / Non-goals
- Scope:
  - `docs/roadmap/development_roadmap.md`
  - `docs/design/roadmap-phases/v0_1_3_remember_intake_interfaces_deterministic_write_planning.md`
  - `docs/design/roadmap-phases/v0_6_assisted_remember_workflow_memory_candidate_generation.md`
  - `docs/design/roadmap-phases/v0_2_scoped_continuity_reflection.md`
  - `docs/decisions/README.md`
  - `docs/decisions/design/ADR-D-0012-separate-memory-candidates-from-committed-memory.md`
  - `docs/decisions/implementation/ADR-I-0012-use-prepare-validate-commit-write-workflow.md`
  - `docs/decisions/implementation/ADR-I-0013-deterministic-helpers-do-not-infer-high-level-meaning.md`
- Non-goals:
  - Rust implementation changes.
  - Public API code changes.
  - Migration or schema changes.
  - Runtime configuration changes.

## Context (workspace)
- Related files/areas:
  - `docs/roadmap/development_roadmap.md`
  - `docs/design/roadmap-phases/`
  - `docs/decisions/`
- Existing patterns or references:
  - ADR tracks are numbered separately for design and implementation decisions.
  - Roadmap phase docs use `v0_*` filenames and are linked from the main roadmap.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`

## Open Questions (max 3)
- None. Use the filenames and ADR numbers requested in the hand-off because the local repo currently ends at ADR-D-0011 and ADR-I-0011.

## Assumptions
- A1: The supplied hand-off is the approved integration scope for this docs update.
- A2: v0.1.3 belongs after v0.1.2 and before v0.2; v0.6 belongs after v0.5 and before v1.0+.
- A3: v0.1.3 remains deterministic and non-generative; v0.6 owns assisted candidate generation.

## Tasks

### Task_1: Add v0.1.3 and v0.6 phase docs
- type: docs
- owns:
  - docs/design/roadmap-phases/v0_1_3_remember_intake_interfaces_deterministic_write_planning.md
  - docs/design/roadmap-phases/v0_6_assisted_remember_workflow_memory_candidate_generation.md
- depends_on: []
- description: |
  Create the new roadmap phase documents from the supplied hand-off.
- acceptance:
  - v0.1.3 phase doc defines deterministic remember intake/write-planning concepts, workflow, non-goals, validation, and acceptance criteria.
  - v0.6 phase doc defines future assisted remember workflow and candidate generation.
  - v0.1.3 explicitly avoids automatic semantic inference and defers generation to v0.6.
- validation:
  - kind: file-review
    required: true
    owner: orchestrator
    detail: "Check both new phase docs exist and contain the expected titles."

### Task_2: Update main development roadmap
- type: docs
- owns:
  - docs/roadmap/development_roadmap.md
  - docs/design/roadmap-phases/v0_2_scoped_continuity_reflection.md
- depends_on: [Task_1]
- description: |
  Add v0.1.3 and v0.6 to the version table and phase sections, update API evolution, YAGNI rules, and cross-version invariants.
- acceptance:
  - Version table includes v0.1.3 after v0.1.2 and v0.6 after v0.5.
  - Roadmap sections include v0.1.3 before v0.2 and v0.6 before v1.0+.
  - API evolution includes v0.1.3 prepare/validate/commit workflow.
  - YAGNI and invariants include the shared manual/future-generated safe write path.
  - Existing v0.2 phase context acknowledges v0.1.3 as the intervening write-plan phase.
- validation:
  - kind: link-check
    required: true
    owner: orchestrator
    detail: "Check new roadmap phase links resolve to existing files."

### Task_3: Add and index ADRs
- type: docs
- owns:
  - docs/decisions/README.md
  - docs/decisions/design/ADR-D-0012-separate-memory-candidates-from-committed-memory.md
  - docs/decisions/implementation/ADR-I-0012-use-prepare-validate-commit-write-workflow.md
  - docs/decisions/implementation/ADR-I-0013-deterministic-helpers-do-not-infer-high-level-meaning.md
- depends_on: []
- description: |
  Add the new accepted ADRs and update the ADR index.
- acceptance:
  - ADR-D-0012, ADR-I-0012, and ADR-I-0013 files exist.
  - ADR front matter follows the repository template shape.
  - `docs/decisions/README.md` links each new ADR in the correct track.
  - ADR numbering remains contiguous per track.
- validation:
  - kind: file-review
    required: true
    owner: orchestrator
    detail: "Check ADR files, README links, and per-track numbering."

### Task_4: Final docs consistency review
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Review the integrated docs for ordering, links, scope separation, and stale contradictions.
- acceptance:
  - v0.1.3 does not absorb v0.6 assisted generation scope.
  - v0.6 uses the v0.1.3 write-plan path.
  - New ADRs align with roadmap content.
  - Reviewer status is APPROVED.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "Run docs search/link/numbering sanity checks."
  - kind: review
    required: true
    owner: reviewer
    detail: "Final review of documentation integration."

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_3]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. No UI/user-flow changes.

## Rollback / Safety
- Documentation-only changes can be reverted by restoring the touched docs from git.

## Progress Log (append-only)

- 2026-05-09 Plan started:
  - Summary: Researcher mapped current ADR numbering and roadmap structure; user hand-off supplies the integration scope.
  - Validation evidence: pending
  - Notes: Proceeding with requested filenames and ADR numbers.
- 2026-05-09 Implementation completed:
  - Summary: Added v0.1.3 and v0.6 phase docs, updated the main roadmap, added three ADRs, and updated the ADR index.
  - Validation evidence: Link resolver found no missing roadmap/ADR links; ADR numbering is contiguous through ADR-D-0012 and ADR-I-0013; `git diff --check -- docs` passed with CRLF warnings only.
  - Notes: Final reviewer gate pending.
- 2026-05-09 Final review completed:
  - Summary: Reviewer approved the docs integration.
  - Validation evidence: Reviewer confirmed roadmap ordering, v0.1.3/v0.6 scope separation, ADR numbering, and link checks.
  - Notes: No issues found.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-09 Decision:
  - Trigger / new insight: Local repo already includes v0.1.2 ADRs through ADR-D-0011 and ADR-I-0011.
  - Plan delta (what changed): Use requested ADR-D-0012, ADR-I-0012, and ADR-I-0013 filenames without renumbering.
  - Tradeoffs considered: Researcher suggested shorter alternate filenames, but the hand-off gave explicit target filenames.
  - User approval: hand-off requested applying these updates.
- 2026-05-09 Decision:
  - Trigger / new insight: Existing v0.2 phase text described v0.1.2 flowing directly into v0.2.
  - Plan delta (what changed): Added the v0.2 phase doc to Task_2 owns so it can acknowledge v0.1.3 as the intervening write-plan phase.
  - Tradeoffs considered: Updating only the opening context avoids broad v0.2 scope churn.
  - User approval: hand-off requested placing v0.1.3 between v0.1.2 and v0.2.

## Notes
- Risks:
  - Section ordering and detailed-draft links can drift when inserting v0.1.3 and v0.6.
  - v0.1.3 must remain deterministic and not become assisted generation.
- Edge cases:
  - Existing v0.2 wording may need to mention v0.1.3 as an intervening phase after v0.1.2.
