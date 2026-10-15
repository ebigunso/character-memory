# Plan: four logged residuals of the v0.2 recall slices are fixed, each with its measurement

- status: in-progress
- generated: 2026-10-15
- last_updated: 2026-10-16
- work_type: code

## Goal
- Close three residuals that the v0.2 slices logged and did not fix, chosen by the decider on 2026-10-15 (ruling 81; a fourth, the second person under activity pressure, was dropped on its root cause): a character with a long history of beliefs about one person recalls them as fast as it recalls its promises; a character whose recall statistics once failed recovers without an operator; and what a busy occasion brings does not depend on how its identifiers happen to sort.

## Definition of Done
- The subject-state read selects exactly what it selects at the base, and at 2,000 beliefs about one person it takes no longer than the same order of time as the trigger and due reads at that size (about 0.15 s on this machine, against about 9 s at the base). The bound is ruling 67's.
- A recall-statistics store marked unhealthy becomes healthy again without operator action and without a new public call: the next time the store is opened, its statistics are rebuilt from the graph store and the marker clears. A rebuild that fails leaves the marker in place with its cause. What retrieval selects with healthy statistics is unchanged.
- Busy-occasion selection keeps the current, salient, then newest observations of an occasion, and gives the same result in both identifier orders.
- Measurement, per the checkpoint rule: Task_1 carries its timing witness at scale (ruling 67's check) and Task_2 its red/green recovery witness; one slice-end run at the final tip covers Task_4's family and the controls. Before and after numbers, a paragraph on what they mean for the character and a verdict go in this plan's Decision Log. The plan is not complete until they exist.

## Scope / Non-goals
- Scope: the four fixes, their tests and witnesses, their measurements, and this plan.
- Non-goals: the unlived-topic relevance floor (needs real embeddings; ADR-I-0036); the reserved root seat's subtree cost; the LoCoMo `question_id`; the ADR-I-0007 constants; any new public API; any change to what retrieval selects beyond the residual each task names.

## Design
- Each task first finds the root cause and states it in its report. If the fix needs a contract change, a new public surface, a stored-shape change or a ruling that is not in this plan, the worker stops and reports instead of working around it (workaround tripwire).
- Task_1 (subject-state read): all four state reads go through `select_state` (`src/adapters/oxigraph/sparql_selectors.rs:433`), which reads only rank columns and applies the bound in Rust (575-602); ruling 67 rejected pushing the bound into the query (about 220 times slower). What differs is the scope: subject state injects a VALUES list of every member with a FILTER EXISTS per row (294-307), while trigger and due use a graph pattern the engine joins (344-399). The fix makes the subject scope a joinable pattern, or resolves membership in Rust; the bound stays in Rust. The result, including the filtered and trace list the selector returns, must be identical.
- Task_2 (statistics recovery): the statistics are derived from the graph, so they can be recomputed from it. The chosen recovery is a rebuild at open when the stored marker is unhealthy, in the composition path that already chooses the in-memory unhealthy fallback (`src/composition.rs` near 309). No new public call, because no application can know when to call it and a character should not need an operator to recover. Known limit: a long-running process whose store fails mid-session stays conservative until it next opens the store. Retrying after the next successful projection was deferred: it adds a rebuild on a hot write path for a failure not seen outside tests; the slice-end verdict judges whether that holds.
- Task_4 (busy-occasion order): which observations of one occasion survive the fanout bound is decided by the link identifier order today (`select_links_touching` sorts refs by id, and `apply_fanout_limits_by_pair` keeps the first 16). The chosen key orders an occasion's eligible observation neighbours before the bound: current first under the lifecycle policy (a suppressed or superseded observation takes no seat), then salient, then newest by memory time (the observation's own time, else its occasion's scene time, as `ParticipantOccasion::memory_time` defines), then identifier as the last resort. It is ruling 70's order (salient current first, then newest, then id) for what is held about a present person, applied to what was observed at an occasion, and ruling 65's rule of ordering before a bound the way the section will judge (observations of one occasion tie on graph and cue score), so a character recalls the striking details of a busy occasion and, among equals, the latest. The same key orders both the pre-hydration path and the materialized plan. The order comes from reusing `select_state`'s salience order scoped to the occasion's observations, which already puts current first and falls back from the observation's time to its occasion's scene time, as `order_current_subject_links` (674) does for subject state; never from hydrating every candidate (ruling 67). Because that reuses Task_1's selector, Task_4 follows Task_1. Rejected: a general key for every relation (changes recall far beyond this route); sorting after hydration or at the pack (the discarded observations were never read); raising the bound (moves the cliff); time before salience (minutes within one occasion say less than salience about what a person keeps).

## Integration
- Base: the renderer plan tip `969faad7` (merged in at `53a301ff`), which carries the whole library stack through the prospective slice, the governing brief and the ratified philosophy. The task branches cut from `31dc6923` are joined by plain merges. The renderer's tasks touch no retrieval code, so the two lines run in parallel and are joined by plain merges before the closeout re-baseline.
- Tasks 1 and 2 run in parallel; Task_4 follows Task_1 because it reuses the state selector Task_1 changes.
- After this plan, the evals Task_6 re-baseline runs once on the finished library; under the brief's section 4 the re-baseline waits for the design amendment.

## Context
- Governing brief: `docs/coding-agent/briefs/active/v0-2-completion-brief.md`. Its ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify docs/coding-agent/briefs/active/v0-2-completion-brief.md on those five framings.", admitted by the owner in this session on 2026-10-16: "Take the seat again, and check your inbox history. You should have a message from the Counsel, which carries my word. Take the quotes as mine." Product philosophy: `docs/project_philosophy.md`, ratification in `docs/project_philosophy-companion.md`. Readings file: `docs/coding-agent/plans/active/v0-2-completion-readings.md`.
- The brief's section 4: "the subject-state read's speed, statistics recovery" proceed; "the busy-occasion ordering fix" waits for the amendment carrying the v0.2 design re-examination.

## Tasks

### Task_1: A long history of beliefs about one person is read as fast as promises are
- type: impl
- owns: src/adapters/oxigraph/sparql_selectors.rs and its tests; any graph port file the selector's signature needs (report it)
- depends_on: []
- acceptance:
  - The root cause, stated with the query before and after.
  - At the base and the tip, the same store gives the same selected beliefs, in the same order, and the same filtered and trace list, in both identifier orders.
  - A timing witness outside the permanent suite at 2,000 beliefs about one person, before and after, interleaved, three warm samples each.
  - Existing tests pass unchanged.
- validation:
  - kind: command; owner: worker; "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; the equality and timing witnesses"
  - kind: review; owner: reviewer; "Tier D: the result is identical, the bound stays in Rust, checked at scale against the base form; the witness is not in the suite"
  - kind: command; owner: orchestrator; "Measurement: the timing witness before and after at 2,000 beliefs (instrument); the familiar-person and state families are controls in the slice-end run. Falsified if any selected belief differs or the after time at 2,000 beliefs is above 1 s."

### Task_2: A character whose recall statistics failed recovers on its own
- type: impl
- owns: src/composition.rs, src/adapters/stats/sqlite.rs, src/adapters/oxigraph/shared.rs (the two MemoryLink exclusions in the object hydrator only), src/adapters/oxigraph/tests.rs, and their tests
- depends_on: []
- acceptance:
  - A witness store where the unhealthy state changes what retrieval selects at the base (so before differs from healthy), marked unhealthy and reopened, is healthy with statistics equal to a fresh build from the same graph, and selects what a never-failed store selects.
  - A rebuild that fails leaves the marker and its cause, and retrieval behaves as it does with an unhealthy marker today.
  - No new public call or setting.
- validation:
  - kind: command; owner: worker; "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review; owner: reviewer; "Tier D: no new public surface, the rebuild is from the graph, a failed rebuild keeps the marker, healthy behavior unchanged"
  - kind: command; owner: orchestrator; "Measurement: the red/green witness above is the instrument (at the base the reopened store selects differently from a healthy one; at the tip identically); the scene and keyless families are controls in the slice-end run. Falsified if the reopened store selects differently from a never-failed store."

### Task_4: What a busy occasion brings does not depend on identifier order
- type: impl
- owns: src/policy/graph_expansion.rs (incident-neighbour ordering before the fanout limits, both paths); src/adapters/oxigraph/shared.rs (the occasion metadata handed to that ordering; not Task_2's hydrator lines); src/adapters/oxigraph/sparql_selectors.rs (the compact salience projection only; not Task_1's subject-state selector); their tests
- depends_on: [Task_1]
- acceptance:
  - The family that showed the dependence (the companion's busy-occasion residual, `calibrate_cue_floors --residuals-only`) selects the same observations in both identifier orders, and with equal salience they are the 16 newest.
  - A test at the policy or adapter boundary shows the selected observation times in both identifier orders with the caps unchanged, plus a salient older observation that survives over newer equals and a suppressed one that takes no seat.
- validation:
  - kind: command; owner: worker; "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review; owner: reviewer; "Tier D: the tie key carries meaning and identifier order is the last resort"
  - kind: command; owner: orchestrator; "Slice-end measurement at the final tip, run by evals-worker: the busy-occasion residual family before and after, both id orders, twice; controls: the other three residual families and the scene and familiar-person families match the base, or each difference is listed as intended (this order also decides admission under the node cap). Falsified if the two orders still differ, or if at equal salience the kept set is not the 16 newest."

## Task Waves
- Wave 1: [Task_1, Task_2]
- Wave 2: [Task_4], held under the brief's section 4 until the design amendment says whether its key stands.
- Task_3 dropped.

## Research
- Research waived: each task's first step finds the root cause and reports it, with the files a fix touches, before any fix is written. The residuals were located by the slices that logged them (rulings 77 and 78; the consolidation slice-end entry). Each task's `owns` is narrowed from its root-cause report before its fix is authorized.

## Progress Log (append-only)
- 2026-10-15: plan written; Wave 1 dispatched for the root-cause step before the plan review had run (a missed gate). Workers hold after the root cause until the plan review closes and their owns are narrowed.
- 2026-10-16: the v0.2 completion brief governs this plan from revision `785391cf` (the brief and the ratified philosophy, merged into this branch at `53a301ff`).

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

- 2026-10-16 Decision: the plan runs under the v0.2 completion brief (Context). Task_1 and Task_2 proceed under its section 4; Task_4 is held there, its root cause and design kept as written for the amendment to confirm or replace. The slice-end run covers Task_1 and Task_2's controls when Wave 1 closes, and Task_4's family when it is released.
  - User approval: the brief's section 4, the owner's ratified words; the hold is the brief's, not an Orchestrator ruling.
  - Record proposed: none.
