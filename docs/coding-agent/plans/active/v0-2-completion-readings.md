# v0.2 completion: readings

The run's readings file under the v0.2 completion brief (`docs/coding-agent/briefs/active/v0-2-completion-brief.md`). The Orchestrator writes a reading here before each value audit; the auditor opens it only after its grades are fixed.

## Readings

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

## Findings
