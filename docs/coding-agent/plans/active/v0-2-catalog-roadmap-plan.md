# Plan: the situation catalog exercises the ratified philosophy, and the roadmap carries the owner's directions for v0.2's close and the phases after it

- status: draft
- generated: 2026-10-16
- last_updated: 2026-10-16
- work_type: docs

## Goal
- Carry out sections 2 and 3 of the v0.2 completion brief: the continuity situation catalog becomes the standard the ratified product philosophy is judged by, and the roadmap and phase drafts carry the owner's directions for how v0.2 closes, what v0.3 is measured by, and where v0.4 and v0.5 contradict the philosophy or accepted records. Nothing about the v0.2 design itself (routes, floors, the fixed room, knowing versus being reminded, the one hop) changes here; that waits for the brief's amendment.

## Definition of Done
- The catalog carries the brief's section 2 revision in full: the failures section (invented continuity, lost continuity, the mirror, and the month-long measure as a reading), the thirteen situations with their human shape and where the ideal AI character departs, the amended purpose paragraph, and the target variants for a topic, a group, a time of year and the self. Every catalog statement it adds or rewrites is a guarantee on what is in mind and how it is held, not a rule about what the character says.
- Each philosophy statement named in the discussion notes' mapping (the 2026-10-16 "Layer 2 opened" entry of `docs/coding-agent/briefs/active/v0-2-completion-notes.md`) has at least one situation, shown by a mapping table in the task report.
- The roadmap and the phase drafts carry the brief's section 3 changes and no others at that altitude: v0.2 closes with the reading of behaviour under its four conditions, and the renderer and example loop come before the evals re-baseline and the paid runs; v0.3's measure, reflection's intent and its added definition of done, with attribution's core moved in and time counted in experience placed as the brief says; v0.4 and v0.5 corrected as the brief says, with familiarity and gist consolidation moved to the phases that need them.
- The owner accepts the revised catalog and the revised roadmap sections (human-only, the brief's pass conditions).

## Planner-added requirements
- The mapping table in Task_1's report: the brief's agent-checkable pass condition needs evidence a reviewer can check line by line; the table is that evidence and is not committed.
- A worker that finds an accepted decision record contradicting a brief change stops and reports it (Task_2): the roadmap may not silently contradict an accepted record, and changing a record is a reserved act of the owner's.

## Scope / Non-goals
- Scope: `docs/design/continuity_situation_catalog.md`; `docs/roadmap/development_roadmap.md`; the phase drafts for v0.2 (its closeout and order only), v0.3, v0.4 and v0.5 under `docs/roadmap/roadmap-phases/`.
- Non-goals: the v0.2 design and its records (the amendment's); the closeout reading's own probes, store and arms (planned after recall and the renderer are frozen, as the brief requires); decision records; the philosophy, the brief and the notes (Counsel's); any code or evals change; v1.0's definition beyond what the phase drafts' changes force.

## Design
- Chosen: two documentation tasks with disjoint files, written from the brief's text on disk, each reviewed for altitude against the brief and the philosophy; no alternative required.

## Context
- Governing brief: `docs/coding-agent/briefs/active/v0-2-completion-brief.md`. Its ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify docs/coding-agent/briefs/active/v0-2-completion-brief.md on those five framings.", admitted by the owner in this session on 2026-10-16: "Take the seat again, and check your inbox history. You should have a message from the Counsel, which carries my word. Take the quotes as mine." Product philosophy: `docs/project_philosophy.md`, ratification in `docs/project_philosophy-companion.md`. Readings file: `docs/coding-agent/plans/active/v0-2-completion-readings.md`.
- The brief's statements bind by their kind marks; the one means in section 2 (guarantees, not restrictions) and the one in section 3 (renderer before the re-baseline) are built from and may be bettered, a departure being recorded as a finding.
- The catalog today keys every situation on a person, place, thread or date (sections A to F, 362 lines). The roadmap is `docs/roadmap/development_roadmap.md` (1,808 lines) with one draft per phase; the v0.2 draft judges the phase at the retrieval tier only ("Behavioral-tier judging is scheduled with the generation phase").
- The notes' 2026-10-16 "Layer 2 opened" entry lists, for the catalog, the philosophy statements with no situation, and for the roadmap, the contradictions found (v0.4's confidence fields, truth stances and source reliability against the philosophy's 8.2 and ADR-D-0030; v0.5's decay and reinforce against ADR-D-0018; gist consolidation in two phases at once). The notes are unratified: the brief decides, and the entry is used only as the brief's section 2 names it, as the checklist.

## Integration
- Branch `feature/2026-10-16/v0-2-catalog-roadmap` from the residuals plan tip `7bf694ff`, which carries the brief; one pull request on the residuals plan's.
- No library code, test or evals file changes, so no measurement item: nothing a character does changes.

## Tasks

### Task_1: The catalog exercises every statement of the ratified philosophy
- type: docs
- owns: docs/design/continuity_situation_catalog.md
- depends_on: []
- acceptance:
  - The brief's section 2 revision is in the catalog: the failures section, the thirteen situations, the purpose paragraph, the target variants.
  - Each added or rewritten situation states the human shape of what is in mind and how it is held, and where the ideal AI character departs, as the brief gives it for that situation.
  - No added or rewritten statement prescribes what the character says or does; each is a guarantee on what is in mind and how it is held.
  - The report carries a table from each philosophy statement in the notes' mapping to the situation that exercises it.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Self-check: the mapping table is complete against the notes' 2026-10-16 Layer 2 entry; every new situation is checked against the brief's section 2 wording for it; no hard-wrapped prose (common.md)"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: the catalog against the brief's section 2 and the ratified philosophy; each situation's wording is a guarantee on recall and inner state, not on words; the mapping table holds; nothing beyond section 2 was added"
  - kind: manual
    required: true
    owner: user
    detail: "The owner accepts the revised catalog (the brief's human-only pass condition)"

### Task_2: The roadmap carries the owner's directions for v0.2's close and the phases after it
- type: docs
- owns: docs/roadmap/development_roadmap.md; docs/roadmap/roadmap-phases/v0_2_scoped_continuity_reflection.md (closeout and order only); docs/roadmap/roadmap-phases/v0_3_memory_generation_and_reflection.md; docs/roadmap/roadmap-phases/v0_4_temporal_validity_attribution_entity_evolution.md; docs/roadmap/roadmap-phases/v0_5_long_horizon_shape_and_associative_recall.md
- depends_on: []
- acceptance:
  - v0.2's closeout includes the reading of behaviour with its four binding conditions, its negative probes and its stated limit, and the rule that if any condition cannot be met the reading is not run and v0.2 closes on the retrieval tier with that limitation written in; the renderer and example loop come before the re-baseline and the paid runs. The v0.2 design text is otherwise untouched.
  - v0.3 carries its measure (the two-characters reading against the same model with the whole transcript), reflection's intent in the owner's terms, the six added definition-of-done items, attribution's core, and time counted in experience placed as the brief says.
  - v0.4 and v0.5 carry the brief's corrections at its altitude: the fact-acceptance machinery leaves the roadmap; decay and reinforce leave v0.5's draft; familiarity and gist consolidation sit in one phase each, where they are needed.
  - No other change at that altitude; an accepted decision record that contradicts a change is reported, not edited.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Self-check: each change traced to the brief's section 3 sentence it carries; a list of every edited heading; no hard-wrapped prose (common.md)"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: the roadmap and drafts against the brief's section 3 and the philosophy; these changes and no others at that altitude; the v0.2 design text untouched; accepted records not contradicted silently"
  - kind: manual
    required: true
    owner: user
    detail: "The owner accepts the revised roadmap sections (the brief's human-only pass condition)"

## Task Waves (explicit parallel dispatch sets)
- Wave 1: [Task_1, Task_2]

## Research
- Research waived: the brief states each change at the altitude it is to be written, and the notes' Layer 2 entry is the Researcher's mapping of the philosophy to the catalog and roadmap, made for Counsel on 2026-10-16. Each worker reads its own files.

## Progress Log (append-only)
- 2026-10-16: plan drafted under the v0.2 completion brief, from revision `785391cf` (the brief and the ratified philosophy).

## Decision Log (append-only)
