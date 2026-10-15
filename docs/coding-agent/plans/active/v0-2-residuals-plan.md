# Plan: three logged residuals of the v0.2 recall slices are fixed, each with its measurement

- status: in-progress
- generated: 2026-10-15
- last_updated: 2026-10-16
- work_type: code

## Goal
- Close three residuals that the v0.2 slices logged and did not fix, chosen by the decider on 2026-10-15 (ruling 81; a fourth, the second person under activity pressure, was dropped on its root cause): a character with a long history of beliefs about one person recalls them as fast as it recalls its promises; a character whose recall statistics once failed recovers without an operator; and what a busy occasion brings does not depend on how its identifiers happen to sort.

## Definition of Done
- The subject-state read selects exactly what it selects at the base, and at 2,000 beliefs about one person it takes no longer than the same order of time as the trigger and due reads at that size (about 0.15 s on this machine, against about 9 s at the base). The bound stays in Rust.
- A recall-statistics store marked unhealthy becomes healthy again without operator action and without a new public call: the next time the store is opened, its statistics are rebuilt from the graph store and the marker clears. A rebuild that fails leaves the marker in place with its original cause. What retrieval selects with healthy statistics is unchanged.
- Busy-occasion selection keeps the current, salient, then newest observations of an occasion, and gives the same result in both identifier orders.
- Measurement, per the checkpoint rule: Task_1 carries its timing witness at scale and Task_2 its red/green recovery witness; when Wave 1 closes, evals-worker runs Task_1's and Task_2's control families at its tip, and the busy-occasion family runs when Task_3 is released. Before and after numbers, a paragraph on what they mean for the character and a verdict go in this plan's Decision Log. The plan is not complete until they exist.

## Planner-added requirements
- Task_1's falsifier of 1 s at 2,000 beliefs: the decider named the residual "Slow subject-state read"; the plan reads slow against the trigger and due reads at the same size (about 0.15 s), and 1 s is the line below which a person-heavy store no longer stalls a turn, with room for machine noise.
- The bound stays in Rust (Task_1, DoD): pushing the bound into the query was measured about 220 times slower when the state reads were built; that measurement, not the ruling that recorded it, is the reason.
- Task_1's batched membership read (Design): the only form measured fast with equal rows; caps run once over all batches because per-batch caps would select differently.
- Task_2's rebuild at open (DoD): the one place that already decides the store's health state, and needs no new call.
- Task_2's failed rebuild keeps the marker and its original cause: the lesson of 2026-07-22 (stats failures retain every observed cause) as far as the stored health shape allows; the limit is in the Design.
- Task_1's timing samples, before and after, interleaved, run twice (`orchestrator.md` measurement rule).
- Task_3's one key for the pre-hydration path and the materialized plan: two orders would select different observations for the same store.
- Task_1's timing witness stays outside the permanent suite: a timing assertion in CI is flaky across machines; the equality witness is what the suite keeps.
- Task_2 adds no public call or setting: an application cannot know when to call one, and a character should not need an operator to recover.
- Task_2's witness must change selection at the base: a witness that selects the same either way proves nothing about recovery.
- Task_3's tie key (current, salient, newest, id): the residual names only identifier dependence; the key is the planner's because removing the dependence needs some meaningful order, and rulings 65 and 70 give it.

## Scope / Non-goals
- Scope: the three fixes, their tests and witnesses, their measurements, and this plan.
- Non-goals: the unlived-topic relevance floor (needs real embeddings; ADR-I-0036); the reserved root seat's subtree cost; the LoCoMo `question_id`; the ADR-I-0007 constants; any new public API; any change to what retrieval selects beyond the residual each task names.

## Design
- Each task's root cause is recorded in its report and the Decision Log. If the fix needs a contract change, a new public surface, a stored-shape change or a ruling that is not in this plan, the worker stops and reports instead of working around it (workaround tripwire).
- Task_1 (subject-state read): all four state reads go through `select_state` (`src/adapters/oxigraph/sparql_selectors.rs:433`), which reads compact rank rows and applies lifecycle policy, order and caps in Rust (538-602). Subject state hands it one VALUES table of every member (294-310). Root cause, measured: the large membership table in that metadata query is the expensive boundary (one 2,000-row table 8.45 s; the same members in 32 tables of at most 64 rows, 0.18 s, identical raw rows).
  - Chosen: read the compact rows in member batches of at most 64, collect every batch's rows, then apply lifecycle aggregation, order and caps once over the whole set; the bound stays in Rust. Queries grow as members/64 (about 313 at 20,000 members), each small. Instrument: the timing witness at 2,000 beliefs; falsifier: any selected belief, order or trace entry differs, or the after time is above 1 s. Structure: one selector changes, no port, shape or dependency change. Evolution: a batch constant, removable if the engine stops being size-sensitive. Verification: the equality witness in both identifier orders pins the contract. Operation: linear in members, no full-store scan. Human: the batch size is named with the measurement that set it. Safety: none.
  - Alternative: a joinable scope pattern (direct relation predicate, reified membership subquery, flat reified membership), measured at 9.7 s, 23 s and 32 s with identical rows. Same structure and verification; operation no better or worse. Rejected on the measurement.
  - Alternative: drop the scope from the query and filter members in Rust. Operation: cost grows with every state memory in the store, not the subject's, and it is unmeasured. Rejected.
  - Alternative: `select_state` called once per batch. Per-batch caps would select differently. Rejected.
