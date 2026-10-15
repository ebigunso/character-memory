# Plan: recall hands over a few situations by weight, says what it could not find, and every memory stays reachable by what it is

- status: draft
- generated: 2026-10-16
- last_updated: 2026-10-16
- work_type: mixed

## Goal
- Build the recall design the owner ratified as section 4a of the v0.2 completion brief: a turn receives a few situations, ordered by weight down to the bound the application gives, each gathered whole within a bound and saying when it was cut, with the ingredients of its weight reported and the three states of nothing-found carried in the ordinary result; and, apart from that, every memory reachable by what it is can be walked in full, resumably, without skips. This replaces the routes-with-floors, the fixed room with reserved seats, the typed pack sections and the score that carried vector similarity as weight.

## Definition of Done
- `retrieve` returns situations, each with its parts, the path by which each part came, its reported weight ingredients (how much it matters, how long since last touched, whether owed and to whom and whether due, whether its party is present, whether it was ever said, how it felt, how far it departed from expectation; each `unknown` when the store has nothing for it), and a cut marker carrying a continuation (the walk's cursor for that situation's anchor) when the gathering bound stopped it. No room is reserved per cue kind, nothing is admitted to fill a slot, and the result has no fixed sections.
- The order of situations, and of parts within one, follows a rule stated openly in the design record over the reported ingredients; a search's similarity is a condition of relevance for a topic or description cue and never a term of weight; the result reports the resemblance search's coverage.
- The result carries two of the absence states for the scene's cues: nothing found (with the paths attempted); something thin (a name resolving to several notions or to one with little behind it, or a match by wording alone, labelled). A miss by resemblance never reads as absence. The third state, verified absence, is not built by this plan: whether it reaches the character inside a turn or through a separate exhaustive check, and whether it must name a snapshot, is with the owner through Counsel (Decision Log, 2026-10-16); it arrives as its own unit on the answer, and until then the turn claims no absence beyond nothing found.
- A familiar person or place brings what is new or unresolved with it; one met once brings that occasion; a description matching by resemblance brings a labelled reminder. Newness is measured against the recorded experience, never against the last retrieval. When a cue touches a familiar thing's history (a past occasion, a thread, a topic in it), the situation gathers that history within the bound and its continuation reaches the rest; nothing of it is hidden.
- A public, resumable walk exists by person, place, thread, time range and identifier, returning every memory reachable that way in a total order with a keyset cursor, neither skipping nor stopping silently; a page says whether more follows.
- The one bounded hop from a surfaced obligation stays; weak associations open no history; aboutness and presence stay distinct; scene cues are considered before the topic; the write path is untouched; time stays the clock.
- Records: three proposed records (situations by weight, replacing ADR-D-0022 and ADR-D-0004's sections; bounded gathering, replacing ADR-I-0006's section cap and keeping its bounds; structural access), a proposed amendment to ADR-D-0023, the return of ADR-I-0036 and ADR-D-0039, and the retirement of the three replaced records by the repository's procedure, each for the owner; the README, the v0.2 phase draft and the roadmap's v0.2 text state the new design and nothing of the old.
- The section 4a measurement is recorded before anything learned or generative enters selection: a deterministic situation view at the final tip compared with the base at `af605e09` on the pressure cases (overlapping office and topic; two people with separate obligations; duplicated accounts; old high-salience matters; keyless scenes; empty queries), against topic-only and each-cue-removed controls, with candidate recall, situation grouping, coverage, unrelated admissions and rendered behaviour measured separately, and cost at a years-sized store with skewed hubs. The numbers, a paragraph on what they mean for the character and a verdict go in this plan's Decision Log.
- A subject's long history of beliefs is read as fast as its promises, with identical rows.
- The memo handed to a model reads the situations in words and the runnable example carries a character through a few turns on it.

## Planner-added requirements
- One bound in `RetrievalContext` replaces `cue_floors` and `section_limits`: 4a's "the bound the application gives" is singular, and two knobs would reintroduce room by the back door.
- A situation's parts report the path they came by; a cut is a count of candidates the bound dropped plus the walk's cursor for the situation's anchor: 4a's "says by which path each part came, and where the bound cut the gathering it says so and offers to continue"; reusing the walk's cursor is the smallest thing that offers to continue, with no second resumable surface.
- Fanout pruning in graph expansion becomes a reported cut: without it a busy occasion's omitted observations are invisible to the result, the defect the busy-occasion root cause measured.
- The walk's cursor is keyset-based (last key seen), never an offset: an offset skips or repeats under concurrent writes, and 4a requires neither skip nor silent stop.
- The resemblance threshold shipped as the default comes from a one-time null distribution on the intended live embedder (authorized by the owner, Decision Log), with sample size and cost recorded; the suite pins the rule with a null measured on the deterministic test embedder, which is never the shipped default: 4a requires the bound "measured against a null distribution on the intended embedder".
- The subject-state read is batched in member tables of at most 64 with one global lifecycle, order and cap pass, and its timing witness (1 s at 2,000 beliefs, interleaved, run twice) stays outside the permanent suite: the only form measured fast with identical rows (root cause of 2026-10-15); per-batch caps would select differently; a timing assertion in CI is flaky across machines.
- Tests the brief lists as standing but that assert floors or slots are restated to assert their behavioural claim: 4a names the obligation and resolution tests as standing, and their floor assertions cannot survive a design with no floors.
- The result's situations keep the obligation's `direction`, `due_state` and `resolved_by`: they are the owed-and-due ingredient 4a names.
- The order rule is chosen by a bounded experiment between a lexicographic order and a stated combination over the reported ingredients (Task_4): 4a demands an order with no hidden formula and reported ingredients; the experiment decides which open rule reads the pressure cases as a person would, and the record states it.
- The walk's equality witness runs at stores of 1, 100 and 2,000 memories per key and with writes between pages: the three sizes catch an off-by-one at the page edge and the cost shape; writes between pages are the case a keyset cursor exists for.
- The live null measurement's artifact (inputs, sample size, cost, the resulting threshold) is kept under `.agent-work/worker/` and promoted into Task_4's report and the gathering record's amendment: a shipped default rests on it.
- The example runs twice to identical bytes and removes its stores: determinism is what makes the memo reviewable, and test and eval runs clean up every store they create (repository practice; the evaluation repository's retention rule).

## Scope / Non-goals
- Scope: the public result contract and the one bound; situation gathering and ordering in `src/usecases/retrieve*`, `src/policy/graph_expansion.rs`, `src/adapters/oxigraph/shared.rs` and the state selectors' ordering; the walk through the graph port, new selectors, the test double and the facade; the two absence states nothing found and something thin; the familiarity reading from share, frequency and recency; the memo and example over situations; the replaced and restated tests; the records and their retirements; the README, the v0.2 phase draft and the roadmap's v0.2 text; the subject-state batched read; the measurement.
- Non-goals: the write path; reflection, disposition and felt tone as computed ingredients (v0.3; reported `unknown` here); time counted in experience (the brief's section 3); consolidation's reach where neither anchor nor recognizable content exists (4a, provisional: v0.3); the statistics recovery (the residuals plan); the catalog and the roadmap's later phases (the catalog and roadmap plan); the paid benchmark runs and the closeout reading (after this plan, under the brief's sections 3 and 5); any learned or generative selection; the evaluation harness's own changes (planned in the evaluation repository); verified absence (its own unit on the owner's answer); amendments to ADR-D-0013, ADR-D-0014 and ADR-I-0017 for their decay wording (left as history; the phase that touches decay rewrites them).

