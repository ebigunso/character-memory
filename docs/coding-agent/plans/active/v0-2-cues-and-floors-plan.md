# Plan: every cue the scene gives can bring its memories, and no cue drowns another

- status: in_progress
- generated: 2026-09-21
- last_updated: 2026-09-21
- work_type: code

## Goal
- When a character arrives somewhere, who is there, where it is and what it is doing bring memories to mind as surely as the topic does. A loud topic cannot crowd them out, and neither can one of them crowd out another. A notion that is part of nearly every experience, the character itself above all, brings its latest occasion, with further occasions admitted as its distinctiveness increases.

## Definition of Done
- A retrieval can say what the character is doing: the id of a thread or an open loop, beside the topic, echoed in the result. It brings what belongs to that thread or loop as the application's ordinary writes recorded it, with no topic needed. Inferring it when absent belongs to the next plan.
- The trace names, for every admitted memory, the set of cue kinds that admitted it, from the closed vocabulary the decider ruled. This plan produces topic, participant, place and activity, and ships only those; the other four arrive with the plans that build their routes. A memory reached by several cues names all of them.
- A notion's weight as a cue falls with the share of experiences it takes part in, on every link path a write can make. A notion in nearly every experience admits its latest occasion, whether it is the self, the one user of a one-to-one deployment, or a room everything happens in. No identity is special-cased (ADR-D-0020).
- Among the cue kinds present in a retrieval, none is starved by another at the three places the census found it can happen: where several content searches merge into one candidate list, where roots are chosen for expansion, and where admitted memories compete for a section. When the room that exists cannot hold every kind's floor, what yields is stated and deterministic.
- A retrieval that gives only a topic, or only one kind of cue, selects what it selects today.
- The floors ship with provisional values and the plan does not complete on them: it completes when the companion repository has measured them at a pinned commit of this slice and the measured-floors record is proposed for the decider.

## Planner-added requirements
- The activity enters beside the topic, not inside `Scene`. Needed because: `Scene` is shared with writes, where an activity field had no reader and was removed; what the character is doing now is a retrieval input only.
- The share of a ubiquitous notion is measured against experiences, not against link counts. Needed because: the census formula counts edges, so in a store where five people attend every episode the notion present in all of them still takes only a fifth of the edges and admits many neighbours; covering one more link path would not change that.
- The root cap is a third place a floor applies. Needed because: a memory protected at the candidate merge is lost again when stronger topic hits take every root, and twelve named participants leave no content root at all.
- A slice-level calibration is handed to the companion repository. Needed because: its final re-measurement needs every route, which this plan does not build; floors measured only then would leave this plan's values provisional for the rest of the phase.

## Scope / Non-goals
- Scope: `src/**`, `tests/**`, `README.md`, `docs/design/database/**` where a stored shape changes, one decision record (the measured floors), this plan.
- Non-goals: the self identified at construction, with its first readers (elapsed time since a pair last met, and the ruling that a pair cue needs a counterpart other than the self); the time route, staleness as age; scope keys, the state route, the setting key and custom values as cues; resolution as an end of currency; one-hop re-cueing from surfaced state; inferring the activity; the order of memories within an activity's thread (all the next plan). Naming in the result which reference brought a memory ("because Bob is here"): a question for the decider, not this slice. Due dates, triggers, actor and counterpart, the single Preference subtype (the prospective-memory plan). The record of which route each cue kind travels, written when the routes are complete. The draft's criterion that selectivity applies beyond entity roots (its own measurement). Write-path warnings; the renderer and the example loop.

