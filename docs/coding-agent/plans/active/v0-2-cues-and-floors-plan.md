# Plan: every cue the scene gives can bring its memories, and says that it did

- status: draft
- generated: 2026-09-21
- last_updated: 2026-09-21
- work_type: code

## Goal
- When a character arrives somewhere, who is there, where it is and what it is doing bring memories to mind as surely as the topic does, and a loud topic cannot crowd them out. Each memory that comes back says which cues brought it. The self is never one of those cues: "me" is in every memory and distinguishes none.

## Definition of Done
- The memory store knows which notion is the character itself, because the application says so at construction (ADR-D-0020). No type or retrieval path gives the self a special role; the general rule below is what keeps it from acting as a cue.
- Every admitted memory reports, in the result, the set of cue kinds that admitted it, from one closed vocabulary. This plan produces topic, participant, place and activity; recency, due, date match and trigger are named and produced by the plans that build their routes. A memory reached by several cues names all of them, not only the strongest.
- A retrieval can say what the character is doing: the id of a thread or an open loop, beside the topic. It cues what belongs to that thread or loop. Inferring it when absent belongs to the next plan, which can read a conversation's recent episodes.
- No cue kind can be starved by another. A memory reached through a participant, the place or the activity is admitted up to a floor for its cue kind however many topical hits compete, at the candidate merge and in the pack's sections. The floors are measured defaults in the pattern of ADR-I-0022, set from the companion repository's loud-topic scenarios, and proposed in a record for the decider.
- A root that is linked to nearly everything cues nearly nothing, on every link path and not only the ones selectivity covers today. This holds for the self, for the one user of a one-to-one deployment, and for any notion, by one rule.
- A retrieval that gives only a topic selects what it selects today.

## Planner-added requirements
- The self is identified at construction in this plan. Needed because: ADR-D-0020 already requires it and nothing implements it; the ruling that a participant cue needs a counterpart other than the self, and the next plan's elapsed time since a pair last met, both need to know which participant is the character.
- The activity enters beside the topic, not inside `Scene`. Needed because: `Scene` is shared with writes, where an activity field had no reader and was removed; what the character is doing now is a retrieval input only, so putting it on the shared type would bring back a field that every stored episode shows as empty.
- The non-selective-root rule is general. Needed because: ADR-D-0020 forbids a special role for the self, and the census found the existing cap does not cover the link an ordinary `remember` makes for a participant.

## Scope / Non-goals
- Scope: `src/**`, `tests/**`, `README.md`, `docs/design/database/**` where a stored shape changes, at most two decision records (the cue vocabulary with the mapping of cue kinds onto routes, which ADR-D-0022 asks the phase to record; the measured floors), this plan.
- Non-goals: the time route (recency, date match, a range the topic names, cadence), elapsed time since a pair last met, staleness as age; scope keys, the state route, the setting key and custom values as cues; resolution as an end of currency and its omission reason; one-hop re-cueing from surfaced state; inferring the activity (all the next plan). Due dates, triggers, actor and counterpart, the single Preference subtype (the prospective-memory plan). Write-path warnings; selectivity beyond what the non-selective-root rule needs; the renderer and the example loop.

## Design
- Chosen: cue kinds are tracked from where a candidate is found to where it is admitted, as a set per memory, and floors are applied where starvation happens. Structure: the census shows two choke points, the merge of several content searches into one candidate list, and the pack's per-section caps; a participant root already enters before content roots. A floor reserves room at both for each cue kind that is present in the retrieval, and unused room returns to the common pool, so a topic-only retrieval is unchanged. Evolution: the next plans add routes (time, state, trigger) that report their own cue kinds into the same set and take floors by the same rule, with no shape change. Verification: service-free on the embedded stores; starvation is shown with a constructed loud topic, and the floor values come from the companion repository's scenarios. Operation: no extra model call; the floor arithmetic is over lists already in memory. Human: the result says why each memory came to mind, which a renderer can use and a developer can read. Safety: nothing is withheld; a floor only guarantees room.
- Alternative: one budget per cue kind end to end (separate candidate lists, separate section quotas). Rejected for now: it multiplies every cap by the number of cue kinds and fixes numbers before anything is measured; reservation with return to the pool gives the guarantee with the caps that exist.
- Alternative: report cue kinds only in the optional trace. Rejected: why a memory came to mind is knowledge a character has, and the companion scenarios assert it on default retrievals.
- The non-selective root, chosen: extend the measured inverse-frequency cap to the link paths it does not cover, so a notion present in nearly every experience admits few or no neighbours whatever link made the connection. Alternative: drop the self from the roots by identity. Rejected by ADR-D-0020, and it would leave the one-user case untouched.
- Why chosen: smallest shape that makes ADR-D-0022's invariant true for the cue kinds that exist today and leaves the later routes a rule to join. Fit: v0.2 draft sections 2 and 3; ADR-D-0020, D-0022, D-0029, D-0038, ADR-I-0022, I-0029.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface: construction of the memory value; `RetrievalContext`; the retrieval result and trace; selectivity statistics keys.
- stance: break
- justification: no external consumers; the one locatable consumer is `CharacterMemoryEvals`, the public companion evaluation repository whose tooling is a development aid and not core library functionality, and its plan follows each library slice. No stored-data migration; statistics are derived and rebuilt.

