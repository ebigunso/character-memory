# Plan: the scene, on every write and as the retrieval input

- status: draft
- generated: 2026-09-21
- last_updated: 2026-09-21
- work_type: code

## Goal
- A write records the scene it happened in, and a retrieval is asked from a scene: when, who, where, what, and optionally a topic. Every admitted memory comes back with its scene as recorded, and the result says what scene it was given and how complete that was. Nothing is withheld by the scene (ADR-D-0038). Durable writes take one turn at a time, so the derivation this slice adds at commit cannot interleave.

## Definition of Done
- Within the process that owns the embedded stores, a commit, a correction, a forgetting and a direct link each take one serialized turn; recall is a reader and never waits on a turn's model-free work beyond the store's own locks.
- A write gives its scene as perceived: the time, participants by identity key, a setting by key or in words or both, an activity by the id of a thread or open loop, and custom values the application already owns. The scene is stored on the episode, in one place, and the three write paths (remember, a caller-built plan, a correction) agree on it.
- A retrieval gives a scene and an optional topic. Only the time is required and defaults to now. A participant given by key cues its notion; by name, every notion currently known by that name; by description, the notions and scenes memory holds that match in content. An ambiguous reference activates each thing it could mean and an unknown one activates nothing, and the trace says which. With no topic the content cue is simply absent; no empty text is embedded.
- Each admitted memory reports its scene as recorded (an interpreted memory reports the scenes of the experiences it rests on), and the trace reports the scene as given and whether it was partial. There is no retrieval option that omits by scene and no computed verdict about audience.
- Entity selectivity has its production input again: a participant given by key or resolved by name is an entity root.
- The README describes what ships; the companion evaluation repository's first Task_5 step has the library commit it needs.