## Design
- Chosen: a set of cue kinds travels with each candidate from where it is found to where it is admitted, and floors apply where starvation happens. Structure: the census shows three choke points, the merge of content searches under one candidate cap, the root cap, and the per-section caps. At each, every cue kind present in the retrieval is guaranteed room up to its floor, and room a kind does not use returns to the common pool, so a retrieval with one kind of cue is unchanged. A memory that carries several kinds satisfies each of them with the one slot it takes. When the room cannot hold one slot per kind present, kinds are served in a fixed order, the scene's cues before the topic (participant, place, activity, topic), because the topic is the cue that needs least help. Floors for participant, place and activity are separate, not one shared floor for the entity route: the census shows a participant's description and the place starving each other with no topic present, so a shared floor would not meet ADR-D-0022's test. Evolution: the next plans' routes (time, state, trigger) report their kinds into the same set and take floors by the same rule. Verification: service-free on the embedded stores, with constructed starvation at each choke point; values measured by the companion repository. Operation: no extra model call and no extra store read for the bookkeeping. Human: the trace says why each memory came to mind, for a developer and for measurement. Safety: nothing is withheld; a floor only guarantees room.
- Alternative: one budget per cue kind end to end. Rejected for now: it multiplies every cap by the number of kinds and fixes numbers before anything is measured.
- Alternative: cue kinds in the result. Rejected for this slice: the ruling puts them in the trace; a bare kind is thin knowledge, and with two people present "participant" says less than the result already does through each memory's recorded scene and each reference's outcome. Naming the reference itself is a richer idea and the decider's to weigh.
- The activity's reach, chosen: read what ordinary writes already record, a memory's own thread list and an open loop's threads and sources, through the field-backed queries the graph port already has. Alternative: derive membership links at commit from those fields, as was done for beliefs about a notion. Rejected here: it changes the stored graph of every existing fixture with thread affiliations and moves the measured baselines for a gain the query already gives; it stays available if a later route needs traversal through membership.
- The ubiquitous notion, chosen: one inverse-frequency rule over experiences for every notion. Alternative: drop the self from the roots by identity. Rejected by ADR-D-0020, and it leaves the one-user case untouched. The rule lowers a root's reach to a minimum of one occasion, selected by recorded Scene.time. Direct episode and observation routes share that episode budget, with at most one observation per selected episode on the observation route; missing-statistics fallback selects the same latest occasion.
- Why chosen: smallest shape that makes ADR-D-0022's invariant true for the cue kinds that exist and gives the later routes a rule to join. Fit: v0.2 draft sections 2 and 3; ADR-D-0020, D-0022, D-0029, D-0038, ADR-I-0022, I-0029.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface: `RetrievalContext`; the retrieval result (the echoed activity) and trace; selectivity statistics.
- stance: break
- justification: no external consumers; the one locatable consumer is `CharacterMemoryEvals`, the public companion evaluation repository whose tooling is a development aid and not core library functionality, and its plan follows each library slice. No stored-data migration; statistics are derived and rebuilt.

## Context (workspace)
- Related files/areas: the routes census `.agent-work/researcher/v0-2-routes-census-report.md` in the main checkout, sections 1, 2, 6, 7 and 8; the plan reviews `.agent-work/reviewer/v0-2-cues-plan-review.md`. `src/usecases/retrieve.rs`, `src/usecases/retrieve/scene.rs`, `src/api/types/retrieval.rs`, `src/policy/retrieval_selectivity.rs`, `src/policy/graph_expansion.rs`, `src/ports/retrieval_stats.rs`, `src/ports/graph_authority.rs`.
- Existing patterns or references: `SectionCueScoreSource` holds one source and `DirectMatch` loses which search supplied it; `RationaleCategory::Temporal` exists and is never produced; selectivity counts entity, relation and object edges, its buckets are About, Involves and PartOfThread, and `remember` links a participant to an observation through Mentions, outside them; bounded expansion follows link objects only, while a memory's thread list and sources are fields with their own queries.
- Design record consulted and deviations from its acceptance: ADR-D-0018, D-0020, D-0022, D-0029, D-0038, ADR-I-0020, I-0022, I-0029. No deviation. ADR-D-0020's construction-time identity of the self is still not implemented and moves to the next plan with its first reader. The companion repository's cue vocabulary differs from the ruled one; its plan reconciles when it follows this slice.

