# Plan: Roadmap and ADR Integration for Controlled Associative Recall

- status: done
- generated: 2026-05-10
- last_updated: 2026-05-10
- work_type: docs

## Goal

- Integrate the controlled associative recall hand-off into the repository documentation and ADR set.
- Preserve the v0.1.2 guard against weak low-selectivity durable pairwise links while documenting v0.5 controlled serendipitous recall through `AssociativeUnit`, `AssociativeMembership`, and `AssociationSupport`.

## Definition of Done

- `docs/roadmap/development_roadmap.md` reflects the updated v0.4/v0.5 direction, v0.1.2 tradeoff, cross-version invariant, and YAGNI rules.
- v0.1.2, v0.4, and v0.5 phase docs align with the hand-off and keep phase boundaries clear.
- The v0.5 phase doc is renamed/replaced as `docs/design/roadmap-phases/v0_5_controlled_associative_recall_clustering.md`, with stale inbound references updated.
- Database docs describe graph-internal associative structures without making weak evidence ordinary relationship truth.
- Project philosophy includes the controlled-serendipity principle.
- Three ADRs are added with local numbering:
  - `ADR-D-0013` controlled serendipitous recall without weak pairwise durable links.
  - `ADR-D-0014` associative membership lifecycle separate from unit lifecycle.
  - `ADR-I-0014` graph-internal associative units instead of a separate weak hint store.
- `docs/decisions/README.md` links the new ADRs.
- Docs-only validation confirms expected new terms exist, stale v0.5 filename/title references are resolved, and no implementation files were changed.

## Scope / Non-goals

- Scope:
  - Documentation and ADR files under `docs/**`.
- Non-goals:
  - No Rust implementation.
  - No schema migration.
  - No runtime behavior changes.
  - No public API code changes.
  - No database backfill or generated artifact changes.

## Context (workspace)

- Related files/areas:
  - `docs/roadmap/development_roadmap.md`
  - `docs/design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`
  - `docs/design/roadmap-phases/v0_4_retrieval_observability_governance.md`
  - legacy v0.5 associative recall phase file
  - `docs/design/database/graph_schema_design.md`
  - `docs/design/database/schema_cheat_sheet.md`
  - `docs/project_philosophy.md`
  - `docs/decisions/README.md`
  - `docs/decisions/design/*.md`
  - `docs/decisions/implementation/*.md`
