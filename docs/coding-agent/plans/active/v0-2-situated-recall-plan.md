# Plan: recall hands over a few situations by weight, says what it could not find, and every memory stays reachable by what it is

- status: draft
- generated: 2026-10-16
- last_updated: 2026-10-16
- work_type: mixed

## Goal
- Build the recall design the owner ratified as section 4a of the v0.2 completion brief: a turn receives a few situations, ordered by weight down to the bound the application gives, each gathered whole within a bound and saying when it was cut, with the ingredients of its weight reported and the three states of nothing-found carried in the ordinary result; and, apart from that, every memory reachable by what it is can be walked in full, resumably, without skips. This replaces the routes-with-floors, the fixed room with reserved seats, the typed pack sections and the score that carried vector similarity as weight.

## Definition of Done
- `retrieve` returns situations, each with its parts, the path by which each part came, its reported weight ingredients (how much it matters, how long since last touched, whether owed and to whom and whether due, whether its party is present, whether it was ever said, how it felt, how far it departed from expectation; each `unknown` when the store has nothing for it), and a cut marker with a continuation when the gathering bound stopped it. No room is reserved per cue kind, nothing is admitted to fill a slot, and the result has no fixed sections.
- The order of situations, and of parts within one, follows a rule stated openly in the design record over the reported ingredients; a search's similarity is a condition of relevance for a topic or description cue and never a term of weight.
- The result carries the three absence states for the scene's cues: nothing found (with the paths attempted), something thin (a name resolving to several notions or to one with little behind it, or a match by wording alone, labelled), verified absence (an exhaustive, healthy structural check at a stated snapshot found nothing for a stated name, predicate or range). A miss by resemblance never reads as absence.
- A familiar person or place brings what is new or unresolved with it; one met once brings that occasion; a description matching by resemblance brings a labelled reminder. Newness is measured against the recorded experience, never against the last retrieval, and the history is reached whole through the walk.
- A public, resumable walk exists by person, place, thread, time range and identifier, returning every memory reachable that way in a total order with a cursor, neither skipping nor stopping silently; a page says whether more follows.
- The one bounded hop from a surfaced obligation stays; weak associations open no history; aboutness and presence stay distinct; scene cues are considered before the topic; the write path is untouched; time stays the clock.
- Records: a design record replacing ADR-D-0022 and ADR-D-0004's sections, an implementation record replacing ADR-I-0006's section cap while keeping its bounds, a record for structural access, the strengthening of ADR-D-0023, and the return of ADR-I-0036 and ADR-D-0039, each proposed for the owner; the README, the v0.2 phase draft and the roadmap's v0.2 text state the new design and nothing of the old.
- The section 4a measurement is recorded before anything learned or generative enters selection: a deterministic situation view compared with the base behaviour on the pressure cases (overlapping office and topic; two people with separate obligations; duplicated accounts; old high-salience matters; keyless scenes; empty queries), against topic-only and each-cue-removed controls, with candidate recall, situation grouping, coverage, unrelated admissions and rendered behaviour measured separately, and cost at a years-sized store with skewed hubs. The numbers, a paragraph on what they mean for the character and a verdict go in this plan's Decision Log.
- A subject's long history of beliefs is read as fast as its promises (the batched membership read), with identical rows.
- The memo handed to a model reads the situations in words (the past tense for settled matters, labelled reminders, oldest-first within what happened) and the runnable example carries a character through a few turns on it.

