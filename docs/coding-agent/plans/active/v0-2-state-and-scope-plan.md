# Plan: arriving somewhere brings what the character currently holds about who and what is there

- status: draft
- generated: 2026-09-21
- last_updated: 2026-09-21
- work_type: code

## Goal
- A character that meets someone again arrives knowing where things stand with them: what it believes about them now, what is open between them, how long it has been. The same holds for a place, a piece of work, or a context the application names. This comes from the scene alone, with no topic, and it brings the current version of each thing, never the superseded or the settled one.

## Definition of Done
- The application says at construction which notion is the character (ADR-D-0020). No type or retrieval path gives the self a special role; the identity has two readers here: a pair is the character and a counterpart, and the time since a pair last met.
- Every interpreted memory carries scope keys derived at commit from the scenes of the experiences it rests on: the notions present, each pair of the character with a counterpart, the setting key, each custom value, the threads it belongs to. They are never authored and never exposed as identifiers a caller must discover (ADR-D-0024). A memory resting on several experiences takes the keys they share.
- The state route: for the scopes the present scene implies, retrieval admits the current interpreted memories held about them, with no topic needed, under a floor like every other cue kind. It reports the cue kind that implied each scope (participant, place, activity); current state is not a cue kind of its own. The setting key and custom values, stored and echoed since the scene slice, cue through it.
- Currency ends by supersession or by resolution. A matter that was resolved is no longer current state; it is omitted with a reason that says so, and recall by topic still reaches it.
- One bounded hop from surfaced state: an open loop brings what it rests on, a thread its latest decision.
- The result reports the time since the character and each counterpart present last met, from recorded scene times.
- A retrieval that gives only a topic selects what it selects today.

## Planner-added requirements
- Pair keys are derived, which needs the self's identity. Needed because: "where things stand between us" is state about a pair, ADR-D-0024 names the pair as a scope, and without the self a pair cannot be told from two bystanders.
- A custom value implies a scope and reports as the place kind. Needed because: the ruled cue vocabulary is closed and has no kind for an application-defined context; the v0.2 draft introduces custom values as scene metadata for domains with their own scope model (a game zone, a project code), which is a setting in the broad sense.
- A retrieval that lists the self by key never takes the self as a counterpart. Needed because: the ruling that a pair cue needs a counterpart other than the self, left unimplemented by the cues slice.

## Scope / Non-goals
- Scope: `src/**`, `tests/**`, `README.md`, `docs/design/database/**`, at most two decision records (the derivation rules from scene to scope key, which ADR-D-0024 leaves uncovered; how resolution ends currency), this plan.
- Non-goals: the time route (recency for the character, date matches, a range the topic names, cadence), staleness as age, inferring the activity from a conversation's recent episodes, the order of memories within an activity's thread (the next plan). Due dates, triggers, actor and counterpart on open loops and commitments, the single Preference subtype (the prospective-memory plan); until then "what is open between us" is reached through scope keys, not through direction. A stored scope object. Scope keys on episodes and observations (their scene is the source). Reflection selecting its input by scope (v0.3). Write-path warnings; selectivity beyond entity roots; the renderer and the example loop.

## Design
- Chosen: scope keys are a derived field on interpreted memories, computed in the existing derived-at-commit pass from the source scenes in hand, and the state route is a graph read over them filtered by currency. Structure: one derivation function from a scene to keys; one intersection rule for several sources; one query by keys on the graph port, crate-internal; the route feeds the same candidate set as every other route, with its kinds and under the floors of the cues slice. Evolution: reflection in v0.3 selects by the same keys; the prospective-memory plan adds direction on top of the pair key. Verification: service-free; the companion repository's scenarios for state across scenes, open loops between a pair and long gaps are the acceptance instrument. Operation: one indexed graph query per retrieval; keys are short strings. Human: the application still passes only what it perceives. Safety: nothing is withheld by scope; a scope implies what to bring, never what to hide (ADR-D-0038).
- Alternative: no stored keys, answer the state route by traversal from each notion through its beliefs. Rejected: ADR-D-0024 decides scoped operations select by derived keys and never by walking the graph through a broad entity, and the cues slice deliberately made a ubiquitous notion bring little by traversal; state about the one person the character always talks to is exactly what traversal would now suppress.
- Alternative: a stored scope object with a label. Rejected by ADR-D-0024 until a named consumer needs it.
- Alternative: keys on every memory, episodes included. Rejected: an episode's scene is the authored source and is queryable; a second copy would have to be kept in step.
- Resolution, chosen: a `Resolves` link from a later memory ends the currency of what it resolves, read from the graph the way supersession is; nothing is stored on the resolved memory. Alternative: a stored resolved flag. Rejected for the reason the stored current flag was.
- Why chosen: smallest shape that lets the scene bring current state and gives v0.3 its selection rule. Fit: v0.2 draft sections 1, 2 and 2.1; ADR-D-0018, D-0020, D-0022, D-0024, D-0028, D-0038, ADR-I-0020.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface: construction of the memory value; the stored shape of interpreted memories; the retrieval result and trace; lifecycle omission reasons.
- stance: break
- justification: no external consumers; the one locatable consumer is `CharacterMemoryEvals`, the public companion evaluation repository whose tooling is a development aid and not core library functionality, and its plan follows each library slice. No stored-data migration: memories committed before this plan carry no scope keys, which the scene plan already accepted.