## Context (workspace)
- Related files/areas: the routes census `.agent-work/researcher/v0-2-routes-census-report.md` in the main checkout, sections 1, 2, 6, 7 and 8 (the pipeline stages, what per-item evidence can and cannot carry, the exact selectivity formula and its uncovered link path, every cap and whether it was measured, and which paths can starve which). `src/usecases/retrieve.rs`, `src/usecases/retrieve/scene.rs`, `src/api/types/retrieval.rs`, `src/policy/retrieval_selectivity.rs`, `src/policy/graph_expansion.rs`, `src/ports/retrieval_stats.rs`, `src/composition.rs`, `src/memory.rs`.
- Existing patterns or references: `SectionCueScoreSource` holds one source and `DirectMatch` loses which content search supplied it; `RationaleCategory::Temporal` exists and is never produced; selectivity buckets are About, Involves and PartOfThread, and `remember` links a participant through Mentions to an observation, outside them.
- Design record consulted and deviations from its acceptance: ADR-D-0018, D-0020, D-0022, D-0029, D-0038, ADR-I-0020, I-0022, I-0029. No deviation. The companion repository's cue vocabulary differs from the library's ruled one (it has pair, own day, recent and salient); the companion plan reconciles to the library's when it follows this slice.

## Open Questions (max 3)
- None for the decider. The rulings of 2026-09-21 recorded in the scene plan cover this slice, and plan approval is waived for plans inside them. Floors are set by the orchestrator from measurement and proposed in a record the decider accepts.

## Assumptions
- A1: The memory value is constructed in few enough places that a required self identity is a contained change. Source: census section 10; checked by Task_1.
- A2: Tracking a set of cue kinds per candidate through the merges costs no extra store read. Source: census section 2 (the merges are in memory); checked by Task_2.
- A3: A reservation that returns unused room to the pool leaves a topic-only retrieval's selection, sections and order unchanged. Source: unverified; Task_3 shows it against the scene stack's tip, and the companion repository measures the baselines.
- A4: The statistics store can count the uncovered link path with a new key without changing the existing buckets' values. Source: census section 6; checked by Task_4.

## Tasks

### Task_1: The store knows which notion is the character
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: []
- description: |
  The application names the character's own notion when it constructs the memory value, by identity key (ADR-D-0020, ADR-I-0020). The notion need not exist yet; it is an ordinary entity when it does. Nothing treats it as a special role: no type, no retrieval path, no validation on writes. Its one use in this plan is that a root for the self is never reported as a participant cue; keeping it from pulling memories in is Task_4's general rule, not an exception here.
- acceptance:
  - A memory value cannot be constructed without saying which notion is the character, and says so in the README's first example.
  - A retrieval whose scene lists the self by key reports the self as resolved and never as a participant cue for any memory.
  - No write path and no stored shape changes.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review against ADR-D-0020 (no special role)"

