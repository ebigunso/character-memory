# Plan: the situation catalog exercises the ratified philosophy, and the roadmap carries the owner's directions for v0.2's close and the phases after it

- status: draft
- generated: 2026-10-16
- last_updated: 2026-10-16
- work_type: docs

## Goal
- Carry out sections 2 and 3 of the v0.2 completion brief: the continuity situation catalog becomes the standard the ratified product philosophy is judged by, and the roadmap and phase drafts carry the owner's directions for how v0.2 closes, what v0.3 is measured by, and where v0.4 and v0.5 contradict the philosophy or accepted records. Nothing about the v0.2 design itself (routes, floors, the fixed room, knowing versus being reminded, the one hop, familiarity's place in it) changes here; the brief's amendment, section 4a, places those in its own design plan.

## Definition of Done
- The catalog carries the brief's section 2 revision in full: the failures section (invented continuity, lost continuity, the mirror, and the month-long measure as a reading), the thirteen situations with their human shape and where the ideal AI character departs, the amended purpose paragraph, and the target variants for a topic, a group, a time of year and the self. The whole catalog is written as guarantees on what is in mind and how it is held, not as rules about what the character says: existing restriction-shaped statements are reworded into that form without changing what their situation tests.
- Each philosophy statement named in the discussion notes' mapping (the 2026-10-16 "Layer 2 opened" entry of `docs/coding-agent/briefs/active/v0-2-completion-notes.md`) has at least one situation, shown by a mapping table in the task report.
- The roadmap and the phase drafts carry the brief's section 3 changes and no others at that altitude, as Task_2's acceptance lists them. One section 3 change is open against its pass condition and carried to the amendment: familiarity's move to the phase that needs it: section 4a makes familiarity part of v0.2's recall design ("Familiarity decides what a thing brings, not how the cue was given"), so the 4a design plan edits v0.2's design text and the roadmap's familiarity rows, and section 3's pass condition for familiarity is met there.
- The record that the roadmap's removal of the fact-acceptance machinery departs from (ADR-D-0005's deferral) has a proposed successor for the owner to accept or decline.
- The owner accepts the revised catalog and the revised roadmap sections (human-only, the brief's pass conditions).

## Planner-added requirements
- The mapping table in Task_1's report: the brief's agent-checkable pass condition needs evidence a reviewer can check line by line; the table is that evidence and is not committed.
- Catalog identifiers stay stable (Task_1): the roadmap (26 references), the v0.2 draft (31), the v0.3 draft, ADR-D-0027, ADR-D-0033, ADR-D-0034, ADR-I-0035 and the evaluation repository's situated-recall plan cite them; a renumber would break citations this plan may not repair.
- The roadmap cites new situations by the brief's names (Task_2): the two tasks run in parallel, so Task_2 cannot know Task_1's new identifiers.
- Near-neighbour situations are cross-referenced (Task_1): no two unconnected entries describe one situation.
- Task_3, a proposed successor to ADR-D-0005's deferral: the brief removes from the roadmap what an accepted record defers to later stages; an accepted record may not be contradicted silently, and changing one is the owner's act.
- A worker that finds another accepted record contradicting a brief change stops and reports it (Task_2): for a conflict nobody has seen yet; the known one is Task_3's.

## Scope / Non-goals
- Scope: `docs/design/continuity_situation_catalog.md`; `docs/roadmap/development_roadmap.md`; the phase drafts for v0.2 (its closeout, its order, and pointer lines naming another phase), v0.3, v0.4 and v0.5 under `docs/roadmap/roadmap-phases/`; one proposed design record.
- Non-goals: the v0.2 design and its records (the amendment's), familiarity's placement included, since section 4a places it in v0.2's recall design and the 4a design plan carries it; the closeout reading's own probes, store and arms (planned after recall and the renderer are frozen, as the brief requires); editing any accepted record; the philosophy, the brief and the notes (Counsel's); any code or evals change; v1.0's definition beyond what the phase drafts' changes force.

## Design
- Chosen: three documentation tasks with disjoint files, written from the brief's text on disk, each reviewed for altitude against the brief and the philosophy; no alternative required.

## Context
- Governing brief: `docs/coding-agent/briefs/active/v0-2-completion-brief.md`. Its ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify docs/coding-agent/briefs/active/v0-2-completion-brief.md on those five framings.", admitted by the owner in this session on 2026-10-16: "Take the seat again, and check your inbox history. You should have a message from the Counsel, which carries my word. Take the quotes as mine." Product philosophy: `docs/project_philosophy.md`, ratification in `docs/project_philosophy-companion.md`. Readings file: `docs/coding-agent/plans/active/v0-2-completion-readings.md`. Run record: `docs/coding-agent/plans/active/v0-2-completion-run.md`.
- The brief's statements bind by their kind marks. Section 2's means ("written throughout as guarantees on what is in mind and how it is held, not as restrictions on behaviour") is built as written, for the whole catalog. Section 3's means (renderer and example loop before the re-baseline and the paid runs) is built as written.
- The catalog keys every situation on a person, place, thread or date (sections A to F, 362 lines). Several new situations sit beside existing ones (return after absence by A1 and D11; a story told five times by C4; the hundred Tuesdays by E13; lately by D10; a familiar place by D3; the last thing she said by C6): they are cross-referenced, not merged.
- ADR-D-0005 (accepted) defers first-class `Assertion`, `Claim`, `EvidenceLink`, `BeliefAssessment` and `SourceAssessment` to later stages and rejected never implementing them; its revisit clause (line 95) names source credibility and contradiction handling. The roadmap's v0.4 row keeps the ontology "behind ADR-D-0005's revisit clause". ADR-D-0016 (line 61) and ADR-D-0035 touch the same concepts.
- The roadmap is `docs/roadmap/development_roadmap.md` (1,808 lines) with one draft per phase; the v0.2 draft judges the phase at the retrieval tier only ("Behavioral-tier judging is scheduled with the generation phase"). Familiarity appears in v0.2's design text and in v0.5; gist consolidation in the v0.3 draft and the v0.5 roadmap section.
- The notes' Layer 2 entry is used only as the brief's section 2 names it, as the checklist.

## Integration
- Branch `feature/2026-10-16/v0-2-catalog-roadmap` on the residuals plan branch, which carries the brief; one pull request on the residuals plan's.
- No library code, test or evals file changes, so no measurement item: nothing a character does changes.

## Tasks

### Task_1: The catalog exercises every statement of the ratified philosophy
- type: docs
- owns: docs/design/continuity_situation_catalog.md
- depends_on: []
- acceptance:
  - The brief's section 2 revision is in the catalog: the failures section, the thirteen situations under the brief's names, the purpose paragraph, the target variants.
  - Each new situation states the human shape of what is in mind and how it is held, and where the ideal AI character departs, as the brief gives it; each near-neighbour among the existing situations is cross-referenced.
  - Every statement in the catalog is a guarantee on what is in mind and how it is held; existing restriction-shaped statements are reworded without changing what their situation tests.
  - No existing identifier changes and no situation is removed or merged; new situations take new identifiers.
  - The report carries a table from each philosophy statement in the notes' mapping to the situation that exercises it.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Self-check: the mapping table is complete against the notes' 2026-10-16 Layer 2 entry; every new situation is checked against the brief's section 2 wording for it; every reworded statement listed with its identifier; no hard-wrapped prose (common.md)"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: the catalog against the brief's section 2 and the ratified philosophy; each statement is a guarantee on recall and inner state, not on words; rewordings keep what each situation tests; identifiers stable; the mapping table holds"
  - kind: manual
    required: true
    owner: user
    detail: "The owner accepts the revised catalog (the brief's human-only pass condition)"

### Task_2: The roadmap carries the owner's directions for v0.2's close and the phases after it
- type: docs
- owns: docs/roadmap/development_roadmap.md; docs/roadmap/roadmap-phases/v0_2_scoped_continuity_reflection.md (closeout, order, and pointer lines naming another phase); docs/roadmap/roadmap-phases/v0_3_memory_generation_and_reflection.md; docs/roadmap/roadmap-phases/v0_4_temporal_validity_attribution_entity_evolution.md; docs/roadmap/roadmap-phases/v0_5_long_horizon_shape_and_associative_recall.md
- depends_on: []
- acceptance:
  - v0.2: the closeout includes the reading of behaviour with its four binding conditions, its negative probes and its stated limit, and the rule that if any condition cannot be met the reading is not run and v0.2 closes on the retrieval tier with that limitation written in; the order is renderer and example loop, then the reading, then the evals re-baseline and the paid runs; a pointer naming another phase follows a move this task makes; the v0.2 design text is otherwise untouched.
  - v0.3: its measure (the two-characters reading against the same model with the whole transcript in context), reflection's intent in the owner's terms, the six added definition-of-done items, attribution's core (witnessed or told) moved in, and time counted in experience written as a decision for v0.3's planning: in v0.3 if cheap over the trace store, otherwise the first named item of v0.4.
  - v0.4: what the character holds about the world over time (validity intervals, aliases and roles over time, who else knows, source reliability as the character's own belief about a source); attribution's core leaves it; the first-class fact-acceptance objects (ADR-D-0005's `Assertion`, `Claim`, `EvidenceLink`, `BeliefAssessment`, `SourceAssessment`) leave the roadmap, while ADR-D-0035's assertions stay; the roadmap cites ADR-D-0040 as proposed, and the citation is removed at integration if Task_3 proposes no record.
  - v0.5: the long horizon proper (a decade of gists, recognition of a returning stranger, query-time associative activation, durable units on measured demand); decay and reinforce leave its draft; the long-horizon consolidation of periphery into gist sits in v0.5 alone, and v0.3's episode and day gists stay, by the ruling in the Decision Log.
  - No other change at that altitude; new situations are cited by the brief's names; another accepted record found contradicting a change is reported, not edited.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Self-check: each change traced to the brief's section 3 or 5 sentence it carries; a list of every edited heading; no hard-wrapped prose (common.md)"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: the roadmap and drafts against the brief's sections 3 and 5 and the philosophy; these changes and no others at that altitude; the v0.2 design text untouched; accepted records not contradicted silently"
  - kind: manual
    required: true
    owner: user
    detail: "The owner accepts the revised roadmap sections (the brief's human-only pass condition)"

### Task_3: The deferral of the fact-acceptance objects is put to the owner as a record
- type: docs
- owns: docs/decisions/design/ADR-D-0040-*.md (one new file; ADR-D-0040 is the next free number on every branch)
- depends_on: []
- acceptance:
  - Propose an ADR if the admission test passes (`durable-docs-authoring/references/adr.md`): the brief's removal of the first-class fact-acceptance objects from the roadmap, as the outcome of ADR-D-0005's revisit, with status proposed; ADR-D-0035's assertions (commitments carried by ordinary beliefs) stay.
  - ADR-D-0005 and every other accepted record stay unedited; the report names each accepted record whose text the proposal would affect (ADR-D-0016 and ADR-D-0035 checked).
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: one decision, at the brief's altitude, consistent with the philosophy's 8.2 and ADR-D-0030; no accepted record edited"
  - kind: manual
    required: true
    owner: user
    detail: "The owner accepts or declines the proposed record (a reserved act, carried by Counsel)"

## Task Waves (explicit parallel dispatch sets)
- Wave 1: [Task_1, Task_2, Task_3]

## Research
- Research waived: the Orchestrator read the brief, its notes (the Layer 2 mapping), the catalog's sections and headings, the roadmap and phase drafts' sizes, ADR-D-0005's deferral and revisit clause, and the plan review's census of inbound citations of catalog identifiers (roadmap, v0.2 and v0.3 drafts, ADR-D-0027, ADR-D-0033, ADR-D-0034, ADR-I-0035, the evaluation repository's situated-recall plan). Each task's scope, acceptance and validation are written from those.

## Progress Log (append-only)
- 2026-10-16: plan drafted under the v0.2 completion brief, from revision `785391cf` (the brief and the ratified philosophy).

- 2026-10-16: plan-draft value audit dispatched at `6fa3b094` (plan at 1182deea), text verbatim: "You are the value auditor. Read references/value-audit-mandate.md of the orchestration-harness skill and follow it exactly. Position: plan draft. Plan: docs/coding-agent/plans/active/v0-2-catalog-roadmap-plan.md. Governing brief: docs/coding-agent/briefs/active/v0-2-completion-brief.md. Changes since: none."
- 2026-10-16: verdict as returned (comparison lines in the readings file). Position: plan draft. Documents read: the plan; the brief; the philosophy (pointer line common.md:13); its companion; common.md (Standing Approvals at HEAD: the Counsel entry "pending", not in effect, no item relies on it). Read to understand items, never as support: ADR-D-0005, ADR-D-0035's title and decision lines, the catalog's headings and purpose, the roadmap's v0.4 and v0.5 rows, gist lines of the v0.3 and v0.5 drafts, the record number listing (highest ADR-D-0039). Notes never opened; readings opened once after grading. Product basis: philosophy (companion, second ratification) and the brief, section 8, 4a at line 63. Not audited: the engineering side (no document). Missing inputs: none. Value documents changed in range: none.
  - DoD: catalog carries section 2 in full, guarantees throughout | maps to the brief | cited | brief s2 and its means "is written throughout as guarantees ..." | -
  - DoD: mapping table | maps to the brief | cited | brief s2 pass condition | -
  - DoD: roadmap carries s3 and no others; familiarity open and carried | maps to the brief | inferred | extends brief s3 "Familiarity and gist consolidation move down ..." and its pass condition, with 4a "Familiarity decides what a thing brings ..." and line 63 "waits on nothing"; the plan's reason was stale: the destination is decided, what remains is sequencing, cheap to undo and openly left unmet | -
  - DoD: proposed successor to ADR-D-0005's deferral | maps to the brief | inferred | extends brief s3 removal statement and 4a's "records the Orchestrator proposes and the owner accepts"; cheap to undo | -
  - DoD: owner accepts catalog and roadmap | maps to the brief | cited | brief s2 and s3 human-only conditions | -
  - Planner-added: mapping table uncommitted | maps to the brief | cited | brief s2 pass condition | -
  - Planner-added: identifiers stable | maps to the brief | inferred | extends s2 "adds thirteen situations"; cheap to undo | -
  - Planner-added: roadmap cites by the brief's names | maps to the brief | inferred | extends s3 altitude line; cheap to undo | -
  - Planner-added: near-neighbours cross-referenced | maps to the brief | inferred | extends s2; cheap to undo | -
  - Planner-added: Task_3's record | maps to the brief | inferred | as the ADR-D-0005 DoD item | -
  - Planner-added: stop and report on an unseen record conflict | maps to the brief | inferred | extends s1 governance; tightens | -
  - Non-goal: v0.2 design and records, familiarity included | maps to the brief | cited | s4 proceeds list; 4a line 55 | -
  - Non-goal: the reading's probes, store, arms | maps to the brief | cited | s3 | -
  - Non-goal: editing any accepted record | maps to the brief | inferred | extends 4a line 55; tightens | -
  - Non-goal: the value documents | maps to the brief | cited | s1; line 13 | -
  - Non-goal: code or evals | maps to the brief | cited | s4 | -
  - Non-goal: v1.0 | maps to the brief | cited | s3 "at the altitude stated and no further" | -
  - Design | internal mechanics | not audited | - | -
  - Context: both means built as written | maps to the brief | cited | s2 means; s3 means (line 30, s8 framing) | -
  - Context: the notes' Layer 2 entry as the checklist only | maps to the brief | cited | s2 pass condition | -
  - Integration | internal mechanics | not audited | not irreversible; no merge to a shared branch; s5's merge constraint untouched and its condition met | -
  - Task_1 | maps to the brief | cited | s2 | -
  - Task_2 | maps to the brief | cited | s3 (reading, four conditions, limitation rule; v0.3 measure and DoD gains; attribution's core; time in experience; v0.4 and v0.5), s5; ADR-D-0035's assertions kept apart on the brief's exact set and philosophy 8.2 | -
  - Task_2 sub-item: conditional ADR-D-0040 citation | maps to the brief | inferred | extends s3; cheap to undo | -
  - Task_3 | maps to the brief | inferred | as the ADR-D-0005 item; acceptance human-only | -
  - Decision Log: ADR-D-0005 ruling | maps to the brief | cited | s3 "removed from the roadmap rather than kept conditional" | -
  - Decision Log: familiarity waits | maps to the brief | inferred | as the DoD item | -
  - Decision Log: gist reading | maps to the brief | inferred | extends s3; the added-not-substituted part cited on s3's DoD gains and philosophy 8.2 | -
  - Decision Log: time in experience | maps to the brief | cited | s3 | -
  - Decision Log: pointers follow moves | maps to the brief | inferred | extends s3 | -
  - Decision Log: identifiers, names, cross-references | maps to the brief | inferred | as above | -
  - Decision Log: "throughout" | maps to the brief | cited | s2 means | -
  - Decision Log: order | maps to the brief | cited | s3 means and s5 | -
  - Decision Log: re-review fixes | as graded above; the number is internal mechanics, not audited (ADR-D-0040 next free) | -
  - Human-only conditions pending: the owner accepts the revised catalog; the owner accepts the revised roadmap sections.
  - Scenarios: none
  - No item ask-now or ungraded. Inferred calls journaled: familiarity's sequencing; the proposed record; identifiers stable; names cited; cross-references; the stop-and-report rule; no record edits; the conditional citation; the gist reading; pointers follow moves.
- 2026-10-16: the auditor's note outside the record applied as wording: the familiarity reason names 4a and the 4a design plan as the unit that carries familiarity's move. No grade changes; the closeout audit grades the changed text.

- 2026-10-16: Wave 1 done. Task_1 catalog (75f4bb55, Claude worker; Tier A APPROVED; spelling made consistent and G8's coverage clause dropped on the reviewer's minor notes). Task_2 roadmap (6cd1e757, Claude worker; Tier A APPROVED; four minor advisories kept as written). Task_3 ADR-D-0040 (5f368440, cm-worker; Tier A CHANGES REQUESTED: carry ADR-D-0005's subtype decision forward in the boundary, name ADR-D-0016 and ADR-D-0001 in More Information, exact consulted model name; returned to the worker). Human-only acceptances sent to the owner through Counsel. Judgement calls: Task_1's G8 says "once reflection produces the character's own tendencies" for "testable from v0.3" (the catalog names no phases); Task_2 moved the C3/C5 acceptance line to v0.3 with attribution's core and let volatility, review-after and the confidence view leave v0.4 with the machinery (ADR-D-0018, ADR-D-0030); v0.4 keeps a point-in-time recall line as a reading of validity intervals, also listed as a planning question.
- 2026-10-16: handed to the 4a design plan's records task: the stale decay wording in ADR-D-0013 and ADR-D-0014 (overridden by ADR-D-0018) and ADR-I-0017's `last_reinforced_at`. Notes for Counsel: philosophy 2.4 lists five departures while the catalog's purpose lists nine (four grounded in 7.4, 8.3, 10 and the brief); the brief's G10 clause reads "empty only for the AI" where the coherent reading, and the catalog, say "empty only for a person".

- 2026-10-16: Task_3 ADR-D-0040 revised (64eb0c3a, consulted entry b71ed5c2; Tier A re-review APPROVED), merged here at 6c8479f6. Wave 1 complete; all Reviewer items pass. Pending: the three human-only items (the owner accepts the catalog; the roadmap sections; the proposed record), carried through Counsel. Closeout audit next.

## Decision Log (append-only)
- 2026-10-16 Decision: the plan review of b3d03f6b (CHANGES REQUESTED) applied, with these rulings.
  - ADR-D-0005: the roadmap carries the owner's ratified removal, and Task_3 proposes the record that makes it the outcome of ADR-D-0005's revisit, for the owner to accept. Holding the v0.4 portion instead was rejected: the brief states the removal outright, and the record is the owner's act either way.
  - Familiarity's placement goes with the 4a design plan: section 4a places familiarity in v0.2's recall design, so that plan edits v0.2's design text and the roadmap's familiarity rows; this plan leaves them.
  - Gist consolidation: "move down to the phases that need them" is read as the later phase where its use arises. v0.3's episode and day gists stay, since its reflection is built on them and ADR-D-0027 and ADR-D-0033 rest on them; what sits in v0.5 alone is the long-horizon consolidation of periphery into gist, the decade of gists; the peak-and-end item is added to v0.3's compression, not substituted for it. Cheap to undo; logged for the closeout audit.
  - Time counted in experience is written into the roadmap as a decision for v0.3's planning, which judges "cheap" from a cost reading over the trace store; this plan does not judge it.
  - A pointer in v0.2's text naming another phase (for example attribution fields "(v0.4)") follows the move, and is not design text.
  - Catalog identifiers are stable; new situations take new identifiers and are cited in the roadmap by the brief's names; near-neighbours are cross-referenced.
  - "Written throughout" is read as the whole catalog, so existing restriction-shaped statements are reworded; "nothing beyond section 2" is dropped.
  - The order is renderer and example loop, the reading, then the re-baseline and the paid runs (sections 3 and 5).
- 2026-10-16 Decision: the re-review of b5207af4 (CHANGES REQUESTED, one major) applied: the gist ruling keeps v0.3's episode and day gists; ADR-D-0040 is fixed as Task_3's number and Task_2's citation is conditional; familiarity's move is recorded as open against the section 3 pass condition; the cross-referencing requirement is listed; ADR-D-0035's assertions are kept apart from the first-class objects that leave.
- 2026-10-16 Decision: ADR-D-0040 supersedes ADR-D-0005 in full and carries its subtype decision (derived kinds by `derived_type`, no per-kind objects) forward in its boundary, since that decision is live in code and cited by the roadmap and partial supersession does not exist; ADR-D-0016's EvidenceLink line and ADR-D-0001's expectation are named in the record. The owner is asked to accept the record in that shape.