## Planner-added requirements
- One bound in `RetrievalContext` replaces `cue_floors` and `section_limits`: 4a's "the bound the application gives" is singular, and two knobs would reintroduce room by the back door.
- The parts of a situation report the path they came by, and the gathering records every candidate the bound dropped as a count with a continuation: 4a's "says by which path each part came, and where the bound cut the gathering it says so and offers to continue"; the count and continuation are the smallest form that offers to continue.
- Fanout pruning in graph expansion becomes a reported cut, not a silent trace fact: without it a busy occasion's omitted observations are invisible to the result, the defect the busy-occasion root cause measured.
- The cursor of the walk is keyset-based (last key seen), not an offset: an offset skips or repeats under concurrent writes, and 4a requires neither skip nor silent stop.
- Verified absence requires a healthy statistics store and a complete walk at a snapshot token; where the engine gives no snapshot, the result says the check was at the time of reading: 4a requires "an exhaustive, healthy structural check ... at a stated snapshot", and claiming a snapshot the engine does not give would be an invented guarantee.
- The null distribution for resemblance is measured on the deterministic test embedder in this plan, and the live embedder's distribution is a named follow-up needing the owner's spend decision (Open Questions): 4a requires the bound "measured against a null distribution on the intended embedder", which costs embedding calls.
- Subject-state batching with one global lifecycle, order and cap pass: the only form measured fast with identical rows (root cause of 2026-10-15); per-batch caps would select differently.
- Tests that the brief lists as standing but that assert floors or slots are restated to assert their behavioural claim: 4a names the obligation and resolution tests as standing, and their floor assertions cannot survive a design with no floors.
- The result's situations keep the obligation's `direction`, `due_state` and `resolved_by`: they are the owed-and-due ingredient 4a names.

## Scope / Non-goals
- Scope: the public result contract and the one bound; situation gathering and ordering in `src/usecases/retrieve*`, `src/policy/graph_expansion.rs` and the state selectors' ordering; the walk through the graph port, new selectors, the test double and the facade; the absence states; the familiarity reading from share, frequency and recency; the memo and example over situations; the replaced and restated tests; the records; the README, the v0.2 phase draft and the roadmap's v0.2 text; the subject-state batched read; the measurement.
- Non-goals: the write path; reflection, disposition and felt tone as computed ingredients (v0.3; they are reported `unknown` here); time counted in experience (the brief's section 3); consolidation's reach where neither anchor nor recognizable content exists (4a, provisional: v0.3); the statistics recovery (the residuals plan); the catalog and the roadmap's later phases (the catalog and roadmap plan); the paid benchmark runs and the closeout reading (after this plan, under the brief's sections 3 and 5); any learned or generative selection.

## Design
- Chosen: contract first, then gathering; a situation is a set of parts around an anchor (a person present, a place, a thread, an unsettled matter, a topic), gathered through the anchor's own links one bounded hop and through shared subject and thread, ordered by an open lexicographic rule over reported ingredients, cut at the bound with a continuation; the walk is a separate port surface with a keyset cursor; vector hits are admitted as a condition (above the measured null) and never weighed. Structure: one result type owns the handed-over memories (no pack beside it), no duplicated state, two new port methods and one cursor type, the coupling direction policy-depends-on-ports unchanged. Evolution: the order rule is one documented function and the only place a later learned weight would replace; `cue_floors`, `section_limits`, the road table's `reserves` and `contribution` columns and the seat rounds are deleted; six records' `depends_on` retargeted once. Verification: the situation view is deterministic on the embedded stores with deterministic embeddings; the walk is checked against a full scan; tests pin the contract (parts, paths, ingredients, cut, absence states), not the order function's internals; both identifier orders and repeat runs are controls. Operation: candidate reads are the existing selectors, gathering is bounded by the existing depth, fanout and hub caps, and the batched subject read removes the 9 s outlier; cost at a years-sized store is measured before latency is promised. Human: a situation reads as a person would name it; the cut and the absence states say what the character does not have. Safety: no new trust boundary; the walk exposes nothing the facade did not already return in full over time. (consumers: see the Compatibility stance)
- Alternative: a flat list of memories with ingredients and paths, no situation grouping, ordered by the same open rule. Structure: simpler (no gathering, no anchor); evolution: the grouping would be added later under pressure; verification: easier to pin; human: the model receives twelve memories and must reassemble the situation itself, which 4a's first gives-statement names as the failure ("not twelve memories in reserved slots"). Rejected on the gives-statement.
- Alternative: contract and gathering in one task. Structure and evolution the same; verification: no intermediate state to measure against; operation the same; human: one very large change with no control point. Rejected: the intermediate state (situations grouped by anchor with the existing candidates) gives the measurement a control arm that isolates the gathering's effect from the contract's.
- Why chosen: it is the only one of the three that delivers 4a's statements as written and leaves a measurable step between the contract and the behaviour change.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- Stance: break. Consumers: the companion evaluation repository's adapter (`crates/cmem-eval/src/adapter.rs`, binds `pack`, sections and `cue_floors`) and the renderer's uncommitted WIP; both are ours, and the evaluation repository adapts under its own plan amendment. No external consumer exists (pre-consumer rule). The persisted format is untouched.