### Task_2: A memory says which cues brought it, and a retrieval can say what the character is doing
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/decisions/**
- depends_on: [Task_1]
- description: |
  One closed vocabulary of cue kinds (topic, participant, place, activity, recency, due, date match, trigger). Every admitted memory reports in the result the set of kinds that admitted it: topic for the topic search, place for the setting's words, participant for a root the scene named or a participant's description, activity for the thread or open loop the retrieval names. The set is kept through every merge, so a memory reached twice names both. The kinds with no route yet exist in the vocabulary and are never produced, and the documentation says which plan produces each. `RetrievalContext` gains the activity beside the topic: the id of a thread or an open loop, which becomes a root like a named participant and is reported unknown if it names nothing. `RationaleCategory` and `SectionCueScoreSource` are reconciled with the vocabulary so there is one way to say why, not two. Propose the record ADR-D-0022 asks for: the vocabulary and which route each kind travels.
- acceptance:
  - A memory reached by the topic and by a participant reports both kinds; one reached only through a description of a person reports participant, only through the setting's words place, only through the activity activity.
  - A memory that is a participant's neighbour only through the self reports no participant cue.
  - An activity naming a thread brings that thread's memories with no topic given; an id naming nothing is reported unknown and activates nothing.
  - The cue kinds are in the result with the trace off, and nothing in the trace says the same thing a second way.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the result shape and the proposed record"
  - kind: manual
    required: true
    owner: user
    detail: "The decider accepts or returns each proposed record, in a batch at the slice boundary"

### Task_3: No cue kind is starved
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/decisions/**
- depends_on: [Task_2]
- description: |
  Floors per cue kind at the two places the census found starvation: where several content searches merge into one candidate list under one cap, and where admitted memories compete for a section's cap. A cue kind present in the retrieval is guaranteed room up to its floor at both; room it does not use returns to the common pool; a cue kind not present reserves nothing. The floor values are defaults to be measured: ship with provisional values that make the constructed starvation cases pass, name them provisional, and propose the record with the measured values once the companion repository's loud-topic scenarios have run against this task.
- acceptance:
  - With a topic that alone fills the candidate cap and a section, a memory reachable only through a participant, one only through the place and one only through the activity are each admitted.
  - With no competing topic, nothing is displaced by a reservation: the pack is what it would be without floors.
  - A topic-only retrieval selects the same ids, in the same sections and order, as before this task.
  - The result or trace shows when a floor, not its score, admitted a memory.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the floor rule against ADR-D-0022 and ADR-I-0022"
  - kind: manual
    required: true
    owner: user
    detail: "The decider accepts or returns the measured-floors record"

### Task_4: A notion that is in nearly everything cues nearly nothing
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: [Task_3]
- description: |
  The measured inverse-frequency cap on an entity root's neighbours covers every link path a write can make for a participant, including the one from a participant to an observation that an ordinary `remember` makes, which has no bucket today. One rule for every notion: the self, the one user of a one-to-one deployment, a place everything happens in. The fallback when statistics are missing or unhealthy stays conservative and is stated in the README. No identity is special-cased.
- acceptance:
  - In a store where one notion takes part in every experience and another in a few, a retrieval naming both brings the second one's memories and few or none through the first, on the `remember` path and on a caller-built path alike.
  - The same holds whether or not the ubiquitous notion is the self.
  - Existing selectivity buckets keep their values for the existing measured cases.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, tracing the census section 6 coverage table against the change"

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]
- Wave 3: [Task_3]
- Wave 4: [Task_4]

One crate, shared files: sequential, one worker at a time, each PR stacked on the previous one, the first stacked on the scene stack. After each task the library commit is handed to the companion evaluation repository, whose own plan schedules what follows.

## Rollback / Safety
- Each task is one PR and reverts on its own in reverse order; once the companion repository has followed a slice, a revert is coordinated with it.

## Progress Log (append-only)

- (none yet)

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-21 Decision: the routes work is two plans, and this is the first.
  - Trigger / new insight: the routes census showed the pipeline has content recall and participant roots and nothing else, that per-item evidence cannot carry more than one cue, that a loud topic can remove every description and setting hit at the candidate merge and fill a section over a participant's memories, and that the cap on a ubiquitous root does not cover the link `remember` makes.
  - Plan delta (what changed): this plan makes the cue kinds that exist today reportable and unstarvable and identifies the self; the next plan adds the time and state routes, scope keys, elapsed time, staleness, resolution and activity inference on the same rule.
  - Tradeoffs considered: one plan for all of it was rejected as too large to review well; the split follows the line between reporting and guaranteeing what exists and adding routes that need new stored or queried facts.
  - User approval: plan approval waived for plans inside the rulings; decisions judged by character behavior are logged for presentation.
  - Record proposed: up to two, in Task_2 and Task_3.

## Notes
- Risks: floors interact with the measured defaults of ADR-I-0022, whose pollution and context-size baselines change when admission changes; the companion repository re-measures once, at its own Task_6. Provisional floor values could be mistaken for measured ones; they are named provisional until the record.
- Edge cases: a scene whose only participant is the self; an activity that names a closed thread; a memory reached by every cue kind at once; a retrieval with more cue kinds present than a small section has room for.
