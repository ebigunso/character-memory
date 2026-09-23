# Plan: recall room is divided by five principles, and every memory says which cues admitted it

- status: in-progress
- generated: 2026-09-23
- last_updated: 2026-09-23
- work_type: code

## Goal
- How recall room is divided can be explained in five sentences, and the code has one place for each of them. Room is reserved only for what someone gave. Each road contributes as much as its match can vouch for. A range the caller gives becomes the window recency reads, so the two do not compete. Once reservations are served, the order is score with ties to the newest, and turns among kinds exist only at root selection. What a road opens follows the road. A place the scene names by key brings what was formed there, as a reminder. It does not push out what the character knows about the people in the room, and a constant key such as "home" does not put the same belief in every pack. Every admitted memory reports, with the trace off, the cue kinds that admitted it, so the memo can say "you asked about this", "about someone present", "matched the topic", "reminded by the place" or "from lately", and never prints a reminder as plain history. The special cases this replaces are deleted, the dead code the audits found is removed, and the records say what is true. The prospective and renderer slices then build on explicit rules, not on this log.

## Definition of Done
- One road table in the code decides everything a root's road decides. Every root carries the roads that reached it. Its reported cue kinds, whether it expands (opens history or is a leaf), whether it may hold reserved room and what it contributes are all read from that table, never set separately in each constructor. The README prints the table.
- Principle 1: room is reserved only for what someone gave or what a stored intention was waiting for. Reserved room is served from the head of that road's own order. The recency floor stays at a default of zero: a caller who sets it has given that reservation. An anniversary shared with nobody present holds no reserved room.
- Principle 2: a road contributes as much as its match can vouch for. Exact roads (keys, names, a caller range, dates, recency) contribute up to the room. A description contributes up to its floor, as today, until a similarity bound is measured on a production embedder. The topic contributes up to the room. A road's score is the larger of zero and its similarity.
- Principle 3: a caller range replaces "now" as recency's window instead of competing with it. The date-match tie priority is deleted, and so is the dependence on the order roots are pushed.
- Principle 4: after reservations, one order at every stage: the stage's score, with ties going to the newest; at root selection, where search roads give no score, salience comes first, as the section will judge it (ruling 65). Turns among kinds exist only at root selection, and only for roads that expand. Rounds among the scopes of one given kind (several people the scene names) stay at every stage, and they run per kind.
- Principle 5: what a root opens follows the road. A participant key or name, the activity and the topic open history at their own strength. A description, a setting key, a custom value, recency, a range and an anniversary are leaves: they bring what rests on their own occasion, what was observed and concluded there, and open no history through a notion or thread (rulings 46 and 54).
- A memory reached by a setting key or a custom value is a reminder. It has a cue component of zero and is a leaf. It holds the place floor, takes only room nothing else claims, and takes no turn. The place road offers its memories newest first and records no state scope.
- Every admitted memory's `MemoryScenes` entry carries the cue kinds that admitted it (`admitted_by`), with the trace off.
- The audits' hygiene items are done with no change to what comes to mind, apart from what the Compatibility stance lists. A scene time reads back with the offset it was given, which is one stored-time correction. A same-id write with the same instant and a different offset is a new collision rejection. Each is shown failing before the fix and passing after, in its own task. The domain field `scene_local_date` is gone. One shared test support module replaces the repeated helpers and embedders.
- ADR-I-0036 (proposed) labels the activity floor as provisional and no longer describes the path from content hits to entity roots. A new one-decision record is drafted as proposed: a setting key or custom value brings what was formed there, as a reminder.
- The slice-end measurement numbers exist, each with a paragraph on what it means for the character and a verdict, in this plan's Decision Log. The plan is not complete until they do.

## Planner-added requirements
- The place road (setting key and custom values) offers its memories newest first by memory time, then id, not by salience (ruled 2026-09-23). The score decides among what it contributed. Needed because: a floor served from the head of a salience order would put the same top-salience home belief in every pack whenever a constant home key is given. Every other reminding road already offers its occasions newest first.
- A setting key or a custom value records no state scope, and the rounds among state scopes run per kind at every stage, as `scopes_for_kind` already does at roots. Needed because: `order_section_state` takes every state scope together, so the office key and the activity each get a round equal to a person's. The fix for six people would then only half-land.
- A kind's reserved room is served from its roads' own orders, with roads that expand before leaf roads, never sorted by the member's own standing. Needed because: today the helper sorts every kind's queue full-standing members first and exempts the time kinds, which is a special case. Ruling 54 requires a time reservation to go to the latest occasions even when the topic also found an older one.
- A description takes no spare turn at root selection. Needed because: principle 4 allows turns among kinds only to roads that expand. Today a description root is in its kind's queue and can take that kind's spare turn (ruled a defect, 2026-09-23).
- The recency floor stays, at a default of zero (ruled 2026-09-23). Needed because: a caller who sets it has given that reservation, which principle 1 allows. It is already built and measured (ruling 55), and it reserves nothing at zero.
- A road's score is the larger of zero and its similarity (ruled 2026-09-23, R3). Needed because: the real embedded vector adapter returns negative cosines (the Tier D probe returned -1), and a cue-less root scores zero. Without a stated boundary, a negative search hit and a time root would compare by accident.
- Root order is ONE lexicographic key with a total final tie-break (ruled 2026-09-23, Tier A re-check):
  1. Score, descending. A road's score is the larger of zero and its similarity.
  2. Among equal scores, salience descending. This is the section's own standing measure: `RankedObject.salience_component` (`retrieve.rs:856` at d36d96d), set by `salience_component()` from the object's stored `salience_score` (`retrieve.rs:1795`) and weighted at 0.10 in `final_score` (`retrieve.rs:886`). Salience, like score, is compared with `f32::total_cmp`.
  3. Timestamp present before timestamp absent.
  4. Timestamp, descending.
  5. The row of the root's best road in the road table, the earliest row among the roads that reached it. The row order is given roads, then search roads, then time roads, which reproduces the parent's given-then-search-then-time order.
  6. The root's position in that road's own list: recency, the range, the anniversary and the place newest first by memory time; the topic and descriptions in their search order.
  7. Id.

  A root carries a salience and a timestamp exactly when its score is zero and a time or place road reached it. Its selector returns the stored `salience_score` and the memory time with each id, in the same query. Every other root carries neither: its salience counts as zero and its timestamp is absent. A clamped topic hit that recency also reached therefore carries both. A root with a positive score carries neither, so an equal positive overlap keeps the parent's order. The anniversary's "shared first" order applies only when reserved room is served (ruling 61(3)). Its own list is newest first.

  Needed because: principle 3 removes push order as the tie-break, and at root selection a recency occasion, an unshared anniversary and a clamped search hit all score zero. Salience comes before time because the root stage orders what carries no score the way the section will judge it (ruling 65). The time slice measurement at d36d96d showed that ordering zero-score roots by time alone cuts a salience-1.0 unshared anniversary at the 12-root cap in a 367-day daily store: twelve recency roots fill every seat, so the section never weighs it.
- Memory time is one clock. For an episode it is its recorded scene time. For an observation it is `observed_at`, else its episode's scene time. For any other memory it is its creation time. The rank key, the time selectors and the place selector all use this definition. The selectors compute it with one shared SPARQL expression and return it with each id, so place and recency roots compare on the same clock, and root selection and sections agree. Needed because: a place ordered by creation time and recency ordered by scene time would compare two clocks at root ties and could disagree with the section order.
- Parity retained and changes intended (R3, narrowed). Parity is claimed only on the parity fixtures: no range, no anniversary, no description, no reserved recency, and no negative or zero search score at the parent. On those fixtures root selection equals the parent's in both identifier orders. Section parity also needs a fixture where every admitted member's final score exceeds 0.25 plus 0.10 times the cue-less occasion's salience, as the time plan limited it; nonnegative similarity alone does not prove it (`retrieve.rs:883-886`). The intended changes are listed here, and nothing in them is contradicted elsewhere:
  - Zero-score roots of different salience can reorder at the root stage when the cap binds: a salient unshared anniversary takes a root seat ahead of ordinary recent days (ruling 65).
  - A clamped zero-score search hit orders after timestamped roots of equal salience. For example, at a root cap of two with a topic floor of one, two zero-score topic roots and a recent time root, the second slot goes to the time root.
  - Among equal-score search hits, the road row comes before id, replacing the parent's type then id.
  - The anniversary contributes up to the room.
  - Descriptions take no spare turn.
  - A kind's reserved room is served knowing roads first, by road.
  - Places are reminders ordered by memory time (Task_2).
  - The hint link is removed (Task_3).
- Reporting is the kinds that admitted a memory, not a known-or-reminded bit (ruled 2026-09-23; this refines ruling 60 and answers ask 20). Needed because: a binary is wrong both ways. A 0.05 topic hit would read as known, and an occasion the caller asked for by range would read as resemblance. The kinds are already computed as `ranked.cue_kinds`, so the field costs no new read and no parallel list.
- Removing the remember hint is its own behavior task. Needed because: it moves an observation one hop further from a present keyed participant and lowers that participant's Mentions-to-observation selectivity count, so it changes scores.