## Context
- Governing brief: `docs/coding-agent/briefs/active/v0-2-completion-brief.md`, section 4a. The brief's ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify docs/coding-agent/briefs/active/v0-2-completion-brief.md on those five framings.", admitted by the owner in this session on 2026-10-16: "Take the seat again, and check your inbox history. You should have a message from the Counsel, which carries my word. Take the quotes as mine." The amendment's ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify the design amendment, section 4a of docs/coding-agent/briefs/active/v0-2-completion-brief.md, on those seven framings.", admitted by the owner's statement in this session on 2026-10-16 admitting Counsel's relays standing ("Admit relays, standing"; the Standing Approvals entry in `docs/coding-agent/rules/common.md` accepted "I accept the standing rule."). Product philosophy: `docs/project_philosophy.md` (7.2 to 7.6, 8.1, 8.5), ratification in `docs/project_philosophy-companion.md`. Readings file: `docs/coding-agent/plans/active/v0-2-completion-readings.md`. Run record: `docs/coding-agent/plans/active/v0-2-completion-run.md`.
- 4a's two provisional statements (a coverage preference, if measurement shows one is needed, stated openly as a policy; bounded retrieval cannot find a connection with neither anchor nor recognizable content) are built as written; an item relying on either is sent to Counsel as the audit states it.
- Research: `.agent-work/researcher/v0-2-situated-recall-redesign-research.md` (2026-10-16), with file and line for every area below. Root causes carried in: `.worktrees/residual-state-read/.agent-work/worker/residual-state-read-rootcause.md` (subject-state read: one VALUES table 8.45 s, 64-row batches 0.18 s, identical rows); `.agent-work/worker/residual-busy-occasion-rootcause.md` (the fanout keeps the 16 lowest identifiers of 25 observations; pruning is not reported).
- Today: `retrieve` at `src/usecases/retrieve.rs:104-590`; the road table at 1343-1523; floors `select_with_cue_floors` 973-1069 at three stages; the pack `build_pack` 1071-1228; seat rounds `src/usecases/retrieve/state.rs:62-139`; `final_score` 913-917 (0.65 cue, 0.25 graph, 0.10 salience); result types `src/api/types/retrieval.rs` (`RetrieveOutcome` 247, `MemoryScenes` 262, `AdmissionRoad` 265, `ContinuityContextPack` 394, `IncludedDerivedMemory` 423-447, `ContextPackSection` 817, `RetrievalCueFloors` 128, `ContinuitySectionLimits` 190); scene reference resolution `src/usecases/retrieve/scene.rs:48-122, 265-273` (stands); graph expansion `src/policy/graph_expansion.rs:107`, one hop at retrieve.rs:1415-1432 and 1791-1793, fanout trace only at retrieval.rs:674; state selectors `src/adapters/oxigraph/sparql_selectors.rs:433-602` with callers at 245, 327, 344, 366, 401; share `src/usecases/retrieval_selectivity.rs:109-110, 276` and the participant budget `graph_expansion.rs:1089, 1122`; the facade `src/memory.rs:57-162`; the graph port `src/ports/graph_authority.rs:345-464` (no cursor); the test double `src/test_support.rs:282`; `select_episodes_by_time` already totally ordered with a one-extra-row "more" (`sparql_selectors.rs:1036-1038`, retrieve.rs:273, 285).
- Ingredient data today: salience on every memory type; last touched (`LastInteraction.seconds_since`, `seconds_since_support`, `MemoryThread.last_touched_at`); owed and due (`assertions`, `due_at`, `direction`, `DueState`); party present computable from assertion subjects against resolved participants; ever said only as `Observation.speaker_entity_id`; felt tone none (ADR-D-0033: words, never a number); departure from expectation none. The last two and "ever said" for matters are `unknown` in v0.2.
- Records: ADR-D-0022, ADR-D-0004, ADR-I-0006 accepted and contradicted; ADR-I-0036 and ADR-D-0039 proposed, returned by 4a (returning I-0036 leaves ADR-I-0022 as the live defaults record, and the share budget and the named-notion exemption become unrecorded, so the new implementation record re-decides them); ADR-D-0023 strengthened; D-0009, D-0010, D-0013, D-0018, D-0024, D-0027, D-0029, D-0033, D-0034, D-0037, D-0038, I-0010, I-0024, I-0026 intact. `depends_on` ADR-D-0022 in D-0023, D-0024, D-0026, D-0029, D-0039, I-0036. Next free numbers: ADR-D-0041 (ADR-D-0040 is the catalog plan's), ADR-I-0037.
- Tests: replaced (floors, standing, room): `src/memory/retrieval_floor_tests.rs`, `tests/scene_reminder_tests.rs`, `tests/retrieval_scope_tests.rs`, the seat round in `tests/retrieval_state_tests.rs`, the section and reservation tests in `tests/retrieval_time_tests/{recency,range,anniversary}.rs` (their time substance stands), the room tests in `tests/public_facade_tests.rs`, the unit tests at retrieve.rs:2145-3294, the section-limit tests in `src/memory/retrieval_scene_tests/{descriptions,presence}.rs`. Standing with floor assertions to restate: `tests/retrieval_obligation_tests.rs`, `tests/retrieval_resolution_tests.rs`. Standing untouched: lifecycle, scene, write-turn, notion, last-interaction, vector port, write planning, adapter, expansion and selectivity tests.
- Measurement instrument: the companion's pressure families (research Q6: `calibrate_cue_floors` overlapping, `consolidation.rs` office cases, `obligations.rs` Iris and Rowan, `situated_v1.toml` B2 and C4, `residuals.rs` old-salient-place and empty queries, `time_and_obligations.rs`), their topic-only and cue-removed controls, both identifier orders, repeat runs. The harness work (a situation-view reader replacing the floor sweep, the adapter off `pack`, the Task_6 re-baseline re-anchored, Task_9 replaced) is planned in the evaluation repository's situated plan amendment, not here; this plan states what those measurements let it decide. A years-sized synthetic store with skewed hubs does not exist and is built there as a deterministic generator.
- Renderer plan (superseded): what survives into Task_6 is the past tense for settled matters, labelled reminders, the oldest-first stream of what happened, the honest opening on references, the elapsed phrasing and the runnable example; WIP `ae319921` on `feature/2026-10-15/renderer-memo` is its starting material.
- Rulings carried: 71 (the as-of cut), 73 (UTC when no offset), the busy-occasion root cause (identifier order is a defect), 85.

## Integration
- Branch `feature/2026-10-16/v0-2-situated-recall-plan` from the renderer plan tip `af605e09` (the whole library stack plus the brief with 4a); one pull request per task stacked on it; the residuals plan's statistics-recovery branch joins by plain merge when it lands (different files).
- Wave order: records, contract and the batched read first; gathering and the walk second; memo and example, docs last. The measurement's before arm is taken at `af605e09` by the evaluation worker before Task_4 starts, and its after arm at Task_4's tip, then again at the final tip as the slice-end run.
- After this plan: the evaluation repository's re-baseline (Task_6 there) at the final tip, then the closeout reading (brief section 3), then the paid runs (section 5).

## Open Questions (max 3)
- The resemblance null distribution on the intended embedder costs live embedding calls (not a benchmark run; outside the brief's section 5 wording but a spend). This plan measures the null on the deterministic test embedder and names the live measurement as a follow-up; the owner decides whether the spend happens in v0.2.

## Assumptions
- A1: Oxigraph 0.5.7 exposes no read snapshot across queries; the walk reports "at the time of reading" unless Task_3's probe finds one. Checked by Task_3.
- A2: A situation anchored on a present person or an unsettled matter can be gathered from the existing selectors (state, obligations, links one hop, shared subject and thread) without a new stored shape. Checked by Task_4; a stored-shape need stops and reports.
- A3: The companion's pressure families can be read as situations by a reader over the new result without changing their stores or scenarios. Source: research Q6. Checked by the evaluation worker's instrument.

## Tasks

### Task_1: The records say what recall is now, and the old ones are replaced or returned
- type: docs
- owns: docs/decisions/design/ADR-D-0041-*.md (new), docs/decisions/design/ADR-D-0042-*.md (new), docs/decisions/implementation/ADR-I-0037-*.md (new); the status and supersession lines only of ADR-D-0022, ADR-D-0004, ADR-I-0006, ADR-I-0036, ADR-D-0039 and ADR-D-0023; the `depends_on` lines of D-0023, D-0024, D-0026, D-0029; docs/decisions/README.md index lines
- depends_on: []
- acceptance:
  - Propose an ADR if the admission test passes (`durable-docs-authoring/references/adr.md`) for each of: situations by weight down to the application's bound, with cues as how a situation is found, no reserved room, no fixed sections, the ingredients reported and the order rule stated openly, any coverage preference as an open policy (replaces ADR-D-0022 and ADR-D-0004's sections); the bounded gathering (keeps ADR-I-0006's depth, fanout, hub, allowlist, retention and bounded-failure bounds, replaces its section cap, adds the cut-and-continue reporting and re-decides the share-based participant budget and the named-notion exemption that returning ADR-I-0036 un-records); complete structural access as a resumable walk by what a memory is, distinct from a turn's recall, with resemblance bounded and reported.
  - ADR-D-0023 gains the strengthening 4a states (the one hop is how an obligation begins to gather its situation); ADR-I-0036 and ADR-D-0039 are marked returned with the brief's section 4a as the reason; the three contradicted records are marked superseded by their successors; the `depends_on` lines point at the successors.
  - No record's Decision text is edited other than status, supersession and dependency lines; one decision per new record; consulted entries are model names only.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: each new record states one decision at 4a's altitude, cites the philosophy statements it traces to, keeps ADR-I-0006's bounds, re-decides the share budget and the named-notion exemption openly; no accepted record's decision text changed; dependency graph navigable"
  - kind: manual
    required: true
    owner: user
    detail: "The owner accepts or returns each proposed record and each supersession (reserved acts, carried by Counsel)"

### Task_2: The result is situations with a cut marker and absence states, under one bound
- type: impl
- owns: src/api/types/retrieval.rs; src/memory.rs (the retrieve signature and docs only); src/usecases/retrieve.rs (the result assembly: `build_pack`, `prompt_ready_sections`, `section_for_object`, `section_limit`, `select_with_cue_floors` at the sections stage, the road table's `reserves` and `contribution` columns); src/usecases/retrieve/state.rs (the seat rounds); tests/retrieval_obligation_tests.rs and tests/retrieval_resolution_tests.rs (floor and slot assertions only); the replaced tests listed in Context (deletion or restatement)
- depends_on: []
- acceptance:
  - `RetrievalContext` carries one bound and no `cue_floors` or `section_limits`; `RetrieveOutcome` carries `situations` (each: anchor, parts with the path each came by, reported ingredients with `unknown`, a cut marker with a continuation) and `absences` (nothing found with attempted paths; something thin with its kind; verified absence with scope and snapshot statement), keeps `scene`, `activity`, `time_range`, `scene_references`, `rationale`, `trace`, and has no `pack`, `memory_scenes`, sections or floor admissions.
  - At this task's tip, the set of memories handed over for every existing retrieval test store equals the base's set (grouping by anchor only; the gathering changes nothing yet), the seat rounds and section caps are gone, and the result's parts carry `direction`, `due_state` and `resolved_by` where the base's pack did.
  - The replaced tests are deleted or restated per Context; the standing obligation and resolution tests keep their behavioural claims with floor and slot assertions restated against situations.
  - No reserved room, no fixed sections, no floor remains in the public API, the trace vocabulary or the README table (README is Task_7's; this task reports the lines).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; a same-set witness over every retrieval test store, base against tip, both identifier orders"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: the contract matches 4a's statements field by field; no shim keeps the pack alive; the same-set witness holds; restated tests assert the claim, not the mechanism"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: control only. The evaluation worker runs the situated fixtures and the pressure families at this tip through the adapted reader; falsified if any handed-over set differs from the base (the before arm at af605e09)."

### Task_3: Every memory reachable by what it is can be walked in full
- type: impl
- owns: src/ports/graph_authority.rs (new walk methods and a cursor type); new functions in src/adapters/oxigraph/sparql_selectors.rs and src/adapters/oxigraph/embedded.rs; src/test_support.rs (forwarding arms); src/memory.rs (the walk entry point); tests/structural_walk_tests.rs (new)
- depends_on: [Task_2]
- acceptance:
  - A public walk by person, place (scope key), thread, time range and identifier returns pages in a total order with a keyset cursor, says whether more follows, and over a store with writes between pages neither skips nor repeats a memory that existed before the walk began.
  - A walk run to the end over each key equals a full scan of the store for that key (the witness, both identifier orders, stores of 1, 100 and 2,000 memories).
  - A1 is checked: the walk's snapshot statement is what the engine gives, and the report says which.
  - The walk never degrades silently: a bounded or failed read surfaces as an error or an explicit incomplete page, never an empty one.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; the full-scan equality witness"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: the cursor is keyset-based, the order is total, no page can be silently short, the double forwards every new method, the facade's docs state the guarantee"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: the full-scan equality witness at 2,000 memories per key, timed, before (none exists) and after; falsified if any key's walk misses or repeats a memory, or if a 2,000-memory walk exceeds the same order of time as the batched subject read."

### Task_4: A turn receives a few situations by weight, gathered whole within a bound, and knows what it does not have
- type: impl
- owns: src/usecases/retrieve.rs (candidate finding, gathering, ordering, absence states; not the result assembly Task_2 owns, now stable); src/usecases/retrieve/scene.rs (the topic and description hits as a condition); src/policy/graph_expansion.rs (incident-neighbour ordering before the fanout limits in both paths, and fanout pruning as a reported cut); the ordering in `select_state` (src/adapters/oxigraph/sparql_selectors.rs:565-600) and its callers' order arguments; src/usecases/retrieval_selectivity.rs (familiarity from share, frequency and recency); their tests
- depends_on: [Task_2]
- acceptance:
  - A situation anchored on a present person, a place, a thread or an unsettled matter gathers what rests on it through its own links one bounded hop and, through shared subject and thread, what bears on it that nobody linked; where the bound cut the gathering the situation says so with a continuation; fanout pruning is such a cut.
  - Situations and their parts are ordered by one rule stated in the design record over the reported ingredients, with identifier as the last resort; the companion's busy-occasion family selects the same observations in both identifier orders. The rule is chosen by a bounded experiment between a lexicographic order and a stated combination on the pressure families, recorded in the report and the Decision Log; the chosen rule is deterministic and has no hidden formula.
  - A topic or description vector hit admits a situation only above the null distribution measured on the test embedder, is labelled as a match by wording where that is all that connects it, and never contributes to weight.
  - A familiar person or place (read from share, frequency and recency) brings what is new or unresolved with it, measured against the recorded experience; one met once brings that occasion; a description brings a labelled reminder.
  - The three absence states are produced: nothing found names the paths attempted; something thin covers ambiguous names, names with little behind them (the share threshold recorded) and wording-only matches; verified absence is produced only when the walk of Task_3 ran to the end for that name, predicate or range on a healthy store, with its snapshot statement.
  - A coverage policy is added only if the measurement shows the present person crowded out by a loud topic, and then as a named, documented policy in the design record (4a, provisional); the report states whether it was needed, with the numbers.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; both identifier orders on the busy-occasion fixture; the order-rule experiment's numbers"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D and Tier A: gathering follows the anchor's links and shared subject and thread and nothing else; the order rule is open and deterministic; similarity is a condition, never weight; the cut is reported wherever a bound dropped a candidate; absence states are never produced from a resemblance miss; no floor, seat or section returned under another name"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement (the brief's section 4a): the evaluation worker runs the deterministic situation view at this tip against the base at af605e09 on the pressure families (overlapping office and topic, two people with separate obligations, duplicated accounts, old high-salience matters, keyless scenes, empty queries), with topic-only and each-cue-removed controls, both identifier orders, twice; candidate recall, situation grouping (against hand-labelled situations in two families), coverage, unrelated admissions and rendered behaviour measured separately; cost at a years-sized synthetic store with skewed hubs. Falsified if the present person's situation is absent under a loud topic without a named policy, if any pressure case loses a candidate the base found that a person would have in mind, if the two identifier orders differ, or if the years-sized cost grows faster than linearly in the matched candidates."

### Task_5: A subject's long history of beliefs is read as fast as its promises
- type: impl
- owns: `select_subject_state` in src/adapters/oxigraph/sparql_selectors.rs (lines 245-325) and its tests; not `select_state`'s ordering (Task_4's)
- depends_on: []
- acceptance:
  - The membership read is batched in tables of at most 64 members, with lifecycle aggregation, order and caps applied once over all rows; the result, including the filtered and trace lists, is identical to the base at every batch boundary, in both identifier orders.
  - A timing witness outside the permanent suite at 2,000 beliefs about one person, before and after, interleaved, run twice.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; the equality and timing witnesses"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: identical rows across batches, one global pass, no port or shape change, the witness outside the suite"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: the timing witness at 2,000 beliefs (instrument); falsified if any row differs or the after time is above 1 s."

### Task_6: The situations are handed to a model in words, and an example carries a character through a few turns
- type: impl
- owns: src/api/types/retrieval/memo.rs (new) and its `RetrieveOutcome::memo` entry; examples/*.rs (new); Cargo.toml (the example's dev dependencies only); the README's example section (a named block; the rest is Task_7's)
- depends_on: [Task_4]
- acceptance:
  - The memo reads each situation as a person would hand it to an actor: the present, then each situation with its parts in words, how each came, what is owed and by whom in the right tense (past when settled), reminders labelled as resemblance, what the character does not have (the absence states), and where a gathering was cut; nothing is phrased as an instruction.
  - The example builds the embedded stores in a temporary directory, carries a character through a few turns, prints the memo each turn, runs twice to identical bytes, and cleans up.
  - The renderer plan's R4 witnesses hold (a context value in a different setting, a topic-only descendant, a matter between two others with one present, a settled matter): no line presents a resemblance as knowing, a reminder as the current place, a settled matter as open, or owing when direction is none.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; cargo run --example twice with a byte comparison; the printed memos of the fixtures filed in the report directory"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: every memo line read as an actor would; the R4 witnesses; no imperative; the absence states and cuts legible"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: rendered behaviour is one of the five measures in Task_4's slice-end run; the owner's read of the fixture memos is the falsifier for phrasing (any line that misleads an actor)."

### Task_7: The README, the v0.2 phase draft and the roadmap state the new design and nothing of the old
- type: docs
- owns: README.md (except Task_6's example block); docs/roadmap/roadmap-phases/v0_2_scoped_continuity_reflection.md (design sections); docs/roadmap/development_roadmap.md (the v0.2 rows and section 2.16, and the familiarity rows the catalog plan left to this plan)
- depends_on: [Task_1, Task_2]
- acceptance:
  - Every line the research lists as stating the replaced design (README 51-67, 76-101, 122-154, 138-140; the draft 56-68, 82-84, 96-98, 121, 141-145, 155, 186, 194; the roadmap 283, 325, 1312-1316, 1335, 1371, 1432, 1653-1664) states situations by weight, the one bound, the walk, the absence states, and familiarity as 4a states it; no floor, room, section or `pack.render` remains.
  - The roadmap's familiarity rows place it where 4a does (v0.2's recall design), closing the item the catalog plan left open.
  - The phase draft's judging line names the closeout reading the catalog plan wrote (no second wording).
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: against the brief's 4a and the proposed records; consumer-facing README reads purpose before backend; no hard-wrapped prose; nothing of the old design left"
  - kind: manual
    required: true
    owner: user
    detail: "The owner's read of the README's recall section (with the records)"

## Task Waves (explicit parallel dispatch sets)
- Wave 1: [Task_1, Task_2, Task_5]
- Wave 2: [Task_3, Task_4]
- Wave 3: [Task_6, Task_7]

Task_2 and Task_4 both touch `src/usecases/retrieve.rs`; the file is split by function as the owns lines state, Task_2 first. Task_4 and Task_5 both touch `sparql_selectors.rs` in different functions. The evaluation worker's before arm runs at `af605e09` during Wave 1.

## Rollback / Safety
- Each task is one pull request on the stack and reverts in reverse order; nothing stored changes; the walk adds port methods and removes nothing. A revert of Task_2 restores the pack and the floors wholesale, so Task_2 is not merged alone.

## Research
- Researcher report: `.agent-work/researcher/v0-2-situated-recall-redesign-research.md` (2026-10-16), read whole, with the two root-cause reports named in Context and the superseded renderer plan. Unknowns it could not resolve are carried as bounded experiments inside the tasks that need them (the order rule and the coverage policy in Task_4; the snapshot in Task_3; the live-embedder null as the Open Question) and as the evaluation repository's synthetic large store.

## Progress Log (append-only)
- 2026-10-16: plan drafted under the v0.2 completion brief's section 4a, from revision `af605e09` (the brief with its amendment, the accepted Standing Approvals entry).

## Decision Log (append-only; re-plans and major discoveries)
- 2026-10-16 Decision: the plan is one plan with seven tasks, contract before gathering, the walk beside the gathering, the memo and docs after.
  - Trigger / new insight: the owner's amendment (4a) replaces the recall selection and room, the result contract and the renderer as specified, and leaves the storage, time, obligation, resolution, lifecycle, scene, write-turn and notion work standing. The research maps each area.
  - Plan delta: new plan; supersedes the renderer plan and takes two residuals (the subject-state read, the busy-occasion order) from the residuals plan with their root causes. The busy-occasion key the residuals plan proposed (current, salient, newest, id) is not carried: ordering is Task_4's open rule, and identifier-order independence is its acceptance.
  - Rulings in this plan: felt tone, departure from expectation and "ever said" for matters are reported `unknown` in v0.2 (no data; ADR-D-0033 forbids a number for feeling); the null distribution is measured on the test embedder here (Open Question for the live one); a coverage policy enters only on the measurement, as 4a's provisional line says; the share budget and the named-notion exemption are re-decided in the implementation record rather than left unrecorded by returning ADR-I-0036.
  - User approval: the brief's amendment, the owner's ratified words; authorization under the Plan Gate's second source after the plan review and the plan-draft value audit.
  - Record proposed: three (Task_1), plus supersessions and returns.