## Planner-added requirements
- The serialized write turn comes first in this plan. Needed because: this slice adds work at commit (scene storage on three paths, reference checks), and the planning census mapped interleavings between every pair of writers that such work widens; ADR-I-0035 already requires the turn for the generation phase, and the decider ruled on 2026-09-21 that it lands here.
- Scope keys are NOT derived in this slice. Needed because (as a deletion from the draft's list for this slice, not an addition): their only reader is the state route, which belongs to the routes plan; a key nothing reads is the pattern the groundwork just deleted. The scene stored here is the single authored source they will be derived from.
- `source_conversation_id` on the episode is replaced by the scene's setting key. Needed because: a conversation is a setting (v0.2 draft section 1), and keeping both would store one fact twice.

## Scope / Non-goals
- Scope: `src/**`, `tests/**`, `README.md`, `docs/design/database/**` where the stored shape changes, up to two decision records, this plan.
- Non-goals: the time, state and trigger routes, admission floors, the cue kinds named in the trace, elapsed time since a pair last met, staleness as age (the routes plan); scope keys (with the state route); direction, due and trigger on open loops and commitments, and the single Preference subtype (the prospective-memory plan); write-path warnings; the renderer and the example loop; names or descriptions of people on the write side (consolidation, v0.3); sameness and containment read-through; a reader-side snapshot guarantee for recall (v0.3); whether `api` and `domain` stay public modules; un-suppression.

## Design
- Chosen: one `Scene` value, given as perceived, used on both sides. On a write it is stored on the episode; an interpreted memory has no scene of its own and reports the scenes of its sources. On a retrieval it is the input, beside an optional topic; the facade resolves nothing for the caller and exposes no lookup (ADR-D-0029, ADR-I-0020). References are resolved inside recall through current beliefs: a key is an entity id, a name goes through the exact-name query the groundwork added, a description is a content cue. Structure: the scene lives in one place per memory; write paths share one conversion; recall gains one step, reference resolution, before the existing expansion, which starts from the resolved notions as entity roots. Evolution: the routes plan adds cue kinds and floors over the same input with no shape change; scope keys are derived later from what is stored here. Verification: service-free; resolution, partial scenes and reporting are testable on the embedded stores; the companion repository's situated scenarios are the acceptance instrument as its adapter forwards each field. Operation: a retrieval with a named participant costs one graph query per name; a description costs one vector search. Human: a consumer passes what the character perceives and reads back what was recorded. Safety: no gate, no verdict (ADR-D-0038).
- Alternative: separate scene types for writes and retrievals. Structure: two conversions and two vocabularies for one idea. Rejected: the v0.2 draft defines one scene, and the difference (a write has no topic; a write-side participant is a key) is validation, not shape.
- Alternative: store the scene on every memory, interpreted ones included. Structure: a second copy of what the sources record, to be kept in step. Rejected for the same reason the stored current flag was: one authored source, read through.
- Alternative: resolve references in the application through a lookup surface. Rejected by ADR-I-0020 and ADR-D-0029.
- The serialized turn, chosen: one async mutex owned by the facade, held for the whole of a write operation including its vector and stats work, so a late vector or stats write cannot land after a newer turn's. Alternative: conditional graph writes per operation (compare-and-set on revision). Rejected: it closes the collision races only, leaves the late vector and stats families open, and must be repeated in every write path. Recall takes no part in the mutex. A service-mode vector store shared by several processes is outside what one process can serialize; the README says so.
- Why chosen: smallest shape that gives the scenarios their input and the routes plan a fixed base. Fit: v0.2 draft sections 1, 3 and 9; ADR-D-0029, ADR-D-0034, ADR-D-0038, ADR-I-0020, ADR-I-0035.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface: `RememberInput`, `EpisodeDraft`, `Episode`, `RetrievalContext`, the retrieval outcome and trace, the graph vocabulary for episodes.
- stance: break
- justification: no external consumers; the one locatable consumer is `CharacterMemoryEvals`, the public companion evaluation repository whose tooling is a development aid and not core library functionality, and its plan follows each library slice (library first, harness immediately after). No stored-data migration.

## Context (workspace)
- Related files/areas: the groundwork census `.agent-work/researcher/v0-2-groundwork-census-report.md` in the main checkout, question 4 (what remember and retrieve already take that a scene subsumes, and the three construction paths a write-time step must cover) and question 5 (what each facade method reads and writes, and the interleaving families). `src/memory.rs`, `src/composition.rs`, `src/api/types/{write_plan,draft,retrieval}.rs`, `src/domain.rs`, `src/usecases/{write_planning,remember,correct_forget,link,retrieve}.rs`, `src/policy/{graph_expansion,retrieval_selectivity}.rs`, `src/adapters/oxigraph/*`.
- Existing patterns or references: the exact-name query on the graph authority port and the derived-at-commit pattern from the groundwork; `RetrievalContext.current_context` and `query_text` are concatenated into one embedding today; `RememberInput.participant_entity_ids`, `thread_ids`, `started_at` are what a scene subsumes on the write side.
- Design record consulted and deviations from its acceptance: ADR-D-0020, D-0022, D-0024, D-0029, D-0034, D-0038, ADR-I-0016, I-0020, I-0022, I-0035. No deviation; scope keys (ADR-D-0024) are deferred to their reader, not dropped.

## Open Questions (max 3)
- None for the decider. The rulings of 2026-09-21 cover this slice; plan approval is waived for plans inside them, and this plan is reviewed (Tier D and Tier A) before dispatch.

## Assumptions
- A1: One facade-owned async mutex serializes every write without deadlock, because no write operation calls another facade write. Source: census question 5 method table; checked by Task_1.
- A2: The content cue for a description can reuse the existing vector search over interpreted memories and episodes, since beliefs about a notion are interpreted memories with text. Source: groundwork Task_3; checked by Task_3.
- A3: The ADR-I-0022 continuity baselines do not move for retrievals that give only a topic. Source: unverified; Task_3 keeps the topic-only path byte-for-byte and the companion repository measures it.

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
  One serialized turn per write operation (commit and remember, correct, forget, link) within a process, held across the operation's graph, vector and stats work. Recall and prepare take no part. Show with tests that the interleavings the census named cannot occur between two writers in one process: the collision preflight race, objects visible without their links, a late vector write or delete landing after a newer turn, a late stats state overwriting a newer one. Say in the README what is and is not serialized (one process, embedded stores; a service-mode store shared across processes is not). Propose an ADR if the admission test passes and ADR-I-0035 does not already carry the decision.
- acceptance:
  - Concurrent writers in one process produce the same stores as some serial order of them, shown for at least: two commits with one id, a commit racing a forget of the same memory, a correction racing a link.
  - Recall is not blocked for the duration of a write's embedding call, or the README states plainly that it is and why.
  - No facade write calls another facade write while holding the turn.
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
  The `Scene` value and its place on a write: the time, participants by identity key, a setting (key, words, or both), an activity (the id of a thread or an open loop), custom values. Stored on the episode, once; `source_conversation_id` and the separate participant, thread and time hints on `RememberInput` are replaced by it, with their present behavior kept (participants and the activity still yield the links they yield today). The three construction paths agree: remember, a caller-built plan, and a correction's replacement inherit or state the scene through one conversion. A participant key must name an existing or same-plan entity; an activity must name an existing or same-plan thread or open loop.
- acceptance:
  - A scene given on a write round-trips through the graph exactly, including a setting given only in words and custom values, on all three paths.
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
  `RetrievalContext` takes a scene and an optional topic; `query_text` and `current_context` become the topic. The time is required and defaults to now. References are resolved inside recall: a key is an entity root; a name goes through the exact-name query and may yield several notions (ambiguous) or none (unknown); a description is a content cue. Resolved notions are entity roots for the existing expansion, which gives entity selectivity its production input back. With no topic there is no content cue and nothing is embedded for it. Each admitted memory reports its scene as recorded, an interpreted memory reporting the scenes of its sources; the trace reports the scene as given, whether it was partial, and how each reference resolved. No option omits by scene. Propose an implementation record for the scene's retrieval-level shape if the admission test passes.
- acceptance:
  - A retrieval with a participant by key and no topic returns memories involving that notion through expansion, with their scenes reported; by name it does the same, reports ambiguity when two notions share the name and unknown when none bears it; by description it reaches a notion through a belief found by content.
  - A topic-only retrieval returns what it returns today, byte for byte in the pack, for the existing tests.
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

## Notes
- Risks: holding the turn across an embedding call makes writers wait on the provider; that is the accepted cost of not letting a late vector write land after a newer turn, and Task_1 measures it. Reference resolution by description can be noisy; the routes plan's floors and the companion repository's scenarios are where that is judged.
- Edge cases: a name shared by the self and another notion; a participant key for a notion that exists but has no current name; an activity id naming a thread that was later closed.