- Task_2 (statistics recovery): the statistics are derived from the graph, so they can be recomputed from it. The chosen recovery is a rebuild at open when the stored marker is unhealthy, in the composition path that already chooses the in-memory unhealthy fallback (`src/composition.rs` near 309). No new public call, because no application can know when to call it and a character should not need an operator to recover. Known limit: a long-running process whose store fails mid-session stays conservative until it next opens the store. Retrying after the next successful projection was deferred: it adds a rebuild on a hot write path for a failure not seen outside tests; the slice-end verdict judges whether that holds. Second known limit: the stored health keeps one cause (`RetrievalStatsHealth::last_error_cause`), so a rebuild that fails at open leaves the original cause and its own failure is not stored; storing both would change the persisted health shape, which this plan excludes, and the rebuild is retried at the next open. Instrument: the red/green witness; falsifier: the reopened store selects differently from a never-failed store. Lens: structure, the rebuild reuses the graph's own conversion with two filters removed; evolution, no new call or setting to retire; verification, the witness and a failed-rebuild test; operation, a rebuild cost only at an unhealthy open; human, an unhealthy store heals at open without anyone acting; safety, none.
- Task_3 (busy-occasion order): which observations of one occasion survive the fanout bound is decided by the link identifier order today (`select_links_touching` sorts refs by id, and `apply_fanout_limits_by_pair` keeps the first 16). The chosen key orders an occasion's eligible observation neighbours before the bound: current first under the lifecycle policy (a suppressed or superseded observation takes no seat), then salient, then newest by memory time (the observation's own time, else its occasion's scene time, as `ParticipantOccasion::memory_time` defines), then identifier as the last resort. It is ruling 70's order (salient current first, then newest, then id) for what is held about a present person, applied to what was observed at an occasion, and ruling 65's rule of ordering before a bound the way the section will judge (observations of one occasion tie on graph and cue score), so a character recalls the striking details of a busy occasion and, among equals, the latest. The same key orders both the pre-hydration path and the materialized plan. The order comes from the occasion metadata the adapter already reads for every episode and observation neighbour in one batched query per hop (`select_participant_occasions`, `sparql_selectors.rs:792`, called from `shared.rs:676-689`): it gains a salience column, `ParticipantOccasion` (`src/policy/graph_expansion.rs:71`) gains the field beside its retention and `memory_time`, and the policy orders by its own short comparator (`select_state`'s comparator sits in Task_1's function, so the key exists twice, about six lines each, until a later change merges them); never from hydrating every candidate. Instrument: the busy-occasion residual family, both identifier orders; falsifier: the orders still differ, or at equal salience the kept set is not the 16 newest. Lens: structure, one existing read gains a column and no new query; evolution, a second short ordering rule beside `select_state`'s; verification, a policy-boundary test in both identifier orders; operation, no added query on the expansion path; human, the key reads as the rule it states; safety, none. Alternative: reuse `select_state` scoped to the occasion's observations, a second read per occasion on every expansion and a dependency on Task_1; rejected for that cost. Rejected: a general key for every relation (changes recall far beyond this route); sorting after hydration or at the pack (the discarded observations were never read); raising the bound (moves the cliff); time before salience (minutes within one occasion say less than salience about what a person keeps).

## Integration
- Base: the renderer plan tip `969faad7` (merged in at `53a301ff`), which carries the whole library stack through the prospective slice, the governing brief and the ratified philosophy. The task branches cut from `31dc6923` are joined by plain merges. The renderer's tasks touch no retrieval code, so the two lines run in parallel and are joined by plain merges before the closeout re-baseline.
- Tasks 1 and 2 run in parallel; Task_3 touches different functions of the same files and joins by plain merge when released.
- After this plan, the evals Task_6 re-baseline runs once on the finished library; under the brief's section 4 the re-baseline waits for the design amendment.

## Context
- Governing brief: `docs/coding-agent/briefs/active/v0-2-completion-brief.md`. Its ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify docs/coding-agent/briefs/active/v0-2-completion-brief.md on those five framings.", admitted by the owner in this session on 2026-10-16: "Take the seat again, and check your inbox history. You should have a message from the Counsel, which carries my word. Take the quotes as mine." Product philosophy: `docs/project_philosophy.md`, ratification in `docs/project_philosophy-companion.md`. Readings file: `docs/coding-agent/plans/active/v0-2-completion-readings.md`. Run record: `docs/coding-agent/plans/active/v0-2-completion-run.md`.
- The brief's section 4: "the subject-state read's speed, statistics recovery" proceed; "the busy-occasion ordering fix" waits for the amendment carrying the v0.2 design re-examination.

