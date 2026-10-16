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

- Verdict comparison for the situated recall plan revision (2026-10-16, at 6e6eea8c), as returned:
  - Reading compared: Goal unread as an item; DoD 1 to 10 agree (DoD 3: needs the owner against ask-now); Planner-added 1, 3, 4, 6, 7, 8, 10, 11, 12 agree; 2 diverges (covered on gives 3; auditor inferred, the cursor does not resume a matter- or topic-anchored gathering); 5 diverges (covered on the constraint and the Decision Log's answer; ask-now, outward-facing with no standing approval, the log not support, not counted against the Plan Gate); 9 agrees with the direction mark; Non-goals agree; Compatibility agrees; A1, A2, A3 agree; Task_1, 2, 3, 5, 6, 7 agree; Task_4 diverges (covered; ask-now on the live null spend); Decision Log 1 agrees; 2 diverges (covered on the in-session answer; ask-now); 3 diverges on the shipped-threshold ruling only; 4 agrees; 5 agrees.
  - Findings compared: none found under Findings; departures on item lines: DoD 4's omitted disposition agrees (nothing to hand over in v0.2); Planner-added 2's continuation gap for matter and topic anchors bears on the design and not recorded, graded inferred.
  - Correction inside the run: the continuation gap recorded as a finding below and escalated; the live-null item is read as needing the owner's authorization held by the Orchestrator, which it has.

### v0-2-residuals-plan, plan draft (revision af3f048a)

- Goal (three residuals) | covered | brief section 4: "the subject-state read's speed, statistics recovery ... proceed"; the busy-occasion fix "waits".
- DoD: subject-state read selects the same, at parity with trigger and due reads | covered | brief section 4, the subject-state read's speed.
- DoD: an unhealthy statistics store heals at the next open, no new call | covered | brief section 4, statistics recovery; philosophy 8.5 (completeness as a promise about reach) is what a conservative store weakens.
- DoD: busy-occasion selection keeps current, salient, then newest | extends | philosophy 7.3 (the unexpected is what comes to mind and lasts) and 8.2 (feeling kept by keeping the experience); held under brief section 4.
- DoD: measurement at the Wave 1 tip, busy-occasion family on release | covered | brief section 4 split of what proceeds and what waits.
- Planner-added: 1 s falsifier | covered | internal mechanics; no engineering philosophy.
- Planner-added: bound stays in Rust | covered | internal mechanics.
- Planner-added: batched membership read | covered | internal mechanics.
- Planner-added: rebuild at open | covered | brief section 4, statistics recovery.
- Planner-added: failed rebuild keeps marker and original cause | covered | internal mechanics.
- Planner-added: timing samples interleaved, run twice | covered | internal mechanics.
- Planner-added: one key for both expansion paths | covered | internal mechanics.
- Planner-added: Task_1 timing witness outside the suite | covered | internal mechanics.
- Planner-added: Task_2 no public call or setting | covered | brief section 4, statistics recovery without an operator.
- Planner-added: Task_2 witness must change selection at the base | covered | internal mechanics.
- Planner-added: Task_3 tie key | extends | philosophy 7.3 and 8.2; held under brief section 4.
- Non-goals (unlived-topic floor, root seat cost, LoCoMo question_id, ADR-I-0007 constants, no new API, no other selection change) | covered | brief section 4, the re-examination owns the design.
- Task_1 | covered | brief section 4.
- Task_2 | covered | brief section 4.
- Task_3 | covered | brief section 4, held until the amendment.
- Decision Log 2026-10-15, decider chose the residuals | covered | owner's ruling 81, superseded in force by brief section 4.
- Decision Log 2026-10-15, Task_2 root cause and graph read | covered | internal mechanics.
- Decision Log 2026-10-15, Task_4 (now Task_3) root cause and key | extends | philosophy 7.3; held.
- Decision Log 2026-10-15, drop Task_3 (second person) | covered | owner, 2026-10-15, "Drop it".
- Decision Log 2026-10-15, first review applied | covered | internal mechanics.
- Decision Log 2026-10-16, runs under the brief, busy-occasion held | covered | brief section 4.
- Decision Log 2026-10-16, Task_1 root cause and design | covered | internal mechanics.
- Decision Log 2026-10-16, re-review applied | covered | internal mechanics.
- Decision Log 2026-10-16, second re-review applied | covered | internal mechanics.

- Verdict comparison (2026-10-16), as returned:
  - Reading compared: Goal unread as an item; DoD 1 agrees; DoD 2 agrees; DoD 3 diverges (Orchestrator extends, philosophy 7.3 and 8.2, held; auditor ask-now, the brief withholds the key and rulings 65 and 70 are not grounds); DoD 4 read covered, recorded not audited (internal); 1 s falsifier diverges (covered as internal; auditor inferred, user-observable latency line); bound in Rust, batched read, timing samples, witness outside the suite, Task_2 witness design consistent (internal); rebuild at open agrees; failed rebuild keeps marker and cause diverges (covered as internal; auditor inferred, developer-visible diagnostics); one key for both paths diverges (covered as internal; auditor ask-now, part of the held key); no public call or setting diverges (covered, "without an operator"; auditor inferred, the brief does not say "without an operator"); Task_3 tie key diverges (extends; ask-now); mid-session limit assumption unread; non-goals agree; integration unread; Task_1 agrees; Task_2 agrees; Task_3 diverges (covered, held; ask-now); DL residuals chosen agrees on scope, the plan-approval waiver unread; DL Task_2 root cause consistent; DL Task_4 key diverges (extends; ask-now); DL drop of the second-person residual agrees on the grade, diverges on the source; DL first review applied diverges (covered as internal; ask-now for its key part); DL runs under the brief agrees; DL Task_1 root cause consistent; DL re-review of 7bf694ff diverges (covered as internal; ask-now for its Task_3 design part); DL second re-review consistent.
  - Findings compared: none found.
  - Correction inside the run: the diverging items were the busy-occasion task and its key, which leave the plan in the revision below; items read as internal that the auditor graded inferred are read by their user-facing effect in the next reading.

### v0-2-residuals-plan, plan draft, revision narrowed to the statistics recovery

- Goal (statistics recovery; the other two moved to the 4a design plan) | covered | brief section 4a, "What this leaves standing of the open stack: the storage and schema work".
- DoD: unhealthy store heals at the next open, no new call, failed rebuild keeps marker | covered | brief section 4, statistics recovery; section 4a, storage work stands.
- DoD: measurement with witness and controls | covered | internal mechanics.
- Planner-added: rebuild at open | covered | brief section 4, statistics recovery.
- Planner-added: failed rebuild keeps marker and original cause | extends | philosophy 8.6 (expose retrieval rationale) and section 10 (a decision that stops a memory's influence is visible).
- Planner-added: no public call or setting | extends | philosophy section 9 (a memory system, not a database client).
- Planner-added: witness changes selection at the base | covered | internal mechanics.
- Non-goals (moved residuals, no selection change, no API, no stored-shape change) | covered | brief section 4a replaces the recall selection.
- Design limit: mid-session failure stays conservative until next open | extends | brief section 4, statistics recovery.
- Task_1 | covered | brief section 4, statistics recovery.
- Decision Log 2026-10-16: narrowed under 4a, moved tasks, waiver corrected | covered | brief section 4a; brief section 1 governance.

- Verdict comparison for the narrowed revision (2026-10-16), as returned:
  - Reading compared: Goal diverges (Orchestrator covered by 4a's storage line; auditor: busy-occasion move cited, subject-state read's move inferred, since section 4 names that read among work that proceeds); DoD heals at next open agrees; DoD measurement agrees (internal); rebuild at open agrees; failed rebuild keeps marker and cause agrees; no public call agrees; witness at base agrees; Non-goals diverge on the subject-state read's move only; mid-session limit agrees; Task_1 agrees; Decision Log narrowing agrees. Unread under this heading: the hydrator-exclusions Design item, Integration, Scope, the earlier Decision Log entries.
  - Findings compared: none found.
  - Correction inside the run: the subject-state read's move is an inferred deferral, not covered; the 4a design plan's Context states where that work goes, so the brief's "proceeds" item is not lost.

### v0-2-residuals-plan, closeout

- The rebuild at open and the cleared marker (composition.rs, adapters/stats/sqlite.rs) | covered | brief section 4, statistics recovery; 4a storage work stands.
- The hydrator's two MemoryLink exclusions deleted; one test expectation changed | covered | internal mechanics; no caller observes a change (reviewer's survey).
- A failed rebuild keeps the marker and original cause; the repeated-failure test | extends | philosophy 8.6, section 10, 8.5.
- Documentation corrected in five places (README, graph schema design, cheat sheet, vector payload design) to state the rebuild and its limits | covered | consumer-facing docs follow the code; philosophy section 9 (honest about what the library does).
- Measurement verdict (witness red/green; controls equal) | covered | internal mechanics; brief section 4.
- Judgement calls: the test expectation change; two doc consistency touches outside owns | covered | internal mechanics.
- Orchestrator rulings: the plan narrowed under 4a; the one-cause limit accepted; the waiver line corrected | covered | brief sections 1, 4, 4a.
- Human-only items: none for this unit.

- Verdict comparison for the residuals plan, closeout (2026-10-16), as returned:
  - Reading compared: rebuild at open agrees; hydrator exclusions agrees; failed rebuild keeps cause agrees (extends / inferred); documentation agrees; measurement verdict agrees; judgement calls agree; the rulings line diverges on two parts (the subject-state read's move: covered against inferred; the one-cause limit: covered against inferred), the waiver correction agrees; human-only items agree. Unread under the closeout heading: the Standing Approvals entry and Counsel line; the renderer supersession ruling; the PRs opened; the healthy-reopen item; the sqlite rebuild's delete-and-rewrite; the mid-session limit; the go for Task_1; the run record; lessons.
  - Findings compared: none found.
  - Correction inside the run: the two parts read as covered are read as extending in later readings.

### v0-2-situated-recall-plan, plan draft, revision with the owner's answer on verified absence

- Goal | covered | brief 4a.
- DoD: situations, paths, ingredients, cut with continuation; no room, no sections | covered | 4a gives 1, 2, 3.
- DoD: open order rule; similarity a condition; coverage reported | covered | 4a gives 2; the resemblance constraint.
- DoD: three absence states, verified absence in the turn if the exhaustive check is cheap, else the compromise returns to the owner | covered | the owner's answer of 2026-10-16 recorded in the Decision Log; 4a gives 5 and the bounded-recall constraint; the snapshot clause as clarified.
- DoD: familiarity and history when touched | covered | 4a gives 4 and 3.
- DoD: the walk | covered | 4a structural-access constraint.
- DoD: one hop and the rest | covered | 4a means and told constraint.
- DoD: records | covered | 4a records paragraph.
- DoD: measurement | covered | 4a measurement constraint.
- DoD: subject-state speed | covered | section 4.
- DoD: memo and example | covered | section 3 means; philosophy 9.
- Planner-added: one bound; paths and cut; fanout as cut; keyset cursor | covered | 4a gives 1, 3; the constraint.
- Planner-added: "cheap" as no slower than a turn's recall at the three-year store, linear growth | extends | the owner's answer names the criterion, not the number.
- Planner-added: live-embedder null shipped | covered | 4a resemblance constraint; the owner's authorization in the Decision Log.
- Planner-added: batched read and witness; standing tests restated; direction and due kept; walk witness sizes; null artifact; example determinism | covered | 4a; internal mechanics.
- Planner-added: order-rule experiment | extends | 4a gives 1 and 2; philosophy 8.1.
- Non-goals | covered | 4a; sections 3, 4, 5; decay wording left as history.
- A1 (snapshot as of the moment it looked) | covered | 4a as clarified.
- A2, A3 | covered | internal mechanics; 4a measurement constraint.
- Task_1 | covered | 4a records paragraph.
- Task_2 | covered | 4a gives 1, 2, 3, 5.
- Task_3 (the walk; the exhaustive check and its cost measurement) | covered | 4a structural-access constraint; the owner's answer.
- Task_4 | covered | 4a gives 1 to 5; the owner's answer for the absence form; the provisional coverage policy built as written; the order rule extends.
- Task_5 | covered | section 4.
- Task_6 | covered | philosophy 9; section 3 means.
- Task_7 | covered | section 3; 4a "What it replaces".
- Decision Log entries, the owner's answers included | covered | 4a; the owner's quoted words of 2026-10-16.

## Findings

- 2026-10-16, situated recall plan: the walk's cursor, used as a cut situation's continuation, has no key for a situation anchored on an unsettled matter or a topic, so for those anchors "offers to continue" reaches nothing. Reading: bears on the design (the character experiences an empty offer). Escalated to Counsel as a value question on 2026-10-16; the continuation for matter- and topic-anchored situations is held until the owner answers.
  - 2026-10-16: Counsel returned the question as below the owner's level and clarified 4a; the hold is released; the finding is closed by the brief's clarified text.
