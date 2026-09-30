# Plan: the retrieval result becomes the notes handed to the model, and an example carries a character through a few turns

- status: in-progress
- generated: 2026-09-23
- last_updated: 2026-09-23
- work_type: code

## Goal
- An application that has a retrieval result can hand a language model what a person would write on a card for an actor before a scene: who is here and when you last met them, which promises and open matters stand and which way each runs, what fell due, what happened, and what you hold. Each memory comes with how it came to mind and the scene it was recorded in. How it came keeps knowing apart from being reminded, so a resemblance to a stranger never reads as knowing someone and an anniversary never reads as a date asked about. The notes come from the result alone, in one call, with no model, no clock, no store read and no judgment about what may be said. That judgment stays with the model (ADR-D-0038). A runnable example in the library shows a character writing a few experiences, meeting someone again, retrieving for the new scene and printing those notes, deterministically and offline, so the README's worked example is real code.

## Definition of Done
- The result gains one public projection: `RetrieveOutcome::memo(&self) -> Memo`. `Memo` is one exported type whose only public surface is `Display`. Its structure (sections, entries) is crate-internal. It is authoritative inside the crate and asserted by unit tests (ADR-I-0029). An application that wants another format formats the retrieval result, which is already the public structure.
- The memo carries every admitted memory of every section of the pack exactly once, in the section reading order the Design fixes.
  - "What happened" is one combined stream of the pack's episodes and observations:
    - Before sorting, the stream is the pack's episodes in pack order followed by its observations in pack order.
    - It is then stably sorted by recorded time, oldest first, so cause comes before effect. Items with no available time come after the dated ones, keeping that concatenation order. Equal times also keep it, so an episode precedes an observation at the same time.
    - An episode's time is its scene time. An observation's time is its `observed_at`; failing that, the scene time of the `SourceScene::Recorded` in its own `memory_scenes` entry whose `episode_id` equals its `episode_id`; failing that, it has no time. Creation time and the retrieval time are never used.
    - The stream is built in the memo's construction, not in its display.
  - Every other section stays per section, in the order the pack already has.
  - Nothing is dropped, folded, truncated or deduplicated by the memo (ADR-D-0038, ADR-D-0018).
- Each entry carries its own text and the result's typed facts, reused and joined by object reference. The text is an episode's summary, an observation's text, a thread's title and summary labelled in progress, or an interpreted memory's text. The facts are `admitted_by`, `resolved_by`, `direction`, `due_state`, `seconds_since_support` and `sources`. The display prints:
  - settled, when `resolved_by` is non-empty;
  - you owe this, for `OwedByCharacter`; this is owed to you, for `OwedToCharacter`; and nothing about owing when `direction` is `None`;
  - overdue, due today or not yet due;
  - last supported an elapsed time ago;
  - the recorded source scenes, or that a source is forgotten or missing.

  No score, rank, cue kind, rationale, telemetry, trace or id appears in the text. Who a promise concerns is told by the memory's own text.
- Each entry says how it came to mind, phrased from its `admitted_by` set and nothing else, with one phrase per road present, in the enum's order. Each phrase is true for everything that road can bring:
  - `Participant`: connected to someone present, through an occasion shared with them or something held about them. This is true for Bob in a matter between Bob and Alice.
  - `Place`: reminded by a context it was formed in, a setting or a context value. It never says where the character is now.
  - `Activity`: connected to what you are doing.
  - `Topic`: recalled through what is being talked about, possibly through something connected to it.
  - `PersonDescription`: reminded by a resemblance to how someone here was described; that may be a stranger.
  - `SettingWords`: reminded by a resemblance to these surroundings; it may have been somewhere else.
  - `Range`: from, or resting on an occasion within, the dates asked about.
  - `Anniversary`: from, or resting on, an occasion on this day in an earlier year.
  - `Recency`: from, or resting on, something lately.
  - `Due`: came with a promise or open matter that fell due.

  The words belong to the memo's display, the one place they live. The retrieval result and its README give meanings and no phrasing. Unit tests assert the typed set, never the words. An entry whose `admitted_by` is empty says nothing about how it came.
- The memo infers no standing of its own. It never derives how a memory came from its section, position, source scene or score. It never derives whom a memory concerns, or that the character owes or is owed, from anything but `direction`. Headings name kinds, never a state or a relation to anyone present: "promises and open matters" (commitments and open loops), "ongoing threads", "what happened", and "what you hold" (relationship notes, preferences, character signals, derived memories).
- The memo opens with the present as given: the scene's time and setting, then each reference with its resolution, in the honest words the rulings fixed (rulings 6, 10, 11; ADR-D-0029):
  - a key or a name that resolved is known, and says when you last met, from `LastInteraction.seconds_since`, or never;
  - an ambiguous name could be any of several people you know;
  - an unknown key is nobody you know recorded under that key;
  - an unknown name is nobody you know by that name, and a participant can carry both an unknown key and an unknown name;
  - a participant description is taken as a reminder of someone like that, and as nobody in particular;
  - setting words are taken as a reminder of similar surroundings.

  The activity, if given, is echoed as found or unknown. A part not given is not mentioned, and the memo never says the scene is complete.
