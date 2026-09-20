# Plan: the scene, on every write and as the retrieval input

- status: draft
- generated: 2026-09-21
- last_updated: 2026-09-21
- work_type: code

## Goal
- A write records the scene it happened in, and a retrieval is asked from a scene: when, who, where, what, and optionally a topic. Every admitted memory comes back with its scene as recorded, and the result says what scene it was given and how complete that was. Nothing is withheld by the scene (ADR-D-0038). Durable writes take one turn at a time, so the derivation this slice adds at commit cannot interleave.

## Definition of Done
- Among calls that share one facade, a commit, a correction, a forgetting and a direct link each take one serialized turn. Embedding happens before the turn is taken, so no operation waits on a model call made by another. Recall takes no turn and never waits for one; it may still observe a write in progress, and the README says so.
- A write gives its scene as perceived: the time, participants by identity key, a setting by key or in words or both, an activity by the id of a thread or open loop, and custom values the application already owns. The scene is stored on the episode, in one place, with each part individually queryable, and remember and a caller-built plan agree on it; a correction's replacement keeps the scene of what it corrects.
- A retrieval gives a scene and an optional topic. Only the time is required and defaults to now. A participant given by key cues its notion; by name, every notion currently known by that name; by description, whatever memory holds whose text matches in content. An ambiguous name activates each notion it could mean and an unknown one activates nothing, and the trace says which; a description is a content cue and is never reported as known or unknown. With no topic, no topic is embedded. The setting, the activity and the custom values of a retrieval's scene are reported back and cue nothing in this slice: their readers (scope keys, the activity and place cues) arrive with the routes plan.
- Each admitted memory reports its scene as recorded, and the result reports the present scene as given and whether it was partial; both are in the result whether or not the trace is requested. An interpreted memory reports the scenes of the experiences it rests on, oldest first up to a stated bound with the remainder counted, whether or not those experiences were themselves admitted. A memory with no recorded experience behind it (a thread, a belief given by the application, and its corrections) reports no scene, explicitly; the present scene is never substituted. There is no retrieval option that omits by scene and no computed verdict about audience.
- Entity selectivity has its production input again: a participant given by key or resolved by name is an entity root.
- The README describes what ships; the companion evaluation repository's first Task_5 step has the library commit it needs.