## Context (workspace)
- Related files/areas: the routes census `.agent-work/researcher/v0-2-routes-census-report.md` in the main checkout, sections 3, 4 and 5 (what time is read today, what current state means in code, where derivation at commit would sit, the graph shape of the setting key and custom values and what a lookup costs). `src/usecases/write_planning.rs` (the derived-at-commit pass), `src/domain.rs`, `src/adapters/oxigraph/*`, `src/ports/graph_authority.rs`, `src/policy/graph_expansion.rs` (currency), `src/usecases/retrieve.rs`, `src/composition.rs`, `src/memory.rs`.
- Existing patterns or references: Supersedes and About links are derived at commit from a memory's own fields; currency is read from incoming Supersedes links; `Resolves` exists as a relation name and changes nothing today; the setting key is a distinct predicate with no selector on the port; hydration reads every quad, so a query by key must not go through it.
- Design record consulted and deviations from its acceptance: ADR-D-0018, D-0020, D-0022, D-0023, D-0024, D-0028, D-0029, D-0034, D-0038, ADR-I-0016, I-0020, I-0022. No deviation.

## Open Questions (max 3)
- None blocking. Decided under the standing instruction and logged: a custom value reports as the place kind.

## Assumptions
- A1: The source scenes of an interpreted memory are in hand or cheaply read at commit, inside the write turn. Source: census section 4; checked by Task_2.
- A2: A query by scope key can be answered without full-store hydration. Source: census section 5; checked by Task_3.
- A3: Reading resolution from links costs no more than reading supersession does. Source: census section 4; checked by Task_4.
- A4: The floors and cue-kind sets of the cues slice take a fifth source of candidates with no change of rule. Source: the cues plan's Design; checked by Task_3.

## Tasks

### Task_1: The store knows which notion is the character
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: []
- description: |
  The application names the character's own notion when it constructs the memory value, by identity key (ADR-D-0020, ADR-I-0020). The notion need not exist yet and is an ordinary entity when it does. Nothing treats it as a special role: no type, no validation on writes, no exception in traversal. Its readers arrive in Task_2 (pair keys) and Task_5 (time since a pair last met); this task adds the identity and the one rule that a retrieval never takes the self as a counterpart.
- acceptance:
  - A memory value cannot be constructed without saying which notion is the character, and the README's first example says so.
  - A scene listing the self by key reports it like any key (resolved when the notion exists, unknown when it does not) and takes no counterpart from it.
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