## Design
- Chosen: contract first, then gathering; a situation is a set of parts around an anchor (a person present, a place, a thread, an unsettled matter, a topic), gathered through the anchor's own links one bounded hop and through shared subject and thread, ordered by an open rule over reported ingredients, cut at the bound with the walk's cursor as its continuation; the walk is a separate port surface with a keyset cursor; vector hits are admitted as a condition (above the measured null) and never weighed. Instrument: the deterministic situation view on the pressure families against the base; falsifier: a present person's situation absent under a loud topic with no named policy, a candidate the base found and a person would have in mind lost, the two identifier orders differing, or cost at a years-sized store growing faster than linearly in matched candidates. Structure: one result type owns the handed-over memories (no pack beside it), no duplicated state, two new port methods and one cursor type, policy depends on ports as today. Evolution: the order rule is one documented function and the only place a later learned weight would replace; `cue_floors`, `section_limits`, the road table's `reserves` and `contribution` columns and the seat rounds are deleted; six records' `depends_on` retargeted once. Verification: the situation view is deterministic on the embedded stores with deterministic embeddings; the walk is checked against a full scan; tests pin the contract (parts, paths, ingredients, cut, absence states), not the order function's internals; both identifier orders and repeat runs are controls. Operation: candidate reads are the existing selectors, gathering is bounded by the existing depth, fanout and hub caps, and the batched subject read removes the 9 s outlier; cost at a years-sized store is measured before latency is promised. Human: a situation reads as a person would name it; the cut and the absence states say what the character does not have. Safety: no new trust boundary; the walk exposes nothing the facade did not already return in full over time. (consumers: see the Compatibility stance)
- Alternative: a flat list of memories with ingredients and paths, no situation grouping, ordered by the same open rule. Structure: simpler (no gathering, no anchor). Evolution: the grouping would be added later under pressure. Verification: easier to pin. Operation: the same reads and bounds. Human: the model receives twelve memories and must reassemble the situation itself, which 4a's first gives-statement names as the failure ("not twelve memories in reserved slots"). Safety: the same. Rejected on the gives-statement.
- Alternative: contract and gathering in one task. Structure and evolution the same. Verification: no contract-only tip to review on its own. Operation and safety the same. Human: one very large change with no checkpoint. Rejected: landing the contract first lets its review and the test restatement close before the behaviour change, and the measurement's control arm is the base at `af605e09` either way.
- Why chosen: it is the only one of the three that delivers 4a's statements as written and leaves a reviewable step between the contract and the behaviour change.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- Stance: break. Consumers: the companion evaluation repository's adapter (`crates/cmem-eval/src/adapter.rs`, binds `pack`, `section_limits` and `cue_floors`) and the renderer's uncommitted WIP; both are ours, and the evaluation repository adapts under its own plan amendment. No external consumer exists (pre-consumer rule). The persisted format is untouched.