## Planner-added requirements
- The serialized write turn comes first in this plan. Needed because: this slice adds work at commit (scene storage on three paths, reference checks), and the planning census mapped interleavings between every pair of writers that such work widens; ADR-I-0035 already requires the turn for the generation phase, and the decider ruled on 2026-09-21 that it lands here.
- Scope keys are NOT derived in this slice. Needed because (as a deletion from the draft's list for this slice, not an addition): their only reader is the state route, which belongs to the routes plan; a key nothing reads is the pattern the groundwork just deleted. The scene stored here is the single authored source they will be derived from.
- `source_conversation_id` on the episode is replaced by the scene's setting key. Needed because: a conversation is a setting (v0.2 draft section 1), and keeping both would store one fact twice.

## Scope / Non-goals
- Scope: `src/**`, `tests/**`, `README.md`, `docs/design/database/**` where the stored shape changes, up to two decision records, this plan.
- Non-goals: the time, state and trigger routes, admission floors, the cue kinds named in the trace, elapsed time since a pair last met, staleness as age (the routes plan); scope keys and the setting and custom-value cues that read them (with the state route); the activity as a cue, and inferring the activity from the conversation's recent episodes; setting words as searchable content; direction, due and trigger on open loops and commitments, and the single Preference subtype (the prospective-memory plan); write-path warnings; the renderer and the example loop; names or descriptions of people on the write side (consolidation, v0.3); sameness and containment read-through; a reader-side snapshot guarantee for recall (v0.3); whether `api` and `domain` stay public modules; un-suppression.

## Design
- Chosen: one `Scene` value, given as perceived, used on both sides. On a write it is stored on the episode; an interpreted memory has no scene of its own and reports the scenes of its sources. On a retrieval it is the input, beside an optional topic; the facade resolves nothing for the caller and exposes no lookup (ADR-D-0029, ADR-I-0020). References are resolved inside recall through current beliefs: a key is an entity id, a name goes through the exact-name query the groundwork added, a description is a content cue over the text memory already embeds, so it reaches a scene only through what an episode's text says; setting words are stored and reported and not embedded, which keeps the measured baselines of ADR-I-0022 where they are. Description matching is provisional and judged by measurement (ADR-D-0029 leaves it uncovered). Structure: the scene lives in one place per memory; write paths share one conversion; recall gains one step, reference resolution, before the existing expansion, which starts from the resolved notions as entity roots. Evolution: the routes plan adds cue kinds and floors over the same input with no shape change; scope keys are derived later from what is stored here. Verification: service-free; resolution, partial scenes and reporting are testable on the embedded stores; the companion repository's situated scenarios are the acceptance instrument as its adapter forwards each field. Operation: a retrieval with a named participant costs one graph query per name; a description costs one vector search. Human: a consumer passes what the character perceives and reads back what was recorded. Safety: no gate, no verdict (ADR-D-0038).
- Alternative: separate scene types for writes and retrievals. Structure: two conversions and two vocabularies for one idea. Rejected: the v0.2 draft defines one scene, and the difference (a write has no topic; a write-side participant is a key) is validation, not shape.
- Alternative: store the scene on every memory, interpreted ones included. Structure: a second copy of what the sources record, to be kept in step. Rejected for the same reason the stored current flag was: one authored source, read through.
- Alternative: resolve references in the application through a lookup surface. Rejected by ADR-I-0020 and ADR-D-0029.
- The serialized turn, chosen: one async mutex owned by the facade, held for a write operation's graph, vector and stats work, so a late vector or stats write cannot land after a newer turn's. The text to embed is known before the turn, so embedding happens first and the turn holds no model call (ADR-I-0035: nothing waits on a model); an embedding spent on a write that is then rejected is the accepted cost. `remember` and `commit` take the turn once between them. Alternative: conditional graph writes per operation (compare-and-set on revision). Rejected: it closes the collision races only, leaves the late vector and stats families open, and must be repeated in every write path. Recall takes no part in the mutex. The turn coordinates calls through one facade; separately constructed facades, and a service-mode vector store shared by several processes, are outside it, and the README says so. It does not give recall a snapshot: that part of ADR-I-0035 stays with v0.3.
- Why chosen: smallest shape that gives the scenarios their input and the routes plan a fixed base. Fit: v0.2 draft sections 1, 3 and 9; ADR-D-0029, ADR-D-0034, ADR-D-0038, ADR-I-0020, ADR-I-0035.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface: `RememberInput`, `EpisodeDraft`, `Episode`, `RetrievalContext`, the retrieval outcome and trace, the graph vocabulary for episodes.
- stance: break
- justification: no external consumers; the one locatable consumer is `CharacterMemoryEvals`, the public companion evaluation repository whose tooling is a development aid and not core library functionality, and its plan follows each library slice (library first, harness immediately after). No stored-data migration.

## Context (workspace)
- Related files/areas: the groundwork census `.agent-work/researcher/v0-2-groundwork-census-report.md` in the main checkout, question 4 (what remember and retrieve already take that a scene subsumes, and the three construction paths a write-time step must cover) and question 5 (what each facade method reads and writes, and the interleaving families). `src/memory.rs`, `src/composition.rs`, `src/api/types/{write_plan,draft,retrieval}.rs`, `src/domain.rs`, `src/usecases/{write_planning,remember,correct_forget,link,retrieve}.rs`, `src/policy/{graph_expansion,retrieval_selectivity}.rs`, `src/adapters/oxigraph/*`.
- Existing patterns or references: the exact-name query on the graph authority port and the derived-at-commit pattern from the groundwork; `RetrievalContext.current_context` and `query_text` are concatenated into one embedding today; `RememberInput.participant_entity_ids` and `started_at` are what a scene subsumes on the write side; `entity_ids`, `thread_ids`, `ended_at` and an observation's own time and speaker are distinct facts and stay.
- Design record consulted and deviations from its acceptance: ADR-D-0020, D-0022, D-0024, D-0029, D-0034, D-0038, ADR-I-0016, I-0020, I-0022, I-0035. No deviation; scope keys (ADR-D-0024) are deferred to their reader, not dropped.

## Open Questions (max 3)
- None for the decider. The rulings of 2026-09-21 cover this slice; plan approval is waived for plans inside them, and this plan is reviewed (Tier D and Tier A) before dispatch.

## Assumptions
- A1: One facade-owned async mutex serializes every write without deadlock. `remember` calls `commit` today, so the two share one acquisition below the facade methods; no other write calls another. Source: Tier D plan review; checked by Task_1.
- A2: The content cue for a description reuses the existing vector search over memory text unchanged; nothing new is embedded. Source: groundwork Task_3; checked by Task_3.
- A3: The ADR-I-0022 continuity baselines do not move for retrievals that give only a topic. Source: unverified; Task_3 keeps selection, sections, order and the embedded query text unchanged against the shape Task_2 leaves, and the companion repository measures it.

## Tasks

### Task_1: Durable writes take one turn at a time
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/decisions/**
- depends_on: []
- description: |
  One serialized turn per write operation (commit and remember, correct, forget, link) within a process, among calls sharing one facade, held across the operation's graph, vector and stats work, including a correction retry that only repairs vectors or stats. Embedding happens before the turn. Recall and prepare take no part. Show with tests that the interleavings the census named cannot occur between two writers: the collision preflight race, a late vector write or delete landing after a newer turn, a late stats state overwriting a newer one (objects visible without their links is already closed by the single graph batch). Say in the README what is and is not serialized (calls through one facade; not separately constructed facades, not a service-mode store shared across processes), that a consumer's embedding provider may impose its own limits, and that recall may observe a write in progress. Propose an ADR if the admission test passes and ADR-I-0035 does not already carry the decision.
- acceptance:
  - Concurrent writers through one facade produce the same stores as some serial order of them, shown for at least: two commits with one id, a commit racing a forget of the same memory, a correction racing a link.
  - No writer or reader waits on an embedding call made by another operation, shown with a stalled provider.
  - Concurrent writes all complete; none deadlocks, `remember` included.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; trace each census interleaving family against the change"

### Task_2: A write records the scene it happened in
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/design/database/**
- depends_on: [Task_1]
- description: |
  The `Scene` value and its place on a write: the time, participants by identity key, a setting (key, words, or both), an activity (the id of a thread or an open loop), custom values. Stored on the episode, once, each part individually queryable in the graph (the routes plan derives scope keys from it; memories committed before then carry none, acceptable with no consumers). Dispositions of today's carriers: `source_conversation_id` becomes the setting key (an application with both a place and a session puts the session in the custom values); `participant_entity_ids` become the scene's participants and yield the links they yield today; `started_at` becomes the scene's time, and an observation's time still defaults from it while an explicit one is kept; `entity_ids` (notions involved, not present), `thread_ids` (affiliation, possibly several), `ended_at`, and an observation's speaker stay as they are. The activity is stored and reported and yields no link in this slice. A scene stated on an `EpisodeDraft` wins over the one on `RememberInput`, as explicit draft values do today. Remember and a caller-built plan share one conversion; a correction authors no episode scene, and a corrected episode keeps its scene unless the replacement states one. The correction guard that compares the original source uses the setting key, never the setting's words; rename `original_source_ref` to match, or cut it if no caller needs it. A participant key must name an existing or same-plan entity; an activity must name an existing or same-plan thread or open loop.
- acceptance:
  - A scene given on a write round-trips through the graph exactly, including a setting given only in words and custom values, through remember and through a caller-built plan, and survives a correction of its episode.
  - One conservation test covers involved notions different from participants, several thread affiliations, an interval, and an explicitly timed observation: each yields what it yields today.
  - A write with only a time is valid; a participant given by name or description on a write is rejected with an error that says resolution on the write side is not supported yet.
  - Nothing that read `source_conversation_id` or the old hints reads a second copy; the correction path's source-conversation check uses the setting key.
  - The schema reference documents describe the stored scene.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the public Scene shape against ADR-D-0029 and the v0.2 draft section 1"

### Task_3: A retrieval is asked from a scene
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/decisions/**
- depends_on: [Task_2]
- description: |
  `RetrievalContext` takes a scene and an optional topic; `query_text` and `current_context` become the topic (a caller that used both composes them itself). The time is required and defaults to now. References are resolved inside recall: a key is an entity root; a name goes through the exact-name query and may yield several notions (ambiguous) or none (unknown); a description is a content cue. Resolved notions are entity roots for the existing expansion, which gives entity selectivity its production input back. A key or a name enters as a root with a stated strength and is never presented as a vector hit. With no topic the topic search is skipped, not run on empty text; a description is still embedded as its own cue; with neither and no resolvable participant the pack may be empty, since the time and state routes are later. The setting, activity and custom values are reported back and cue nothing yet. Each admitted memory reports its scene as the Definition of Done states (sources' scenes bounded and ordered; none, explicitly, where no experience is recorded), in the result itself; the result also carries the scene as given and whether it was partial, and the trace adds how each reference resolved. No option omits by scene. Propose an implementation record for the scene's retrieval-level shape if the admission test passes.
- acceptance:
  - A retrieval with a participant by key and no topic returns memories involving that notion through expansion, with their scenes reported; by name it does the same, reports ambiguity when two notions share the name and unknown when none bears it; by description it reaches a notion through a belief found by content.
  - A topic-only retrieval selects the same ids, in the same sections and order, from the same embedded query text, as before this task; shown against the shape Task_2 leaves.
  - With no topic, an embedder that fails on the absent topic is never called for it, while a description is embedded; an interpreted memory resting on two episodes, one of them not admitted, reports both scenes; a thread and an application-given belief report no scene.
  - A scene with only a time is accepted and reported as partial; nothing is omitted because of any part of the scene.
  - The facade exposes no way to look a notion up by name.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the retrieval input, the trace additions and any proposed record against ADR-D-0029, D-0038 and ADR-I-0020"
  - kind: manual
    required: true
    owner: user
    detail: "The decider accepts or returns each proposed record, in a batch at the slice boundary"

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]
- Wave 3: [Task_3]

One crate, shared files: sequential, one worker at a time, each PR stacked on the previous one, the first stacked on the groundwork stack. After Wave 3 the companion repository forwards the scene, the reference time, the absent topic and the reference outcomes, and its situated scenarios begin to run.

## Rollback / Safety
- Each task is one PR and reverts on its own in reverse order; once the companion repository has followed a slice, a revert is coordinated with it.

## Progress Log (append-only)

- (none yet)

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-21 Decision: the rulings this plan rests on, given by the decider before drafting.
  - Trigger / new insight: the decision list for the rest of the phase.
  - Plan delta (what changed): what a write records of the scene; the serialized write turn at the start of this slice; no recall-side partition and no computed audience verdict (ADR-D-0038 replaces ADR-D-0019); sameness and containment read-through not in this slice; plan approval waived for plans inside the rulings. The planner defers scope keys to the routes plan, where their reader is.
  - Tradeoffs considered: in the Design section.
  - User approval: rulings 2026-09-21; plan approval waived.
  - Record proposed: up to two, in Task_1 and Task_3, each if its admission test passes.

- 2026-09-21 Decision: plan revised after the Tier D and Tier A plan reviews, before dispatch.
  - Trigger / new insight: both reviews asked for changes. Tier A: the turn held a model call against ADR-I-0035; the retrieval scene's setting, activity and custom values had no stated reader; the plan overstated what recall is protected from. Tier D: the write-side carriers a scene does not subsume had no disposition; a byte-for-byte promise contradicted the episode's shape change; memories with no recorded experience had no defined scene; scene words were promised as searchable with no producer.
  - Plan delta (what changed): embedding moves before the turn; setting, activity and custom values are reported and cue nothing until the routes plan; carriers are disposed of one by one in Task_2; the topic-only invariant is selection, sections, order and query text; no-scene is explicit for threads and application-given beliefs; setting words are not embedded.
  - Tradeoffs considered: making the activity thread an expansion root now was inside the rulings and was left to the routes plan, where the activity cue and its trace entry are defined together.
  - User approval: not required; inside the rulings, plan approval waived.
  - Record proposed: unchanged.
- 2026-09-21 Record of the decider's rulings for the rest of the phase, so later slice plans cite one place. A write records time, participants by identity key, setting as key or words or both, and custom values; names and descriptions of people on writes stay with v0.3; scope keys are derived at commit, never authored, and an interpreted memory takes the keys its sources share. The trace names cue kinds from one closed vocabulary (topic, participant, place, activity, recency, due, date match, trigger); recency carries salience as a weight; current state is not a cue; a pair cue needs a counterpart other than the self. An intention is an open loop or commitment with a trigger (a participant appearing or a topic arising), not a new subtype. Open loops and commitments carry actor, counterpart and an optional due instant, due on and after it until resolved or superseded. The two preference subtypes become one `Preference` whose holder is the actor. No un-suppression in v0.2. The reflection signal leaves v0.2 and returns with the v0.3 plan. Selectivity beyond entity roots is decided by measurement, either outcome accepted in advance. Floors and warning thresholds are set by the orchestrator in records the decider accepts; warnings only warn. v0.2 exits on the retrieval tier; the behavioral tier waits for v0.3. No recall-side partition and no audience verdict (ADR-D-0038).

## Notes
- Risks: an embedding computed before the turn is wasted when the write is then rejected. Reference resolution by description can be noisy; the routes plan's floors and the companion repository's scenarios are where that is judged.
- Edge cases: a name shared by the self and another notion; a participant key for a notion that exists but has no current name; an activity id naming a thread that was later closed.