## Scope / Non-goals
- Scope: the production modules, named test files and documents each task lists under owns; the proposed revision of ADR-I-0036; one new proposed design record; this plan.
- Non-goals:
  - The phase correctness fixes (vector identity, whole-store hydration at `shared.rs:145`, unbounded retrieval reads), which are on the base branch.
  - A similarity bound for descriptions, and any production-embedder measurement.
  - Due dates and triggers (the prospective slice adds them to the road table).
  - How the memo phrases each cue kind (the renderer slice consumes `admitted_by`).
  - Floor values and their calibration.
  - The replacement of ADR-I-0022 (closeout).
  - Replacing ADR-D-0024, whose invariant stays true.
  - A relevance threshold for the topic.
  - Migrating stored data.
  - A new cue kind or a new road.
  - Moving the write turn into the facade (considered and deleted by value audit; Decision Log, R4).
  - These audit findings, left for a later cleanup:
    - the empty-search duplication at `qdrant/store.rs:277` and `qdrant_edge/mod.rs:233`;
    - the `scene_pool` rename at `ports/vector_candidate.rs:13`;
    - the truncation notes at `candidate_record.rs:94` and `tie_closure.rs:102,114`;
    - the `correct()` embedding input at `vector_indexing.rs:86-131`;
    - the serde of `scene_cue_searches` at `retrieval.rs:555,928`.

## Design
- Chosen, one road table. Every root holds the set of roads that reached it. What it reports, whether it expands, the kinds it may reserve under and its contribution budget come from this table. The table is one match in `retrieve.rs`, and the README copies it. This is the final table; Task_1 keeps the place row as it is today (a key opens history at a score of one), and Task_2 changes it:

  | road | floor kind | admitted by (reported) | expands | reserves | contributes |
  | --- | --- | --- | --- | --- | --- |
  | participant key or name | participant | participant | opens history | yes | the notion, expanded |
  | setting key, custom value | place | place | leaf | yes | up to the root cap, newest first by memory time |
  | activity | activity | activity | opens history | yes | the thread's members |
  | topic | topic | topic | opens history | yes | up to the candidate cap |
  | participant description | participant | person description | leaf | yes | the larger of one and the participant floor |
  | setting words | place | setting words | leaf | yes | the larger of one and the place floor |
  | range | date match | range | leaf | yes | up to the room, plus one to mark that more exists |
  | anniversary shared with someone present | date match | anniversary | leaf | yes | up to the room |
  | anniversary shared with nobody | date match | anniversary | leaf | no | up to the room |
  | recency (no range given) | recency | recency | leaf | only what the caller's recency floor sets (default zero) | up to the room |

  The row order is part of the root key: given roads (participant, place key, activity), then search roads (topic, descriptions), then time roads. "The room" is today's time budget, the largest section cap in force. The prospective slice adds its due and trigger rows.
- Principle 1, reserved room. It replaces:
  - the per-root `date_match_floor_eligible` flag on `CandidateRoot` and `RankedObject`;
  - `floor_kinds` (`retrieve.rs:945`);
  - the helper's queue sort `!matches!(kind, Recency | DateMatch) && reminder` (`retrieve.rs:993-1001`).

  Each root carries the kinds it may reserve under, read from the table. Each kind's queue is its roads' own orders, with roads that expand first. Deleted: the flag in both structs and both merges, `floor_kinds`, and the time-kind exemption.