## Context
- Governing brief: `docs/coding-agent/briefs/active/v0-2-completion-brief.md`, section 4a. The brief's ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify docs/coding-agent/briefs/active/v0-2-completion-brief.md on those five framings.", admitted by the owner in this session on 2026-10-16: "Take the seat again, and check your inbox history. You should have a message from the Counsel, which carries my word. Take the quotes as mine." The amendment's ratification reached this session as Counsel's relay of the owner's words of 2026-10-16, "I ratify the design amendment, section 4a of docs/coding-agent/briefs/active/v0-2-completion-brief.md, on those seven framings.", admitted by the owner's statement in this session on 2026-10-16 admitting Counsel's relays standing ("Admit relays, standing"; the Standing Approvals entry in `docs/coding-agent/rules/common.md` accepted "I accept the standing rule."). From 2026-10-16 the owner is reached through Counsel only ("Reach me through Counsel."). Product philosophy: `docs/project_philosophy.md` (7.2 to 7.6, 8.1, 8.5), ratification in `docs/project_philosophy-companion.md`. Readings file: `docs/coding-agent/plans/active/v0-2-completion-readings.md`. Run record: `docs/coding-agent/plans/active/v0-2-completion-run.md`.
- 4a's two provisional statements (a coverage preference, if measurement shows one is needed, stated openly as a policy; bounded retrieval cannot find a connection with neither anchor nor recognizable content) are built as written; an item relying on either is sent to Counsel as the audit states it.
- Research: `.agent-work/researcher/v0-2-situated-recall-redesign-research.md` (2026-10-16), with file and line for every area; it is the map of the code, records and tests this plan touches, and the tasks' `owns` are drawn from it and from the root causes' "scope a fix would need" sections: `.worktrees/residual-state-read/.agent-work/worker/residual-state-read-rootcause.md` (subject-state read) and `.agent-work/worker/residual-busy-occasion-rootcause.md` (the fanout keeps the 16 lowest identifiers of 25; the first irreversible choice is in `shared.rs` `bounded_graph_visible_refs`, so a fix in policy alone is incomplete; equal-salience ordering needs compact metadata the `ParticipantOccasion` carrier lacks).
- Ingredient data today (research Q2): salience on every memory type; last touched; owed and due (`assertions`, `due_at`, `direction`, `DueState`); party present computable from assertion subjects against resolved participants; ever said only as an observation's speaker; felt tone none (ADR-D-0033: words, never a number); departure from expectation none. The last two and "ever said" for matters are `unknown` in v0.2.
- Records (research Q7): ADR-D-0022, ADR-D-0004, ADR-I-0006 accepted and contradicted; ADR-I-0036 and ADR-D-0039 proposed, returned by 4a (returning I-0036 leaves ADR-I-0022 as the live defaults record, and the share budget and the named-notion exemption become unrecorded, so the gathering record re-decides them); ADR-D-0023 strengthened; the intact list as 4a gives it. `depends_on` ADR-D-0022 in D-0023, D-0024, D-0026, D-0029, D-0039, I-0036; `src/policy/graph_expansion.rs` and `src/policy.rs` cite records in comments. Stale decay wording in ADR-D-0013 and ADR-D-0014 (overridden by ADR-D-0018) and ADR-I-0017's `last_reinforced_at` were found by the catalog and roadmap plan; they are left as history (Non-goals). Next free numbers: ADR-D-0041, ADR-D-0042, ADR-D-0043 (ADR-D-0040 is the catalog plan's), ADR-I-0037. Retirement procedure: `durable-docs-authoring/references/adr.md` (status, header line, move to `superseded/` with the suffix, inbound references repaired, absence search).
- Tests (research Q5): the replaced list (floors, standing, room) and the standing list with floor assertions to restate (`tests/retrieval_obligation_tests.rs`, `tests/retrieval_resolution_tests.rs`); lifecycle, scene, write-turn, notion, last-interaction, vector port, write planning, adapter, expansion and selectivity tests stand untouched.
- Measurement instrument (research Q6): the companion's pressure families, their topic-only and cue-removed controls, both identifier orders, repeat runs. The harness work (a situation-view reader replacing the floor sweep, the adapter off `pack`, the re-baseline re-anchored, the floor calibration task replaced, a deterministic years-sized generator with skewed hubs) is planned in the evaluation repository's situated plan amendment; this plan states what those measurements let it decide. The before arm is being captured at `af605e09` (code equal to b3aba0b2) by the evaluation worker.
- Renderer plan (superseded): what survives into Task_6 is the past tense for settled matters, labelled reminders, the oldest-first stream of what happened, the honest opening on references, the elapsed phrasing and the runnable example; WIP `ae319921` on `feature/2026-10-15/renderer-memo` is its starting material.
- Rulings carried, from `.agent-work/orchestrator/v0-2-load-bearing-decisions.md`: 71 (the as-of cut), 73 (UTC when no offset), 85 (LoCoMo as of its final timestamp), 86 (the owner through Counsel).

## Integration
- Branch `feature/2026-10-16/v0-2-situated-recall-plan` from the renderer plan tip `af605e09` (the whole library stack plus the brief with 4a); one pull request per task stacked on it; the residuals plan's statistics-recovery branch joins by plain merge when it lands (different files).
- Wave order: records, contract and the batched read first; gathering and the walk second; memo and example, docs last. The measurement's before arm is at `af605e09`; the after arm runs at Task_4's tip and again at the final tip as the slice-end run.
- After this plan: the evaluation repository's re-baseline at the final tip, then the closeout reading (brief section 3), then the paid runs (section 5).

## Open Questions (max 3)
- With the owner, through Counsel (2026-10-16): does a turn's recall itself tell the character a thing is verifiably absent, or only "nothing found" with the certainty coming from a separate exhaustive check? (Counsel: the snapshot part is below the owner's level; the brief's clause now reads "as of the moment it looked, which it says".)
- With the owner, through Counsel (2026-10-16): when a situation about an unsettled matter or a topic is cut, must the character be able to reach the rest of that matter or topic itself, or is it enough that the turn says it was cut and the rest is reachable by person, place or thread? Held: the continuation for matter- and topic-anchored situations.

## Assumptions
- A1: Oxigraph 0.5.7 exposes no read snapshot across queries. Task_3 probes it and reports what the engine gives, as input to the verified-absence unit.
- A2: A situation anchored on a present person or an unsettled matter can be gathered from the existing selectors (state, obligations, links one hop, shared subject and thread) without a new stored shape. Checked by Task_4; a stored-shape need stops and reports.
- A3: The companion's pressure families can be read as situations by a reader over the new result without changing their stores or scenarios. Source: research Q6. Checked by the evaluation worker's instrument.

## Tasks

