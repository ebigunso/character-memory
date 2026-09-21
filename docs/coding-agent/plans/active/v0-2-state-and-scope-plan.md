# Plan: arriving somewhere brings what the character currently holds about who and what is there

- status: in_progress
- generated: 2026-09-21
- last_updated: 2026-09-21
- work_type: code

## Goal
- A character that meets someone again arrives knowing where things stand with them: what it currently holds about them, whoever it learned it from, what is still open, when they last met and how long ago that was. The same holds for a place, a piece of work, or a context the application names. This comes from the scene alone, with no topic. A matter that was settled no longer counts as where things stand, and is still remembered when the conversation turns to it.

## Definition of Done
- The state route: for every notion the present scene resolves (a participant given by key, or by a name the character currently knows them by), retrieval admits the current interpreted memories that are about that notion, read through the memories' own subjects, with no topic needed. For the activity it admits the current memories of that thread, read through their own thread lists. It reports the cue kind that implied each (participant, activity); current state is not a cue kind of its own. It takes the floors of the cues slice.
- When several scopes are implied, each brings something before any brings a second: within a cue kind, scopes are served in rounds, and within a scope the order is salience. One long relationship cannot crowd out the five other people in the room.
- The last interaction with each resolved participant is brought with its state, and the result reports the time since then, from recorded scene times. Never met is reported as never met, not as zero. A participant given only by description has no identity to measure against and is not reported.
- The setting key and custom values, stored and echoed since the scene slice, become cues: interpreted memories carry a scope key for the setting key and one per custom value (its name and its value together), derived at commit from the scenes of the experiences they rest on, never authored, never an identifier a caller must discover (ADR-D-0024). A memory resting on several experiences carries the keys they share. The state route reads them.
- Resolution ends being current state and nothing else. A memory that a later one resolves, or a commitment that a later one fulfils, is left out of the state route with a reason that says so. Every other route admits it as before (ADR-D-0018), and wherever it is admitted the result says it is resolved and by which memory, so a settled debt never looks open.
- An open loop surfaced by the state route brings the experiences it rests on, within the existing caps.
- A retrieval that gives only a topic selects what it selects today.

## Planner-added requirements
- State about a notion is read through subjects, not through who was present when the memory was formed. Needed because: on meeting Bob a person expects what they hold about Bob, whoever told them, and a remark about the weather made beside Bob is not state about Bob. The memory's own subject list already says what it is about and already yields a link at commit; a second, presence-derived copy would be wrong as well as redundant.
- No pair key and no identity of the self in this plan. Needed because (as deletions from the first draft): the store is first-person, so what the character holds about Bob already is its side of the pair; a pair key adds nothing until open loops and commitments carry an actor and a counterpart, which is the prospective-memory plan, and that plan is where ADR-D-0020's construction-time identity gets its first real reader. The time since a pair last met needs only the experiences the counterpart took part in, since every experience in the store is the character's own.
- Stored scope keys only for what no existing field says: the setting key and custom values. Needed because: those are recorded on episodes as scene parts and are not queryable from an interpreted memory; notions and threads already are.
- `FulfillsCommitment` ends being current state the way `Resolves` does. Needed because: the relation exists and changes nothing today; a kept promise that still surfaces as owed is the same failure as a settled matter that still looks open.

## Scope / Non-goals
- Scope: `src/**`, `tests/**`, `README.md`, `docs/design/database/**`, at most one decision record (the derivation rules from scene to scope key, which ADR-D-0024 leaves uncovered, if the admission test passes), this plan.
- Non-goals: the identity of the self, a pair scope, actor, counterpart, due dates and triggers on open loops and commitments, the single Preference subtype (the prospective-memory plan). Recency for the character, date matches, a range the topic names, cadence, staleness as age, inferring the activity from a conversation's recent episodes, which members of a thread matter most beyond salience (the time plan). A stored scope object. Scope keys on episodes and observations (their scene is the source). A thread's "latest decision" (no subtype means it, and nothing may infer it from prose). Reflection selecting its input by scope (v0.3). Write-path warnings; selectivity beyond entity roots; the renderer and the example loop.