## Open Questions (max 3)
- None blocking. For the decider at the slice boundary: whether the result should name which reference brought a memory.

## Assumptions
- A1: A set of cue kinds per candidate survives the content merge, root selection, the winning-score merge and section assignment with no extra store read. Source: Tier D plan review; checked by Task_1.
- A2: The graph port's existing thread and provenance queries give an activity its reach with no new stored link. Source: Tier D plan review, R3; checked by Task_1.
- A3: The statistics store can count experiences per notion, across every link path, without changing what the existing buckets report for the measured cases of ADR-I-0022. Source: census section 6; checked by Task_2.
- A4: Reservation with return to the pool leaves a topic-only and a single-kind retrieval unchanged. Source: unverified; shown by Task_3 against the scene stack's tip.

## Tasks

### Task_1: A retrieval can say what the character is doing, and the trace says which cues brought each memory
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: []
- description: |
  `RetrievalContext` gains the activity beside the topic: the id of a thread or an open loop, echoed in the result. It brings what belongs to it as ordinary writes recorded it: for a thread, the memories whose own thread list names it and what is linked to it; for an open loop, its threads and the experiences it rests on. An id that names nothing, or names something of another kind, is reported unknown and activates nothing; an activity that exists and whose memories are all filtered or out of reach is reported as found, with nothing admitted. The trace names, per admitted memory, the set of cue kinds that admitted it: topic for the topic search, place for the setting's words, participant for a root the scene named or a participant's description, activity for the activity. The set is kept through every merge. Only these four kinds ship. `RationaleCategory` and `SectionCueScoreSource` are reconciled with the vocabulary so the trace has one way to say why.
- acceptance:
  - A memory reached by the topic and by a participant names both kinds in the trace; one reached only through a description of a person names participant, only through the setting's words place, only through the activity activity.
  - With no topic, an activity naming a thread brings a memory that an ordinary write affiliated to that thread through its own thread list and no authored link; an activity naming an open loop brings the experiences it rests on.
  - An unknown activity id and one of the wrong kind are reported unknown; an existing activity with nothing admissible is reported found.
  - The activity is echoed in the result; nothing in the trace says the same thing two ways.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the retrieval input and the trace vocabulary"

### Task_2: A notion that is in nearly every experience cues its latest occasion
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: [Task_1]
- description: |
  A notion's share is measured against experiences, not against link counts, and the cap on an entity root's neighbours that follows from it covers every link path a write can make for a participant, including the one to an observation that an ordinary `remember` makes. One rule for every notion. It lowers a ubiquitous root's reach to its latest occasion by recorded Scene.time, shared across the episode and observation paths. The conservative fallback when statistics are missing or unhealthy has the same minimum and is stated in the README.
- acceptance:
  - In a store of a few dozen experiences with several participants each, where one notion takes part in all of them and another in a few, a retrieval naming both brings the second one's memories and the latest occasion through the first, on the `remember` path and on a caller-built path alike.
  - The same holds whether or not the ubiquitous notion is the one the application thinks of as the character.
  - The measured cases of ADR-I-0022 keep their results, or each difference is listed with its cause.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, tracing the census section 6 formula and coverage against the change"