### Task_1: The records say what recall is now, and the old ones are replaced or returned
- type: docs
- owns: docs/decisions/design/ADR-D-0041-*.md, ADR-D-0042-*.md, ADR-D-0043-*.md (new, as the admission test admits); docs/decisions/implementation/ADR-I-0037-*.md (new); ADR-D-0023 (a proposed amendment section and its status line only); the retirement of ADR-D-0022, ADR-D-0004 and ADR-I-0006 by the repository's procedure (status, header line, move to docs/decisions/superseded/, inbound references in docs/decisions/** and in code comments under src/ repaired; README and roadmap references are Task_7's and are listed for it); ADR-I-0036 and ADR-D-0039 status lines (returned); the `depends_on` lines of D-0023, D-0024, D-0026, D-0029; docs/decisions/README.md index lines
- depends_on: []
- acceptance:
  - Propose an ADR if the admission test passes (`durable-docs-authoring/references/adr.md`) for each decision 4a names: situations by weight (replacing ADR-D-0022 and ADR-D-0004's sections), the bounded gathering (replacing ADR-I-0006's section cap, keeping its other bounds, and re-deciding the share-based participant budget and the named-notion exemption that returning ADR-I-0036 un-records), and complete structural access; each record carries one decision at 4a's altitude and names the philosophy statements it traces to.
  - ADR-D-0023 carries a proposed amendment with 4a's strengthening; ADR-I-0036 and ADR-D-0039 are marked returned with the brief's section 4a as the reason; the three contradicted records are retired by the procedure, with every inbound reference outside README and roadmap repaired and the README and roadmap references listed in the report for Task_7.
  - No accepted record's Decision text is edited other than the amendment sections, status, supersession and dependency lines; consulted entries are model names only.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: each new record states one decision at 4a's altitude and traces to the philosophy; the gathering record keeps ADR-I-0006's bounds and re-decides the share budget and the named-notion exemption openly; the retirement procedure is complete (absence search over docs/decisions and src/); the dependency graph is navigable; what each superseded record governed is governed afterwards"
  - kind: manual
    required: true
    owner: user
    detail: "The owner accepts or returns each proposed record, amendment and retirement (reserved acts, carried by Counsel)"