- Existing patterns or references:
  - Roadmap phase docs use `vX_Y_slug.md` filenames.
  - ADR numbering is track-specific.
  - Current latest ADRs are `ADR-D-0012` and `ADR-I-0013`.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`

## Open Questions

- None. The hand-off specifies the ADR topics, target v0.5 filename, and core content.

## Assumptions

- The hand-off text is authoritative where it conflicts with the current older v0.5 "advanced associative recall" wording.
- The existing old v0.5 file should be replaced by a renamed file, not kept as a parallel phase doc.
- Docs-only validation is sufficient unless implementation files are unexpectedly touched.

## Tasks

### Task_1: Update main roadmap

- type: docs
- owns:
  - `docs/roadmap/development_roadmap.md`
- depends_on: []
- description: |
  Update the roadmap version table, v0.1.2/v0.4/v0.5 sections, cross-version invariants, and YAGNI rules to match the hand-off.
- acceptance:
  - v0.5 row is titled "Controlled associative recall and clustering" with the requested summary.
  - v0.1.2 includes the serendipitous recall tradeoff and weak co-occurrence durable-association rule.
  - v0.4 remains observability/governance and explicitly defers associative cluster machinery to v0.5.
  - Cross-version invariant distinguishes entity incidence, query-time activation, candidate evidence, active associative unit, and strong durable relation.
  - YAGNI rules block low-value pairwise association edges, cluster-level-only status, unbounded spreading activation, and related early implementation.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Review roadmap diff against hand-off sections 1, 2, 3, 8, and 9."
  - kind: search
    required: true
    owner: worker
    detail: "Verify the roadmap contains the requested v0.5 title and invariant wording."

### Task_2: Update roadmap phase docs

- type: docs
- owns:
  - `docs/design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`
  - `docs/design/roadmap-phases/v0_4_retrieval_observability_governance.md`
  - legacy v0.5 associative recall phase file
  - `docs/design/roadmap-phases/v0_5_controlled_associative_recall_clustering.md`
  - `docs/roadmap/development_roadmap.md`
- depends_on: [Task_1]
- description: |
  Add the v0.1.2 tradeoff/rule sections, add v0.4 observability concepts/goals/acceptance/non-goal, and replace the old v0.5 phase with the complete controlled associative recall draft.
- acceptance:
  - v0.1.2 phase doc includes both requested subsections without weakening existing guardrails.
  - v0.4 phase doc includes activation, rejected expansion, cluster expansion, membership decision, coactivation, and candidate diagnostics as observability concepts.
  - v0.5 phase doc uses the target filename and includes `AssociativeUnit`, `AssociativeMembership`, `AssociationSupport`, query-time activation, promotion/decay, bounded expansion, and the explicit priority order.
  - References to the old v0.5 filename are updated or removed.
- validation:
  - kind: search
    required: true
    owner: worker
    detail: "Run targeted `rg` checks for stale v0.5 filename/title and required new concepts."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Review phase boundaries so v0.4 does not implement v0.5 machinery."

### Task_3: Update database docs

- type: docs
- owns:
  - `docs/design/database/graph_schema_design.md`
  - `docs/design/database/schema_cheat_sheet.md`
- depends_on: [Task_2]
- description: |
  Add associative recall boundary content to graph schema design and future associative recall concepts to the schema cheat sheet.
- acceptance:
  - Graph schema docs distinguish ordinary relationship truth from associative recall evidence.
  - Docs describe `AssociativeUnit`, `AssociativeMembership`, and `AssociationSupport`.
  - Retrieval rule states associative structures cannot override suppression, deletion, supersession, currentness, provenance, or graph authority.
  - Cheat sheet includes the design rule and retrieval quality rule from the hand-off.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Review database docs for graph-authority consistency and scope control."
  - kind: search
    required: true
    owner: worker
    detail: "Verify required associative concept names appear in both database docs."

### Task_4: Update project philosophy

- type: docs
- owns:
  - `docs/project_philosophy.md`
- depends_on: [Task_1]
- description: |
  Add the principle that serendipitous recall should be supported without false continuity.
- acceptance:
  - Philosophy distinguishes entity incidence, query-time associative activation, candidate association, active associative cluster, and strong durable relation.
  - Recurring entities are framed as continuity anchors, not traversal invitations.
  - The wording is principle-level and does not prescribe implementation beyond the documented design direction.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Review philosophy update for consistency with ADR-D-0010 and roadmap invariant wording."

### Task_5: Add ADRs and update index

- type: docs
- owns:
  - `docs/decisions/README.md`
  - `docs/decisions/design/ADR-D-0013-controlled-serendipitous-recall-without-weak-pairwise-links.md`
  - `docs/decisions/design/ADR-D-0014-represent-associative-membership-lifecycle-separately.md`
  - `docs/decisions/implementation/ADR-I-0014-use-graph-internal-associative-units.md`
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Add the three requested ADRs using local numbering and update the ADR README links.
- acceptance:
  - ADR files use status `accepted`, date `2026-05-10`, and the requested decision content.
  - Design ADR numbering continues from `ADR-D-0012`.
  - Implementation ADR numbering continues from `ADR-I-0013`.
  - README entries link to all three new files in the local style.
- validation:
  - kind: search
    required: true
    owner: worker
    detail: "Verify ADR numbering, filenames, titles, and README links."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Review ADR content against hand-off section 10."

### Task_6: Final docs validation and review

- type: review
- owns: []
- depends_on: [Task_2, Task_3, Task_4, Task_5]
- description: |
  Run final docs-only validation and a reviewer pass before reporting completion.
- acceptance:
  - Required terms are present in expected docs.
  - Stale old v0.5 filename/title references are resolved.
  - `git diff --check` passes.
  - Reviewer status is APPROVED or findings are addressed.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "`git diff --check`"
  - kind: search
    required: true
    owner: orchestrator
    detail: "Targeted `rg` checks for required concepts and stale references."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review completed docs/ADR diff against plan acceptance criteria."

## Task Waves

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3, Task_4]
- Wave 4 (parallel): [Task_5]
- Wave 5 (parallel): [Task_6]

## E2E / Visual Validation Spec

- Not applicable. This is docs-only work with no UI/user-flow changes.

## Rollback / Safety

- Product documentation edits will be limited to the files listed in task ownership.
- The current branch is dedicated to this integration.
- If needed, revert this branch's docs changes before merging; do not modify unrelated user changes.

## Progress Log

- 2026-05-10 18:04 Plan drafted on branch `codex-controlled-associative-recall-docs`.
  - Summary: Created active harness plan from user hand-off and researcher findings.
  - Validation evidence: Researcher identified current local ADR numbering and doc layout.
  - Notes: Product docs are pending user approval.
- 2026-05-10 18:08 Plan approved by user; implementation started.
  - Summary: Proceeding with documentation integration.
  - Validation evidence: User approval in thread.
  - Notes: Execute approved waves in order.
- 2026-05-10 18:25 Waves 1-4 completed: [Task_1, Task_2, Task_3, Task_4, Task_5]
  - Summary: Updated roadmap, phase docs, database docs, philosophy, and ADR set.
  - Validation evidence: Targeted `rg` checks found required concepts in expected docs; `git diff --check` passed with line-ending warnings only.
  - Notes: Final reviewer pass pending.
- 2026-05-10 18:36 Wave 5 completed: [Task_6]
  - Summary: Addressed reviewer status/role terminology finding and completed final validation.
  - Validation evidence: Stale-reference search returned no matches; targeted status/role conflation search returned no matches; `git diff --check` passed with line-ending warnings only; Reviewer status APPROVED.
  - Notes: `docs/coding-agent/lessons.md` change is intentional harness process metadata.

## Decision Log

- 2026-05-10 18:04 Decision: Use the hand-off's specified ADR topics and filename.
  - Trigger / new insight: Researcher noted local repo already has design ADRs through `ADR-D-0012`, implementation ADRs through `ADR-I-0013`, and an old v0.5 phase filename.
  - Plan delta (what changed): Number new ADRs as `ADR-D-0013`, `ADR-D-0014`, and `ADR-I-0014`; replace old v0.5 phase file with `v0_5_controlled_associative_recall_clustering.md`.
  - Tradeoffs considered: Keeping old v0.5 as a parallel file would preserve history but increase conflicting terminology.
  - User approval: pending.

## Notes

- Risks:
  - v0.1.2 already contains low-information link guard language; edits must preserve that guard.
  - v0.4 should remain observability-only and not imply cluster machinery exists before v0.5.
  - Database docs should stay at design/schema level, not implementation migration level.
- Edge cases:
  - Some old terms like `ClusterSummary` remain valid in the new draft; searches for stale terms must distinguish valid retained terms from outdated cluster-first framing.