## Design
- Chosen: the state route is one bounded read per implied scope over what commit already records, filtered by a currency rule local to this route. Structure: a notion's state is the interpreted memories whose subjects include it, reached by a selector on the graph port that does not go through full-store hydration; a thread's state is its members by their own thread lists (the read the cues slice already has); a setting or custom scope is reached by a stored key through the same kind of selector. The route feeds the existing candidate set with cue-kind sets and under the floors of the cues slice; it does not pass through entity-root selectivity, which bounds experiences and deliberately leaves beliefs alone. Evolution: the prospective-memory plan adds direction on top; the time plan adds recency and staleness; v0.3's reflection may select by the same keys, which is not a reason for anything here. Verification: service-free; Task_1's acceptance cases are run at the baseline first, with an owner for that evidence; the companion repository's scenarios for state across scenes, open loops and long gaps are the acceptance instrument. Operation: a small number of indexed graph reads per retrieval, no model call. Human: the application still passes only what it perceives. Safety: a scope implies what to bring, never what to hide (ADR-D-0038).
- Alternative: derive a key per notion present and per pair from the source scenes (the first draft). Rejected: presence is not aboutness; the self's key would land on every memory and, in a one-to-one deployment, so would the pair's; intersection over sources would strip a relationship note of the person it is about.
- Alternative: answer notion state by the existing entity expansion. Rejected unless the baseline shows it suffices: expansion is capped for ubiquitous notions by design and ranks by proximity, not by currency or salience; Task_1 shows the failing case first.
- Alternative: a stored resolved flag. Rejected for the reason the stored current flag was: read it from the links.
- Alternative: resolution as a recall-wide filter. Rejected by ADR-D-0018: a change of currency never changes recall eligibility.
- Why chosen: smallest shape that lets the scene bring current state. Fit: v0.2 draft sections 1, 2 and 2.1; ADR-D-0018, D-0022, D-0024, D-0028, D-0029, D-0034, D-0038.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface: the stored shape of interpreted memories (scope keys); the retrieval result (resolved marking, last interaction and time since) and trace; the graph port.
- stance: break
- justification: no external consumers; the one locatable consumer is `CharacterMemoryEvals`, the public companion evaluation repository whose tooling is a development aid and not core library functionality, and its plan follows each library slice. No stored-data migration: memories committed before this plan carry no scope keys, which the scene plan already accepted.

## Context (workspace)
- Related files/areas: the routes census `.agent-work/researcher/v0-2-routes-census-report.md` in the main checkout, sections 3, 4 and 5; the plan reviews `.agent-work/reviewer/v0-2-state-plan-review.md`. `src/usecases/retrieve.rs`, `src/usecases/retrieve/scene.rs`, `src/usecases/retrieve/activity.rs`, `src/usecases/write_planning.rs` (the derived-at-commit pass), `src/domain.rs`, `src/adapters/oxigraph/*`, `src/ports/graph_authority.rs`, `src/policy/graph_expansion.rs` (currency today), `src/api/types/retrieval.rs`.
- Existing patterns or references: Supersedes and About links are derived at commit from a memory's own fields; currency is read from incoming Supersedes links; `Resolves` and `FulfillsCommitment` exist as relation names and change nothing; the setting key is a distinct predicate with no selector on the port; participants are stored on an episode as one JSON literal, while a keyed participant is also linked to its episode or observations; hydration reads every quad, so a selector must not go through it.
- Design record consulted and deviations from its acceptance: ADR-D-0018, D-0020, D-0022, D-0023, D-0024, D-0028, D-0029, D-0034, D-0038, ADR-I-0016, I-0020, I-0022. No deviation. ADR-D-0020's construction-time identity and ADR-D-0024's pair scope are not delivered here and are named in the Non-goals with the plan that delivers them.

## Open Questions (max 3)
- For the decider at the slice boundary, not blocking: which cue kind a custom value reports. The ruled vocabulary is closed and has no kind for an application-defined context; this plan reports it as place, provisionally, because the draft introduces custom values as scene metadata for domains with their own scope model (a game zone, a project code). A project code is not a place, and the companion scenarios will read the kind, so the decider may prefer a ninth kind.

## Assumptions
- A1: The existing entity expansion does not bring a ubiquitous notion's current beliefs reliably, ranked by salience and currency. Source: unverified; the cues slice's log says a belief about a ubiquitous notion stayed admitted. Task_1 shows the failing case at the baseline before building, and if none exists the route shrinks to ordering and rounds.
- A2: A selector by subject and one by scope key can be answered without full-store hydration. Source: Tier D plan review; checked by Task_1 and Task_2.
- A3: The source scenes of an interpreted memory are in hand or cheaply read inside the write turn. Source: census section 4; checked by Task_2.
- A4: The latest experience a notion took part in can be found through its links and recorded scene times without scanning every episode's participant literal. Source: Tier D plan review; checked by Task_4.
- A5: The floors and cue-kind sets of the cues slice take another source of candidates with no change of rule. Source: the cues plan's Design; checked by Task_1.