### Task_2: The result is situations under one bound, and nothing is reserved any more
- type: impl
- owns: src/api/types/retrieval.rs; src/memory.rs (the retrieve signature and docs only); src/usecases/retrieve.rs (the result assembly and the room mechanics: `build_pack`, `prompt_ready_sections`, `section_for_object`, `section_limit`, `select_with_cue_floors` and its three call sites, the road table's `reserves` and `contribution` columns, `final_score`'s use as the fill order); src/usecases/retrieve/state.rs (the seat rounds); tests/retrieval_obligation_tests.rs and tests/retrieval_resolution_tests.rs (floor and slot assertions only); the replaced tests listed in the research (deletion or restatement)
- depends_on: []
- acceptance:
  - `RetrievalContext` carries one bound and no `cue_floors` or `section_limits`; `RetrieveOutcome` carries `situations` (each: anchor, parts with the path each came by, reported ingredients with `unknown`, a cut marker with the walk's cursor type as continuation, which Task_3 fills), `absences` (nothing found with attempted paths; something thin with its kind) and the resemblance search's coverage, keeps `scene`, `activity`, `time_range`, `scene_references`, `rationale`, `trace`, and has no `pack`, `memory_scenes`, sections or floor admissions.
  - At this task's tip, the existing candidates fill situations grouped by their anchor, in one ranked order to the one bound, with no floor, seat round or section cap left anywhere, configurable or internal; the parts carry `direction`, `due_state` and `resolved_by` where the base's pack did; ingredients the base already computed are reported, the rest `unknown`.
  - The replaced tests are deleted or restated per the research; the standing obligation and resolution tests keep their behavioural claims with floor and slot assertions restated against situations; the touched-file suppression census and empty and non-empty parity tests for every adapter implementing a touched port are in the report.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; the suppression census; a witness that no retrieval test store selects more than the bound and none selects by section"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: the contract matches 4a's statements field by field; no shim keeps the pack, a floor, a seat or a cap alive under another name; restated tests assert the claim, not the mechanism; suppression census and adapter parity evidence present"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: control only, run twice. The evaluation worker runs the situated fixtures and the pressure families at this tip through the adapted reader; the handed-over candidates (before grouping) are compared with the base at af605e09; differences are listed and each is explained by a removed floor, seat or cap, or the task is reopened."

### Task_3: Every memory reachable by what it is can be walked in full
- type: impl
- owns: src/ports/graph_authority.rs (new walk methods, using the cursor type of src/api/types/retrieval.rs); new functions in src/adapters/oxigraph/sparql_selectors.rs and src/adapters/oxigraph/embedded.rs; src/test_support.rs (forwarding arms); src/memory.rs (the walk entry point); the cut marker's continuation in src/api/types/retrieval.rs (the one public cursor type is defined there by Task_2, since a public result returns it; the port in graph_authority.rs uses that type and defines no second); tests/structural_walk_tests.rs (new)
- depends_on: [Task_2]
- acceptance:
  - A public walk by person, place (scope key), thread, time range and identifier returns pages in a total order with a keyset cursor, says whether more follows, and over a store with writes between pages neither skips nor repeats a memory that existed before the walk began.
  - A walk run to the end over each key equals a full scan of the store for that key, at stores of 1, 100 and 2,000 memories, both identifier orders.
  - A1 is probed and the report states what read-snapshot semantics the engine gives, for the verified-absence unit.
  - The walk never degrades silently: a bounded or failed read surfaces as an error or an explicit incomplete page, never an empty one; every adapter implementing the port passes empty and non-empty parity tests for the new methods.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; the full-scan equality witness; the adapter parity tests"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: the cursor is keyset-based, the order is total, no page can be silently short, the double forwards every new method, the facade's docs state the guarantee"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement: the full-scan equality witness at 2,000 memories per key, timed, run twice (no before exists); falsified if any key's walk misses or repeats a memory, or if a 2,000-memory walk of any key takes more than 1 s on this machine."

### Task_4: A turn receives a few situations by weight, gathered whole within a bound, and knows what it does not have
- type: impl
- owns: src/usecases/retrieve.rs (candidate finding, gathering, ordering, absence states; not the result assembly Task_2 owns, now stable); src/usecases/retrieve/scene.rs (the topic and description hits as a condition, coverage reported); src/policy/graph_expansion.rs (incident-neighbour ordering before the fanout limits in both paths, fanout pruning as a reported cut, `ParticipantOccasion` gaining the compact salience the ordering needs); src/adapters/oxigraph/shared.rs (`bounded_graph_visible_refs` and the occasion metadata it hands to the ordering); `select_participant_occasions` and the ordering block of `select_state` (src/adapters/oxigraph/sparql_selectors.rs:565-600) and its callers' order arguments; src/usecases/retrieval_selectivity.rs (familiarity from share, frequency and recency); their tests
- depends_on: [Task_2]
- acceptance:
  - A situation anchored on a present person, a place, a thread or an unsettled matter gathers what rests on it through its own links one bounded hop and, through shared subject and thread, what bears on it that nobody linked; when a cue touches a familiar thing's history, that history is gathered within the bound; where the bound cut the gathering (fanout pruning included) the situation says so with the walk's cursor for its anchor as continuation.
  - Situations and their parts are ordered by one rule stated in the design record over the reported ingredients, with identifier as the last resort; the companion's busy-occasion family selects the same observations in both identifier orders. The rule is chosen by a bounded experiment: question, which of a lexicographic order and a stated combination reads the pressure families as a person would; evidence, the two orders' outputs on those families against hand-labelled situations in two families; effect, the chosen rule is written into the design record's amendment and shipped, deterministic and with no hidden formula.
  - A topic or description vector hit admits a situation only above the null distribution (the shipped threshold from the one-time live-embedder measurement, sample size and cost recorded; the suite pins the rule with the test-embedder null), is labelled as a match by wording where that is all that connects it, and never contributes to weight; the result reports the resemblance search's coverage.
  - A familiar person or place (read from share, frequency and recency) brings what is new or unresolved with it, measured against the recorded experience; one met once brings that occasion; a description brings a labelled reminder.
  - The two absence states are produced in the turn: nothing found names the paths attempted; something thin covers ambiguous names, names with little behind them (the share threshold recorded) and wording-only matches; the turn claims no verified absence (that state is its own unit, on the owner's answer).
  - A coverage policy is added only if the measurement shows the present person crowded out by a loud topic, and then as a named, documented policy in the design record (4a, provisional); the report states whether it was needed, with the numbers. Entity-neutrality check: no rule names an entity role or kind.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; both identifier orders on the busy-occasion fixture; the order-rule experiment's numbers; the live null measurement's sample size and cost, its artifact under .agent-work/worker/ (PROMOTE: a shipped default rests on it)"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D and Tier A: gathering follows the anchor's links and shared subject and thread and nothing else; the order rule is open and deterministic; similarity is a condition, never weight; the cut is reported wherever a bound dropped a candidate, in both expansion paths; absence states never come from a resemblance miss; entity-neutrality holds; no floor, seat or section returned under another name"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement (the brief's section 4a): the evaluation worker runs the deterministic situation view at this tip against the base at af605e09 on the pressure families (overlapping office and topic, two people with separate obligations, duplicated accounts, old high-salience matters, keyless scenes, empty queries), with topic-only and each-cue-removed controls, both identifier orders, twice; candidate recall, situation grouping (against hand-labelled situations in two families), coverage, unrelated admissions and rendered behaviour measured separately; cost at a years-sized synthetic store with skewed hubs. Falsified if the present person's situation is absent under a loud topic without a named policy, if any pressure case loses a candidate the base found that a person would have in mind, if the two identifier orders differ, or if the years-sized cost grows faster than linearly in the matched candidates."

### Task_5: A subject's long history of beliefs is read as fast as its promises
- type: impl
- owns: `select_subject_state` and `select_state` except its ordering block (src/adapters/oxigraph/sparql_selectors.rs:245-325 and 433-564, 595-602) and their tests; not lines 565-600 (Task_4's)
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
    detail: "Measurement: the timing witness at 2,000 beliefs, before and after, run twice (instrument); falsified if any row differs or the after time is above 1 s."

### Task_6: The situations are handed to a model in words, and an example carries a character through a few turns
- type: impl
- owns: src/api/types/retrieval/memo.rs (new) and its `RetrieveOutcome::memo` entry; examples/*.rs (new); Cargo.toml (the example's dev dependencies only); the README's example section (a named block; the rest is Task_7's)
- depends_on: [Task_4]
- acceptance:
  - The memo reads each situation as a person would hand it to an actor: the present, then each situation with its parts in words, how each came, what is owed and by whom in the right tense (past when settled), reminders labelled as resemblance, what the character does not have (the absence states), and where a gathering was cut; nothing is phrased as an instruction.
  - The example builds the embedded stores in a temporary directory, carries a character through a few turns, prints the memo each turn, runs twice to identical bytes, and removes its stores.
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
    detail: "Measurement: rendered behaviour is one of the five measures in Task_4's slice-end run, run twice; the owner's read of the fixture memos (through Counsel) is the falsifier for phrasing: any line that misleads an actor."

### Task_7: The README, the v0.2 phase draft and the roadmap state the new design and nothing of the old
- type: docs
- owns: README.md (except Task_6's example block); docs/roadmap/roadmap-phases/v0_2_scoped_continuity_reflection.md (design sections); docs/roadmap/development_roadmap.md (the v0.2 rows and section 2.16, and the familiarity rows the catalog plan left to this plan)
- depends_on: [Task_1, Task_2, Task_3, Task_4]
- acceptance:
  - Every line the research lists as stating the replaced design states situations by weight, the one bound, the walk, the two absence states, and familiarity as 4a states it; no floor, room, section or `pack.render` remains; the README and roadmap references to retired records that Task_1 listed are repaired.
  - The roadmap's familiarity rows place it where 4a does (v0.2's recall design), closing the item the catalog plan left open.
  - The phase draft's judging line names the closeout reading the catalog plan wrote (no second wording).
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A: against the brief's 4a and the proposed records; consumer-facing README reads purpose before backend; no hard-wrapped prose; nothing of the old design left; retired-record references repaired"
  - kind: manual
    required: true
    owner: user
    detail: "The owner's read of the README's recall section (with the records), through Counsel"

## Task Waves (explicit parallel dispatch sets)
- Wave 1: [Task_1, Task_2, Task_5]
- Wave 2: [Task_3, Task_4]
- Wave 3: [Task_6, Task_7]

Task_2 and Task_4 both touch `src/usecases/retrieve.rs`; the file is split by function as the owns lines state, Task_2 first. Task_4 and Task_5 both touch `sparql_selectors.rs` in different lines and different waves. Task_3 and Task_4 both touch `retrieval.rs` only at the cursor field Task_2 typed. The evaluation worker's before arm is at `af605e09`.

## Rollback / Safety
- Each task is one pull request on the stack and reverts in reverse order; nothing stored changes; the walk adds port methods and removes nothing. A revert of Task_2 restores the pack and the floors wholesale, so Task_2 is not merged alone.

## Research
- Researcher report: `.agent-work/researcher/v0-2-situated-recall-redesign-research.md` (2026-10-16), read whole, with the two root-cause reports named in Context and the superseded renderer plan. Unknowns it could not resolve are carried as bounded experiments inside the tasks that need them (the order rule and the coverage policy in Task_4; the snapshot in Task_3) and as the evaluation repository's synthetic large store.

## Progress Log (append-only)
- 2026-10-16: plan drafted under the v0.2 completion brief's section 4a, from revision `af605e09` (the brief with its amendment, the accepted Standing Approvals entry).

- 2026-10-16: plan-draft value audit dispatched at `1c380f3a`, text verbatim: "You are the value auditor. Read references/value-audit-mandate.md of the orchestration-harness skill and follow it exactly. Position: plan draft. Plan: docs/coding-agent/plans/active/v0-2-situated-recall-plan.md. Governing brief: docs/coding-agent/briefs/active/v0-2-completion-brief.md. Changes since: none."
- 2026-10-16: verdict as returned (comparison lines in the readings file). Position: plan draft. Documents read: the plan; the brief; the philosophy; its companion; common.md (pointer lines and Standing Approvals, the Counsel entry accepted and committed); readings after grading only; notes never opened. Product basis: philosophy (the owner also the product owner). Not audited: the engineering side (no document). Missing inputs: none. Value documents changed in range: none.
  - DoD 1 | brief | cited | 4a gives 1, 2, 3; philosophy 8.1 | -
  - DoD 2 | brief | cited | 4a gives 2; the resemblance constraint; 4a provisional under gives 1 (provisional) | -
  - DoD 3 (nothing found and thin in the turn; verified absence as a separate call) | brief | ask-now | 4a gives 5 "The states reach the character in the ordinary result" against the constraint "Bounded foreground recall" | the two statements pull apart and the plan resolved the conflict itself. Value question: when a character is asked in a turn about something it never lived, does that turn's recall itself tell it the thing is verifiably absent, or only "nothing found", the certainty coming when the application asks separately? direction
  - DoD 4 | brief | cited | 4a gives 4 and 3; philosophy 7.4; disposition as background is v0.3's, reported unknown, no conflict | -
  - DoD 5 | brief | cited | 4a structural-access constraint; philosophy 8.5 | -
  - DoD 6 | brief | cited | 4a means and told constraint | -
  - DoD 7 | brief | cited | 4a records paragraph; section 3 pass condition | -
  - DoD 8 | brief | cited | 4a measurement constraint | -
  - DoD 9 | brief | cited | section 4 | -
  - DoD 10 | brief | cited | philosophy 9; section 3 means | -
  - Planner-added 1 to 4, 7 to 9 | brief | cited | 4a gives 1, 2, 3; the constraint; section 4; standing tests | -
  - Planner-added 5 (verified absence in its own call, graph health, snapshot or "at the time of reading") | brief | ask-now | as DoD 3; and "at the time of reading" departs from the told "at a stated snapshot" | value question as DoD 3; and must verified absence name a point in the record it was checked against? direction
  - Planner-added 6 (live-embedder null as shipped threshold) | brief | ask-now | 4a resemblance constraint covers the intent | outward-facing (paid embedding calls) with no standing approval; the plan's quoted in-session authorization is not a standing approval; if given, it answers it and the item does not count against the plan under the Plan Gate. Value question: may the run spend on live embedding calls, bounded to one measurement, and does section 5 reach any paid provider call? | -
  - Planner-added 10 (order-rule experiment) | brief | inferred | extends 4a gives 1 and 2; philosophy 8.1; cheap to undo | direction
  - Planner-added 11, 13, 14 | internal mechanics | not audited | - | -
  - Planner-added 12 (amendment for D-0013, D-0014, I-0017) | scope expansion | ask-now | no brief statement names them | Value question: should accepted records whose wording a later one overrides be amended now, or left as history? | -
  - Non-goals | brief / internal | cited / not audited | 4a means; gives 2; told constraint; provisional under gives 3; sections 3, 4, 5; measurement constraint | -
  - A1 | brief | ask-now | 4a gives 5 "at a stated snapshot" | as Planner-added 5 | -
  - A2 | internal | not audited | - | -
  - A3 | brief | cited | 4a measurement constraint (tightens) | -
  - Task_1 | brief | cited | 4a records paragraph; carries Planner-added 12's ask-now | -
  - Task_2 | brief | cited | 4a gives 1, 2, 3, 5; 4a pass condition | -
  - Task_3 | brief | ask-now | the walk cited; verify_absence carries DoD 3's conflict | as DoD 3 direction
  - Task_4 | brief | ask-now | 4a gives 1 to 5 and the measurement cover the substance; provisional coverage policy | the live null call (Planner-added 6); the order rule direction | direction
  - Task_5 | brief | cited | section 4 | -
  - Task_6 | brief | cited | philosophy 9; 4a gives 5; the owner's read loosens nothing | -
  - Task_7 | brief | cited | section 3; 4a "What it replaces" | -
  - Decision Log 1 | brief | cited | 4a gives 2; provisional under gives 1; the share seed means; "the renderer as specified" | -
  - Decision Log 2 (live null authorized in session) | brief | ask-now | as Planner-added 6 | -
  - Decision Log 3 (review rulings) | brief | ask-now | the verify_absence ruling | as DoD 3 direction
  - Decision Log 4 (re-review minors) | brief / internal | ask-now | the decay amendment it adds | as Planner-added 12
  - Human-only conditions pending: the owner's blind reading at closeout (4a); the owner accepts the revised roadmap sections (section 3); the owner accepts the revised catalog (section 2). No proxy settles one.
  - Scenarios: none
- 2026-10-16: acting on the verdict. Planner-added 6 and Decision Log 2 are met by the owner's answer recorded in the Decision Log ("You can do the embedding call.", 2026-10-16, in this session). Planner-added 12 is the Orchestrator's own line and is dropped (Non-goals). DoD 3, Planner-added 5, A1, Task_3's verify_absence and Decision Log 3's ruling are one question for the owner, sent to Counsel (below); verified absence is carved out of this plan as its own unit on the answer, and the plan is audited again as a revision.

- 2026-10-16: plan-draft value audit dispatched at `6e6eea8c`, text verbatim: "You are the value auditor. Read references/value-audit-mandate.md of the orchestration-harness skill and follow it exactly. Position: plan draft. Plan: docs/coding-agent/plans/active/v0-2-situated-recall-plan.md. Governing brief: docs/coding-agent/briefs/active/v0-2-completion-brief.md. Changes since: none."
- 2026-10-16: verdict as returned (comparison lines in the readings file). Position: plan draft. Documents read: the plan, the brief, the philosophy, its companion, common.md (pointer lines and Standing Approvals); readings after grading only; notes never opened. Product basis: philosophy. Not audited: the engineering side. Missing inputs: none. Value documents changed in range: none.
  - DoD 1, 2, 4 to 10 | brief | cited | 4a gives 1 to 5, means, told constraint, measurement constraint, records paragraph; section 3 and 4; philosophy 8.1, 8.5, 9; DoD 4's omitted disposition noted as v0.3's, reported unknown | -
  - DoD 3 (two states in the turn; verified absence held) | brief | ask-now | 4a gives 5 against the bounded-recall constraint; 4a pass condition names three states | the plan delivers two of three with the third pending the owner's answer. Value question as sent to Counsel (in the turn or on a separate check; a named snapshot) | direction
  - Planner-added 1, 3, 4, 7, 8 | brief | cited | 4a gives 1, 2, 3; the constraint; standing and replaced lists | -
  - Planner-added 2 (cut = count plus the walk's cursor for the anchor) | brief | inferred | extends 4a gives 3; a situation anchored on a matter or a topic has no walk key that resumes its gathering, so for those anchors the continuation does not reach what the bound dropped; cheap to undo | -
  - Planner-added 5 (live-embedder null shipped) | brief | ask-now | 4a resemblance constraint covers the intent | outward-facing with no standing approval; the plan's quoted authorization is the Orchestrator's writing, not support; the only reason, so it does not count against the Plan Gate; the call waits on the authorization the Orchestrator holds from the owner. Value question: may the run spend once on live embedding calls before the reading | -
  - Planner-added 6, 10, 11, 12 | internal mechanics | not audited | - | -
  - Planner-added 9 (order-rule experiment) | brief | inferred | extends 4a gives 1 and 2; philosophy 8.1; cheap to undo | direction
  - Non-goals | brief / internal | cited (verified absence ask-now as DoD 3; harness placement not audited) | 4a means, gives 2, told constraint, provisional under gives 3; sections 2 to 5; measurement constraint; the records paragraph closes the list, so leaving D-0013, D-0014, I-0017 is no departure | -
  - Compatibility stance: break | brief | cited | 4a "What it replaces" | -
  - A1, A2 | internal mechanics | not audited | - | A1 informs the held question
  - A3 | brief | cited | 4a measurement constraint | -
  - Task_1, 2, 3, 5, 6, 7 | brief | cited | 4a records paragraph, gives 1 to 5, structural-access constraint, section 3 and 4, philosophy 8.1, 8.4, 8.5, 9 | -
  - Task_4 | brief | ask-now | 4a gives 1 to 5, the resemblance constraint, the provisional coverage policy (provisional) | carries Planner-added 5's spend only, not counted against the Plan Gate; the order rule as Planner-added 9 | direction
  - Decision Log 1 | brief | cited | 4a gives 2, provisional under gives 1, the share seed means, records paragraph | -
  - Decision Log 2 | brief | ask-now | as Planner-added 5 | -
  - Decision Log 3 | brief / internal | ask-now (the shipped-threshold ruling only, as Planner-added 5); the rest cited or not audited | -
  - Decision Log 4 | internal | not audited | - | the decay minor withdrawn by Decision Log 5
  - Decision Log 5 (carve-out, question to Counsel) | brief | ask-now | 4a gives 5 and the bounded-recall constraint | the question is open; the Open Questions section read "None" while it was outstanding | direction
  - Human-only conditions pending: the owner's blind reading at closeout (4a); the owner accepts the revised roadmap sections (section 3). Record acceptances are reserved acts through Task_1.
  - Scenarios: none
- 2026-10-16: acting on the verdict. The plan waits for the owner's answer on verified absence (Counsel: part 1 with the owner; part 2 below the owner's level, the brief's clause reworded "as of the moment it looked, which it says", committed at 93929a11). The audit's "Findings compared" named a departure as bearing on the design and not recorded: the continuation reaches nothing for a matter- or topic-anchored situation; recorded in the readings file, escalated to Counsel as a value question, and the continuation for those anchors held. Open Questions now name both. Inferred calls journaled: the continuation as the walk's cursor (with its gap); the order-rule experiment (direction). The plan is authorized only when an audit on the owner's answers holds nothing.

## Decision Log (append-only; re-plans and major discoveries)
- 2026-10-16 Decision: the plan is one plan with seven tasks, contract before gathering, the walk beside the gathering, the memo and docs after.
  - Trigger / new insight: the owner's amendment (4a) replaces the recall selection and room, the result contract and the renderer as specified, and leaves the storage, time, obligation, resolution, lifecycle, scene, write-turn and notion work standing. The research maps each area.
  - Plan delta: new plan; supersedes the renderer plan and takes two residuals (the subject-state read, the busy-occasion order) from the residuals plan with their root causes. The busy-occasion key the residuals plan proposed (current, salient, newest, id) is not carried: ordering is Task_4's open rule, and identifier-order independence is its acceptance.
  - Rulings in this plan: felt tone, departure from expectation and "ever said" for matters are reported `unknown` in v0.2 (no data; ADR-D-0033 forbids a number for feeling); a coverage policy enters only on the measurement, as 4a's provisional line says; the share budget and the named-notion exemption are re-decided in the implementation record rather than left unrecorded by returning ADR-I-0036.
  - User approval: the brief's amendment, the owner's ratified words; authorization under the Plan Gate's second source after the plan review and the plan-draft value audit.
  - Record proposed: three new, one amendment, two returns, three retirements (Task_1).
- 2026-10-16 Decision: the owner authorized the live-embedder null measurement: "You can do the embedding call." (in this session, 2026-10-16). Task_4 measures it once, bounded, outside the suite, records sample size and cost, and ships that threshold as the default; the Open Question is closed.
- 2026-10-16 Decision: the plan review of f9364c29 (CHANGES REQUESTED) applied, with these rulings.
  - Task_2's same-set control was contradictory with removing the room mechanics and is dropped; Task_2 fills situations by anchor to the one bound with the existing candidates, and the measurement's control arm is the base at `af605e09`. Its measurement item explains every candidate difference by a removed floor, seat or cap.
  - (Superseded by the entry below.) Verified absence was read as a separate call outside the turn.
  - A cut's continuation is the walk's cursor for the situation's anchor, not a second resumable surface.
  - "History reached whole the moment anything touches it" is read as: when a cue touches a familiar thing's history, the situation gathers it within the bound and the cursor reaches the rest; the walk is the application's route to the whole independent of a cue. Resemblance coverage is reported in the result.
  - The shipped resemblance threshold is the live measurement's; the test-embedder null only pins the rule in the suite.
  - Owns widened from the root causes' scope sections: Task_4 gains `shared.rs` and `select_participant_occasions`; Task_5 gains `select_state` except its ordering block; Task_1 owns the full retirement procedure outside README and roadmap and a proposed amendment to ADR-D-0023 (an accepted record's text changes only by a proposed amendment the owner accepts).
  - The health that verified absence needs is the graph read's, not the statistics store's.
  - Lesson candidates taken: owns are checked against a root cause's scope section; a planner-added invariant is checked against every acceptance bullet of its task (lessons.md).
- 2026-10-16 Decision: the re-review of 8e77735d APPROVED with three minors, applied: the ADR-D-0013, ADR-D-0014 and ADR-I-0017 amendment listed as planner-added; one public cursor type, defined in `api/types/retrieval.rs` by Task_2; the live null measurement's artifact kept and promoted.
- 2026-10-16 Decision: the plan-draft value audit (at 1c380f3a) returned four value questions. The live-embedder spend is met by the owner's in-session authorization already logged. The decay-wording amendment is dropped as scope expansion. Verified absence is carved out: its two parts (inside the turn or by a separate exhaustive check; a named snapshot or "as of when I looked") are one value question sent to Counsel for the owner on 2026-10-16: "When a character is asked in a turn about something it never lived, does that turn's recall itself tell it the thing is verifiably absent, so it can give the flat no that the philosophy reserves for verified absence, or does the turn tell it only 'nothing found' (with what was tried) and the certainty come only when the application asks for it separately, by an exhaustive check whose cost grows with the store? ... must verified absence name a point in the record it was checked against, or is 'as of when I looked' acceptable, since the embedded graph engine gives no read snapshot across queries?" The part held: verified absence (its state in the result, any exhaustive check, the snapshot statement). Everything else continues. The answer, when it arrives as Counsel's relay, is recorded here and the unit is planned by amendment.
  - User approval: the brief's amendment; authorization under the Plan Gate's second source after the new audit.
  - Record proposed: none new.