### Task_2: An interpreted memory knows which scopes it belongs to
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/design/database/**
  - docs/decisions/**
- depends_on: [Task_1]
- description: |
  Scope keys are derived in the commit pass from the scenes of the experiences an interpreted memory rests on, and stored on it: one per notion present by key, one per pair of the character and a counterpart present by key, one for the setting key, one per custom value, one per thread the memory belongs to. A memory resting on several experiences takes the keys they share; one given by the application with no experience behind it takes the keys of its subjects and threads only. Keys are never authored (an authored value is rejected like an authored Supersedes link) and never appear in a caller-facing identifier. A correction's replacement derives its own. Words create no key: a participant or a setting given only in words is perception, not identity. Propose the record ADR-D-0024 leaves open: the derivation rules.
- acceptance:
  - A belief formed from a one-on-one conversation carries the pair key, the counterpart's key, the conversation's setting key and any custom value; one formed from two conversations with different people carries only what they share.
  - A caller cannot author or alter scope keys, on any write path.
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
    detail: "The decider accepts or returns each proposed record, in a batch at the slice boundary"

### Task_3: The scene brings the current state of what it implies
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: [Task_2]
- description: |
  The state route. The present scene implies scopes by the same derivation a write uses (notions and pairs from keyed participants, the setting key, custom values, the activity's thread). Retrieval admits the current interpreted memories carrying those keys through one crate-internal query on the graph port, filtered by currency, into the same candidate set as the other routes, reporting the cue kind that implied each scope (participant for a notion or pair, place for the setting key or a custom value, activity for the thread) and taking the floors of the cues slice. It never goes through full-store hydration. A pair's state ranks before a single notion's when both apply, because what is between the two of them is more specific than what is known about one.
- acceptance:
  - With a scene naming one counterpart and no topic, the pack holds what the character currently believes about that person and what is open between them, and none of what was superseded.
  - A setting key and a custom value each bring the current state formed under them, with no topic and no participant.
  - In a store where the counterpart takes part in every experience, the state route still brings the state about them, which traversal alone would not.
  - A topic-only retrieval selects the same ids, in the same sections and order, as before this task.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the route against ADR-D-0022, D-0024 and D-0038"

### Task_4: What was settled is no longer current, and says why it is absent
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - docs/decisions/**
- depends_on: [Task_3]
- description: |
  A `Resolves` link from a later interpreted memory ends the currency of what it resolves, read from the graph as supersession is; the resolved memory keeps its content and retention. The state route and default recall omit it with a reason that says resolution; recall that explicitly includes history returns it; a topic still reaches it when history is included. One bounded hop from surfaced state: an admitted open loop brings the experiences it rests on, an admitted thread its most recent decision, within the existing caps and floors. Propose the record for how resolution ends currency if the admission test passes.
- acceptance:
  - An open loop that a later memory resolves is absent from the scene's current state and is reported omitted by resolution; with history included it returns.
  - Suppressing the resolving memory does not make the resolved one current again, as with supersession.
  - An open loop surfaced by the state route brings the experience it came from, within caps.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review against ADR-D-0018 (recall is complete, currency selects versions and never removes)"
  - kind: manual
    required: true
    owner: user
    detail: "The decider accepts or returns each proposed record"

### Task_5: The character knows how long it has been
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: [Task_4]
- description: |
  The result reports, for each counterpart present by key, the time between the scene's reference time and the latest earlier experience whose recorded scene had both the character and that counterpart, from recorded scene times and never from creation times. A counterpart never met before is reported as such, not as zero. A counterpart given only in words has no identity to measure against and is not reported. Forgotten experiences do not count.
- acceptance:
  - After experiences with a counterpart a day, a month and a year before the reference time, the result reports the day; with none, it reports never met.
  - A caller-built experience and a `remember` one count alike; a suppressed one does not.
  - The value is in the result with the trace off.
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
- Wave 5: [Task_5]

One crate, shared files: sequential, one worker at a time, each PR stacked on the previous one, the first stacked on the cues stack. After each task the library commit is handed to the companion evaluation repository, whose own plan schedules what follows.

## Rollback / Safety
- Each task is one PR and reverts on its own in reverse order; once the companion repository has followed a slice, a revert is coordinated with it.

## Progress Log (append-only)

- (none yet)

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-21 Decision: the second half of the routes work is again two plans, and this is the state half.
  - Trigger / new insight: the cues slice made the cue kinds that existed reportable and unstarvable and left the routes that need new stored or queried facts. State needs scope keys and the self; time needs neither and reads only recorded times. The cues slice also made a ubiquitous notion bring little by traversal, which is right for occasions and means the state about the one person a character always talks to must come by another road.
  - Plan delta (what changed): this plan adds the self's identity, scope keys, the state route, resolution and the time since a pair last met; the next adds recency, date matches, staleness and activity inference.
  - Tradeoffs considered: elapsed time sits here and not in the time plan because its only input besides recorded times is the pair, which this plan defines.
  - User approval: plan approval waived for plans inside the rulings; decisions judged by character behavior are logged for presentation.
  - Record proposed: up to two, in Task_2 and Task_4.

## Notes
- Risks: deriving keys inside the write turn reads source scenes and lengthens the turn; intersection over many sources can leave a memory with no keys, which is correct and means only a topic or traversal reaches it. State for a pair can be large after a long relationship; the floors and section caps bound it, and which of it matters most is the time plan's and the renderer's.
- Edge cases, with the expected result: a scene whose only keyed participant is the self implies no pair and no counterpart, and brings the character's own state through its notion key; a memory whose sources share nothing carries only its thread keys; a participant known only in words implies no scope.
