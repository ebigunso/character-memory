# Plan: a character whose recall statistics once failed recovers without an operator

- status: done
- generated: 2026-10-15
- last_updated: 2026-10-16
- work_type: code

## Goal
- Close the statistics residual the v0.2 slices logged: a recall-statistics store marked unhealthy stays conservative forever today, so one transient failure makes every later recall of that character conservative across restarts. The other two residuals of this plan (the subject-state read's speed and the busy-occasion order) moved to the plan for the brief's design amendment (section 4a), which replaces the recall selection they rest on; their root causes travel with them.

## Definition of Done
- A recall-statistics store marked unhealthy becomes healthy again without operator action and without a new public call: the next time the store is opened, its statistics are rebuilt from the graph store and the marker clears. A rebuild that fails leaves the marker in place with its original cause. What retrieval selects with healthy statistics is unchanged.
- Measurement, per the checkpoint rule: the red/green recovery witness, before and after, and evals-worker's scene and keyless families as controls at the task's tip. Before and after numbers, a paragraph on what they mean for the character and a verdict go in this plan's Decision Log. The plan is not complete until they exist.

## Planner-added requirements
- The rebuild at open (DoD): the one place that already decides the store's health state, and needs no new call.
- A failed rebuild keeps the marker and its original cause: the lesson of 2026-07-22 (stats failures retain every observed cause) as far as the stored health shape allows; the limit is in the Design.
- No public call or setting: an application cannot know when to call one, and a character should not need an operator to recover.
- The witness must change selection at the base: a witness that selects the same either way proves nothing about recovery.

## Scope / Non-goals
- Scope: the statistics recovery, its tests and witness, its measurement, and this plan.
- Non-goals: the subject-state read's speed and the busy-occasion order (moved to the section 4a design plan); any change to what retrieval selects with healthy statistics; any new public API; any stored-shape change.

## Design
- The root cause is recorded in its report and the Decision Log. If the fix needs a contract change, a new public surface, a stored-shape change or a ruling that is not in this plan, the worker stops and reports instead of working around it (workaround tripwire).
- Chosen: the statistics are derived from the graph, so they can be recomputed from it. The store rebuilds at open when the stored marker is unhealthy, in the composition path that already chooses the in-memory unhealthy fallback (`src/composition.rs` near 309). No new public call, because no application can know when to call it and a character should not need an operator to recover. Known limit: a long-running process whose store fails mid-session stays conservative until it next opens the store. Retrying after the next successful projection was deferred: it adds a rebuild on a hot write path for a failure not seen outside tests; the measurement verdict judges whether that holds. Second known limit: the stored health keeps one cause (`RetrievalStatsHealth::last_error_cause`), so a rebuild that fails at open leaves the original cause and its own failure is not stored; storing both would change the persisted health shape, which this plan excludes, and the rebuild is retried at the next open. Instrument: the red/green witness; falsifier: the reopened store selects differently from a never-failed store. Lens: structure, the rebuild reuses the graph's own conversion with two filters removed; evolution, no new call or setting to retire; verification, the witness and a failed-rebuild test; operation, a rebuild cost only at an unhealthy open; human, an unhealthy store heals at open without anyone acting; safety, none.
- Alternative: retry the rebuild after the next successful projection, mid-session. Operation: a rebuild on the write path; verification: a mid-session failure test. Rejected for now, as above.
- Alternative: a public repair call. Evolution: a call to keep and document; human: someone must know to call it. Rejected.

## Integration
- Base: the renderer plan tip, merged into this branch, which carries the whole library stack through the prospective slice, the governing brief with its amendment and the ratified philosophy. The task branch `feature/2026-10-15/residual-stats-recovery` (WIP `d45bad71`, cut from `31dc6923`) joins by plain merge.
- The section 4a design plan replaces the recall selection on a sibling line; this change touches the statistics store, composition and the graph hydrator, not selection.

## Context
- Governing brief: `docs/coding-agent/briefs/active/v0-2-completion-brief.md`, with its amendment, section 4a. The brief's ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify docs/coding-agent/briefs/active/v0-2-completion-brief.md on those five framings.", admitted by the owner in this session on 2026-10-16: "Take the seat again, and check your inbox history. You should have a message from the Counsel, which carries my word. Take the quotes as mine." The amendment's ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify the design amendment, section 4a of docs/coding-agent/briefs/active/v0-2-completion-brief.md, on those seven framings.", admitted by the owner's statement in this session on 2026-10-16 admitting Counsel's relays standing (choosing "Admit relays, standing" when asked how the session should treat Counsel's relays). Product philosophy: `docs/project_philosophy.md`, ratification in `docs/project_philosophy-companion.md`. Readings file: `docs/coding-agent/plans/active/v0-2-completion-readings.md`. Run record: `docs/coding-agent/plans/active/v0-2-completion-run.md`.
- The amendment leaves standing "the storage and schema work"; statistics recovery is storage health under that.

## Tasks

### Task_1: A character whose recall statistics failed recovers on its own
- note: numbered Task_2 in the root-cause records, messages and earlier revisions of this plan.
- type: impl
- owns: src/composition.rs, src/adapters/stats/sqlite.rs, src/adapters/oxigraph/shared.rs (the two MemoryLink exclusions in the object hydrator only), src/adapters/oxigraph/tests.rs, and their tests
- depends_on: []
- acceptance:
  - A witness store where the unhealthy state changes what retrieval selects at the base (so before differs from healthy), marked unhealthy and reopened, is healthy with statistics equal to a fresh build from the same graph, and selects what a never-failed store selects.
  - A rebuild that fails leaves the marker and its original cause, and retrieval behaves as it does with an unhealthy marker today; a test covers a store that failed before and fails again at the rebuild.
  - No new public call or setting.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: no new public surface, the rebuild is from the graph, a failed rebuild keeps the marker, healthy behavior unchanged; the hydrator's callers surveyed for any visible change"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: the red/green witness above is the instrument (at the base the reopened store selects differently from a healthy one; at the tip identically); evals-worker runs the scene and keyless families as controls at the task's tip. Falsified if the reopened store selects differently from a never-failed store."

## Task Waves (explicit parallel dispatch sets)
- Wave 1: [Task_1]

## Research
- Root-cause report: `.agent-work/worker/residual-stats-recovery-rootcause.md`, folded into the Design and the Decision Log. The moved tasks' reports (`.worktrees/residual-state-read/.agent-work/worker/residual-state-read-rootcause.md`, `.agent-work/worker/residual-busy-occasion-rootcause.md`) go to the section 4a design plan.

## Progress Log (append-only)
- 2026-10-15: plan written; Wave 1 dispatched for the root-cause step before the plan review had run (a missed gate). Workers hold after the root cause until the plan review closes and their owns are narrowed.
- 2026-10-16: the v0.2 completion brief governs this plan from revision `785391cf` (the brief and the ratified philosophy, merged into this branch at `53a301ff`).
- 2026-10-16: plan-draft value audit dispatched at `4a1fe068` (plan at af3f048a), text verbatim: "You are the value auditor. Read references/value-audit-mandate.md of the orchestration-harness skill and follow it exactly. Position: plan draft. Plan: docs/coding-agent/plans/active/v0-2-residuals-plan.md. Governing brief: docs/coding-agent/briefs/active/v0-2-completion-brief.md. Changes since: none."
- 2026-10-16: verdict as returned (its two comparison lines are in the readings file):
  - Position: plan draft
  - Documents read: the brief, `docs/project_philosophy.md` (by the pointer line), its companion, `docs/coding-agent/rules/common.md` (no Standing Approvals section, so no standing approval in effect), the plan; code read only to understand Task_2's hydrator change (`shared.rs`, `embedded.rs`, `write_planning.rs`, `correct_forget.rs`, `stats_projection.rs`); the notes not opened; the readings file opened once after grading.
  - Product basis: the philosophy (companion, "Version ratified 2026-10-16 (second, current)") and the brief in the product owner's words (brief section 8).
  - Not audited: the engineering side has no document.
  - Missing inputs: none
  - Value documents changed in range: none (plan draft, no range)
  - DoD 1, subject-state read selects the same and is fast at 2,000 beliefs | maps to the brief | cited | brief section 4: "work that does not (the subject-state read's speed, statistics recovery, ...) proceeds" (tagged inferred, ratified with the brief on the framing "what waits on the design amendment and what proceeds"); "the bound stays in Rust" is internal mechanics | -
  - DoD 2, an unhealthy statistics store heals at the next open, with no new call; a failed rebuild keeps the marker | maps to the brief | cited | brief section 4: "statistics recovery ... proceeds" | -
  - DoD 3, busy-occasion selection keeps current, salient, then newest, the same in both identifier orders | maps to the brief | ask-now | brief section 4: "work that builds on the choices under review (... the busy-occasion ordering fix ...) waits"; brief preamble: "The v0.2 design itself ... is under re-examination ... and is not settled by this brief"; brief section 1: the logged decisions are "not grounds for anything until the re-examination says which choices stand" | Reasons: the brief withholds this decision for the amendment; the key's stated grounds (rulings 65 and 70) are logged decisions the brief says are not grounds; philosophy 8.5 and 7.3 lean toward the key but cannot settle what the brief reserves; the plan already holds this under section 4, so the grade adds no stop beyond the brief's own. Value question: when a character was at an occasion where it noticed more than it will have in mind at once, which of those details should come to mind (the most striking first, then the most recent), or should the re-examination change how much of a busy occasion comes to mind at all? | -
  - DoD 4, measurement at the Wave 1 tip, busy-occasion family on release | internal mechanics | not audited | - | -
  - Planner-added: Task_1's 1 s falsifier at 2,000 beliefs | maps to the brief | inferred | extends brief section 4, "the subject-state read's speed" (the line at which a turn no longer stalls is user-observable latency) | -
  - Planner-added: the bound stays in Rust | internal mechanics | not audited | - | -
  - Planner-added: batched membership read | internal mechanics | not audited | - | -
  - Planner-added: Task_2 rebuilds at open | maps to the brief | cited | brief section 4: "statistics recovery ... proceeds" | -
  - Planner-added: a failed rebuild keeps the marker and its original cause (with the Design's one-cause limit) | maps to the brief | inferred | extends philosophy 8.6, "Expose retrieval rationale", and section 10, "If a memory stops influencing behavior, someone decided that, and the decision is visible" | -
  - Planner-added: timing samples interleaved, run twice | internal mechanics | not audited | - | -
  - Planner-added: Task_3 uses one key for the pre-hydration path and the materialized plan | maps to the brief | ask-now | as DoD 3 | Same reasons and value question as DoD 3 | -
  - Planner-added: Task_1's timing witness stays outside the suite | internal mechanics | not audited | - | -
  - Planner-added: Task_2 adds no public call or setting | maps to the brief | inferred | extends brief section 4, "statistics recovery", and philosophy section 9, "The library should feel like a memory system, not a database client" | -
  - Planner-added: Task_2's witness must change selection at the base | internal mechanics | not audited | - | -
  - Planner-added: Task_3's tie key (current, salient, newest, id) | maps to the brief | ask-now | as DoD 3 | Same reasons and value question as DoD 3 | -
  - Assumption (Design, Task_2): a store that fails mid-session stays conservative until its next open; retry-after-projection is deferred | maps to the brief | inferred | extends brief section 4, "statistics recovery" (recovery at open is a partial reading of it; cheap to undo) | -
  - Scope and non-goals | maps to the brief | cited | brief preamble: "The v0.2 design itself (the routes, the floors, the fixed room ...) is under re-examination ... not settled by this brief"; section 4; the scope's busy-occasion part is graded under Task_3 | -
  - Integration: the evals re-baseline waits for the design amendment; task branches join the plan branch by plain merges | maps to the brief / internal mechanics | cited (re-baseline) / not audited (merges) | brief section 4: "the evals re-baseline ... waits"; the merges are into the run's own plan branch, not outward-facing | -
  - Task_1 | maps to the brief | cited | brief section 4, "the subject-state read's speed" | -
  - Task_2 | maps to the brief | cited | brief section 4, "statistics recovery" | -
  - Task_3 | maps to the brief | ask-now | as DoD 3 | Same reasons and value question as DoD 3 | -
  - Decision Log 2026-10-15, the decider chose the residuals; one plan | maps to the brief | cited | brief section 4 for Task_1 and Task_2 (ruling 81 is not support, per brief section 1); one plan rather than several is internal mechanics | -
  - Decision Log 2026-10-15, "plan approval is waived under the standing authorization" | maps to the brief | ask-now | none: common.md at HEAD has no Standing Approvals section, and a standing approval never discharges plan approval | Reason: it would loosen who decides (plan approval) with nothing in the documents covering it; this is the Orchestrator's own line and can be dropped or redone, for example by stating the plan's authority as the brief route only. Value question: now that the completion brief governs the rest of v0.2, does your earlier waiver of plan approval still let work on this plan start without your approval? | -
  - Decision Log 2026-10-15, Task_2 root cause; delete the hydrator's two link exclusions | internal mechanics | not audited | - | no observable change found; the full caller survey is assigned to the Reviewer | -
  - Decision Log 2026-10-15, Task_4 (now Task_3) root cause and key | maps to the brief | ask-now | as DoD 3 (the key part) | Same reasons and value question as DoD 3 | -
  - Decision Log 2026-10-15, drop the second-person residual | maps to the brief | cited | brief section 4: the residual is not among the work that "proceeds"; preamble: "the fixed room ... is under re-examination"; the entry's reading that the seat round "is correct" is not settled by any document | -
  - Decision Log 2026-10-15, the first plan review applied | maps to the brief / internal mechanics | ask-now (its Task_4 key part only); the rest not audited | as DoD 3 | Same reasons and value question as DoD 3 | -
  - Decision Log 2026-10-16, the plan runs under the brief; Task_1 and Task_2 proceed; busy-occasion held | maps to the brief | cited | brief section 4 | -
  - Decision Log 2026-10-16, Task_1 root cause and design | internal mechanics | not audited | - | -
  - Decision Log 2026-10-16, re-review of 7bf694ff applied | maps to the brief / internal mechanics | ask-now (its Task_3 design part); the rest not audited | as DoD 3 | Same reasons and value question as DoD 3 | -
  - Decision Log 2026-10-16, second re-review applied | internal mechanics | not audited | - | -
  - Human-only conditions pending: brief section 2, "Human-only: the owner accepts the revised catalog."; brief section 3, "Human-only: the owner accepts the revised roadmap sections." Neither is touched by this plan.
  - Scenarios: none
- 2026-10-16: inferred calls noted: Task_1's 1 s falsifier; the failed rebuild keeping marker and original cause; Task_2 adds no public call; Task_2's mid-session limit.
- 2026-10-16: the amendment landed (brief section 4a, committed at `a9e0daac`, merged here); Counsel's relays are admitted standing by the owner in this session. This plan is narrowed to the statistics recovery and is audited again as a revision.


- 2026-10-16: plan-draft value audit of the narrowed plan dispatched at `fb726717`, text verbatim: "You are the value auditor. Read references/value-audit-mandate.md of the orchestration-harness skill and follow it exactly. Position: plan draft. Plan: docs/coding-agent/plans/active/v0-2-residuals-plan.md. Governing brief: docs/coding-agent/briefs/active/v0-2-completion-brief.md. Changes since: none."
- 2026-10-16: verdict as returned (comparison lines in the readings file). Position: plan draft. Documents read: the brief (sections 1 to 8 with the 4a amendment and its ratification record), the philosophy by the pointer line, its companion, common.md (Standing Approvals: one entry, Counsel's relays, "Acceptance of this entry as written: pending", so none in effect; nothing in this plan relies on one), the plan whole as claims; code read only to judge whether the hydrator change is observable (shared.rs 138-173, embedded.rs callers, write_planning.rs, correct_forget.rs, lib.rs, composition.rs 302-316); notes never opened; readings opened once after grading. Product basis: the philosophy (companion, second version, current) and the brief in the product owner's words (section 8; 4a carries its own record). Not audited: the engineering side (no document). Missing inputs: none. Value documents changed in range: none.
  - DoD 1 (heals at next open, failed rebuild keeps original cause, healthy selection unchanged) | maps to the brief | cited | brief section 4 "statistics recovery ... proceeds"; 4a "What this leaves standing ... the storage and schema work"; 4a "What it replaces: the recall selection and room" (untouched here) | -
  - DoD 2 (measurement) | internal mechanics | not audited | - | the scene and keyless families are not the paid runs section 5 forbids; if a control run would spend, section 5 binds
  - Planner-added: rebuild at open, no new call | maps to the brief | cited | brief section 4 | -
  - Planner-added: failed rebuild keeps marker and original cause, one-cause limit | maps to the brief | inferred | extends philosophy 8.6 "Expose retrieval rationale", section 10 "Avoid silent forgetting ...", 8.5 "Forgetting is always someone's decision ..."; cheap to undo | -
  - Planner-added: no public call or setting | maps to the brief | inferred | extends philosophy section 9 "a memory system, not a database client" and 8.1; cheap to undo | -
  - Planner-added: witness changes selection at the base | internal mechanics | not audited | - | -
  - Scope | maps to the brief | cited | brief section 4; 4a storage work stands | -
  - Non-goal: busy-occasion order moves to the 4a plan | maps to the brief | cited | brief section 4 (waits); 4a replaces the selection | -
  - Non-goal: subject-state read's speed moves to the 4a plan | maps to the brief | inferred | extends 4a "What it replaces: the recall selection and room"; section 4 lists the read among work that proceeds and 4a's storage line does not name it; a deferral within the run, cheap to undo | -
  - Non-goals: no healthy-selection change, no public API, no stored-shape change | brief / internal | cited / cited / not audited | 4a; philosophy section 9 | -
  - Design assumption: mid-session failure stays conservative until next open | maps to the brief | inferred | extends brief section 4 "statistics recovery"; cheap to undo | -
  - Design: delete the hydrator's two MemoryLink exclusions | internal mechanics | not audited | - | auditor's own check found no observable change; the Reviewer's caller survey is the right check
  - Integration | internal mechanics | not audited | - | merges into the run's own branch, not outward-facing
  - Task_1 | maps to the brief | cited | brief section 4; 4a storage work stands | -
  - Decision Log 2026-10-15 (four residuals, one plan, waiver line) | maps to the brief | cited | section 4; the waiver line is withdrawn by the narrowing entry | -
  - Decision Log 2026-10-15 Task_2 root cause | internal mechanics | not audited | - | -
  - Decision Log 2026-10-15 Task_4 key | maps to the brief | cited | 4a replaces the selection; the key travels to the 4a plan | -
  - Decision Log 2026-10-15 drop the second-person residual | maps to the brief | cited | 4a "No room is reserved per kind of cue ..."; the entry's "is correct" reading is not relied on | -
  - Decision Log 2026-10-15 first review applied; 2026-10-16 old Task_1 root cause; re-review; second re-review | internal mechanics | not audited | - | -
  - Decision Log 2026-10-16 runs under the brief | maps to the brief | cited | section 4 | -
  - Decision Log 2026-10-16 narrowing, waiver corrected, authority line | maps to the brief | cited | section 4; 4a; section 1 Governance "The Orchestrator decides and logs everything below direction level"; the authority line restates the Plan Gate's second source and this verdict approves nothing by itself; that source also requires a plan review closed after the narrowing, which the log does not yet record | -
  - Human-only conditions pending: brief section 2 (catalog), section 3 (roadmap), 4a (the owner's blind reading at closeout). None touched by this plan.
  - Scenarios: none
  - No item ask-now or ungraded. Inferred calls journaled: failed-rebuild cause retention with its one-cause limit; no public call; the mid-session limit; the subject-state read's move to the 4a plan.
- 2026-10-16: the narrowed plan goes to a plan re-review (the Plan Gate's second source needs one closed after the revision); go for Task_1 follows it.

- 2026-10-16: Task_1 built (38b7e702; docs follow-up afb97202 after the reviewer's R1 P2 on README and the database design documents, two consistency touches outside owns accepted). Worker gates pass (532 tests); witness base healthy 4 / degraded 1 / reopened 1 unhealthy, tip 4 / 1 / 4 healthy, same ids and order, replayed twice. Tier D APPROVED through afb97202 (`.agent-work/reviewer/residual-stats-recovery-review.md`). Judgement calls: the expectation change in `oxigraph_queries_links_separately_from_memory_objects` (a query asking for a link now gets it). Pending: evals-worker's scene and keyless controls at the tip, then the measurement verdict, then the closeout audit. Pull requests opened: the plan on #144's branch, the task on the plan branch.

- 2026-10-16 (record correction, appended): the plan re-review of the narrowed revision returned APPROVED at a06c4a11 (two nits, applied) before the go for Task_1 was sent; the entry above that said the re-review was pending was written before the verdict arrived.
- 2026-10-16: closeout value audit dispatched at `763df84b`, text verbatim (paths repository-relative for the committed record): "You are the value auditor. Read references/value-audit-mandate.md of the orchestration-harness skill and follow it exactly. Position: closeout. Plan: docs/coding-agent/plans/active/v0-2-residuals-plan.md. Governing brief: docs/coding-agent/briefs/active/v0-2-completion-brief.md. Changes since: 785391cf."
- 2026-10-16: verdict as returned (comparison lines in the readings file). Position: closeout. Documents read: the plan whole as claims; the brief whole and its diff in range; common.md at HEAD (pointer line; Standing Approvals entry carrying the owner's acceptance); the philosophy; its companion; the run record's Scenarios; the range diffs of README, lessons, the renderer plan, the three database design documents, composition.rs, adapters/stats/sqlite.rs, adapters/oxigraph/shared.rs and tests.rs; code read only to judge the hydrator change; notes never opened; readings after grading. Product basis: the philosophy and the brief in the product owner's words. Not audited: the engineering side. Missing inputs: none. Value documents changed in range: the brief (preamble; section 2's empty-week clause corrected by Counsel, no record of its own, not relied on; 4a added with its own record, support; two Counsel clarifications in 4a tagged inferred, not relied on); common.md (Counsel line; Standing Approvals entry in effect on its record; nothing relies on it); philosophy and companion unchanged; no pointer line removed or changed.
  - Goal | brief | cited | section 4; 4a storage work stands | -
  - DoD 1 | brief | cited | section 4; 4a "What it replaces" untouched | -
  - DoD 2 | internal mechanics | not audited | the controls are deterministic families, not the paid runs; no spend in range | -
  - Planner-added: rebuild at open | brief | cited | section 4 | -
  - Planner-added: failed rebuild keeps marker and original cause (one-cause limit) | brief | inferred | extends philosophy 8.6, section 10; cheap to undo | -
  - Planner-added: no public call or setting | brief | inferred | extends philosophy section 9 and 8.1; cheap to undo | -
  - Planner-added: witness changes selection at the base | internal mechanics | not audited | - | -
  - Scope | brief | cited | section 4; 4a | -
  - Non-goal: busy-occasion order to the 4a plan | brief | cited | section 4; 4a | -
  - Non-goal: subject-state read to the 4a plan | brief | inferred | extends 4a; a deferral within the run, cheap to undo | -
  - Non-goals: healthy selection, public API, stored shape | brief / brief / internal | cited / cited / not audited | section 4; philosophy section 9 | -
  - Design: rebuild from the graph at open | brief | cited | section 4 | -
  - Design assumption: mid-session limit | brief | inferred | extends section 4; cheap to undo | -
  - Design: the hydrator's two exclusions | internal mechanics | not audited | ports are pub(crate); no production caller builds a link ref; the renamed test asserts expansion carries no link | -
  - Integration | internal mechanics | not audited | merges into the run's own branch | -
  - Context | brief | cited | the brief's own records | -
  - Task_1 | brief | cited | section 4; 4a | -
  - Decision Log 2026-10-15 entries | brief / internal | cited / not audited | section 4; 4a; the waiver line withdrawn by the narrowing entry | -
  - Decision Log 2026-10-16: runs under the brief; narrowing (busy-occasion cited, subject-state inferred); measurement verdict | brief | cited / inferred | sections 1, 4, 4a; the witness reproducible in range; the re-review's closing was not recorded before the go (a gate-record gap, corrected above) | -
  - Changes: rebuild at open and cleared marker | brief | cited | section 4; 4a | -
  - Changes: failed rebuild keeps the marker and cause, one transaction rolled back on failure | brief | inferred | philosophy 8.6, section 10 | -
  - Changes: healthy store reopens without a graph read; selection unchanged | brief | cited | section 4 | -
  - Changes: sqlite rebuild (pub(crate)) deletes and rewrites derived index rows | internal mechanics | not audited | derived index recomputed from the graph of record; not a memory operation; not outward-facing | -
  - Changes: query_objects hydrates links when asked | internal mechanics | not audited | - | -
  - Changes: README and three design documents state the rebuild and its limits | brief | cited | section 4 | -
  - Changes: lessons (three entries); run record created; renderer plan superseded; common.md Counsel line and Standing Approvals | internal mechanics / brief | not audited / cited | 4a "the renderer as specified"; section 3 means not departed from; the Standing Approvals entry the owner's own decision on its record | -
  - Judgement calls and rulings (test expectation; two doc touches; PRs opened; the verdict; the narrowing; the one-cause limit; the waiver correction; go after the re-review; the renderer superseded; the Standing Approvals entry) | internal / brief | not audited / cited / inferred | as above; a PR opened for review is not a merge | -
  - Decision records whose wording changed after acceptance: none.
  - Human-only conditions pending: the owner accepts the revised catalog (section 2); the owner accepts the revised roadmap sections (section 3); the owner's blind reading at closeout (4a). None touched by this unit.
  - Scenarios: (1) a developer opens the revised catalog and finds what is in mind and how it is held, never a rule about what the character must say: prior not yet, now not yet; (2) the owner sits down at v0.2's closeout with blind transcripts and a control arm: prior not yet, now not yet; (3) a direction-level question reaches Counsel as a character-behaviour question: prior not yet, now not yet. Getting closer: no. No scenario moved and the unit by design changes nothing a turn receives; it is authorized ground-clearing under section 4 and 4a's storage work, measured and done, and does not set the run back, but it is no evidence toward the design; the next closer step is the 4a design plan, not more on this line.
- 2026-10-16: acting on the verdict. No item held; inferred calls journaled (the one-cause limit; no public call; the mid-session limit; the subject-state read's move). The unit closes: status done, the plan moves to completed, the Scenarios line goes to the run record. The verdict's "getting closer: no" is carried to the owner through Counsel with each scenario's state and the audit's reason, as the closeout rules state; no further unit starts on that word.

## Decision Log (append-only)
- 2026-10-15 Decision: the decider chose these four residuals for v0.2 (ruling 81) and asked for them to run while Codex capacity is available. One plan with four tasks, not four plans: each task is small, and they share one base and one measurement pattern.
  - User approval: the decider chose the scope; plan approval is waived under the standing authorization.
  - Record proposed: none.
- 2026-10-15 Decision: Task_2's root cause and graph read.
  - Root cause: an unhealthy marker is persisted by `mark_unhealthy` and nothing ever clears it; composition opens the store unchanged, and selectivity skips counters while it is unhealthy, so one transient failure makes every later recall conservative, across reopens.
  - Found while drafting: the graph object query selects links, but the shared object hydrator drops them (`src/adapters/oxigraph/shared.rs`, two filters), although the adapter already converts a link. A rebuild through it would lose explicit links (an entity's Involves link), so rebuilt statistics would not equal a fresh projection.
  - Chosen: delete the two exclusions so a query that selects links hydrates them through the existing conversion. No port, public surface or stored shape changes, and a caller that does not ask for links gets what it got before. Rejected: a private all-links port method (a new contract for data the query already expresses); appending links only in `query_objects` (a second dispatch path, leaving the hydrator inconsistent with its own conversion); reading raw RDF from composition (bypasses the port).
  - Reviewer checks the hydrator's callers: the derived-memory selectors pass only derived refs, and graph expansion rejects a link root and hydrates links on its own path.
- 2026-10-15 Decision: Task_4's root cause and key, as the Design states. Captures at 31dc6923 ran twice, byte-identical: the reminder keeps 16 of 25 observations of the busy occasion, and in both identifier orders the kept set is exactly the 16 lowest identifiers, so the original order loses the newest detail and keeps the five oldest. Tasks 1, 2 and 4 touch different functions of `sparql_selectors.rs` and `shared.rs`; their branches are joined by plain merges.
- 2026-10-15 Decision (proposed to the decider): drop Task_3.
  - Root cause: Participant and Activity scopes are already ordered apart (`src/usecases/retrieve/state.rs`), and principle 4's round is correct. In the activity-pressure story the 12-seat derived section holds 4 activity and 5 topic beliefs (5 and 4 at floor 2), which leaves 3 seats for the people present; a round over 3 seats gives the first person 2 and the second 1. Without the activity, 4 seats give 2 each.
  - Current numbers (Task_4's capture at 31dc6923, measurements 4 and 5, both identifier orders): 2 and 2 without the activity; with it, 2 and 1 at floors 0, 1 and 2, with 5, 5 and 4 on-topic and 4, 4 and 5 activity-thread beliefs, matching the older reading at fa90d57.
  - Reading: what fills the seats is the section budget, and scene order breaks the odd seat. Two each would need a thirteenth seat or would evict an activity or topic belief that outranks the participant beliefs. Neither is a fix of a defect. The residual is withdrawn, and the consolidation's note is corrected by this entry.
  - User approval: the decider, 2026-10-15, in this session: "Drop it" (answering whether to drop residual 3). Task_3 is dropped; Wave 1 is [Task_1, Task_2, Task_4].
- 2026-10-15 Decision: the plan review (Tier D and Tier A) returned changes, applied: Task_1's design keeps the bound in Rust and fixes the scope (the earlier text pointed at the shape ruling 67 rejected); Task_4's key puts current first and reuses `select_state`'s order, so Task_4 follows Task_1, and its measurement tests the newest-16 rule with a control set; Task_2's witness must change selection at the base, and its mid-session limit is recorded; measurement follows the checkpoint rule; Task_3 is removed. Lesson candidate: when a Design says to do it like a sibling, quote the sibling's code location and check it against the rulings it touches.
  - Next: a re-review of this revision, then the plan-draft value audit against the product philosophy, then go for Wave 1.

- 2026-10-16 Decision: the plan runs under the v0.2 completion brief (Context). Task_1 and Task_2 proceed under its section 4; the busy-occasion task (Task_3) is held there, its root cause and design kept as written for the amendment to confirm or replace. The slice-end run covers Task_1 and Task_2's controls when Wave 1 closes, and the busy-occasion family when Task_3 is released.
  - User approval: the brief's section 4, the owner's ratified words; the hold is the brief's, not an Orchestrator ruling.
  - Record proposed: none.
- 2026-10-16 Decision: Task_1's root cause and design, from its report. One VALUES table of every member makes the metadata query slow (8.45 s at 2,000); 32 tables of at most 64 rows return identical rows in 0.18 s; the joinable scope variants were measured no faster (9.7 s to 32 s). Chosen: batched membership, one global lifecycle, order and cap pass, the bound in Rust; `owns` narrowed to the two selectors.
- 2026-10-16 Decision: the re-review of 7bf694ff (CHANGES REQUESTED) applied: Task_1's design above; the planner-added list completed and the falsifier reason put in the decider's words; per-task lens lines; Task_3 takes the smaller design (a salience column on the occasion metadata read, one comparator), drops its dependency on Task_1 and narrows its `owns`; Task_2 records the one-cause limit and tests a repeated failure; measurement runs at the Wave 1 tip for Tasks 1 and 2 and names evals-worker; counts say three.
- 2026-10-16 Decision: the second re-review (24d13fb7) APPROVED with two minor findings, applied: Task_3 writes its own comparator (the shared one sits in Task_1's function), and every task states all six lens groups.
- 2026-10-16 Decision: the plan narrows to the statistics recovery under the brief's amendment (section 4a).
  - The amendment replaces "the recall selection and room"; the subject-state read (Task_1) rests on `select_state`'s selection and the busy-occasion order (Task_3) on graph expansion's selection, so both move to the section 4a design plan with their root causes, which decides what of them survives. The verdict's ask-now items on the busy-occasion key leave this plan with that task.
  - Statistics recovery is storage health, which the amendment leaves standing; it is renumbered Task_1.
  - Correction to the 2026-10-15 entry "plan approval is waived under the standing authorization": that waiver does not authorize this plan. Its authority is the governing brief: a plan-draft value audit after the plan review that holds nothing, under the owner's ratification of the brief.
  - User approval: the brief and its amendment, the owner's ratified words.
  - Record proposed: none.
- 2026-10-16 Decision: measurement verdict for Task_1 (statistics recovery).
  - Instrument, the red/green witness on one store (4 healthy beliefs; one failure marks the statistics unhealthy; reopen): base 31dc6923 healthy 4 selected, degraded 1, reopened 1 and still unhealthy; tip 38b7e702 healthy 4, degraded 1, reopened 4 and healthy, the same ids in the same order as a never-failed store; replayed twice (`.worktrees/residual-stats-recovery/.agent-work/worker/recovery-witness-results.json`).
  - Controls, run by evals-worker at harness 3a712252: the scene and keyless families, six sections, both identifier orders, 410 rows per capture, at 38b7e702 against the code-equivalent base 31dc6923, each pin twice: exactly equal (`evals:.agent-work/worker/residual-stats-controls.md`, SHA256 febfb412...).
  - What it means for the character: a character whose recall statistics once failed is conservative for the rest of that session and whole again the next time its memory opens, with nobody acting; a character whose statistics never failed recalls exactly as before.
  - Verdict: the residual is closed. Known limits stand as the Design states: a mid-session failure stays conservative until the next open; a rebuild that fails keeps the original cause only.
  - Record proposed: none.
