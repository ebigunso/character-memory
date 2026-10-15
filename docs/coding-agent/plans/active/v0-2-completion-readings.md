# v0.2 completion: readings

The run's readings file under the v0.2 completion brief (`docs/coding-agent/briefs/active/v0-2-completion-brief.md`). The Orchestrator writes a reading here before each value audit; the auditor opens it only after its grades are fixed.

## Readings

### v0-2-situated-recall-plan, plan draft

- Goal | covered | brief 4a, the whole amendment.
- DoD: situations with parts, paths, ingredients, cut with continuation; no room, no sections | covered | 4a gives 1, 2, 3.
- DoD: open order rule; similarity a condition; coverage reported | covered | 4a gives 2 and the constraint on reach by resemblance.
- DoD: absence states in the turn; verified absence as a separate call | extends | 4a gives 5 ("The states reach the character in the ordinary result") with the constraint "bounded foreground recall"; the separate call is the Orchestrator's reading of how an exhaustive check and a bounded turn coexist.
- DoD: familiarity, history gathered when touched | covered | 4a gives 4.
- DoD: the walk | covered | 4a constraint "complete structural access ... resumable retrieval that neither skips nor silently stops".
- DoD: one hop, weak associations, aboutness, scene before topic, write path, time as clock | covered | 4a means and constraint.
- DoD: records | covered | 4a "Records" paragraph.
- DoD: the measurement | covered | 4a "Measurement" constraint.
- DoD: subject-state read speed | covered | brief section 4 (proceeds), carried here.
- DoD: memo and example | covered | brief section 3 means (renderer and example loop before the re-baseline); philosophy 9.3.
- Planner-added: one bound | covered | 4a gives 1 "down to the bound the application gives".
- Planner-added: paths and cut as count plus cursor | covered | 4a gives 3.
- Planner-added: fanout pruning reported | covered | 4a gives 3 "where the bound cut the gathering it says so".
- Planner-added: keyset cursor | covered | 4a constraint "neither skips nor silently stops"; internal mechanics otherwise.
- Planner-added: verified absence in its own call, graph health, snapshot statement | extends | 4a gives 5 and the bounded-recall constraint.
- Planner-added: live-embedder null as the shipped threshold | covered | 4a constraint "measured against a null distribution on the intended embedder"; owner's in-session authorization.
- Planner-added: batched subject read, witness outside the suite | covered | internal mechanics.
- Planner-added: standing tests restated | covered | 4a "What this leaves standing".
- Planner-added: direction, due_state, resolved_by kept | covered | 4a gives 2 (owed, to whom, due).
- Planner-added: order-rule experiment | extends | 4a gives 2 (comparable ingredients, reported) and the measurement constraint.
- Planner-added: walk witness sizes and writes between pages | covered | internal mechanics.
- Planner-added: records amendment for D-0013, D-0014, I-0017 | extends | 4a's records paragraph (intact records constrain the design; two of them contradict a later one).
- Planner-added: null artifact kept | covered | internal mechanics.
- Planner-added: example deterministic and cleans up | covered | internal mechanics.
- Non-goals (write path, v0.3 ingredients unknown, time in experience, consolidation's reach, stats recovery, catalog and later roadmap, paid runs and reading, learned selection, harness work) | covered | 4a (v0.3 items; time as clock), brief sections 3, 4, 5; common.md on harness work.
- Design: contract then gathering, anchor-based situations | extends | 4a gives 1 and 3 name the experience, not the decomposition.
- Compatibility: break | covered | no consumers (repository rule); the companion adapts in its own plan.
- Assumptions A1 to A3 | covered | internal mechanics.
- Task_1 | covered | 4a records paragraph.
- Task_2 | covered | 4a gives 1, 2.
- Task_3 | covered | 4a structural-access constraint; extends for verify_absence (as above).
- Task_4 | covered | 4a gives 1 to 5, both provisional statements built as written.
- Task_5 | covered | brief section 4.
- Task_6 | covered | brief section 3 means; philosophy 9.3.
- Task_7 | covered | 4a records paragraph; brief section 3 pass condition for familiarity.
- Decision Log: one plan, seven tasks; v0.2 unknown ingredients; coverage policy only on measurement; share budget re-decided | covered / extends | 4a; ADR-D-0033; 4a provisional line 1.
- Decision Log: live null authorized | covered | owner, in session, 2026-10-16.
- Decision Log: review applied (verify_absence separate; cursor; history when touched; shipped threshold; owns; graph health) | extends | 4a gives 3, 4, 5 and the bounded-recall constraint.
- Decision Log: re-review minors applied | covered | internal mechanics.

- Verdict comparison for the situated recall plan (2026-10-16, at 1c380f3a), as returned:
  - Reading compared: Goal unread as an item; DoD 1, 2, 4 to 10 agree; DoD 3 diverges (Orchestrator extends; auditor ask-now, the two ratified statements conflict and the auditor never resolves a conflict); Planner-added 1 to 4 agree; 5 diverges (extends; ask-now, plus "at the time of reading" against "at a stated snapshot"); 6 diverges (covered on the constraint and the in-session authorization; ask-now, outward-facing with no standing approval, the grade on the record); 7, 8, 9 agree; 10 agrees with the direction mark added; 11 agrees; 12 diverges (extends; ask-now scope expansion); 13, 14 agree; Non-goals agree; Design and Compatibility unread; A1 diverges (covered, internal; ask-now, the snapshot statement is handed to the character); A2 agrees; A3 agrees in effect; Task_1, 2, 5, 6, 7 agree; Task_3 diverges (covered; ask-now carried from DoD 3); Task_4 diverges (covered; ask-now on the live null call); Decision Log 1 agrees; 2 diverges (covered; ask-now); 3 diverges (extends; ask-now); 4 diverges (covered, internal; ask-now, the decay amendment).
  - Findings compared: none found; departures noted (DoD 3 and Planner-added 5 from gives 5; DoD 4's omitted disposition from gives 4) are from gives-statements and carried in their grades.
  - Correction inside the run: verified absence carved out and its question sent to the owner; the decay amendment dropped; the next reading reads an exhaustive check as needing the owner, not as the Orchestrator's extension.

### v0-2-situated-recall-plan, plan draft, revision without verified absence

- Goal | covered | brief 4a.
- DoD: situations, paths, ingredients, cut with continuation; no room, no sections | covered | 4a gives 1, 2, 3.
- DoD: open order rule; similarity a condition; coverage reported | covered | 4a gives 2; the resemblance constraint.
- DoD: nothing found and thin in the turn; verified absence held as its own unit on the owner's answer | needs the owner (the held part); the two built states covered by 4a gives 5.
- DoD: familiarity and history when touched | covered | 4a gives 4 and 3.
- DoD: the walk | covered | 4a structural-access constraint.
- DoD: one hop and the rest | covered | 4a means and told constraint.
- DoD: records | covered | 4a records paragraph.
- DoD: measurement | covered | 4a measurement constraint.
- DoD: subject-state speed | covered | section 4.
- DoD: memo and example | covered | section 3 means; philosophy 9.
- Planner-added: one bound; paths and cut; fanout as cut; keyset cursor | covered | 4a gives 1, 3; the constraint.
- Planner-added: live-embedder null shipped | covered | 4a resemblance constraint; the owner's answer in the Decision Log.
- Planner-added: batched read and witness | covered | internal mechanics.
- Planner-added: standing tests restated | covered | 4a standing and replaced lists.
- Planner-added: direction, due_state, resolved_by | covered | 4a gives 2.
- Planner-added: order-rule experiment | extends | 4a gives 1 and 2; philosophy 8.1.
- Planner-added: walk witness sizes; null artifact; example determinism | covered | internal mechanics.
- Non-goals (verified absence held; decay wording left as history; the rest) | covered | 4a; sections 3, 4, 5; the held part needs the owner.
- A1 (snapshot probe as input to the held unit) | covered | internal mechanics.
- A2, A3 | covered | internal mechanics; 4a measurement constraint.
- Task_1 | covered | 4a records paragraph.
- Task_2 | covered | 4a gives 1, 2, 3, 5.
- Task_3 (the walk) | covered | 4a structural-access constraint.
- Task_4 | covered | 4a gives 1 to 5, provisional coverage policy built as written; the order rule extends.
- Task_5 | covered | section 4.
- Task_6 | covered | philosophy 9; section 3 means.
- Task_7 | covered | section 3; 4a "What it replaces".
- Decision Log entries | covered | as above; the carve-out entry records the question sent and the part held.

## Findings
