# Plan: v0.1.2 Documentation Integration

- status: done
- generated: 2026-05-08
- last_updated: 2026-05-08
- work_type: docs

## Goal
- Incorporate the supplied v0.1.2 roadmap, ADR, and design documentation package into the repository documentation without overwriting unrelated local documentation or violating repo doc-scope boundaries.

## Definition of Done
- New v0.1.2 ADR and roadmap-phase documents are present in the repository.
- Existing philosophy, roadmap, ADR index, and database design docs are updated where the supplied package supersedes or extends them.
- Superseded roadmap-phase docs do not remain as competing active phase drafts.
- Documentation validation confirms links and expected files are coherent.
- The completed plan redacts user-identifying local filesystem paths.
- Reviewer approves the documentation integration.

## Scope / Non-goals
- Scope:
  - `docs/project_philosophy.md`
  - `docs/roadmap/development_roadmap.md`
  - `docs/decisions/**`
  - `docs/design/database/**`
  - `docs/design/roadmap-phases/**`
- Non-goals:
  - Runtime configuration changes.
  - Code changes.
  - Copying the external package `README.md` into repository docs.
  - Adding service startup or environment-variable guidance to schema reference docs.

## Context (workspace)
- Related files/areas:
  - External package: supplied v0.1.2 roadmap/docs/ADR package
  - Repository docs under `docs/`