- The elapsed phrasing is one function from whole seconds to a phrase in the largest whole unit (just now under a minute, then minutes, hours, days, years). It is used for last met and for the age of support, and it is the only wording the library adds to a time.
- The memo addresses the character as you. It never names the self's id, and never adds a purpose, a name the scene or the pack did not give, an identity for a description, a disclosure verdict, a summary, or any sentence the result does not carry a typed fact for (ADR-D-0023, ADR-D-0029, ADR-D-0038, ADR-I-0013).
- A runnable example under the library's own examples uses the public facade on the embedded stores, with scenes and fixed instants, and the self's notion id given at construction. It:
  - writes a few experiences, including a commitment with a direction and a due instant, an open loop the other party later settles, and a scene Bob was not in;
  - retrieves for a new scene with Bob present and no topic, then with the topic of the field guide, then with the topic of the draft, and prints the memo for each;
  - writes the scripted reply as a new experience.

  It carries its own small deterministic embedding provider, reads no environment and no network, prints the same bytes on every run, and prints no ids. Its stores live in a temporary directory that is removed after the facade is closed, on success and on an ordinary error, because the facade is held in an outer async scope that awaits close before removing the directory. What each memo contains is recorded as a baseline observation and never made an acceptance: the script is never reshaped so a memo reads well.
- The README shows the example's code and its printed memo as its worked example, and describes what the memo contains, what it never does and how to run the example.
- A retrieval selects, orders and scores exactly what it selects at the parent commit. The memo is a pure function of the result and touches no retrieval code.
- The slice-end measurement numbers exist, each with a paragraph on what they mean for the character and a verdict, in this plan's Decision Log. The plan is not complete until they do.

## Planner-added requirements
- One public displayable type, with the structure crate-internal. Needed because: ADR-I-0029 says tests assert variants and fields and never message text, so the structure must exist and be tested. A second public structure would mirror the result an application can already format itself (Tier A 9).
- The memo is a projection of the whole result, not of the pack. Needed because: the pack holds no scene references, no last interactions, no memory scenes and no `admitted_by`.
- One canonical rendering and no style value. Needed because: no consumer reads a style.
- Entries carry the result's own typed facts, reused, and the display is the one place their words live. Needed because: ADR-I-0029 forbids a fact carried only in prose and a message rendered in two places.
- How a memory came is phrased from `admitted_by` alone, with wording true for everything the road can bring. Needed because: rulings 60, 62 and 72 exist so that the last step does not turn a resemblance, a run of recent occasions or an anniversary into plain history. A phrase stronger than what its road proves (co-location for a context value, a match for a topic descendant) is the same false continuity in smaller type (Tier D R1).
- The memo infers no standing. Direction is printed only from `direction`, and headings name kinds. Needed because: "connected to someone present" is true for Bob in a matter between Bob and Alice, while "you owe" is not (Tier D R2, Tier A 6).
- The memo keeps the pack's order within every section except "what happened", and prints no score. Needed because: the pack is already ranked and the score lives only in the trace (ruling 32). "What happened" is one stream, oldest first, ruled from behavior: cause before effect, in one telling. The time plan deferred chronological presentation to the renderer.
- The example constructs the facade through the public constructor, with settings from explicit overrides and its own deterministic embedding provider. Needed because: the crate's test support is compiled only for tests, and a README example must not depend on a test feature.
- Fixed instants and caller-supplied ids in the example. Needed because: the README's evidence and the companion's reading instrument both need the same bytes on every run.