## Tasks

### Task_1: Meeting someone brings what the character currently holds about them
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: []
- description: |
  The state route for notions and the activity. For every notion the scene resolves, by key or by a currently known name (an ambiguous name implies each notion it could mean; a description implies none), admit the current interpreted memories whose own subjects include it, through a crate-internal selector on the graph port that avoids full-store hydration. For the activity, its thread's current members through the read the cues slice added. Current here means not superseded; resolution arrives in Task_3. The candidates join the existing set with the cue kind that implied them and take the existing floors. Within a cue kind, implied scopes are served in rounds, one memory per scope per round, and within a scope the order is salience, then recency of creation, then id. Scopes are ordered as the scene gives them (participants in scene order, several notions under one name in id order, then the activity), so the same scene always gives the same result. The guarantee has to hold to the pack, not only into the shared candidate set: a state candidate carries which scope implied it, a memory implied by several scopes counts for each with its one slot, and at each of the three places the cues slice reserves room (the candidate merge, root selection where it applies, and the section caps) the room a cue kind holds is shared among its scopes by the same rounds. State how this is carried, in the smallest form the existing floors helper allows; if it cannot be carried without changing the helper's rule for the other kinds, stop on that point and report. An open loop admitted this way brings the experiences it rests on, within the existing caps. First show, at the baseline, the case this route exists for and that it fails there.
- acceptance:
  - With a scene naming one person and no topic, the pack holds what the character currently holds about them, including a belief formed in a conversation with someone else, and nothing that was superseded; a remark formed beside them that is not about them is not brought as their state.
  - With six people present, one of whom has hundreds of current beliefs, each of the six brings something.
  - In a store where that person takes part in every experience, their state still comes; the report shows what the baseline brought in the same case.
  - A name that could mean two people brings state for both, and the result already says which they are.
  - A topic-only retrieval selects the same ids, in the same sections and order, as before this task.
  - The report holds a baseline run at the task's parent commit for each case above, saying which failed there and which already passed; a case that already passes is not claimed as delivered by this task.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the acceptance cases run at the parent commit, with what each brought recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, reproducing the baseline evidence at the parent commit; Tier A review of the route against ADR-D-0022, D-0029 and D-0038"