- Principle 2, what a road contributes. It replaces three budgets computed at the call sites in `retrieve.rs` (the time budget at 204-208, the anniversary's larger of one and its floor at 252, and the per-key root cap at 148-151) with the table's contribution column. It also clamps every search score to the larger of zero and the similarity. Changed: the anniversary rises to the room (ruled 2026-09-23). This supersedes ruling 56's clause bounding it. It is safe because an unshared anniversary carries no score and loses ties to the newest, and the slice falsifier must hold.
- Principle 3, the range as recency's window. It replaces the two `query_episodes_by_time` calls (the range at 209-239, recency at 279-301) with one call. Its window is the range when one is given, and otherwise ends at the scene's reference time. What it contributes is reported as date match when a range is given and as recency when not. Deleted:
  - the date-match-first key and the whole tie permutation (`retrieve.rs:790-826`);
  - ruling 56's date-match-before-recency rule for unclaimed room;
  - the dependence on pushing the range, then the anniversary, then recency.
- Principle 4, one order. Rank key at sections: score, then memory time descending, then type, then id. Memory time is the one clock defined in Planner-added requirements. At root selection the roots are sorted once by the single lexicographic root key in Planner-added requirements: score, salience, timestamp present, timestamp descending, best road's table row, position in that road's list, id. After reservations, spare turns among kinds go to members from roads that expand, and only when at least two kinds have such members. The remaining room is filled in the one order. At the candidate merge, the order is the search score, then type and id. Rounds among the scopes of one given kind stay at every stage, per kind: `order_state` at roots through `scopes_for_kind`, and `order_section_state` at sections changed to do the same. Deleted:
  - the source-based chain in `select_candidate_roots` (`retrieve.rs:1424-1446`);
  - every `matches!(kind, Recency | DateMatch)` in the helper (`retrieve.rs:995`, `1010`, `1022`);
  - description spare turns.
- Principle 5, what a root opens follows the road. It replaces the pair `full_standing_score` and `full_standing_kinds`, which each constructor sets by hand (participant at `retrieve.rs:109`, activity at `activity.rs:63`, place at `retrieve.rs:172`, `from_vector` at `retrieve.rs:1353`), with the table's expands column. Kept exactly: the merge rules of rulings 47 and 54. A memory keeps its best score. What is reached through it inherits only from roots that expand. A leaf road sets proximity only where no expanding road reached. The leaf rule applies at an interpreted memory.
- Place and custom values (ruling 59). A setting-key or custom-value root has a cue component of zero and is a leaf. It holds the place floor from the head of the place road's order, which is newest first. It takes no turn and records no state scope. A memory also found by the topic keeps the topic's score and expands through it. Ruling 28 is settled the same way. The README says that a project is given as the activity, not as a custom value.
- Reporting (ruling 60 as refined 2026-09-23). `MemoryScenes`, which already holds one entry per admitted memory, gains `admitted_by`: the cue kinds of the admitted ranked object, passed from pack building to `memory_scenes` with no read. It is filled with the trace on or off. The name avoids a clash with the renderer's `Standing` enum. The renderer plan must consume this field and phrase each kind itself.
- Alternative: keep the time-kind exemptions and add place as a third. Rejected by ruling 61: that grows the set of special cases.
- Alternative: a keyed place keeps full standing but takes no turn. Rejected by ruling 59: at a score of one it would still outscore every person's state and open history it does not know.
- Alternative: the place road keeps its salience order. Rejected by ruling (2026-09-23): the constant-home falsifier would hold by construction.
- Alternative: remove rounds among people at sections. Rejected by ruling (2026-09-23): principle 4 concerns turns among kinds, and rounds among the scopes of one given kind measured well.
- Alternative: a known-or-reminded bit. Rejected by ruling (2026-09-23): it is wrong both ways, and the admitting kinds already exist.
- Alternative: replace ADR-D-0024. Rejected by ruling (2026-09-23): its invariant stays true. A new one-decision record states what a place key brings.
- Measurement. It runs once, at slice completion (orchestrator rule, Measurement Of Behavior Changes). If a family regresses, the cause is found by re-measuring at the intermediate tips, which exist as commits: Task_1 for the principles, Task_2 for places, Task_3 for the hint link. Instrument: the companion evaluation repository's generated runner. The existing families are scene overlap, keyless, familiar person, time and shared interpretation. Two families are new and are built before Task_1 is dispatched, so before-numbers exist at the base:
  - Keyed setting: a constant home key on every memory; six keyed people present with an office key and a topic; retrievals across many distinct topics at home; an unlived topic at home.
  - Activity pressure: an activity whose thread holds many members, beside a topic.

  Each run compares before (the base tip) with after (the slice tip) on the same inputs, identifiers opposed to time, both identifier orders, run twice. Any one of the following falsifies the design:
  - Keyed setting: a person present brings fewer of their state memories with the office key present than without it; the same top-salience home belief appears in every pack across the distinct topics; or the share of the pack taken by an unlived topic rises above its recorded number.
  - Activity pressure: the topic keeps fewer on-topic memories with the activity present than alone, beyond the activity floor's reservation.
  - Principle 3: asked about last Tuesday with the range set, any memory from today is reported as recency or contributed by the time read.
  - Existing families: on-topic survival under same-place pressure falls below the scene-words after-numbers; the keyless probe brings fewer of the latest occasions of equal salience (the keyless family's salience distribution is recorded before the base numbers are taken); the familiar person's state shrinks; an ordinary unshared anniversary displaces a recent occasion of equal salience; the reminder leak reopens in the shared-interpretation family.

  The validator has no measurement kind, so the item is `kind: command`, `owner: orchestrator`, naming the evals worker as the runner.
- Why chosen: ruling 61's principles, made into the one table the code and the README share. The special cases become rows, and the next slice adds rows rather than exceptions. Fit: philosophy on knowing versus being reminded (section 7.2), presence is not aboutness, and a character arriving with what the room calls for; ADR-D-0018, D-0022, D-0023, D-0024, D-0029, D-0038, ADR-I-0036.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface:
  - `MemoryScenes` gains `admitted_by`.
  - `SceneReferenceResolution::ContentCue` becomes `Reminder`.
  - `RetrievalContext.scene` loses its serde default, so the scene is required in JSON.
  - The lifecycle fields and filter reasons that only ever hold one value are removed (`api/types/retrieval.rs:713-739`).
  - The stored episode gains the given offset as its own value (seconds east of UTC) beside the unchanged UTC instant literal. Replay equality compares instant and offset, so a same-id write with the same instant and another offset is a new collision rejection.
  - The domain and draft field `scene_local_date` is removed. The local date is `scene.time.date_naive()`, and the stored anniversary index is written from it.
  - The statistics store loses `has_episode_index` and `is_fresh`, and stores written earlier in the phase are no longer read.
  - The crate ports `GraphAuthorityStore`, `VectorCandidateStore` and `MemoryEmbedder` lose their `Box` forwarding impls. The public `EmbeddingProvider` keeps its impl.
  - `LifecycleFilterDecision` loses `action` (always omitted) and `retention_state` (always none), and `LifecycleFilterReason` loses `Active`, `SuppressedIncludedByPolicy` and `SupersededIncludedByPolicy` (never built).
  - The graph port's time selectors and `query_scope_state` return each id with its memory time and stored salience.
  - `RetrievalTrace` loses `anniversary_has_more`.
- stance: break
- justification: there are no external consumers. The one locatable consumer is `CharacterMemoryEvals`, the public companion evaluation repository, which follows each library slice and regenerates its stores on every run. There is no migration.

## Context (workspace)
- Related files/areas, with line numbers at 1d0987f (the base may shift them):
  - `src/usecases/retrieve.rs`:
    - roots: participants at 101, keyed state at 128-176, activity at 178-203;
    - time budget 204-208, range 209-239, anniversary 252-278, recency 279-301;
    - candidate-root merge 657-712 and tie permutation 790-826;
    - `final_score` 883, `floor_kinds` 945, `select_with_cue_floors` 958;
    - section filling 1054-1105;
    - `CandidateRoot` 1328, `select_candidate_roots` 1377 with the source-based chain 1424-1446.
  - `src/usecases/retrieve/scene.rs`: description budget 209-215; candidate-merge call 307; `memory_scenes` 350.
  - `src/usecases/retrieve/state.rs`: `scopes_for_kind` 7; `order_section_state` 61; `order_state` 74.
  - `src/adapters/oxigraph/sparql_selectors.rs`: `select_state` orders by salience at about 270.
  - `src/usecases/write_planning.rs`: the hint loop 354-371; the commit derivation of Involves 1261-1311.
  - `src/api/types/retrieval.rs`: `RetrievalContext` 14; `RetrievalCueFloors` 127; `MemoryScenes` 248; `SceneReferenceResolution` 302.
- Rulings: the load-bearing decisions log, items 39 to 61, and the coordinator's rulings of 2026-09-23 recorded in this plan's Decision Log.
- Neighbour plans: the time plan (`docs/coding-agent/plans/completed/v0-2-time-plan.md`); the prospective and renderer plans (`v0-2-prospective-plan.md` and `v0-2-renderer-plan.md`, on their own plan branches, held until this slice lands); the phase fixes brief `.agent-work/orchestrator/phase-correctness-fixes-dispatch.txt`.
- Design records consulted, and deviations from their acceptance: ADR-D-0018, D-0022, D-0023, D-0024, D-0029, D-0038, ADR-I-0022, I-0036. ADR-I-0036 is corrected in Task_8. There is no other deviation. ADR-D-0024's invariant holds, and the new record depends on it.

## Open Questions (max 3)
- None. The contradictions found while drafting and the Tier A findings were ruled on 2026-09-23 (Decision Log).

## Assumptions
- A1: The base, the tip of `feature/2026-09-22/phase-correctness`, holds 1d0987f's retrieval shape plus the three phase fixes. The keyed-state read is bounded and ordered in the query, so the place order changes in that one query. Source: the phase fixes brief. Checked by Task_1 at dispatch; it stops and reports if the shape differs.
- A2: Apart from the changes the tasks list (the anniversary budget, the score clamp, knowing-first by road, description spare turns, place reporting, expansion and order, per-kind section rounds, and the hint link), every acceptance of the scene-words and time plans holds unchanged. Source: those plans' promises restated as rows of the table. Checked by Tasks 1 to 3, which list every changed expectation with before and after from runs.
- A3: The admitted ranked object's cue kinds can reach `memory_scenes` with no read. Source: pack building at `retrieve.rs:1042-1130` and `memory_scenes` at `scene.rs:350`. Checked by Task_4.
- A4: Keeping the UTC instant literal unchanged, and storing the offset as a separate value, leaves every time read (recency, range, anniversary, participant occasions) exactly as it is for every offset chrono accepts, including ±23:00. Source: the Tier D probe, which showed that an offset-carrying literal at ±23:00 is left unbound by the query engine's cast while UTC storage works. Checked by Task_6's ±23:00 controls.
- A5: The keyed-setting and activity-pressure families exist in the evaluation repository, with before-numbers at the base, before Task_1 is dispatched. Source: the practice of ruling 51. This is a precondition of dispatch.

## Tasks

### Task_1: One road table and five principles replace the special cases
- type: impl
- effort: 9 worker-hours
- owns:
  - src/usecases/retrieve.rs
  - src/usecases/retrieve/scene.rs
  - src/usecases/retrieve/activity.rs
  - src/api/types/retrieval.rs
  - src/ports/graph_authority.rs
  - src/adapters/oxigraph/sparql_selectors.rs
  - src/adapters/oxigraph/embedded.rs
  - src/adapters/oxigraph/tests.rs
  - src/memory/retrieval_floor_tests.rs
  - src/memory/retrieval_time_tests.rs
  - src/memory/retrieval_turn_tests.rs
  - src/memory/retrieval_scene_tests.rs
  - tests/scene_reminder_tests.rs
  - tests/public_facade_tests.rs
  - README.md
  - the test GraphAuthorityStore impls, updated mechanically for the selector return shapes only: src/usecases/remember.rs:1183, src/usecases/correct_forget.rs:3502, src/usecases/link.rs:579, src/memory/write_turn_tests.rs:561
- depends_on: []
- description: |
  Base: the tip of feature/2026-09-22/phase-correctness (see Integration). Confirm A1 first.

  Record at the parent commit, through the public facade on the real embedded stores, with identifiers opposed to time:
  - a range set to last Tuesday in a store with a busy today;
  - an unshared anniversary of ordinary salience in a store more than a year deep with daily experiences;
  - a description root that takes a spare turn at root selection;
  - a recency occasion also found by the topic under a recency floor of one;
  - the time plan's parity cases;
  - zero and negative raw search scores at a saturated root cap: a root cap of two, a topic floor of one, two zero-score topic roots and a recent time root;
  - two equal positive topic hits, one also reached by recency;
  - a clamped zero-score topic hit that recency also reached;
  - a cross-kind tie at a score of exactly 1.0: a participant root, a place-key root, an activity root and a topic hit at 1.0, under a root cap that cannot hold them all;
  - a section-score-margin control, where a cue-less occasion's final score falls within 0.25 plus 0.10 times its salience of an admitted member's.

  Build, as the Design section states it:
  - the road table, keeping the place row as it is today;
  - principles 1 to 5 with every listed deletion;
  - the score clamp;
  - the time selectors and `query_scope_state` returning each id with its memory time (through the one shared SPARQL expression) and its stored `salience_score`, carried on the root for the root key. Task_1 owns this port change: `query_scope_state` goes from `Vec<MemoryId>` (`ports/graph_authority.rs:420-424`) to the same shape as the time selectors, and every trait impl is updated mechanically. Task_2 then changes only the place query's order and uses the values;
  - the single lexicographic root key and the section rank key on memory time;
  - `CandidateRoot` holding one `MemoryObjectRef` in place of its id and type (`retrieve.rs:1328-1338`);
  - from the time slice completion audit (line numbers at 0ce2f35):
    - delete `RetrievalTrace.anniversary_has_more` (`src/api/types/retrieval.rs:559-560`) and the extra id the anniversary read fetches for it (`src/usecases/retrieve.rs:252-265`). Nobody gave the anniversary, so the marker informs nobody, as with recency's dropped count, and its serde also differed from `time_range_has_more`;
    - replace the test `unshared_anniversary_competes_with_recent_daily_occasions_by_score` (`src/memory/retrieval_time_tests.rs:2111`). It proves a salient anniversary comes only under a section cap of two, so it gives way to the 367-day, root-cap-12 case this task's acceptance requires.

  Keep the merge rules of rulings 47 and 54 exactly. Stop and report if a principle cannot be carried without changing selection among roots the table assigns identically before and after. The README prints the table and the five principles in plain words. The rewrite also covers `README.md:49`, which leaves out anniversaries, and `README.md:82`, which claims unshared anniversaries compete for unclaimed room by score. Ruling 65 makes that claim true, so the README states it as the root key orders it: among zero-score roots, salience first, then newest.
- acceptance:
  - The road table is the only place a root's reported kinds, expansion, reservation and budget are decided. None of these remain: `date_match_floor_eligible`, `floor_kinds`, the tie permutation, the source-based chain in root selection, any `matches!(kind, Recency | DateMatch)`, or `full_standing_kinds`. The report lists each deletion with its lines.
  - With a range set to last Tuesday, no memory is reported as recency and nothing from today is contributed by the time read. With no range, recency contributes as it does at the parent.
  - The anniversary contributes up to the room. In the 367-day daily store with the root cap at 12, in both identifier orders:
    - a salience-1.0 unshared anniversary is in the pack (at the parent it was cut at the root cap, ruling 65);
    - an ordinary unshared anniversary is not in the pack, and no recent day of equal salience is displaced;
    - a shared anniversary holds the date-match floor.
  - A description root takes no spare turn at root selection; the parent case is shown.
  - With the recency floor at one, the reserved slot goes to the latest occasion even when the topic also found an older one. At the default of zero, recency reserves nothing.
  - On the parity fixtures (no range, no anniversary, no description, no reserved recency, no negative or zero search score at the parent), with a topic that fills every cap, the root list, the pack (members, order and scores) and the echoed result equal the parent's in both identifier orders.
  - Scores and ties:
    - A search score below zero enters as zero.
    - At the saturated root cap, the second slot goes to the timestamped time root over the second zero-score topic root. The report names this as an intended change beside the parent's choice.
    - A clamped zero-score topic hit that recency also reached carries a timestamp and orders by it.
    - Two equal positive topic hits keep the parent's order when one is also reached by recency.
    - The 1.0 cross-kind tie orders participant, place key, activity, then the topic hit, as the parent's given-then-search order did. The report shows both runs.
    - The section-score-margin control is reported with its scores, and parity is claimed only where the margin holds.
  - Every difference from the parent falls under an intended change listed in Planner-added requirements.
  - `RetrievalTrace.anniversary_has_more` and its extra read are gone. The section-cap-two anniversary test is replaced by the 367-day, root-cap-12 case. `README.md:49` and `:82` say what the anniversary does under the root key.
  - Every changed test expectation is listed with before and after, from runs in both identifier orders.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the baseline cases run at the parent commit, with what each brought recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, reproducing the baseline at the parent, tracing every former special case to its table row, and checking the score, expansion and proximity merges for unintended change"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A altitude review of the table and the five principles against rulings 44 to 47 and 53 to 61, ADR-D-0022, ADR-D-0029 and the philosophy on knowing versus being reminded"

### Task_2: A place brings what was formed there, as a reminder
- type: impl
- effort: 5 worker-hours
- owns:
  - src/usecases/retrieve.rs
  - src/usecases/retrieve/state.rs
  - src/adapters/oxigraph/sparql_selectors.rs
  - src/adapters/oxigraph/embedded.rs
  - src/adapters/oxigraph/tests.rs
  - tests/retrieval_scope_tests.rs
  - tests/retrieval_state_tests.rs
  - tests/public_facade_tests.rs
  - README.md
- depends_on: [Task_1]
- description: |
  Record at the parent commit, with identifiers opposed to time:
  - six keyed people present with an office key and a topic;
  - six keyed people present with an activity;
  - a constant home key on every memory, under several distinct topics;
  - a setting key and a custom value whose memories share a notion with older experiences.

  Then change the place row of the table:
  - a setting-key or custom-value root has a cue component of zero and is a leaf;
  - it holds the place floor and takes no turn;
  - it records no state scope;
  - the place selector offers its memories newest first by memory time (the one clock, through the shared SPARQL expression), then id. The return shape (id, memory time, salience) was prepared by Task_1, so the root key now uses these values because a place root's score is zero. Only the place selector's order changes: the shared lower-level state selection also serves participants, whose salient-current order stays.

  `order_section_state` runs rounds per kind, as `scopes_for_kind` does at roots.

  The README states that a place key brings what was formed there, as a reminder, and that a project is given as the activity, not as a custom value.
- acceptance:
  - Six keyed people with an office key and a topic: every person brings their state, where two brought nothing at the parent. Office-key memories enter only by the place floor and unclaimed room.
  - Six keyed people with an activity: section rounds run among the people only, and the activity takes no round among them. The parent's interleaving is shown beside the new one.
  - A constant home key on every memory: across the distinct topics, the top-salience home belief is not in every pack, and the place floor's slot goes to the newest home memory.
  - A keyed participant's state keeps its parent order (the facade test `named_subject_fanout_selects_current_salient_then_recent_state` passes unchanged).
  - A memory reached only by a setting key or a custom value has a cue component of zero and brings no older occasion of a notion it names. The same memory, also found by the topic, keeps the topic's score and expands through it.
  - Every changed test expectation is listed with before and after, from runs in both identifier orders.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the baseline cases run at the parent commit and recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, tracing every reader of state scopes and the place selector's order"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A altitude review against ruling 59, item 26 (presence is not aboutness) and ADR-D-0024"

### Task_3: A present participant is recorded once, as present
- type: impl
- effort: 2 worker-hours
- owns:
  - src/usecases/write_planning.rs
  - tests/write_planning_tests.rs
  - tests/retrieval_state_tests.rs
  - tests/public_facade_tests.rs
- depends_on: [Task_2]
- description: |
  Delete the remember hint loop that records a present keyed participant as Mentions from the observation (`write_planning.rs:354-371`). Keep the Involves link from the episode written at commit (`write_planning.rs:1261-1311`). Record at the parent commit a keyed participant's observation reached from that participant, with its score and the participant's selectivity counts.
- acceptance:
  - A write with a present keyed participant stores the Involves link and no Mentions link from the observation.
  - The observation is reached through its episode. The report lists every moved score and statistics count, and every memory admitted at the parent that is no longer admitted, with the reason.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the baseline case run at the parent commit and recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review of the write path and the moved expectations; Tier A altitude review against item 26 (presence is not aboutness)"

### Task_4: Every admitted memory says which cues admitted it
- type: impl
- effort: 2 worker-hours
- owns:
  - src/api/types/retrieval.rs
  - src/usecases/retrieve.rs
  - src/usecases/retrieve/scene.rs
  - src/memory/retrieval_scene_tests.rs
  - tests/public_facade_tests.rs
  - README.md
- depends_on: [Task_3]
- description: |
  `MemoryScenes` gains `admitted_by`: the roads that admitted the ranked object, as a reported enum separate from the floor kinds (the table's "admitted by" column: participant, place, activity, topic, person description, setting words, range, anniversary, recency), so a description is never reported as knowing someone and an anniversary never as a date the caller asked about, passed from pack building to `memory_scenes` with no read, filled with the trace on or off. Nothing else in the result changes. The README defines each reported road by what reached the memory (for example: participant, someone present given by key or name, meaning occasions they were at and what is held about them; person description, a resemblance to how someone present was described, which can be a stranger), and says the words the character uses belong to the consumer; it gives no phrasing.
- acceptance:
  - Every `MemoryScenes` entry carries `admitted_by`, with the trace off.
  - Exact sets, both identifier orders: a range occasion reports range; an anniversary with a range also given reports anniversary, not range; a faint topic hit reports topic; recent occasions with nothing said report recency; a keyed-place memory reports place; a description-only match reports person description (or setting words), never participant or place; a memory reached by a participant key and the topic reports both.
  - The field is identical with the trace on and off, and the pack (members, order and scores) equals the parent's.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, confirming no new read and that the kinds are those of the admitted ranked object"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A altitude review against ruling 60 as refined, philosophy section 7.2 and ADR-D-0038 (reported, never withheld)"

### Task_5: The dead code and duplicated production code the audits found are gone
- type: impl
- effort: 8 worker-hours
- owns:
  - src/ports/graph_authority.rs
  - test callers mechanically affected by removing the Box forwarding or the one-value lifecycle fields (test code only)
  - src/ports/vector_candidate.rs
  - src/ports/embedder.rs
  - src/ports/retrieval_stats.rs
  - src/adapters/stats/sqlite.rs
  - src/policy/retrieval_selectivity.rs
  - src/policy/graph_expansion.rs
  - src/api/types/retrieval.rs
  - src/usecases/retrieve.rs
  - src/usecases/retrieve/scene.rs
  - src/usecases/retrieve/state.rs
  - src/usecases/retrieve/activity.rs
  - src/adapters/oxigraph/shared.rs
  - src/adapters/oxigraph/sparql_selectors.rs
  - src/adapters/oxigraph/rdf_mapping.rs
  - src/adapters/oxigraph/tests.rs
  - src/memory.rs
  - src/composition.rs
  - README.md
- depends_on: [Task_4]
- description: |
  No change to what comes to mind; one commit per item. Locations are at 1d0987f and are re-found at the base.

  Delete:
  - the `Box` forwarding impls on the three crate ports (`ports/graph_authority.rs:439-554`, `ports/vector_candidate.rs:48-71`, `ports/embedder.rs:15-24`), updating `composition.rs` and any test caller mechanically;
  - the always-true `current_subject_state` ARGUMENT of `selectivity_plan_for_entity` (`retrieval_selectivity.rs:205,212`; passed at `retrieve.rs:356`), if the base confirms it is true for every production entity root, and nothing else. The About state override at `:212` becomes unconditional for About specs. Tests that pass false (`:629`, `:830` and the rest) drop the argument; any that asserts the counted About path taken only when false is deleted and listed. `retrieval_selectivity.rs:142-170` is live global-counter loading and is not touched here (its failure block is merged below). KEEP the live `GraphExpansionQuery.current_subject_state` flag (`graph_authority.rs:157`, set on participant entity roots at `retrieve.rs:1587`, read at `shared.rs:603,646` and `graph_expansion.rs:461,555,1198`);
  - `bounded_expansion_node_set` (`graph_expansion.rs:308-377`, caller `:1312`);
  - the one-value lifecycle items (`api/types/retrieval.rs:713-739`; ranked at `retrieve.rs:1800-1802`; filters at `retrieve.rs:1260` and `memory.rs:847`): `LifecycleFilterDecision.action` (always omitted), `LifecycleFilterDecision.retention_state` (always none), and the reasons `Active`, `SuppressedIncludedByPolicy` and `SupersededIncludedByPolicy` (never built). KEEP `LifecycleFilterDecision.superseded_by`, which is filled (`retrieve.rs:1700`, from `graph_expansion.rs:900` and `sparql_selectors.rs:327`);
  - the write-only `RetrieveAssembly.superseded_by` (`retrieve.rs:560`, written `:614-617`, sorted `:780-783`);
  - the statistics store's compatibility code for stores written earlier in the phase (`stats/sqlite.rs:19,40-75,89,106,123,164,214`); the scene-date compatibility read goes with the field in Task_6;
  - the serde default of `RetrievalContext.scene` (`retrieval.rs:15`);
  - the stale comments (`retrieve.rs:5-6`, `graph_expansion.rs:18`).

  Merge:
  - `SectionCounts` (`retrieve.rs:1297-1325`) into the section pressure count it duplicates. It is read at `:1149` into the public `SectionAssignment.rank` (`:1153`); the rank comes from `SectionPressureSummary.included_count` (`:1148`, `:1209-1219`) and `SectionCounts` goes. Invariant: every selected member keeps its one-based rank within its section;
  - one fail-closed helper for the repeated block (`graph_expansion.rs:409,428,508,597,649,1060`, empty plans `410-418`, `431-439`; `retrieve.rs:1602`, `1682-1690`), and one for the two stats-read failure blocks (`retrieval_selectivity.rs:125-138`, `:154-167`);
  - one `From` for the lifecycle policy, replacing the eight hand-built copies (`retrieve.rs:132,216,259,285,1579`, `scene.rs:96`, `activity.rs:83,146`);
  - the twin helpers:
    - `omit_bounded_candidate` and `omit_missing_candidate` (`retrieve.rs:749-777`);
    - `summarize_stale` and `summarize_lifecycle` (`:1233-1276`);
    - the `increment_section` pair;
    - in `graph_expansion.rs`, the `apply_fanout_limits` wrappers with `RootFanoutMode`, and `link_touches_ref` with `other_endpoint` (`:464-467`);
    - `enum_value` (`rdf_mapping.rs:444-449` and `sparql_selectors.rs:841-846`);
    - `sort_objects` (`shared.rs:781` and `graph_expansion.rs:991`);
    - the `sceneTime` parse and its error text (`sparql_selectors.rs:462-468`, `:507-513`);
  - the SPARQL fragments repeated at `sparql_selectors.rs:414,527,466,511,572,624`.

  Stop cloning the kind orders for every section (`retrieve.rs:1062`, `state.rs:114`). Rename `ContentCue` to `Reminder`.

  The scene offset is Task_6's.

  The write turn stays where it is: the facade already holds it and passes it in (`memory.rs:39`, `:105`, `:158`, `:177`), and the 60 throwaway mutexes are test-only (Decision Log, R4).

  Stop and report if any item turns out to change what comes to mind.
- acceptance:
  - Every item above is done. The report names each with its files and its line counts before and after, and lists every deleted test with the deleted code it certified.
  - On the whole suite and on a fixed set of facade retrievals run before and after, the admitted memories, sections, order, scores, `admitted_by` and trace are identical. The trace comparison includes selected and omitted objects interleaved across several sections, and every selected member keeps its one-based section rank.
  - The facade test `named_subject_fanout_selects_current_salient_then_recent_state` passes unchanged.
  - A statistics store at the latest schema initializes and reopens after the compatibility code is gone.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the fixed facade retrievals compared before and after, with the comparison in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, confirming each deletion was dead or write-only, each merge preserves behavior (including the section rank from included_count), and the live query flag current_subject_state is untouched"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A altitude review: nothing kept that the audits found unearned, and nothing added"

### Task_6: A scene time reads back with the offset it was given
- type: impl
- effort: 3 worker-hours
- owns:
  - src/domain.rs
  - src/domain/scene.rs
  - src/api/types/draft.rs
  - src/adapters/oxigraph/rdf_mapping.rs
  - src/adapters/oxigraph/shared.rs
  - src/adapters/oxigraph/vocabulary.rs
  - src/adapters/oxigraph/embedded.rs
  - src/adapters/oxigraph/tests.rs
  - src/usecases/remember.rs
  - src/usecases/write_planning.rs
  - src/memory/retrieval_time_tests.rs
  - tests/write_planning_tests.rs
  - tests/public_facade_tests.rs
  - README.md
  - docs/design/database/graph_schema_design.md
  - docs/design/database/schema_cheat_sheet.md
  - the fixtures that construct an Episode directly, for the mechanical removal of `scene_local_date` only (test callers only): src/test_support.rs:409, src/ports/retrieval_stats.rs:769, src/policy/embedding_surface.rs:279, src/usecases/correct_forget.rs:3365, src/api/types/retrieval.rs:840, src/domain/tests.rs:79
- depends_on: [Task_5]
- description: |
  This is the slice's only persisted-format change outside the behavior tasks, and it is proven red then green, not measured.

  At the parent commit, record what a +09:00, a +23:00 and a -23:00 scene time read back as. The parent reads them as +00:00, which `retrieval_time_tests.rs:1946-1967` pins by writing +10:00 and asserting `Z`.

  Keep the UTC instant literal every query uses exactly as it is (written at `rdf_mapping.rs:139`, read at `shared.rs:295`). An offset-carrying literal is not an option: the query engine leaves ±23:00 unbound (R1). Store the original offset as its own value on the episode, in seconds east of UTC, and read it back to rebuild the time the application gave.

  Delete the domain field `scene_local_date` (`domain.rs:303`, `api/types/draft.rs:157`, the compatibility read at `shared.rs:314`, and its tests at `oxigraph/tests.rs:129-160`). With the offset stored, `scene.time.date_naive()` gives the local date. Keep the stored year and month-day triples as the anniversary index, written from `scene.time` in `rdf_mapping.rs`.

  Replay and collision equality compare instant and offset. No offset is newly rejected and the input contract does not narrow. The one new rejection is a same-id write with the same instant and a different offset, which is now a collision. Invert the pinning test.

  `README.md:80` (at 0ce2f35) advises using the writes' offset at retrieval. Rewrite it to say what is true once the offset is stored: a scene time reads back as given, and the anniversary uses the local day of the offset each experience was written with.
- acceptance:
  - A +09:00, a +23:00 and a -23:00 scene time each read back with their offsets and exact fractional precision after a write and a reopen.
  - Order by instant is unchanged. Recency, range, anniversary and participant-occasion reads return the same results at ±23:00 as the parent.
  - The anniversary index is written from `scene.time`, and an anniversary at +09:00 and at ±23:00 matches the same local day as at the parent.
  - A same-id write with the same instant and another offset is rejected as a collision (the new rejection, red at the parent), and the identical write replayed after reopening the store is accepted.
  - `scene_local_date` no longer exists in the domain, the draft type or the reads. A store at the latest schema initializes and reopens.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the parent readings recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, tracing every reader, writer and collision check of a scene time, confirming the instant literal and every time query are unchanged, and that the anniversary index is derived from scene.time"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A altitude review against ADR-D-0029 and the ruling that the library never makes up a time"

### Task_7: One shared test support replaces the copies
- type: impl
- effort: 5 worker-hours
- owns:
  - src/test_support.rs
  - src/lib.rs (the test_support module registration only)
  - src/memory/ (test files only)
  - tests/ (test files only)
  - the #[cfg(test)] modules inside src/usecases/, src/memory.rs and src/adapters/ (test code only; no production code changes)
- depends_on: [Task_6]
- description: |
  One shared test support module replaces the repeated helpers and the test embedders, including the doubles at `memory.rs:1266` and `remember.rs:1377` where a real adapter's in-memory mode serves. Keep only recording and failure doubles. Merge `retrieval_turn_tests.rs` into the floor tests. Split the oversized `retrieval_time_tests.rs` and `retrieval_scene_tests.rs` by behavior. Every fixture that orders by time has identifiers opposed to time. No production code changes.

  From the time slice completion audit (line numbers at 0ce2f35):
  - drop the `record()` whole-outcome serializer (`retrieval_time_tests.rs:255`) and its 19 `println!` dumps;
  - drop `SUPPORT_AGE_WITNESSES` (`retrieval_scene_tests.rs:2056`);
  - delete `time_range_without_input_baseline` (`retrieval_time_tests.rs:1142`), which duplicates `:1248-1257`;
  - rename the `tier_a_*` tests (`retrieval_time_tests.rs:949-1071`) after the behaviors they check.
- acceptance:
  - The report counts the helper copies and embedders before and after, and names each merged or split file.
  - `record()`, its `println!` dumps, `SUPPORT_AGE_WITNESSES` and `time_range_without_input_baseline` are gone, and no test is named `tier_a_*`. The report maps each old name to its new one.
  - The same tests pass, apart from duplicates removed and listed.
  - Every fixture that orders by time opposes identifier to time, and the report lists the fixtures changed.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the test count before and after in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, confirming no assertion weakened and no production code changed (edits inside production files are confined to #[cfg(test)] modules and the module registration); Tier A altitude review: one module, not a new framework"

### Task_8: Records say what is true, and the slice is measured
- type: design
- effort: 3 worker-hours
- owns:
  - docs/decisions/implementation/ADR-I-0036-retain-measured-retrieval-bounds-with-one-reserved-slot-per-cue-kind.md
  - one new proposed record file under docs/decisions/design/
  - docs/decisions/README.md
- depends_on: [Task_7]
- description: |
  ADR-I-0036, which stays proposed:
  - label the activity floor as provisional, because no activity measurement ran;
  - remove the measured rule for entity roots reached from content hits, a path production never produces;
  - change nothing else.

  Draft a new one-decision design record, proposed, following the durable-docs rules: a setting key or custom value brings what was formed there, as a reminder. It depends on ADR-D-0022 and ADR-D-0024 and replaces neither.

  The orchestrator runs the slice-end measurement and records its numbers, reading and verdict in this plan's Decision Log.
- acceptance:
  - ADR-I-0036 differs from its parent only in the two corrections.
  - The new record states one decision, passes the repository's admission test, names its dependencies and cites rulings 26 and 59.
  - The index lists the new record.
  - The slice-end numbers, their reading against the philosophy and a verdict are in the Decision Log. If a family regressed, the intermediate tips of Task_1, Task_2 and Task_3 were re-measured and the cause is named.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A altitude review of both records against rulings 58 to 61 and the durable-docs rules: one decision per record, invariants rather than mechanism"
  - kind: manual
    required: true
    owner: user
    detail: "The decider accepts or returns the new record and the revised ADR-I-0036, in a batch at the slice boundary"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Slice-end measurement, run by the evals worker at the slice's code tip (Task_7's tip): the scene overlap, keyless, familiar person, time and shared interpretation families plus the new keyed-setting and activity-pressure families, before at the base tip and after, identifiers opposed to time, both orders, twice; only on a regression, re-measure at the Task_1, Task_2 and Task_3 tips to find the cause. Falsified if: a person present brings fewer of their state memories with the office key than without it; the same top-salience home belief is in every pack across distinct topics; the unlived topic's share rises above its recorded number; the topic keeps fewer on-topic memories with the activity than alone beyond the activity floor; asked about last Tuesday with the range set, anything from today is reported as recency; or any existing family falls below its recorded after-numbers (same-place on-topic survival, keyless latest occasions of equal salience, familiar person state, an ordinary unshared anniversary displacing a recent occasion of equal salience, the reminder leak); the salience-1.0 unshared anniversary in the year-deep daily store is absent from the pack (ruling 65)"

## Integration
- The library stack: the cues, fixes, state, scene-words and time slices, then the phase correctness fixes on `feature/2026-09-22/phase-correctness`, then this plan. The prospective memory and renderer slices wait for this slice.
  - Prospective memory adds its due and trigger roads as rows of the road table under principle 1 ("what a stored intention was waiting for") and principle 2 (due is an exact road). It waits for Task_1.
  - The renderer consumes `admitted_by` on `MemoryScenes` and phrases each cue kind itself. Its plan must be revised to read this field in place of any standing it infers. It waits for Task_4.
- Base: Task_1 is cut from the tip of `feature/2026-09-22/phase-correctness` once its three fixes are reviewed. The tip is pinned in the dispatch brief. If the fixes land after Task_1 starts, they are merged forward into Task_1's branch before its pull request opens.
- Inherited and not redone: the leaf rule and its provenance rule, the merge rules of rulings 47 and 54, the bounded reads from the phase fixes, and the time plan's selectors (whose return shape gains a time).

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]
- Wave 3: [Task_3]
- Wave 4: [Task_4]
- Wave 5: [Task_5]
- Wave 6: [Task_6]
- Wave 7: [Task_7]
- Wave 8: [Task_8]

The tasks run in sequence, one worker at a time, because they share one crate and its files. Each pull request stacks on the previous one. The tips of Task_1, Task_2 and Task_3 are kept as commits so a regression can be traced to one of them without measuring each. The evaluation repository builds the two new families before Task_1 is dispatched.

## Rollback / Safety
- Each task is one pull request and reverts on its own in reverse order. Once the companion repository has followed the slice, a revert is coordinated with it. Task_6 adds a stored offset value, removes the `scene_local_date` field and leaves the instant literal and the anniversary index unchanged. Reverting it needs stores written in between to be regenerated, which the companion does on every run.

## Progress Log (append-only)

- 2026-09-23 Base: PR 150 at `67d6735` (the phase correctness fixes merged with the time stack). New families (keyed setting, activity pressure) and the anniversary capability sentinel passed instrument review at companion `52abadb`; the capture runs on `12af98b`, which only moves the calibration entry to an 8 MiB thread after the full capture overflowed the Windows debug main stack at both `d36d96d` and `67d6735` (a harness limit, reviewed). BEFORE captured twice, byte-identical (SHA-256 `a8579019...`), zero bounded failures, 599 rows. What the base shows, as the falsifiers expect: last Tuesday with no topic brings all 5 Tuesdays plus 3 of today's occasions counted as recency; a salient unshared anniversary in the year-deep daily store is cut at the root cap; with the home key the same top home belief is in 6 of 6 packs; the office key shrinks two people's state; the activity takes more than its floor from the topic. Readings stay in the companion's scratch until the slice-end AFTER exists; then the evidence rule decides where they live.
- 2026-09-23 Task_1 done (PR 151, tip `fe38576`), approved by Tier D and Tier A after one fix round. One road table (`RecallRoad::rule`) replaces the special cases; the root key is one total order with salience before time among equal scores. Rulings during the task: the time selectors keep their bounds and gain memory time and salience (about 5 ms at 2000 episodes); shared and unshared anniversaries are separate rows; the activity's floor serves its thread first (rulings log 68). The review round replaced a committed capture harness with focused behavior tests and found that the anniversary read with people present took 20.6 s at 2000 daily episodes (31.3 s after the first rewrite); it now reads compact calendar and link rows and decides in Rust, 6.3 ms, same selections.
- 2026-09-23 Task_2 done (PR 152, tip `f36dd08`), approved by Tier D and Tier A after one fix round. A setting key or custom value is a reminder: cue zero, a leaf, holds the place floor, no spare turn, no state scope. At the parent, an office key left two of six present people with nothing about themselves and a constant home key pinned the same salient belief in every pack; both are gone. The review round found that several place keys were read key by key with a cap each, so the home key's newest memory always took the floor over a newer custom-value memory; several keys now form one place road, merged newest first under one cap. Residual risk to read at the slice end: an old significant memory formed at a place comes only if it is among the newest there or the topic finds it.
- 2026-09-23 Task_3 done (PR 153, tip `9c905e6`), approved by Tier D and Tier A after two fix rounds. The presence hint is gone and ObservedIn is written at commit, so what was observed at an occasion is reached through it (rulings log 69). The review rounds separated presence from aboutness on the read side too (70): presence is Involves on the occasion; About and Mentions of a subject are one aboutness list with one bounded budget, pruned before the hub check, so someone often talked about never makes recall fail; at an entity root, shared occasions come before aboutness. The as-of cut now applies to occasions and observations on every road by each memory's one clock, except what a caller's range contributes (71). A reminder reaches observations only through its own occasion. Lost admissions, each explained: two lowest tail state rows behind shared occasions, one future topic episode, and in tight mention-only cases the notion-creation occasion replaced by the mentioned one. A query-shape fix restored the anniversary fixture from 9.6 s to 3.0 s. Slice-end reading adds: a person with many high-salience remarks, and retrieval latency.
- 2026-09-23 Task_4 done (PR 154, tip `56077a2`), approved by Tier D and Tier A after one contract revision (rulings log 72): `admitted_by` reports the roads that admitted a memory as its own enum (participant, place, activity, topic, person description, setting words, range, anniversary, recency), separate from the floor kinds, so a stranger's description never reads as knowing someone and an anniversary never as a date the caller asked about. No new read; every result equals the parent apart from the new field (36 paired outcomes).
- 2026-09-23 Task_5 done (PR 155, tip `c53baed`), approved by Tier D and Tier A after one fix round: 34 commits, dead and duplicated production code removed with no change to what comes to mind (84 whole-outcome pairs equal apart from the two intended surface changes). The review round restored a count-scope test that had been deleted with a dead route although the logic still ran on live routes, and removed unused global counter reads for About and Mentions (a failed unused read had marked the whole statistics store unhealthy and pushed the live routes into fallback); the stats adapters no longer maintain those counts.
- 2026-09-23 Task_6 done (PR 156, tip `80845e5`), approved by Tier D and Tier A after two follow-ups. A scene time reads back exactly as the application gave it: the UTC instant every query uses is unchanged, the offset is stored beside it and read back at hydration, `scene_local_date` is gone, and a same-id write at the same instant with a different offset is a collision. Review found returned Scene JSON lossy for offsets with a seconds part (RFC 3339 has only minute offsets), so every offset RFC 3339 can express is accepted and a seconds part is rejected where a scene enters the library (rulings log 74, refining 64); a time the library fixes itself is at UTC and the application gives the offset it means (73). An exhaustive check covered all 172,799 representable offsets.
- 2026-09-23 Task_7 done (PR 157, tip `f6926ef`), approved by Tier D and Tier A after one fix round. Test helpers live once on each side of the crate boundary (`src/test_support.rs`, `tests/support`); oversized test files are split by behavior; the review-evidence dumps are gone with their assertions kept; every time-ordered fixture opposes identifiers to time or runs both directions. The review round replaced three vector doubles' own search logic with the real in-memory adapter (only recording and failure hooks remain) and made a shared scene cohort run both id directions. The slice's code is complete at `f6926ef`; the slice-end measurement runs there.
- 2026-09-23 Task_8 records done (PR 158, tip `ef8dbfb`), approved by Tier D and Tier A after one fix round: ADR-I-0036 (proposed) labels the activity floor provisional, allows a provisional default only when labelled as one, adds an activity-pressure calibration to Revisit When, and drops the unused content-hit entity-root rule; ADR-D-0039 (proposed) says a setting key or custom value recalls what was formed there, as a reminder, citing rulings 26 and 59 by title and date. The docs truth sweep brought the README and the database design documents in line with rulings 69 to 74 and replaced source line anchors with named-function links. Both records await the decider; ADR-D-0039 also settles the earlier question of what a custom value is (rulings log item 28, answered by 59). Remaining: the slice-end measurement.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-23 Decision: an observation is reached through its occasion by the ObservedIn relation, written at commit (Task_3, rulings log 69).
  - Trigger / new insight: removing the presence hint lost an observation about a present person, because the store never wrote ObservedIn, and the hint had carried that path by treating presence as aboutness.
  - Plan delta (what changed): Task_3 writes ObservedIn at commit alongside Involves; principle 5's wording now says what a leaf brings, so it cannot be read as "opens nothing".
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.

- 2026-09-23 Decision: consolidate how recall room is divided before any more cue kinds are added.
  - Trigger / new insight: the re-audit (rulings 58 to 61) found that the rules had grown into special cases nobody could explain without the log. It also found that a keyed place crowds out the people present, and that the memo would print reminders as plain history.
  - Plan delta (what changed): new plan. One road table carries the five principles. A keyed place or custom value is a reminder. The result reports standing. The hygiene items and two record corrections are included. The code also required six planner-added rules:
    - the recency floor is removed;
    - the place road offers its memories newest first;
    - each kind's queue lists knowing roads first;
    - descriptions take no spare turns;
    - rounds among people leave the section stage;
    - the hint link is removed in the behavior task, not the hygiene task.
  - Tradeoffs considered: see the alternatives in Design.
  - User approval: plan approval waived for plans inside the rulings; the planner-added choices are logged for presentation.
  - Record proposed: the replacement of ADR-D-0024; a revision of the proposed ADR-I-0036.
- 2026-09-23 Decision: the five contradictions the draft found between the code and the design are ruled.
  - Trigger / new insight: while drafting, five places were found where the code and the decided principles disagree.
  - Plan delta (what changed):
    - Place order, adopted: a place contributes its memories newest first, as descriptions and recency already do, and the score decides among what was contributed.
    - Rounds among people, kept: principle 4 is restated as turns among kinds only at root selection, while rounds among the scopes of one given kind stay at every stage. They are fairness among cues that were equally given, and they measured well (six people each bring their state). `order_section_state` stays. A description taking spare turns at root selection is a defect under principle 4 and is removed.
    - Hint link: agreed as a behavior change.
    - Recency floor, kept at a default of zero: a caller who sets it has given the reservation, which principle 1 allows. It is built and measured, and it costs nothing at zero.
    - Anniversary contribution raised to the room: agreed. It removes a special case and is safe because an unshared anniversary carries no score and loses ties to the newest. The slice falsifier must hold: an ordinary anniversary never displaces a recent occasion of equal salience.
  - Tradeoffs considered: removing the section rounds and the recency floor, as the draft proposed; rejected for the reasons above.
  - User approval: ruled by the coordinator.
  - Record proposed: unchanged.
- 2026-09-23 Note: entry 2 supersedes entry 1's bullets that remove the recency floor and remove rounds among people from the section stage.
- 2026-09-23 Decision: revised after the Tier A review.
  - Trigger / new insight: the Tier A review found five problems.
    - `order_section_state` takes every state scope, so a place key and the activity each got a round equal to a person's.
    - A known-or-reminded bit is wrong both ways.
    - One behavior task could not attribute a regression to its cause.
    - The keyed-setting falsifiers missed the unlived topic's share.
    - Replacing ADR-D-0024 would retire an invariant that stays true.
  - Plan delta (what changed):
    - Rounds run per kind at every stage, and a setting key or custom value records no state scope.
    - The table's standing column becomes two columns, expands and reported. Each admitted memory's `MemoryScenes` entry carries `admitted_by`, the kinds that admitted it. This refines ruling 60 and is the answer to ask 20; the renderer consumes the field.
    - The behavior work is split into Task_1 (table and principles), Task_2 (places) and Task_3 (hint link), with one slice-end measurement and a bisect at those tips only on a regression.
    - The unlived topic's share joins the keyed-setting falsifiers.
    - A new one-decision record replaces the planned replacement of ADR-D-0024.
    - A road's score is the larger of zero and its similarity, which replaces the assumption about embedder signs.
    - `CueKind::is_time` is dropped, and the `CandidateRoot` single-reference merge moves into Task_1.
    - Every hygiene item is named with its audit location, and shared test support is its own task.
    - The compatibility stance lists the lifecycle and serde removals.
  - Tradeoffs considered: measuring at every behavior tip was rejected; measurement runs at checkpoints, and the intermediate tips exist as commits for a bisect.
  - User approval: ruled by the coordinator.
  - Record proposed: a revision of the proposed ADR-I-0036; one new proposed design record.
- 2026-09-23 Decision: revised after the Tier D review (CHANGES REQUESTED on 780a3345) and the hygiene locator's corrections.
  - Trigger / new insight: the Tier D review made four findings.
    - R1: an offset-carrying literal at ±23:00 is left unbound by the query engine's `xsd:dateTime` cast (executed probe).
    - R2: `SectionCounts` is read for the public section rank.
    - R3: the score clamp left the order of equal and zero-score roots undefined.
    - R4: moving the write turn into the facade risks the boundary that the seven write-turn tests protect.

    The locator also showed that the lifecycle `superseded_by` is filled, and that `current_subject_state` names two symbols, of which only the selectivity argument is always true.
  - Plan delta (what changed):
    - R1: the UTC instant literal stays, and the episode stores the given offset as its own value in seconds east of UTC. Collision equality compares instant and offset. Acceptance covers ±23:00 and reads unchanged at ±23:00; no narrower input contract.
    - R2: `SectionCounts` moves from delete to merge, with the rank taken from `SectionPressureSummary.included_count`.
    - R3: a road's score is the larger of zero and its similarity. Among equal scores the kind's own order decides, then id, and a root without a timestamp orders after one with a timestamp. Only zero-score time and place roots carry one, so an equal positive overlap keeps the parent's order. The intended change and the retained parity are stated, and acceptance adds zero, negative, equal-overlap, saturated-cap and section-margin cases.
    - R4: the facade write turn item is considered and deleted by value audit. The 60 call sites are all test-only, and moving the lock risks the boundary the write-turn tests protect (embedding outside the turn, planning inside it, the turn held through repair and statistics), for test-only noise.
    - Only the always-true selectivity argument is removed, and the live query flag stays.
    - The lifecycle item keeps `superseded_by`.
    - The locator's twins `enum_value`, `sort_objects`, the `sceneTime` parse and the stats-read failure blocks are folded in.
    - Housekeeping: task types use the allowed values; Task_5 owns the test callers it affects mechanically; Task_6 names the module registration and means no production code changed; the place order change is confined to the place selector.
  - Tradeoffs considered: narrowing the scene-time input to offsets the query engine binds was rejected, because it would add a rejection for valid input.
  - User approval: ruled by the coordinator.
  - Record proposed: unchanged.
- 2026-09-23 Decision: revised after the bounded Tier A re-check of 641adf15 (six findings).
  - Trigger / new insight:
    - The equal-score root rule was a list of partial preferences, not a total order.
    - The parity claim covered cases the intended changes contradict.
    - The place and recency roots compared on two clocks.
    - `scene_local_date` duplicates what a stored offset gives.
    - The DoD and Task_5 contradicted themselves on corrections and rejections.
    - The offset item sat in a task whose rule is to stop if anything changes, beside a leftover write-turn acceptance line.
  - Plan delta (what changed):
    - Root order is one lexicographic key: score descending, timestamp present, timestamp descending, the best road's table row, position in that road's list, then id.
    - The table rows are reordered to given, then search, then time. A root carries a timestamp exactly when its score is zero and a time or place road reached it. The anniversary's shared-first order applies only to reserved room.
    - A 1.0 cross-kind tie joins the baselines.
    - Parity is claimed only on the parity fixtures (no range, no anniversary, no description, no reserved recency), and the intended changes are listed.
    - Memory time is one clock (an episode's scene time; an observation's `observed_at`, else its episode's scene time; otherwise creation time), computed by one shared SPARQL expression for every selector that returns a time.
    - `scene_local_date` is deleted, and the anniversary index is written from `scene.time`.
    - The DoD names one stored-time correction and one new collision rejection, and "apart from the Compatibility stance".
    - The write-turn acceptance line is deleted.
    - The offset is its own Task_6, so the plan now has eight tasks.
  - Tradeoffs considered: keeping the equal-score rules as separate preferences was rejected, because two of them could disagree on the same pair.
  - Lesson candidate: every ordering rule in a plan is one lexicographic key with a total final tie-break. A list of partial preferences is a plan-review finding.
  - User approval: ruled by the coordinator.
  - Record proposed: unchanged.
- 2026-09-23 Decision: ruling 65 and the second Tier D carry-over findings.
  - Trigger / new insight: the time slice measurement at d36d96d showed that a salience-1.0 unshared anniversary in a 367-day daily store is cut at the 12-root cap. All zero-score roots were ordered by time, twelve recency roots filled the cap, and the section never weighed it. The previous key (score, then timestamp) kept that starvation, so Task_1's "a more salient one still comes" would have failed. The second Tier D pass found three gaps: Task_1 did not own the test trait impls its port change breaks; the place selector's return shape was not prepared anywhere; and the delete locator named live global-counter loading. The offset task also did not own the fixtures that construct an Episode directly.
  - Plan delta (what changed):
    - The root key is now score, salience (`RankedObject.salience_component`, `retrieve.rs:856` at d36d96d), timestamp present, timestamp descending, road row, position in the road's list, then id. A zero-score root reached by a time or place road carries the salience and memory time its selector returned. This is listed as an intended change.
    - Task_1's acceptance checks the salient anniversary in and the ordinary one out, with no equal-salience recent day displaced, in both identifier orders.
    - Task_1 owns the four test GraphAuthorityStore impls and prepares the (id, memory time, salience) shape for the time selectors and `query_scope_state`. Task_2 only changes the place order.
    - The selectivity delete is limited to the always-true argument, and `retrieval_selectivity.rs:142-170` stays.
    - Task_6 owns the six Episode-constructing fixtures for the mechanical `scene_local_date` removal.
  - Tradeoffs considered: giving Task_2 the port and impls was rejected, because Task_1 already owns the port and changes the time selectors in the same way.
  - User approval: ruled by the coordinator (ruling 65).
  - Record proposed: unchanged.

- 2026-09-23 Decision: Tier A bounded re-check of the ruling-65 revision: findings 1 to 3 resolved, ruling 65 judged right for the character. Applied its two wording fixes: principle 4 names the root-stage salience order, and the keyless falsifier counts only occasions of equal salience, with the family's salience recorded before the base run, so the intended reordering cannot read as a false falsification; salience compares with `total_cmp`. No further Tier A round.
  - Lesson candidate: when a plan adds an intended change, every falsifier that measures the same quantity is checked and qualified.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: the time slice completion audit (at 0ce2f35) is folded in, additively.
  - Trigger / new insight: the time slice completion audit found four things this plan's tasks already touch:
    - a trace marker nobody reads;
    - README lines that are stale or become true only under ruling 65;
    - an anniversary test that proves its claim only under a section cap of two;
    - test-file noise (a whole-outcome serializer with 19 dumps, a witness constant, a duplicate baseline, and test names after the review tier rather than the behavior).
  - Plan delta (what changed):
    - Task_1 deletes `RetrievalTrace.anniversary_has_more` and its extra read, replaces the section-cap-two anniversary test with the 367-day, root-cap-12 case, and rewrites `README.md:49` and `:82`.
    - Task_6 rewrites `README.md:80`.
    - Task_7 drops `record()` and its dumps, `SUPPORT_AGE_WITNESSES` and `time_range_without_input_baseline`, and renames the `tier_a_*` tests.
    - The compatibility stance lists the removed trace field.
    - Every file was already in the owning task's owns.
  - Tradeoffs considered: none; each item lands in the task that already owns its file.
  - User approval: ruled by the coordinator.
  - Record proposed: none.

- 2026-09-23 Decision: what the result reports is the roads, not the floor kinds (Task_4 Tier A, rulings log 72).
  - Trigger / new insight: reporting the floor kind made a stranger's description read as "someone present" and an anniversary read as a date the caller asked about, the false continuity rulings 60 and 62 exist to prevent.
  - Plan delta (what changed): the road table gains an "admitted by" column; `admitted_by` is a separate reported enum; Task_4's acceptance uses exact sets including a description-only match and an anniversary with a range; the README defines meanings, not phrasing.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.

- 2026-09-24 Slice-end measurement read: behavior direction confirmed, with one falsifier explained and one residual; timing is open.
  Trigger / new insight: the slice-end AFTER at library c0ed9e21 (harness 1b8f02a) ran twice, byte-identical, against the accepted BEFORE at 67d6735. Its shared fields equal an independent capture at f6926ef, so the later commits changed nothing measured. All eight authored input blocks are identical, there were no bounded failures, and the six protected fixtures are unchanged. Readings, in both identifier orders:
  - What the plan set out to change moved the right way:
    - Office key: with the key present, each of the six people brings one of their own state memories, keyed or not (was 3, 2, 2, 2, 0, 0 with the key).
    - Home belief: the same highest-salience home belief no longer appears across six unrelated topics (6 of 6 to 0 of 6).
    - "Last Tuesday": it no longer brings out-of-range recent episodes (3 without a topic and 6 with one, to 0 and 0).
    - A salience-1.0 unshared anniversary in a 367-day store comes to mind (ruling 65).
    - The familiar person keeps 8 state memories and 8 episodes, and now also brings 11 observations made on those occasions (principle 5, ruling 69).
  - Held: on-topic survival under same-place pressure (0 of 410 below the scene-words numbers); latest-N without keys (90 of 90); the familiar person's state; the ordinary unshared anniversary; no reminder leak in shared interpretation; activity pressure (topic 8 to 6, 6, 5 at floors 0, 1, 2, unchanged).
  - Falsifier fired: the share of the pack taken by an unlived topic at home rose from 16 of 20 to 30 of 31. Attribution at the intermediate tips separates two movements:
    - At Task_2 (places), the home key's exclusive contribution fell from 4 to 1 and the pack stayed at 20. That is principle 1 replacing the old per-key root cap with one reserved place slot: the intended change.
    - At Task_3 (ObservedIn), the pack grew from 20 to 31 (32 without the key). The empty observation section filled with 11 or 12 observations that rest on the admitted episodes' occasions. That is principle 5 working as designed.
    - What it amplifies is not new: every topic hit for an unlived topic was already a near-zero-similarity neighbour (semantic on-topic 0 at both pins, similarities at most 2.4e-6 under deterministic embeddings). A least-bad neighbour now brings its observations too.
    - The falsifier measured a share of overlapping cue kinds. That share is blind to pack growth and counts the key's shrinking reminder as a rise, so the literal trigger is explained, not a regression of the design.
  - Residuals, recorded and not fixed here:
    - An unlived topic still fills spare room with near-zero neighbours, now through observations as well. The fix is a relevance floor on topic hits, which needs calibration against real embeddings (ADR-I-0036 already says the synthetic results establish no similarity bound).
    - With an activity and two people present, the second person brings one state memory instead of two at every activity floor.
    - The oldest salience-1.0 belief about a place no longer comes under the place key alone, and deliberately asking still brings it: presence is not aboutness (ruling 70).
    - The busy-occasion selection still depends on identifier order in one order, as before.
  - Timing is open: the AFTER medians are about 1.5 times the BEFORE across nearly every family, including families whose packs did not grow. But the two captures were taken hours apart, with full test suites running on the machine during the AFTER. A controlled interleaved timing at both pins is running, and the timing reading is appended when it lands.
  - What it means for the character: being keyed into a setting no longer skews what a present person brings or what the character believes; a range asked for is a range kept; what was observed at an occasion comes with it. Asked about something it never lived, the character still reaches for irrelevant memories, and now more of them.
  - Plan delta (what changed): none to the design. Lesson for falsifiers: name the quantity that matters to the character (the count of irrelevant memories, the pack size), not a share of overlapping cue kinds.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.

## Notes
- Risks:
  - Treating a keyed place as a reminder removes the only route by which a setting key brings where things stand at full strength. A deployment that keys only the place and names no one now gets place memories at floor and unclaimed room. The keyed-setting family measures this.
  - Raising the anniversary to the room adds cue-less roots on anniversaries, and ties to the newest keep them behind the latest occasions.
- Edge cases, with the expected result:
  - A memory reached by a setting key and a participant key reports both kinds, holds either floor, and expands through the participant road.
  - A range and an anniversary on one retrieval share the date-match floor from the range's order, then the shared anniversary's.
  - An empty store brings nothing and reports no kind.
  - With no expanding root, no spare turns are taken, and the root cap fills in the one order.