## Scope / Non-goals
- Scope: the memo module and its unit tests, the example, the README, and this plan. No decision record: the memo is ADR-I-0029's projection rule applied to the retrieval result, and its phrasing is what ADR-D-0038 and ruling 72 leave to the consumer.
- Non-goals:
  - any call to a model, any clock read, or any store read;
  - naming who a promise concerns beyond its own text;
  - public entry types;
  - a style value;
  - a token budget, a length cap or any truncation;
  - any ordering beyond the one chronological sort;
  - omission counts and reasons;
  - marking a superseded memory the caller included (ruling 42's known gap);
  - naming which reference brought a memory (ruling 62's smallest form);
  - rendering the rationale, telemetry or trace;
  - a template engine dependency;
  - a hedging or register vocabulary for observations;
  - a prompt wrapper or an instruction to the model;
  - a response or remember step driven by a model;
  - cleanup of the example's directory on panic;
  - the evaluation harness's own context renderer;
  - the short-term store, consolidation, reflection, and anything for v0.3.

## Design
- Chosen, the memo: `RetrieveOutcome::memo(&self) -> Memo`, an inherent method in `src/api/types/memo.rs` (no edit to `retrieval.rs`), with `impl Display for Memo`.
  - Structure, crate-internal:
    - `Memo { scene, activity, references, sections }`, where `references` holds the result's `SceneReferenceResult` values as they are;
    - `sections` is a fixed-order list of `MemoSection { heading, entries: Vec<MemoEntry> }`, where what happened is the one stream built as the Definition of Done states;
    - `MemoEntry { memory: MemoryObjectRef, text: String, admitted_by: BTreeSet<AdmissionRoad>, resolved_by: Vec<MemoryId>, direction: Option<ObligationDirection>, due_state: Option<DueState>, seconds_since_support: Option<i64>, sources: Vec<SourceScene> }`, where every field but `text` is the result's own value, reused.
  - Display: the present, then the four headings. Under each entry come its text, its facts, how it came, and its recorded scenes. The phrase for each `AdmissionRoad` value is one arm of one exhaustive match, so a new road cannot be added without a phrase. Settled is printed as settled, and the resolver's ids stay in the structure. An empty section is not printed, and an empty memo says nothing comes to mind after the present.
  - Construction: bounded by the result's size; small temporary indexes by object reference are allowed.
  - Evolution: a new admission road adds one match arm.
  - Verification: unit tests in the memo module, using fixtures built through the facade on the embedded stores. They assert on the structure: typed membership of every admitted memory, each entry's `admitted_by` equal to the result's, the stream's order, and the reused facts. The one text check is scoped: within a section's display body, each entry's own text appears once and in entry order, which is data, not wording.
  - Human: the text reads as a card handed to an actor.
  - Safety: nothing is withheld and nothing is added.
- Alternative: public `MemoSection` and `MemoEntry`. Rejected (Tier A 9): a second public structure mirroring the result.
- Alternative: `MemoParty`, naming a promise's party by the scene's words. Rejected by the coordinator: direction plus the memory's own text suffice. Joining parties to scene references was the memo's one piece of identity logic.
- Alternative: a text-only renderer; a method on the pack; a `Standing` enum restating typed facts; the memo deriving knowing or reminded itself; one resemblance phrase for both description roads; settled naming the resolver's entry by position; a template engine; a style value; printing scores or cue kinds. All rejected, as recorded in the Decision Log.
- Alternative: heading obligations by a state ("what stands open", "what stands between you"). Rejected: a heading that names a state or a relation tells the model something the result does not type. "Promises and open matters" names the kind.
- Chosen, the example: `examples/a_few_turns.rs`, run with `cargo run --example a_few_turns`, and discovered by Cargo without a manifest edit.
  - Setup: `main` creates a `tempfile::TempDir` and builds settings from explicit overrides: embedded vectors under the directory, in-memory graph and statistics, and no environment read. It opens the facade through the public constructor with the self's notion id and a deterministic embedding provider: a bag of hashed tokens implementing the public embedding trait.
  - Structure: `main` awaits the script as an inner async function borrowing the facade. Whatever the script returns, `main` then awaits `close()`, calls `TempDir::close()` and reports a deletion error. Removal is guaranteed only on paths that reach that close; nothing is claimed for a panic, because dropping the facade does not wait for the embedded store's shutdown (`src/memory.rs:43`).
  - Script, over four fixed instants:
    - Day one: the character meets Bob at the workshop. Bob asks for the field guide, and the character writes the episode and a commitment to bring it, with the character as actor and Bob as counterpart, due on day three. Bob says he will send his draft: an open loop with Bob as actor and the character as counterpart.
    - Day two: Bob sends the draft, written as an episode that resolves the open loop through the public link path.
    - Day two, evening: an episode with Alice at the cafe, a scene Bob was not in.
    - Day four: the character meets Bob at the workshop again.
  - Retrievals: three, with Bob present by key and the workshop's words: with no topic, with the topic of the field guide, and with the topic of the draft. The memo is printed after each. The scripted reply is then written as day four's episode, and nothing more is printed.
  - Baseline reading: the reading of each memo is a baseline observation, recorded in the report and never an acceptance. It covers:
    - what Bob's presence brings;
    - whether the overdue commitment arrives, and how it says it came;
    - whether the settled open loop arrives, or arrives as the day-two episode that settled it (ruling 42);
    - whether the cafe episode comes, and how it says it came.

    The script is never reshaped so a memo reads well.
- Alternative: the example as an ignored test; or printing the ids the reply's write returned. Rejected, as recorded before.
- Alternative: claim cleanup on panic through `TempDir`'s drop. Rejected (Tier D R3): the backend's drop only signals shutdown and does not wait for it, so the directory can still be held.
- Cost of the stream, stated: under a topic, "what happened" loses the pack's relevance order, and the strongest match can sit mid-stream. The model still sees how each entry came, but not which was strongest. This is the price of cause before effect, and the decider may veto the sort.
- Measurement. It runs once, at slice completion (orchestrator rule). Neither task changes retrieval.
  - Instrument: the companion evaluation repository's generated runner. The evals worker prints `memo()` for every retrieval of the existing families (scene overlap with described strangers and unfamiliar places, keyless, familiar person, time with range and anniversary, shared interpretation, keyed setting, activity pressure) and of the prospective slice's obligations family, and compares native results at the base and the tip. No new family is needed. The printed memos and the example's are filed beside the companion's D4 and D1 narrative scenarios for the decider's read.
  - R1 to R3 are controls:
    - R1: any retrieval result differs between the base and the tip in ids, sections, order, scores or `admitted_by`.
    - R2: in Task_1's unit tests, any admitted memory or reference is absent from its memo or doubled; or an entry is out of pack order outside the stream; or the stream breaks its concatenation, time or undated rule.
    - R3: in Task_1's unit tests, any entry's `admitted_by` differs from the result's.

    R2 and R3 run in the library's unit tests because the entries are crate-internal.
  - R4 is the falsifier. The decider reads the memos of the described-stranger, keyless, anniversary, range, keyed-setting and obligations stories and the example's, including these witnesses:
    - a memory reached only by a context value while in a different setting;
    - a topic-only graph descendant;
    - a matter between Bob and Alice with Bob present;
    - a settled matter.

    The design is falsified if any line does any of the following: presents a resemblance as knowing someone; presents a context value as the current place; presents a topic descendant as itself matching; presents an anniversary as a date asked about; presents a recent occasion as something asked for; implies that the character owes or is owed when direction is `None`; or reads a settled matter as open.
  - The item is `kind: command`, `owner: orchestrator`, naming the evals worker as the runner.
- Why chosen: this is the smallest thing a consumer would otherwise write wrongly: settled from `resolved_by`; owed which way, and only when typed; due which way; seconds into words; a description as a reminder and not a person; how each memory came, without turning a reminder into knowing; and the recorded scene beside each memory. It is one projection with one display and one example that makes the README true. Fit: philosophy 7.2 and 9.3; the phase draft section 2.4 and section 9; ADR-D-0018, D-0020, D-0023, D-0029, D-0038, ADR-I-0013, I-0029; rulings 6, 10, 11, 32, 42, 52, 60, 62, 72.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface: the retrieval result gains one method, and the crate exports one type, `Memo`, whose public surface is `Display`. The change also adds a new example target and a README section. No stored shape, no constructor and no existing type changes.
- stance: preserve
- justification: no existing surface changes. The companion evaluation repository reads the native result and may print the memo in its own plan.

## Context (workspace)
- Related files and areas, at the consolidation's final tip ef8dbfb; the prospective slice adds its fields on top:
  - `src/api/types/retrieval.rs`:
    - `RetrieveOutcome` at 245;
    - `MemoryScenes`, with `admitted_by` and `seconds_since_support`, at 260;
    - `AdmissionRoad` at 277;
    - `SourceScene` at 291;
    - `SceneReference`, which distinguishes `ParticipantDescription` and `SettingWords`, at 311;
    - `SceneReferenceResult` at 319;
    - `LastInteraction` at 328;
    - `SceneReferenceResolution`, with `Unknown` described separately for a key and a name and with `Reminder`, at 337;
    - `ContinuityContextPack` at 389;
    - `IncludedDerivedMemory` at 418;
    - `ContextPackSection` at 826.
  - `src/domain.rs`: `Episode` at 299, `Observation` at 325 (with `episode_id` at 328 and `observed_at` at 330), `MemoryThread` at 369, `DerivedMemory` at 395.
  - `src/domain/scene.rs`: the scene, its participants and its setting. `scope_keys` at 100 combines setting keys and custom values.
  - `src/memory.rs`: the drop note at 43; `retrieve` at 125.
  - `src/adapters/qdrant_edge/mod.rs`: drop only signals shutdown, at 321; `close` waits for it, at 145.
  - `src/composition.rs`: the public constructor taking an embedder, at 132.
  - `tests/support/base.rs`: the override-built embedded settings and `close_and_remove_root`, which the example follows.
  - `Cargo.toml`: `tempfile` at 41.
  - `README.md`: the admission-road table at 55-67; the road table at 77-90.
- Neighbour plans: the prospective plan, which provides `direction`, `due_state`, `AdmissionRoad::Due` with its one-hop reach, and the constructor taking the self. The consolidation plan (completed), which provides `admitted_by`.
- Rulings: the rulings log, items 6, 10, 11, 20, 31, 32, 42, 52, 60, 62, 72, 73, 75, 76.
- Reviews: `.agent-work/reviewer/v0-2-renderer-plan-review.md` (Tier D R1 to R4, 2026-09-23) and the Tier A findings 6 and 9.
- Design records consulted, and deviations from their acceptance: ADR-D-0018, D-0020, D-0022, D-0023, D-0028, D-0029, D-0038, ADR-I-0013, I-0029. There is one deviation from the phase draft's illustrative shape: the projection is on the result rather than the pack, and it takes no style value. The draft calls that shape illustrative, so no record is touched.

## Integration
- The library stack runs through the consolidation slice (which includes the write-path warnings), then the prospective plan, then this plan, the last library plan of the phase.
- Base: the prospective plan's last task tip, once it is up as a pull request, pinned in the dispatch brief. This plan touches no file the earlier plans rewrite except the README.
- Inherited and not redone here: every reported field the memo reads. This plan adds one projection and one example.
- Controls that must not change: every retrieval test's expectation, and the whole result at the parent commit and at each tip for the example's own store and scenes.

## Open Questions (max 3)
- None. The chronological order of "what happened" was ruled from behavior, and its cost under a topic is stated in the Design. The decider may veto it at the slice boundary, and the veto is one deleted sort.

## Assumptions
- A1: The base carries `IncludedDerivedMemory.direction: Option<ObligationDirection>`, `due_state: Option<DueState>`, `AdmissionRoad::Due`, and the constructors taking the self, with the names the prospective plan pins. Source: that plan. A precondition of dispatch.
- A2: An admitted memory's `admitted_by` may be empty. Source: the consolidation's Task_4 acceptance does not say non-empty. Checked by Task_1; the display says nothing for an empty set.
- A3: Settings can be built from explicit overrides reading no environment, and `tempfile` is usable from an example. Source: `tests/support/base.rs` and `Cargo.toml:41`. Checked by Task_2.
- A4: The companion evaluation repository can print `memo()` for the results its generated runner already produces. A note for the handoff.

## Tasks

### Task_1: The result says how things stand and how each memory came, in the words a person would hand an actor
- type: impl
- effort: 8 worker-hours
- owns:
  - src/api/types/memo.rs
  - src/api/types/memo/tests.rs
  - src/api/types.rs
  - src/lib.rs
  - src/test_support.rs (only to add a missing fixture helper; test code only)
  - README.md
- depends_on: []
- description: |
  Base: the prospective plan's last tip (see Integration).

  Build the memo as the Design fixes it: `RetrieveOutcome::memo(&self) -> Memo` and `impl Display for Memo`, in one new module, with only `Memo` exported and its structure crate-internal. The memo holds:
  - Every admitted memory of every section, exactly once, in pack order within its section, except the one "what happened" stream. The stream is built from the pack's episodes in pack order followed by its observations in pack order, then stably sorted by recorded time. An observation's time is its `observed_at`, else the matching recorded `SourceScene`'s scene time, else none.
  - For each entry, its own text and the result's facts, reused. A thread is labelled in progress.
  - The present, in the Definition of Done's words: `ParticipantDescription` and `SettingWords` are distinct, and an unknown key and an unknown name are worded apart.
  - The elapsed phrase, as one crate-internal function.

  The display phrases each `AdmissionRoad` value in one exhaustive match, with the meanings the Definition of Done lists. It says nothing about owing when `direction` is `None`. It uses the four kind headings and infers no standing.

  The README gains a section. It says what the memo contains, and what it never does: withhold, truncate, add, infer how a memory came or whom it concerns, or reorder anything except the one stream. It says the memo's words for each admission road are the memo's own, that an application wanting another format formats the result, and that the model decides what to say.
- acceptance:
  - Unit tests on a store built through the facade on the real embedded stores. The store holds:
    - the character and Bob, as notions with known-as beliefs;
    - a commitment with the character as actor and Bob as counterpart, due before the scene's day;
    - an open loop with Bob as actor that a later memory resolves;
    - an open loop between Bob and Alice, with the character as neither;
    - a relationship note last supported long before the scene;
    - an episode with Alice that Bob was not in;
    - an occasion reached only by a person description;
    - a memory reached only by a custom context value, formed in a different setting from the scene's;
    - a memory reached only as a topic-only graph descendant;
    - an occasion from last year on this day;
    - recent occasions, with nothing said;
    - a range.

    Retrievals use Bob present by key and by a name that could mean two people, a setting in words and a participant given only by description, plus retrievals with a range, with a topic and with nothing said. The memo shows:
    - Bob as known and last met; the ambiguous name as several people; the person description as someone like that, taken as nobody in particular; the setting words as similar surroundings.
    - The commitment with `direction` `OwedByCharacter`, `due_state` `Overdue`, and `admitted_by` including `Due`.
    - The resolved open loop, when admitted, with its `resolved_by` ids.
    - The Bob–Alice open loop with `direction` `None` and `admitted_by` including `Participant`. Its display says nothing about owing.
    - The relationship note with its whole seconds.
    - The Alice episode with its recorded scene with Alice and without Bob.
    - Every entry's `admitted_by` equal to the result's, including these exact sets: `PersonDescription` for the description-only occasion; `Place` for the custom-value memory; `Topic` for the topic descendant; `Anniversary`, `Range` and `Recency` for those memories.
    - Every other section in pack order; every admitted memory once as a typed entry; no score or id in the text.
  - Stream order cases, asserted on typed entry order:
    - Mixed episodes and observations: an observation whose `observed_at` lies between two episodes' scene times and differs from its own parent's time sits between those episodes.
    - An observation with no `observed_at` takes its parent's recorded scene time.
    - An observation whose parent is forgotten and which has no `observed_at` is undated and comes after the dated items.
    - Equal times keep the concatenation order, episode before observation.
  - The one text assertion is scoped to a section's body. No test asserts the memo's wording.
  - The elapsed function is tested at the unit boundaries (59 seconds, 60 seconds, an hour, a day, a year) on its numeric or enum-shaped result.
  - Retrieving the store at the parent commit and at this tip gives the same result, twice, in both id orders.
  - The report lists every field the memo reads and records whether any admitted memory had an empty `admitted_by` (A2).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the parent-equality run recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, tracing every field the memo reads, the stream's concatenation and time source, that each entry's admitted_by is the result's and nothing else decides how a memory came or whether anything is owed, that only Memo is public, that no retrieval code is touched, and that the display is the one place the words live; Tier A altitude review against the philosophy (7.2 knowing versus being reminded; 9.3 gist and stance), rulings 60, 62 and 72, each road phrase checked against everything its road can bring, ADR-D-0038 (nothing withheld, no verdict), ADR-D-0023 and ADR-D-0029 (no purpose, no identity from a description) and ADR-I-0029 (structure authoritative, tests on fields)"

### Task_2: A character carries its memory through a few turns, in code anyone can run
- type: impl
- effort: 5 worker-hours
- owns:
  - examples/a_few_turns.rs
  - README.md
- depends_on: [Task_1]
- description: |
  Write `examples/a_few_turns.rs` as the Design describes:
  - `main` owns the `TempDir` and the facade, which is opened with the self's notion id and the example's own provider (about thirty lines in the same file).
  - `main` awaits the script as an inner async function, then, whatever it returned, awaits `close()`, calls `TempDir::close()` and reports any deletion error.
  - The script uses caller-supplied ids, four fixed instants, the experiences, the three retrievals with each memo printed, and the scripted reply written as day four's episode. It prints no ids.

  Record in the report, as a baseline observation, what each memo contains: what Bob's presence brings; whether the overdue commitment arrives and how it says it came; whether the settled open loop arrives as itself marked settled, or as the day-two episode that settled it; and whether the cafe episode arrives and how it says it came. None of it is an acceptance, and the script is not reshaped to change it. A memo that reads badly is filed as a finding for the retrieval side.

  The README replaces its code fragments with the example's code and the memos it prints. It states how to run the example, says the reply is scripted because the library never calls a model, and says the directory is removed once the facade is closed.
- acceptance:
  - `cargo run --example a_few_turns` runs offline with no environment set, prints the three memos, exits zero and leaves no directory behind. Two consecutive runs print identical bytes, shown by a diff in the report.
  - When the script is made to return an ordinary error midway (one check in the report), the facade is closed and the directory removed.
  - The report holds the three memos as printed and the baseline observation of each, with no claim that any memo was made to look a certain way.
  - The example's embedding provider is in the example file and implements the public embedding trait. Nothing under the crate's sources or tests changed for it, and the manifest is unchanged.
  - `cargo test` builds the example, and every existing test passes unchanged.
  - The README's worked example is the example's code and output, and a reader can run it from the README alone.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; cargo run --example a_few_turns twice with the outputs diffed; the ordinary-error cleanup check"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, running the example from the README's instructions and confirming no environment or network read, close awaited before the directory is removed on success and on an ordinary error, no panic-cleanup claim, no printed ids, and that the script was not reshaped for the memos; Tier A altitude review against the phase draft section 2.4 (the write-path friction a consumer feels), the philosophy's warning against anthropomorphic claims in the README text, and ADR-D-0038 (the memo shows Bob what he was not there for and withholds nothing)"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Slice-end measurement, run by the evals worker at this task's tip: native results of the scene overlap, keyless, familiar person, time, shared interpretation, keyed setting, activity pressure and obligations families, before at the base and after, identifiers opposed to time, both orders, twice, with memo() printed for every after-retrieval; the printed memos and the example's filed beside the companion's D4 and D1 narrative scenarios. Controls: R1 any retrieval result differs in ids, sections, order, scores or admitted_by; R2 and R3 are Task_1's unit tests (membership, order and stream rules; admitted_by equality), rerun at this tip. Falsifier: R4, the decider's read of the described-stranger, keyless, anniversary, range, keyed-setting and obligations memos and the example's, with the witnesses (a context-value match in a different setting, a topic-only descendant, a Bob-Alice matter with Bob present, a settled matter): any line that presents a resemblance as knowing someone, a context value as the current place, a topic descendant as itself matching, an anniversary as a date asked about, a recent occasion as something asked for, implies the character owes or is owed when direction is None, or reads a settled matter as open. The numbers, their reading against the philosophy and a verdict go into this plan's Decision Log"

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]

The two tasks share one crate, so they run in sequence, each as one pull request stacked on the previous one; the first sits on the prospective plan's last tip, as Integration states. After Task_2, the library commit is handed to the companion evaluation repository. Its own plan decides whether its context renderer adopts the memo, and whether a narrative reading of rendered text becomes a check.

## Rollback / Safety
- Each task is one pull request and reverts on its own, in reverse order. Nothing is stored. A revert of Task_1 removes a method and a module, and a revert of Task_2 removes an example and a README section. No coordination with stored data is needed.

## Progress Log (append-only)

- (none yet)

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-23 Decision: the renderer is a structured memo on the result with one display, and the example is a fixed script on the embedded stores with the embedder moved under the test-fixtures feature.
  - Trigger / new insight: the phase draft asks for a canonical rendering and an example loop. ADR-I-0029 says prose is a projection derived by the owning type, and tests never assert message text. The pack alone cannot say who is here or where a memory was recorded. The test embedders are compiled only for tests and exist twice. The companion repository holds no rendering check.
  - Plan delta (what changed): new plan.
    - The projection is `RetrieveOutcome::memo`, returning a `Memo` with a closed `Standing` vocabulary and reusing the result's reference and source-scene types.
    - The display is the one place the words live.
    - The sections follow a fixed reading order (what stands between you, what happened, what you hold), with the pack's order inside each.
    - One elapsed function works in whole units.
    - There is no style value, no template engine, no truncation and no score.
    - The example is `examples/a_few_turns.rs`, with fixed instants and caller-supplied ids, cleaned up on exit.
    - The embedder is feature-gated and shared.
  - Tradeoffs considered:
    - Text-only rendering was rejected as untestable under ADR-I-0029 and as leaving the standing to be recomputed by applications.
    - Rendering the pack was rejected because the present and the recorded scenes are on the result.
    - A style value was rejected as a type with no reader.
    - Scores in the memo were rejected as trace-only facts an actor's card would not carry.
    - Chronological reordering was left as an open question rather than a second ordering.
    - The example as an ignored test was rejected because the draft asks for the library's own examples.
  - User approval: plan approval waived for plans inside the rulings; decisions judged by character behavior are logged for presentation; the chronological question is raised for the decider at the slice boundary.
  - Record proposed: none.
- 2026-09-23 Decision: the draft was revised after its Tier A review, before dispatch, under eight rulings.
  - Trigger / new insight:
    - The example's expected memo had been written as an acceptance, which invites reshaping the script until it reads well.
    - Settled needed its references as structure, and honest wording when the resolver is not in the memo.
    - The description wording implied a failed match.
    - Moving the tests' embedder crossed a trait boundary and tied a README example to a test feature.
    - Cleanup was not panic-safe, and printed ids could vary.
    - The chronological question could be ruled from behavior.
    - A counterpart had no words in the memo.
    - Threads and omission counts were unstated.
  - Plan delta (what changed):
    - The example's reading is a baseline observation recorded in the report. A third retrieval, on the draft's topic, is added, and "we settled it" may arrive as the day-two episode.
    - `Standing::Settled { by: Vec<MemoryObjectRef> }` is displayed with the settling memory's text only when it is in the memo.
    - A description is "taken as a reminder of similar surroundings; taken as nobody in particular".
    - The example carries its own thirty-line embedding provider. Task_2 owns only the example, the manifest and the README, and the feature assumption is deleted.
    - Cleanup is by `tempfile::TempDir`, and ids are printed only when every minted id is supplied or derived.
    - "What happened" reads oldest first by recorded scene time, sorted at construction. Every other section keeps pack order, and the open question is closed.
    - A party whose notion a reference resolved to is called by the scene's words.
    - Threads are labelled in progress, and omission counts stay in the rationale.
  - Tradeoffs considered: keeping pack order everywhere was rejected, because a person tells what happened as cause before effect and the time plan sent chronology here. The decider may veto the sort at the slice boundary, and the veto is one deleted sort.
  - User approval: ruled by the coordinator under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: "what happened" is one combined stream and a settled matter names its resolver's entry (Tier D review R1 and R2, ruled).
  - Trigger / new insight: two section lists sorted separately would tell an episode and its observations apart in time. A settled entry that repeated its resolver's text would print one memory twice and break a once-only text check.
  - Plan delta (what changed):
    - The stream is one `Vec<MemoEntry>` of episodes and observations by recorded time, oldest first, undated after dated in pack order, built in `memo()`. The other sections stay per section in pack order. The measurement line and the README requirement name the stream as the one reorder.
    - Settled carries object references. The display says settled by and names the resolver's entry when it is present, never its text, and says settled otherwise.
    - Tests assert typed membership and a scoped body order check, with no once-only text across the memo.
  - Tradeoffs considered: repeating the resolver's text for readability was rejected as a second carrier of one memory.
  - User approval: ruled by the coordinator under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: rebased on the consolidation and prospective slices. Each memory says how it came, phrased from `admitted_by` (rulings 60, 62, 72), and the memo infers no standing.
  - Trigger / new insight:
    - The consolidation slice added `MemoryScenes.admitted_by` as `AdmissionRoad`, and its Integration requires this plan to read it in place of any standing it infers.
    - The earlier memo printed every occasion as plain history, and it headed obligations "what stands between you".
    - The prospective slice adds `AdmissionRoad::Due`.
    - `ContentCue` is now `Reminder`, and the age field is `seconds_since_support`.
  - Plan delta (what changed):
    - Each entry carries `admitted_by`, phrased once per road in one exhaustive match. An empty set says nothing.
    - Headings are neutral.
    - Tests assert the typed set.
    - The base is the prospective plan's last tip, and the fixtures cover every road.
    - Two per-task measurements become one slice-end measurement.
  - Tradeoffs considered: inferring standing from sections or sources was rejected. One resemblance phrase for both description roads was rejected by ruling 72. Naming the reference stays out.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: value audit of every task and planner-added requirement, applied.
  - Task_1: EARNS-ITS-PLACE. Its "record by hand" baseline: DELETE.
  - Task_2: EARNS-ITS-PLACE. Its baseline step and conditional id printing: DELETE. The panic check: DELETE.
  - Per-task measurements: OVERSIZED.
  - The `Standing` enum: OVERSIZED.
  - Settled naming the resolver's entry by position: OVERSIZED.
  - Party words: EARNS-ITS-PLACE.
  - The stream: EARNS-ITS-PLACE.
  - How each memory came: EARNS-ITS-PLACE.
  - No inferred standing: EARNS-ITS-PLACE.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: revised after Tier D (R1 to R4) and Tier A (6, 9), under the coordinator's rulings. This supersedes the two entries above where they keep party words, a state heading, public entries, a "matches" topic phrase, a co-location place phrase, TempDir-on-drop cleanup, or an unspecified stream time source.
  - Trigger / new insight:
    - R1: `Place` can be a custom value formed in another setting, and `Topic` can reach a descendant that never matched. The phrases claimed more than the roads prove.
    - R2 and Tier A 6: a Bob–Alice matter with Bob present is truly connected to someone present, yet the plan forbade saying so while the direction is `None`.
    - Tier A 9: public entry types mirror the result.
    - R3: `TempDir`'s drop does not wait for the embedded store's shutdown, so panic cleanup was unsupported.
    - R4: no observation case could tell `observed_at` from the parent time, and the pre-sort order across two pack vectors was undefined.
  - Plan delta (what changed):
    - `MemoParty` is deleted; direction and the memory's own text suffice.
    - The obligations heading names the kind ("promises and open matters"), and threads get "ongoing threads".
    - Every road phrase is made true for everything that road can bring. `Place` is reminded by a context it was formed in, never where you are now. `Topic` is recalled through what is being talked about, possibly through something connected. `Activity`, the time roads and `Due` gain "or resting on". Witnesses are added: a custom-only match in a different setting, and a topic-only descendant.
    - When direction is `None`, nothing implies owing, and `Participant` still reads as connected to someone present. Acceptance and R4 are aligned.
    - `memo()` returns one exported displayable type, and the entries are crate-internal with unit tests (`tests/memo_tests.rs` becomes `src/api/types/memo/tests.rs`).
    - Cleanup is bounded to paths that await close before removing the directory. The facade sits in an outer async scope, so ordinary errors close it too; an ordinary-error check replaces the panic claim.
    - The stream's pre-sort order is defined (episodes in pack order, then observations in pack order, then a stable sort), and the observation time source is named. Mixed, fallback, undated and tie cases are added.
    - R4 adds "a settled matter read as open". R1 to R3 are named controls and R4 the falsifier, with R2 and R3 in unit tests because the entries are internal.
    - The stream's cost under a topic is stated.
    - The example scene is "a scene Bob was not in".
    - Small cleanups: "one pass, no allocation" is dropped; the opening keeps `ParticipantDescription` apart from `SettingWords`, and an unknown key apart from an unknown name; the stance is `preserve`; the manifest is dropped from Task_2's owns (Cargo discovers the example).
  - Tradeoffs considered:
    - Reading trace scores to phrase a topic root differently from a descendant was rejected: the memo reads no trace, and the weaker phrase is true for both.
    - Panic-safe cleanup through a blocking shutdown was rejected as a library refactor outside this plan.
  - Value audit of this round's additions:
    - the witness cases: EARNS-ITS-PLACE (typed equality cannot catch a false sentence);
    - the stream cases: EARNS-ITS-PLACE (the plan's one reorder);
    - the ordinary-error cleanup check: EARNS-ITS-PLACE (one check);
    - "ongoing threads" as a fifth heading: EARNS-ITS-PLACE (threads are not promises).
  - Lesson: when a revision deletes a mechanism, grep the earlier draft for the records it carried, and carry each forward or name it. Here, deleting `MemoParty` removed the only place the scene's words for a party reached the memo; that is named as dropped, with direction and the memory's text carrying the fact.
  - User approval: ruled by the coordinator; logged for presentation.
  - Record proposed: none.

## Notes
- Risks:
  - The memo's wording is read by a model, and a phrase that reads as an instruction rather than a note would steer it. The display states facts and never imperatives, and the Tier A review reads every line as an actor would.
  - An application that included suppressed or superseded memories sees them in the memo unmarked, because the result does not mark them with the trace off (ruling 42). The README says so.
  - The elapsed phrase in the largest whole unit says one year for anything between one and two years. The README says so.
  - A memory reached by several roads prints several ways it came. That is honest, and it reads longer.
  - Under a topic, the stream puts the strongest match wherever its time falls.
  - A panic in the example can leave its temporary directory behind.
- Edge cases, with the expected result:
  - A scene with only a time prints the time, then the sections.
  - A name that resolved to nobody prints that you know nobody by that name. An unknown key prints that nobody you know is recorded under it.
  - A resolved participant never met prints never met.
  - A memory whose source is forgotten prints that its source is forgotten, and keeps the memory.
  - An obligation with no direction and no due instant prints its text, how it came, and nothing about owing.
  - A matter between Bob and Alice with Bob present prints as connected to someone present, and nothing about owing.
  - A memory admitted through two roads appears once, with both ways it came.
  - A memory with an empty `admitted_by` prints no way it came.
  - A settled matter prints settled.
  - Two items at the same recorded time keep their concatenation order, episode first.
  - An observation's time comes from its `observed_at`, else from its parent's recorded scene. One with neither comes after the dated items.
  - A memo of an empty store prints the present and that nothing comes to mind.
  - The example run twice prints identical bytes.