### Task_2: A place or a named context brings what was formed under it
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/design/database/**
  - docs/decisions/**
- depends_on: [Task_1]
- description: |
  Interpreted memories carry scope keys for the setting key and for each custom value (name and value together, so zone 42 and project 42 never meet), derived in the commit pass from the scenes of the experiences they rest on and stored on the memory. A memory resting on several experiences carries the keys they share; one with no experience behind it carries none. Keys are never authored (an authored value is rejected like an authored Supersedes link), never exposed as an identifier, and words create none. A correction's replacement derives its own. The state route reads them for the present scene's setting key and custom values through a selector like Task_1's, reporting the place kind (provisionally also for a custom value; see Open Questions). Propose a record for the derivation rules ADR-D-0024 leaves open, including that notion and thread scope are read from a memory's own subjects and thread list and are not stored a second time, if the repository's admission test passes; if it does not, say so and propose none.
- acceptance:
  - A belief formed in one conversation carries that conversation's setting key and custom values; one formed from two conversations carries only what they share.
  - A setting key, and a custom value, each bring the current state formed under them, with no topic and no participant.
  - A caller cannot author or alter scope keys on any write path.
  - The schema reference documents describe the stored keys and their derivation.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the derivation rules and the proposed record against ADR-D-0024 and ADR-D-0029"
  - kind: manual
    required: true
    owner: user
    detail: "If a record was proposed, the decider accepts or returns it; the decider rules on the cue kind of a custom value; both in a batch at the slice boundary"

### Task_3: What was settled is no longer where things stand, and never looks open
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: [Task_2]
- description: |
  A `Resolves` or `FulfillsCommitment` link from a later interpreted memory ends being current state for what it points at, read from the graph as supersession is; the resolved memory keeps its content, its retention and its eligibility for every other route (ADR-D-0018). The state route leaves it out with a reason that says resolution. Wherever a resolved memory is admitted, by a topic, a participant root or anything else, the result says it is resolved and names the memory that resolved it. Suppressing the resolving memory does not make the resolved one current state again, as with supersession. The filter is local to the state route; no other reader of currency changes.
- acceptance:
  - An open loop that a later memory resolves is absent from the scene's current state and reported left out by resolution; a matching topic still admits it by default, marked resolved with the resolving memory named.
  - A fulfilled commitment behaves the same way.
  - Suppressing the resolver does not return the resolved memory to current state.
  - Supersession, suppression and every existing omission reason behave as before.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review against ADR-D-0018"

### Task_4: The character knows when they last met and how long it has been
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: [Task_3]
- description: |
  For each notion the scene resolves as a participant, the latest earlier experience that notion took part in, by recorded scene time and never by creation time, is brought with their state (the last interaction ADR-D-0022's validation names), and the result reports the time between it and the scene's reference time. Never met is reported as never met. A participant given only by description is not reported. Forgotten experiences do not count. An experience written by `remember` and a caller-built one count alike. Taking part means any of the ways a write records it: an episode linked to the notion by Involves, or an observation linked to it by Mentions, joined to its episode, in either link orientation the writes produce, so `remember` and a caller-built plan count alike. Found through those links and recorded scene times with the link selector the graph port already has, not by scanning every episode's participant literal and not through full-store hydration.
- acceptance:
  - After experiences with someone a day, a month and a year before the reference time, the result reports the day and the pack holds that experience; with none, it reports never met.
  - A caller-built experience and a `remember` one count alike; a suppressed one does not.
  - The values are in the result with the trace off.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review"

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]
- Wave 3: [Task_3]
- Wave 4: [Task_4]

One crate, shared files: sequential, one worker at a time, each PR stacked on the previous one, the first stacked on the cues stack. After each task the library commit is handed to the companion evaluation repository, whose own plan schedules what follows.

## Rollback / Safety
- Each task is one PR and reverts on its own in reverse order; once the companion repository has followed a slice, a revert is coordinated with it.

## Progress Log (append-only)

- (none yet)

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-21 Decision: the second half of the routes work is again two plans, and this is the state half.
  - Trigger / new insight: the cues slice made the cue kinds that existed reportable and unstarvable and left the routes that need new reads or stored facts. State needs a read by subject and two stored keys; time needs only recorded times.
  - Plan delta (what changed): this plan adds the state route, scope keys for the setting and custom values, resolution, and the last interaction with the time since; the next adds recency, date matches, staleness and activity inference.
  - Tradeoffs considered: the time since a participant was last met sits here because it is state about that participant and needs nothing the time route builds.
  - User approval: plan approval waived for plans inside the rulings; decisions judged by character behavior are logged for presentation.
  - Record proposed: one, in Task_2.
- 2026-09-21 Decision: the draft was rewritten after its Tier D review and its combined Tier A review and value audit, before dispatch.
  - Trigger / new insight: the Tier A pass named the tunnel vision: "derived from the scene" had been read so literally that presence stood in for aboutness, when a memory's own subjects and thread list already say what it is about. Under presence keys a belief about Bob learned from Alice would never come on meeting Bob, a remark about the weather made beside him would, and the self's key would sit on every memory. Both reviews found that Task_4 made resolution a recall-wide filter against ADR-D-0018. Tier D found that the draft denied state to a participant given by a known name, against ADR-D-0029, and that a thread's "latest decision" has nothing in the schema to mean it.
  - Plan delta (what changed): notion and thread state are read from subjects and thread lists; stored keys remain only for the setting key and custom values; the pair key and the identity of the self leave this plan for the prospective-memory plan; scopes are served in rounds with salience order inside a scope; resolution and fulfilment filter the state route only, and a resolved memory is marked wherever it is admitted; name-resolved participants get state and the time since; the last interaction is brought; the thread-decision re-cue and a second decision record are deleted; each acceptance case is first shown to fail at the baseline.
  - Tradeoffs considered: keeping the pair key for ADR-D-0024's sake was rejected for now, because in a first-person store it has no reader until direction exists.
  - User approval: not required; decided under the standing instruction and logged for presentation. One question is raised for the decider (the cue kind of a custom value).
  - Record proposed: one, in Task_2.

## Notes
- Risks: if the baseline already brings a ubiquitous notion's beliefs adequately, Task_1 is smaller than written, and the plan says to find out first. Deriving keys inside the write turn reads source scenes and lengthens the turn. A memory whose sources share no setting carries no key, which is correct. State for one person can be large after a long relationship; rounds, salience order, floors and section caps bound it, and which of it matters most over time is the time plan's and the renderer's.
- Edge cases, with the expected result: a scene whose only participant is the character itself brings what the character holds about itself, like any notion; a name nobody bears implies no scope and is already reported unknown; a memory about two people present takes one slot and counts for both scopes in the round.
