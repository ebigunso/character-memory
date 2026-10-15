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
### v0-2-catalog-roadmap-plan, plan draft (revision 1182deea)

- Goal | covered | brief sections 2 and 3; the v0.2 design waits (preamble and section 4).
- DoD: catalog carries section 2, whole catalog as guarantees | covered | brief section 2, including its means "written throughout as guarantees on what is in mind and how it is held".
- DoD: mapping from the notes' philosophy statements to situations | covered | brief section 2 pass condition.
- DoD: roadmap carries section 3 and no others, familiarity's move open and carried to the amendment | covered | brief section 3 pass condition; section 4 holds work on the choices under review, standing against familiarity among them.
- DoD: proposed successor to ADR-D-0005's deferral | extends | brief section 3 ("removed from the roadmap rather than kept conditional"); the record is the owner's to accept.
- DoD: owner accepts catalog and roadmap | covered | brief sections 2 and 3 human-only conditions.
- Planner-added: mapping table in the report | covered | brief section 2 agent-checkable condition.
- Planner-added: catalog identifiers stable | extends | brief section 2 adds and amends, nowhere renumbers.
- Planner-added: roadmap cites new situations by the brief's names | covered | brief section 2 names them.
- Planner-added: near-neighbours cross-referenced | extends | brief section 2.
- Planner-added: Task_3 proposed record | extends | brief section 3; record acceptance is the owner's reserved act.
- Planner-added: stop and report on an unseen record conflict | covered | brief section 1 governance.
- Non-goals (v0.2 design, familiarity, reading's probes, accepted records, value documents, code and evals, v1.0) | covered | brief preamble, sections 3 and 4.
- Task_1 | covered | brief section 2.
- Task_2 | covered | brief sections 3 and 5.
- Task_3 | extends | brief section 3.
- Decision Log: ADR-D-0005 ruling | extends | brief section 3.
- Decision Log: familiarity waits | covered | brief section 4.
- Decision Log: gist consolidation reading | extends | brief section 3 "move down to the phases that need them".
- Decision Log: time in experience as a v0.3 planning decision | covered | brief section 3.
- Decision Log: v0.2 cross-phase pointers follow moves | extends | brief section 3.
- Decision Log: identifiers stable, names cited | extends | brief section 2.
- Decision Log: "throughout" as the whole catalog | covered | brief section 2 means.
- Decision Log: order renderer, reading, re-baseline, paid runs | covered | brief sections 3 and 5.
- Decision Log: re-review fixes (gists kept, ADR-D-0040, familiarity open, assertions kept apart) | covered | brief sections 3 and 4; ADR-D-0027, ADR-D-0033, ADR-D-0035 unchanged.

- Verdict comparison for the catalog and roadmap plan (2026-10-16), as returned:
  - Reading compared: Goal agrees (reason rests on a wait the brief says is over); DoD catalog agrees; DoD mapping agrees; DoD roadmap with familiarity open diverges (Orchestrator covered on section 4's hold; auditor inferred, 4a ratified and section 4's wait closed, so the deferral is sequencing); DoD ADR-D-0005 successor agrees; DoD owner acceptance agrees; mapping table agrees; identifiers stable agrees; cites by brief's names diverges (covered; auditor inferred); near-neighbours agrees; Task_3 record agrees; stop and report diverges (covered on section 1; auditor inferred); Non-goals agree except editing accepted records (covered; inferred) and the familiarity clause; Task_1, Task_2, Task_3 agree; DL ADR-D-0005 ruling diverges (Orchestrator extends; auditor cited); DL familiarity waits diverges (covered; inferred); gist, time, pointers, identifiers, "throughout", order agree; re-review fixes agree except the conditional ADR-D-0040 citation (covered; inferred). Design, Integration, Context unread by the Orchestrator.
  - Findings compared: none found.
  - Correction inside the run: the familiarity items' reason reworded to name 4a and the unit that carries the move.

### v0-2-catalog-roadmap-plan, closeout

- Catalog revision (failures section, G1 to G13, purpose, variants, guarantees throughout, identifiers stable) | covered | brief section 2 and its means.
- Roadmap and drafts revision (v0.2 closeout and order; v0.3 measure, intent, six DoD items, attribution's core, time in experience; v0.4 reframed, objects leave; v0.5 long horizon, decay and reinforce leave) | covered | brief sections 3 and 5.
- v0.4 point-in-time recall line (a reading of validity intervals, also a planning question) | extends | brief section 3's v0.4 sentence.
- C3/C5 acceptance line moved to v0.3 with attribution's core | extends | brief section 3 "Attribution's core, witnessed or told, moves into v0.3".
- Volatility, review-after and the confidence view leave v0.4 with the machinery | covered | brief section 3 "with confidence and stance"; ADR-D-0018, ADR-D-0030.
- Proposed ADR-D-0040 (supersedes ADR-D-0005 whole, subtype decision carried forward) | extends | brief section 3 removal statement; the owner accepts the record.
- Decay wording in ADR-D-0013, ADR-D-0014 reported, not edited; ADR-D-0040 citations conditional, now resolved | covered | brief section 1 governance.
- Judgement calls (G8 wording; headings; file names kept) | covered | internal mechanics.
- Orchestrator rulings (identifiers stable; gists; familiarity to the 4a plan; "throughout"; ADR-D-0005 shape) | covered / extends | as the plan-draft reading.
- Human-only items pending: the owner accepts the catalog; the roadmap sections; the record | needs the owner | brief sections 2 and 3; a reserved act.

- Verdict comparison for the catalog and roadmap plan, closeout (2026-10-16), as returned:
  - Reading compared: the catalog line (covered) against the auditor's split: failures, twelve situations, purpose, variants, rewordings agree (cited); the nine departures diverge (covered inside the line; inferred, direction); G10's empty-week departure diverges (covered; ask-now on a conflict); identifiers and cross-references diverge (covered; inferred). The roadmap line (covered): closeout and order, v0.3 measure and gains, pointers, time in experience, v0.4, v0.5 agree (cited); the gist ruling diverges (covered; inferred); familiarity left to 4a unread at closeout. v0.4 point-in-time agrees; C3/C5 agrees; volatility and the confidence view diverge (covered on "with confidence and stance"; inferred on the correction statement and 8.3); ADR-D-0040 diverges in part (extends; cited for the decision, inferred for the full supersession); the handoff agrees; the citation diverges (covered; inferred); G8 diverges (internal; user-facing inferred); headings and file names agree; rulings agree except familiarity (covered; inferred); human-only items agree. Unread by the Orchestrator: the Counsel notes item; the common.md change; that no accepted record changed.
  - Findings compared: none found under Findings; both means built as written (agrees, cited). Not recorded and compared: G10's departure bears on the design and not recorded (ask-now); the nine-departures list bears on the design and not recorded (inferred, direction); G8's phase wording agrees (inferred).
  - Correction inside the run: the two unrecorded departures are now findings below; G10 escalated and held; the nine departures journaled as direction for the closeout.

## Findings

- 2026-10-16, catalog and roadmap plan: G10's first departure reads "empty only for a person" where the brief's clause reads "empty only for the AI"; the brief's reason clause and philosophy 8.3 support the catalog's reading. Reading: bears on the design. Escalated to Counsel as a value question on 2026-10-16; the G10 departure text is held as written until the answer.
- 2026-10-16, catalog and roadmap plan: the catalog's purpose lists nine departures from human memory where philosophy 2.4 lists five (the four added are grounded in 7.4, 8.3, 10 and the brief). Reading: bears on the design (which departures the product declares). Noted to Counsel on 2026-10-16 with the G10 question; journaled as an inferred call marked direction for the closeout.
  - 2026-10-16: Counsel corrected the brief's clause (its own error against philosophy 7.5); the catalog stands; the hold is released.