## Tasks

### Task_1: A long history of beliefs about one person is read as fast as promises are
- type: impl
- owns: src/adapters/oxigraph/sparql_selectors.rs (`select_subject_state` and `select_state` only) and its tests
- depends_on: []
- acceptance:
  - The root cause, stated with the query before and after.
  - At the base and the tip, the same store gives the same selected beliefs, in the same order, and the same filtered and trace list, in both identifier orders.
  - A timing witness outside the permanent suite at 2,000 beliefs about one person, before and after, interleaved, run twice.
  - Existing tests pass unchanged.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; the equality and timing witnesses"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: the result is identical, the bound stays in Rust, checked at scale against the base form; the witness is not in the suite"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: the timing witness before and after at 2,000 beliefs (instrument); evals-worker runs the familiar-person and state families as controls at the Wave 1 tip. Falsified if any selected belief differs or the after time at 2,000 beliefs is above 1 s."

### Task_2: A character whose recall statistics failed recovers on its own
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
    detail: "Tier D: no new public surface, the rebuild is from the graph, a failed rebuild keeps the marker, healthy behavior unchanged"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: the red/green witness above is the instrument (at the base the reopened store selects differently from a healthy one; at the tip identically); evals-worker runs the scene and keyless families as controls at the Wave 1 tip. Falsified if the reopened store selects differently from a never-failed store."

### Task_3: What a busy occasion brings does not depend on identifier order
- note: numbered Task_4 in the root-cause records and messages of 2026-10-15; the dropped second-person task held the number 3.
- type: impl
- owns: src/policy/graph_expansion.rs (`ParticipantOccasion`, the shared comparator, incident-neighbour ordering before the fanout limits, both paths); src/adapters/oxigraph/shared.rs (the occasion metadata handed to that ordering; not Task_2's hydrator lines); src/adapters/oxigraph/sparql_selectors.rs (`select_participant_occasions` only); their tests
- depends_on: []
- acceptance:
  - The family that showed the dependence (the companion's busy-occasion residual, `calibrate_cue_floors --residuals-only`) selects the same observations in both identifier orders, and with equal salience they are the 16 newest.
  - A test at the policy or adapter boundary shows the selected observation times in both identifier orders with the caps unchanged, plus a salient older observation that survives over newer equals and a suppressed one that takes no seat.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: the tie key carries meaning and identifier order is the last resort"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Slice-end measurement at the final tip, run by evals-worker: the busy-occasion residual family before and after, both id orders, twice; controls: the other three residual families and the scene and familiar-person families match the base, or each difference is listed as intended (this order also decides admission under the node cap). Falsified if the two orders still differ, or if at equal salience the kept set is not the 16 newest."

## Task Waves (explicit parallel dispatch sets)
- Wave 1: [Task_1, Task_2]
- Wave 2: [Task_3]

The busy-occasion task is held under the brief's section 4 until the design amendment says whether its key stands. The second-person task was dropped on 2026-10-15.

## Research
- Root-cause reports, one per task, folded into the Design and the Decision Log: Task_1 `.worktrees/residual-state-read/.agent-work/worker/residual-state-read-rootcause.md`; Task_2 `.agent-work/worker/residual-stats-recovery-rootcause.md`; Task_3 `.agent-work/worker/residual-busy-occasion-rootcause.md`. Each task's `owns` is narrowed from its report.

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

- 2026-10-16 Decision: the plan runs under the v0.2 completion brief (Context). Task_1 and Task_2 proceed under its section 4; the busy-occasion task (Task_3) is held there, its root cause and design kept as written for the amendment to confirm or replace. The slice-end run covers Task_1 and Task_2's controls when Wave 1 closes, and the busy-occasion family when Task_3 is released.
  - User approval: the brief's section 4, the owner's ratified words; the hold is the brief's, not an Orchestrator ruling.
  - Record proposed: none.
- 2026-10-16 Decision: Task_1's root cause and design, from its report. One VALUES table of every member makes the metadata query slow (8.45 s at 2,000); 32 tables of at most 64 rows return identical rows in 0.18 s; the joinable scope variants were measured no faster (9.7 s to 32 s). Chosen: batched membership, one global lifecycle, order and cap pass, the bound in Rust; `owns` narrowed to the two selectors.
- 2026-10-16 Decision: the re-review of 7bf694ff (CHANGES REQUESTED) applied: Task_1's design above; the planner-added list completed and the falsifier reason put in the decider's words; per-task lens lines; Task_3 takes the smaller design (a salience column on the occasion metadata read, one comparator), drops its dependency on Task_1 and narrows its `owns`; Task_2 records the one-cause limit and tests a repeated failure; measurement runs at the Wave 1 tip for Tasks 1 and 2 and names evals-worker; counts say three.
- 2026-10-16 Decision: the second re-review (24d13fb7) APPROVED with two minor findings, applied: Task_3 writes its own comparator (the shared one sits in Task_1's function), and every task states all six lens groups.