### Task_3: No cue kind present is starved by another
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/decisions/**
- depends_on: [Task_2]
- description: |
  Floors per cue kind at the candidate merge, at root selection and at the section caps, by the rule in the Design section: room up to the floor for each kind present, unused room back to the pool, one slot satisfying every kind its memory carries, and a fixed order (participant, place, activity, topic) when the room cannot hold one slot per kind. The existing hard caps stay; no new tunable is added beyond the floor values. The values ship provisional and are named so. The trace shows when a floor, not its score, admitted a memory. The measured-floors record is proposed at plan closeout, from the companion repository's calibration, not by this task.
- acceptance:
  - With a topic that alone fills the candidate cap, the roots and a section, a memory reachable only through a participant, one only through the place and one only through the activity are each admitted, and each would have been lost at a different one of the three choke points without its floor.
  - With no topic, a participant's description and the place, competing for one section, each keep their floor.
  - A section or root cap smaller than the number of kinds present serves kinds in the stated order, the same way every time; a cap of zero admits nothing and is not an error.
  - A topic-only retrieval and a retrieval with one kind of cue select the same ids, in the same sections and order, as before this task.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the floor rule against ADR-D-0022 and ADR-I-0022"

### Task_4: The floors are measured
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/decisions/**
- depends_on: [Task_3]
- description: |
  Hand the pinned commit of Task_3 to the companion evaluation repository with what a calibration needs: pressure on each kind whose floor is claimed (participant, place, activity) under a loud topic and against each other, read from the trace's cue kinds, with a census of what ran and what did not. The companion repository plans and runs it under its own plan. From its result the orchestrator sets the floor values, and a worker makes them the defaults in this task: the values, the removal of the provisional naming, and the tests that named them. The orchestrator proposes the measured-floors record in the pattern of ADR-I-0022. This does not replace the companion's one final re-measurement of the continuity baselines.
- acceptance:
  - The floor values in the code are the measured ones and nothing calls them provisional.
  - The proposed record states the corpus, the pressure applied to each kind, and what would reopen the values.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review of the value change"
  - kind: review
    required: true
    owner: orchestrator
    detail: "The calibration ran at the pinned commit, native cue evidence was used, and the executed and not-run census is recorded"
  - kind: manual
    required: true
    owner: user
    detail: "The decider accepts or returns the measured-floors record, in a batch at the slice boundary"

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]
- Wave 3: [Task_3]
- Wave 4: [Task_4]

One crate, shared files: sequential, one worker at a time, each PR stacked on the previous one, the first stacked on the scene stack. The ubiquitous-notion rule lands before the floors so that what the floors are measured against does not change afterwards. After each task the library commit is handed to the companion evaluation repository, whose own plan schedules what follows.

## Rollback / Safety
- Each task is one PR and reverts on its own in reverse order; once the companion repository has followed a slice, a revert is coordinated with it.

## Progress Log (append-only)

- 2026-09-21 Task_1 implemented (0da57d3) and approved at Tier D and Tier A: the activity beside the topic, typed and echoed as found or unknown, reaching what ordinary writes record with no new stored link; one set of cue kinds per admitted memory in the trace, replacing two older vocabularies (net about 200 lines removed). A thread's members enter most recent first. Until Task_3, a large thread can take every root after the participants; the README says so.
- 2026-09-21 Task_2 implemented (16692bd) and approved at Tier D: a notion's share is its distinct episodes over all episodes, counted once per episode however the write linked it, on the two paths that lead to experiences; in a store of 24 occasions with five participants each, the notion present in all of them went from 15 admitted observation edges to none on the `remember` path and from 3 episode edges to none on a caller-built one, while a rare participant kept all of its three and the belief about the ubiquitous notion stayed.
- 2026-09-21 Task_3 implemented (4eec349) and approved at Tier D and Tier A: floors per cue kind at the candidate merge, root selection and the section caps, served by one helper in successive rounds in the order participant, place, activity, topic, with unused room returned and original order kept. Three cue-only memories each lost at a different choke point before the change are admitted after it; topic-only and single-kind retrievals are unchanged. Values are 1 per kind and provisional, at `context.cue_floors`.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-21 Decision: the routes work is two plans, and this is the first.
  - Trigger / new insight: the routes census showed the pipeline has content recall and participant roots and nothing else, that per-item evidence cannot carry more than one cue, that cue kinds can starve each other at three places, and that the cap on a ubiquitous root neither covers the link `remember` makes nor measures the right quantity.
  - Plan delta (what changed): this plan makes the cue kinds that exist today reportable and unstarvable; the next plan adds the self's identity, the time and state routes, scope keys, elapsed time, staleness, resolution and activity inference on the same rule.
  - Tradeoffs considered: one plan for all of it was rejected as too large to review well.
  - User approval: plan approval waived for plans inside the rulings; decisions judged by character behavior are logged for presentation.
  - Record proposed: one, the measured floors, at Task_4.
- 2026-09-21 Decision: the draft was revised after its Tier D review and its combined Tier A review and value audit, before dispatch.
  - Trigger / new insight: the Tier A pass named tunnel vision: the draft fixed which link paths the ubiquitous-root cap covers when the quantity it measures was wrong (edges, not experiences); it claimed no starvation while protecting two of three choke points; it moved cue kinds into the result against the ruling without saying so; and it gave the self's identity a task whose only use was an exception by identity, which ADR-D-0020 forbids. Tier D showed the root cap erasing the candidate floor, a no-topic promise that contradicted the floors, an activity that could not reach what ordinary writes record, and a measured-record gate that no one could meet before the phase's end.
  - Plan delta (what changed): cue kinds stay in the trace and only the four produced kinds ship; the self's identity moves to the next plan with its first readers; the ubiquitous-notion rule measures experiences and lands before the floors; floors apply at three choke points with a stated order when room is short; the activity reaches through existing field-backed queries; the measured record moves to a closing task with an owner.
  - Tradeoffs considered: deriving thread-membership links at commit was rejected because it moves every measured baseline for a reach the existing query already gives.
  - User approval: not required; decided under the standing instruction and logged for presentation.
  - Record proposed: unchanged.

- 2026-09-21 Decisions taken inside Task_2, judged by character behavior.
  - One experience is one distinct episode: someone who spoke ten times in one conversation was present on one occasion, and an observation-level unit would measure verbosity.
  - The unit follows what a bucket admits. The share of experiences replaces the share of links only on the paths from a notion to experiences. Beliefs about a notion and thread memberships keep their measured rule: being always there must not silence what the character knows about someone, and bringing the current state about who is present is the state route of the next plan.
  - No statistics rebuild and no migration: a statistics store written before this change has no episode index, which means missing statistics and the conservative fallback, never zero counts, even after later writes. Known gap for a later plan: nothing rebuilds statistics from the graph.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.

- 2026-09-21 Decisions and known risks from Task_3, for the calibration in Task_4.
  - Floors are served in successive rounds, one slot per kind per round: filling one kind's whole floor first would starve a kind that could have had a slot, and the short-room rule is the first round of the same loop.
  - The floors value sits on the context, not on the candidate limits, because it governs the section caps too. It is the calibration knob and a measured default, not something an application is expected to set, and a floor is per cue kind, not per person or place.
  - Known risk: there is no relevance threshold under an admission floor. A search for a description or for the setting's words always returns neighbours, so in a place the character has never been the place floor admits the least-bad place memory. The calibration measures pollution from floor admissions by score, not only starvation; the remedy may be a minimum score for floor eligibility.
  - Known property: at the default expansion depth a memory from the same occasion inherits the participant kind, so a topic-found memory can satisfy the participant floor on its own. That is defensible behavior and it means a reserved kind does not imply independent direct recall. The trace does not distinguish a kind a memory was found by from one it inherited; the calibration derives that from the existing candidate, root and relation traces, and a field is added only if that proves impossible.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: the measured floors, at Task_4.

## Notes
- Risks: admission changes move the pollution and context-size baselines of ADR-I-0022; the companion repository re-measures once, at its own closing task. A calibration corpus authored for other pressures may not isolate these three floors; Task_4 names the pressure it needs.
- Edge cases, with the expected result: a scene whose only participant is ubiquitous brings the latest recorded occasion with that participant; asking about a day still needs the time route; an activity naming a closed thread is found and brings its memories, though the thread itself has no pack section today; a memory carrying every kind takes one slot and satisfies all four floors.