- Existing patterns or references:
  - ADR files are split into `docs/decisions/design/` and `docs/decisions/implementation/`.
  - Roadmap phase docs live under `docs/design/roadmap-phases/`.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`

## Open Questions (max 3)
- None.

## Assumptions
- A1: The external ADRs marked `accepted` should be incorporated as accepted repository ADRs.
- A2: Renamed v0.2/v0.3/v0.4 roadmap phase docs supersede the existing same-version phase docs.
- A3: v0.1.2 roadmap wording may assume v0.1 and v0.1.1 are complete; if existing repo wording is more cautious, preserve accuracy rather than blindly copying stronger claims.

## Tasks

### Task_1: Integrate roadmap and philosophy docs
- type: docs
- owns:
  - docs/project_philosophy.md
  - docs/roadmap/development_roadmap.md
  - docs/design/roadmap-phases/**
- depends_on: []
- description: |
  Merge the supplied entity-neutral continuity, v0.1.2 retrieval guardrail, and renamed later-phase roadmap documents into the repo docs. Remove superseded v0.2/v0.3/v0.4 phase drafts after their replacements are present.
- acceptance:
  - Philosophy doc includes entity-neutral continuity and recurring-entity retrieval cautions where they belong.
  - Development roadmap includes v0.1.2 and the renamed/reframed v0.2 through v0.5 phases.
  - New v0.1.2 and v0.5 roadmap phase files exist.
  - v0.2/v0.3/v0.4 phase docs are represented by the new names, with old competing names removed.
- validation:
  - kind: file-review
    required: true
    owner: worker
    detail: "Check roadmap phase links and filenames after integration."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review roadmap/philosophy diff against supplied docs and repo scope."

### Task_2: Integrate ADR additions and index updates
- type: docs
- owns:
  - docs/decisions/README.md
  - docs/decisions/design/ADR-D-0009-entity-neutral-retrieval-policy.md
  - docs/decisions/design/ADR-D-0010-recurring-entities-are-anchors-not-traversal-invitations.md
  - docs/decisions/design/ADR-D-0011-scope-continuity-around-arbitrary-entities-and-contexts.md
  - docs/decisions/implementation/ADR-I-0008-retrieval-stats-are-derived-policy-metadata.md
  - docs/decisions/implementation/ADR-I-0009-use-sqlite-as-default-retrieval-stats-store.md
  - docs/decisions/implementation/ADR-I-0010-use-continuous-selectivity-and-smooth-fanout.md
  - docs/decisions/implementation/ADR-I-0011-guard-against-low-information-co-occurrence-links.md
- depends_on: []
- description: |
  Add the new accepted design and implementation ADRs from the supplied package and update the ADR index.
- acceptance:
  - ADR-D-0009 through ADR-D-0011 exist under design decisions.
  - ADR-I-0008 through ADR-I-0011 exist under implementation decisions.
  - `docs/decisions/README.md` links the new ADRs in the correct sections.
  - ADR numbering remains contiguous with existing repository ADRs.
- validation:
  - kind: file-review
    required: true
    owner: worker
    detail: "Check new ADR files and index links exist with expected names."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review ADR additions for numbering, status, and index consistency."

### Task_3: Integrate database design updates
- type: docs
- owns:
  - docs/design/database/README.md
  - docs/design/database/graph_schema_design.md
  - docs/design/database/schema_cheat_sheet.md
  - docs/design/database/vector_payload_design.md
- depends_on: []
- description: |
  Add retrieval-stats, selectivity, fanout, and authority-boundary documentation to database design docs while preserving the schema-doc boundary against runtime configuration.
- acceptance:
  - Graph schema docs describe retrieval-stats authority boundaries without service startup/config instructions.
  - Schema cheat sheet includes retrieval-stats schema/table reference material only.
  - Vector payload docs clarify that selectivity/fanout policy metadata is not owned by Qdrant payloads.
  - Database README points to the updated retrieval-stats design coverage if needed.
- validation:
  - kind: search
    required: true
    owner: worker
    detail: "Search database design docs for runtime config terms that should not be present in schema references."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review database docs for scope boundaries and consistency with supplied ADRs."

### Task_4: Final docs validation and harmonization
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Validate the integrated documentation set as a whole and check for stale links, competing phase docs, or scope drift.
- acceptance:
  - Required worker validations for Task_1 through Task_3 are evidenced.
  - No stale roadmap phase names remain as active linked docs.
  - Reviewer status is APPROVED or any issues are resolved.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "Run a repository docs/link sanity search using shell commands."
  - kind: review
    required: true
    owner: reviewer
    detail: "Final review of documentation integration."

## Task Waves (explicit parallel dispatch sets)

Interpretation:
- Tasks listed in the same wave are intended to be dispatched in parallel by default,
  when `owns` are disjoint and dependencies are met.
- Waves are executed sequentially.

- Wave 1 (parallel): [Task_1, Task_2, Task_3]
- Wave 2 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. No UI/user-flow changes.

## Rollback / Safety
- Documentation-only changes can be reverted by restoring the touched docs from git.
- Keep the external package available as comparison input until validation is complete.

## Progress Log (append-only)

- 2026-05-08 Draft created:
  - Summary: Researcher mapped the supplied docs to existing repository docs and identified schema-doc scope boundaries.
  - Validation evidence: pending
  - Notes: Awaiting user approval before execution.
- 2026-05-08 Plan approved:
  - Summary: User approved planned tasks and requested user-identifying local paths be redacted after plan completion.
  - Validation evidence: pending
  - Notes: Redaction is a closeout requirement before moving the plan to completed.
- 2026-05-08 Wave 1 completed:
  - Summary: Task_1, Task_2, and Task_3 workers integrated roadmap/philosophy/phase docs, ADR additions, and database design updates.
  - Validation evidence: Worker validations passed for roadmap phase links/filenames, ADR existence/index/numbering, and database schema-scope boundary search.
  - Notes: Reviewer requested redaction of the remaining user-local source path before approval.
- 2026-05-08 Reviewer change addressed:
  - Summary: Replaced the local external-package filesystem path with a non-identifying package label.
  - Validation evidence: user-local path and user-name pattern search under `docs/` returned no matches.
  - Notes: This addresses the only reviewer-requested change.
- 2026-05-08 Final validation completed:
  - Summary: Cross-doc validation and reviewer gate completed.
  - Validation evidence: `git diff --check -- docs` passed with CRLF warnings only; current roadmap/ADR/database README markdown links resolved; database runtime/config forbidden-term search passed; reviewer status APPROVED.
  - Notes: Plan ready to move to completed.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-08 Decision:
  - Trigger / new insight: External docs include direct replacements for several roadmap phase docs and new accepted ADRs.
  - Plan delta (what changed): Initial plan scopes docs integration into roadmap/philosophy, ADR, database-design, and final review tasks.
  - Tradeoffs considered: Selective integration over blind copy to preserve local edits and schema-doc boundaries.
  - User approval: yes
- 2026-05-08 Decision:
  - Trigger / new insight: User requested local paths revealing user names be redacted after completion.
  - Plan delta (what changed): Added completed-plan redaction to Definition of Done.
  - Tradeoffs considered: Keep source identity understandable while removing user-identifying path segments.
  - User approval: yes

## Notes
- Risks:
  - External schema docs include runtime configuration language that should not be copied into schema references.
  - Keeping old and new v0.2/v0.3/v0.4 phase files would create competing roadmap drafts.
- Edge cases:
  - Existing docs may contain newer local wording than the package; prefer a merge that preserves accurate repository-specific context.
