# v0.2 Design Draft: Situated Recall and Scoped Continuity

Rewritten on 2026-09-17 after the v0.2 planning discussion. The roadmap's section 13 is the summary; this draft is the design. The behavioral standard it serves is the continuity situation catalog, especially section D, a day's recall. Decisions cited by record number are in `docs/decisions/`.

## Version intent

v0.1 stores episodes, observations, entities, soft threads, and derived memories, retrieves a continuity context pack through hybrid recall with bounded expansion, and runs with no external service. Everything it retrieves is cued by the topic of the current query, and most of what a person carries into a moment has no topic yet.

v0.2 makes the character arrive carrying what the moment calls for: who is here and what was last said with them, what is owed in either direction, what is in progress, what fell due, what happened this morning. It does this by taking the present scene as the retrieval input and treating the topic as one cue among several. The same scene, recorded on every memory, is what lets one character stay one character across settings.

The phase is judged by what a builder observes, not by structural completeness. Its character-layer value stays bounded until the generation phase (v0.3) produces derived memories, because until then every derived memory is authored by the caller.

## Inherited from the v0.1.5 closeout

- Admission gating and ranking credit for graph-only evidence. The 2026-09-16 re-baseline in the public companion `CharacterMemoryEvals` evaluation repository (a development aid, not core library functionality) is the planning input: derived memories ingested from dataset summaries took pack slots evidence turns held. Under this draft the item takes a definite shape: the state and time routes have admission floors beside the content route, so derived state and evidence turns are admitted on their own cues rather than competing for vector-scored slots. Pack-admission changes invalidate the pollution and context-size baselines of ADR-I-0022 and re-measure them once.
- Selectivity widening beyond entity roots, which needs retrieval statistics keyed by something other than entities. This phase designs that signal or declines it with recorded evidence.
- Scoped and person-keyed evaluation scenarios (catalog B1 to B3) before any implementation. Under ADR-D-0019 their meaning changed: B1 and B2 measure that the scene is present and correct on recall and, at the behavioral tier, that the character does not disclose across it. They never measure that a memory failed to surface.
- The concurrency question. With no background derivation inside the library, the reflection-scheduling form dissolves; what remains is the concurrent-facade-call question the library already has, and the census answers it.

The planning-time value audit ran on 2026-09-20 and the decider ruled on it the same day. The phase opens with schema groundwork before any route work, because four accepted decisions change stored shapes the routes would otherwise be built on and rebuilt: the archived and deleted retention states, the archive machinery, the lifecycle options whose only legal value is their default, the unread stability measure, the unread scope hint, and the operation id and idempotency key go (an idempotency ledger is declined; retry safety is deterministic ids plus content equality); the confidence score leaves interpreted memory and links together (ADR-D-0030); the stored current flag goes and currency is read from the supersession chain; the entity becomes a notion (ADR-D-0034). The parallel stats-projection clones keep their purpose, stats repair on an idempotent retry, and shrink when the correction path is next touched. Suppression is reversible by ADR-D-0018 and no operation reverses it yet; that is a known gap, decided in a later slice. The user and assistant preference subtypes are decided with the scene slice, where direction replaces them.

Caveat: the benchmark gap-bucket baselines rest on small per-bucket samples and are directional input, not thresholds.

---

# 1. The scene

The scene is the circumstances a memory was formed in and the circumstances recall happens in. On a memory it is who was present, who said it, whether the character was there when it happened, and in which setting. Episodes already carry participants and a source conversation, and this phase reports that part of the scene on every admitted memory; who said what on reflection's outputs arrives with consolidation in v0.3 (ADR-D-0028), and whether the character was there, with the rest of attribution, arrives in v0.4. At retrieval it is:

```text
when      the reference time; the only required field, defaulting to now
who       the participants present, each given as perceived: a description, a name as heard, a perception label, a key or ID the application already owns (ADR-I-0020), or any combination; the self is one of them (ADR-D-0020)
where     the place as perceived, a description as fine as the moment needs, or a key such as the conversation; for a text agent usually the conversation
what      the activity in progress: a thread or open loop the application received from remember, or nothing, in which case it is inferred from the conversation's recent episodes
custom    optional scene metadata for domains that already have their own scope model (a game zone, a project code); the application supplies the value it already owns and never looks one up from the library
```

Only the time is required, and the application never resolves, normalizes, or looks anything up (ADR-D-0029). An identity key cues its one entity, a setting key, a conversation, channel, zone, or project code, cues the memories whose scope it belongs to and never an entity, and an exact name cues every entity that bears it, which may be several; a description is a content cue over the entities and scenes memory holds, so an ambiguous reference activates each thing it could mean and an unknown one activates nothing, and the trace reports which. Recall reads entities through what the character currently believes about them (ADR-D-0034). A belief about a notion is an ordinary interpreted memory about it, so anything the character holds, a kind, a doubt, a relation nobody anticipated, is representable as it is; a belief may also carry a machine-readable assertion where recall reads through it mechanically. This phase builds one, known as, which is what lets an exact name cue every notion that bears it. Reaching the kitchen from the house, and one person from either of two names, are read-throughs of containment and sameness; each arrives with its scenario in the slice that needs it, and until then such beliefs are held as any other belief. Keeping identity consistent across drifting wording is consolidation's work in v0.3, never the application's.

A partial scene degrades gracefully. No participants means no pair recall and no participant-based partition, while a partition over the setting or a custom scope still applies, and the trace says the scene was partial. Enriching a thin scene from the text itself, resolving named speakers to entities, is reflection's work in v0.3 and never an input requirement; where it is unclear whether two references are one, consolidation forms separate notions and holds their sameness as a belief (ADR-D-0034).

There is no purpose field. What the character is trying to do surfaces from memory as an open loop, a commitment, a thread, or a signal (ADR-D-0023). A dispatched task's purpose arrives in the interaction as content and as an open loop with its rationale.

Scope keys on derived memories are derived from each memory's scene at write time, including any custom value, and are never a caller-facing ID scheme: the library mints no scope identifiers a caller must discover, which is what the exclusion of a scope hint by ID means. A stored scope object enters only when a consumer needs something a graph query over scope keys cannot answer (ADR-D-0024, which replaces ADR-D-0011's caller-facing scope object with derived scope keys).

## 1.1 Partitions as explicit policy

Recall is never gated by the scene by default (ADR-D-0019). An application that must enforce a boundary passes a partition as a query-time option over the scene; the trace records it as an applied policy. The B1 and B2 scenarios exercise both the default and the policy.

---

# 2. Situated activation

Recall is activation by the cues the scene supplies plus the topic of the current turn (ADR-D-0022). Nothing scores every memory, so each cue kind has its own route for finding candidates, and each route has an admission floor in the pack so no cue kind can starve another:

```text
content route     the topic, through each store's content lookup: vectors in durable memory, which is the route retrieval has today, and whatever index the short-term store of v0.3 uses
entity route      the participants, the place, the activity's thread, through the graph in durable memory, and in v0.3 through the participants and setting recorded on each short-term entry, which needs no graph; expansion under the v0.1.2 guardrails; whether these three cues share one floor or get sub-floors is a planning question (section 8)
time route        recency for this pair, recency for the character, a range when the topic names one, due dates, date matches, cadence-relative silence, all over timestamps, the graph's in durable memory and, in v0.3, each short-term entry's own
state route       the latest derived state for the scopes the who and what cues imply, active loops and commitments in both directions; a graph read filtered by currency, so it is the currency-side reading of the same cues rather than a sixth cue kind
trigger route     stored intentions whose trigger is a participant present or a topic arising
```

Importance and recency weight activation. Surfaced state re-cues one bounded hop: an open loop pulls the counterpart's objections, a thread pulls its last decision. A scene with no topic is the same retrieval with the content route empty; there is no separate current-state call. Elapsed time since the pair last met is reported with the result.

The section budgets of the pack stay as output categories. Floors are per route, and the plan calibrates them by measurement in the pattern of ADR-I-0022. The content, entity, and time routes are defined over a store-neutral candidate contract, so the short-term store that v0.3 introduces joins them without changing this design. The state and trigger routes read interpreted durable memory only.

## 2.1 Currency and staleness

An item is current when it is the latest in its supersession chain, not resolved, and, once v0.4 adds validity intervals recorded at write time, within its interval. Currency selects versions, never items, and never removes anything (ADR-D-0018, invariant 2.15). Staleness is the age since a memory's last supporting evidence, reported and never enforced: a relationship state last supported fourteen months ago is current and stale, and the character behaves accordingly. Events that end currency are all things that happened and were written: a correction supersedes, a resolution resolves, a farewell or a "let's drop it" closes a thread. There is no stored current flag: a memory is current when its retention allows it and nothing supersedes it, read from the chain.

## 2.2 Prospective memory

Open loops and commitments keep their subtypes (ADR-D-0005) and gain what the trigger route and the renderer need: a direction, an actor and a counterpart, so the character can owe and be owed (ADR-D-0020), and an optional due date. An event-triggered intention surfaces when its counterpart appears or its topic arises (catalog D7); a time-triggered one surfaces on its due date and keeps surfacing while overdue, until it is resolved (D8). Resolution goes through the existing link and correct paths; a facade method for it enters only if the scenarios show callers get the recipe wrong.

## 2.3 Treatment by supersession

A change in how a memory should be treated, "don't bring that up", a correction, a resolution, a note about a source, is a change of the character's state about it and supersedes the current derived memory with a restatement that carries the change. Restated, not appended: the current memory reads as one memory, never as a log of edits. There is no treatment category and no annotation plane. The write path warns, never blocks, on a replacement whose text contains its predecessor nearly verbatim (a sibling of the existing echo warning) and on a chain that churns faster than a threshold. Expression quality belongs to the writer, the consumer today and the generation phase tomorrow, and the catalog's retelling-consistency situation is its behavioral check.

## 2.4 Renderer and example loop

A canonical rendering of a pack, favoring gist and stance over quotation and separating what the character may say from what merely shapes it (philosophy 9.3). A minimal retrieve, respond, remember loop in the library's own examples, so the write-path friction is felt by a consumer before scene work bakes more of it in.

## 2.5 Reflection in this phase

Only a signal: a scope has accumulated enough since its last reflection to be worth one. Generation, the reflection record, and the trigger vocabulary belong to v0.3.

---

# 3. Retrieval changes

```text
RetrievalContext takes a scene (when required; who, where, what, custom optional), a topic, an optional partition policy, and an intent (Continuity, the only variant v0.2 needs, since a scene with no topic replaces a CurrentState variant; the other ADR-I-0016 variants arrive with the phases that need them)
candidate routes per cue kind with admission floors; the content route is what exists today
the temporal rationale category is produced; activation records which route admitted each item
the trace records the scene as given, whether it was partial, the applied partition policy, elapsed time since the pair last met, and every omission on lifecycle or currency grounds with its reason, beside the existing section-limit, partition, and expansion-bound reasons
selectivity applies beyond entity roots with a new statistics key, or the declination is recorded with evidence
```

The seven retrieval modes of the earlier draft, the scope hint by ID, the current-state view type, and interpreting-neighbor co-retrieval are gone.

---

# 4. Evaluation first

The plan in the public companion `CharacterMemoryEvals` evaluation repository, whose tooling is a development aid and not core library functionality, comes before the library plan, against the maintained smoke config and the library version pin. Scenarios are named by catalog situation and carry their retrieval-tier property:

```text
B1 person-keyed separation: scene present and correct on recall; non-disclosure at the behavioral tier; partition policy omits across the scene and the trace says so
B2 group versus one-on-one scenes: same topic, different scenes, both recalled, disclosure follows the scene
B3 differential relationship states: scope-keyed relationship notes retrieved per scene, current only
D1 waking into the day and D8 a deadline arrives: due items admitted with no topic
D4 encountering a person: current state, last interaction, and obligations in both directions on a scene with the person and no topic
D5 resuming work: the activity's thread in order on a scene with the thread and no topic
D7 an event the character was waiting for: a stored intention surfaces when its counterpart appears
D9 anniversaries: a date match surfaces when the person is present
D11 reunion after a gap and C6 departure: elapsed time reported, state current and stale
D13 own day: autobiographical range on the self
graph-only probe and the re-baseline: state and time floors hold under a loud topic; ADR-I-0022 baselines re-measured
tasks and favors: instructions recalled across long gaps (ADR-D-0018)
C4 consistent retelling: the behavioral check on supersession restatements
```

Behavioral-tier judging is scheduled with the generation phase; this phase absorbs every retrieval-tier property first.

---

# 5. Public API additions

Illustrative shape:

The scene is given as perceived: keys are shown here because this example has them, and a description in words, or nothing but the time, is equally valid (ADR-D-0029).

```rust
let scene = Scene::now()
    .with_participants([self_id, alice_id])
    .with_conversation("channel-42")
    .with_activity(thread_id);
let outcome = memory.retrieve(RetrievalContext::in_scene(scene).with_topic("what were we working on?")).await?;
let prompt_text = outcome.pack.render(RenderStyle::default());
```

No reflect, reinforce, resolve, or current-state methods. Open loops and commitments are read from the pack by section and resolved through link and correct.

---

# 6. Acceptance criteria

```text
With a scene and no topic, retrieval returns what the moment calls for: the people present's current state and last interaction, active loops and commitments in both directions, the activity's thread in order, items due, date matches, and recent high-salience episodes, with elapsed time since the pair last met.
A stored intention surfaces when its counterpart appears or its topic arises, and a promise surfaces on its due date, whatever the current topic.
A memory learned in one setting is admitted when retrieved for another, with the scene v0.2 records reported (participants, setting, when); a partition applied as a query option omits across the scene and the trace records the applied policy.
Under a loud topic, the state and time routes still admit their floor, and the ADR-I-0022 baselines are re-measured once.
Retrieval produces the temporal rationale category, and catalog D1, D4, D5, D7, D8, D9, D11, and D13 pass at the retrieval tier.
Currency never removes an item: every omission on lifecycle or currency grounds names a resolution, a supersession, or a suppression, and staleness is reported as age.
A near-verbatim superseding restatement and a churning chain each raise a write-path warning; C4 is the behavioral check.
Selectivity beyond entity roots is either applied with its new signal or declined with recorded evidence.
The pack renderer and the example loop exist, and the README describes what ships.
```

---

# 7. Not in v0.2

```text
reflection that generates text, the reflection record, and trigger vocabulary (v0.3)
first-class OpenLoop, Commitment, CharacterSignal, RelationshipState object types
a current-state view type, a scope hint by ID, a purpose field, a retrieval-mode enumeration
a stored scope object without a named consumer
interpreting-neighbor co-retrieval
involuntary recall from weak cues, D12 (v0.5)
the short-term store and the mechanical write (v0.3, beside reflection)
reinforce, decay, archival
attribution fields beyond what the scene implies (v0.4)
```

---

# 8. Questions for planning, not for the decider

```text
the scene's field shapes at retrieval and on stored memories, and how "what" is inferred when absent
how scope keys are derived from a scene at write time and namespaced for custom scopes
the candidate mechanism per route, the floors, whether participants, place, and activity share the entity route's floor, and their measurement design
the time route's windows: pair recency, cadence, due dates, date matching
where direction and due date live on the open-loop and commitment subtypes
the superset and churn warning heuristics and thresholds
the renderer's style options and its treatment of the scene
the concurrent-facade-call census result and whether anything in the write path needs a guard
```

Implementation records expected with the plan, each written when its contract is set: the route-and-floor admission contract (in the pattern of ADR-I-0006), elapsed time and the other time cues as measured ranking signals (ADR-I-0022), familiarity and stability derived from evidence (ADR-I-0017), the scene's retrieval-level shape, and the trigger route's contract.

---

# 9. Library boundary

Continuity structures are memory state, not agent orchestration. The library takes the scene and reports it, never decides disclosure; the application chooses partitions; purpose comes from memory; the model reconstructs the response. If scope modeling grows past derived keys, the value audit names the consumer before any object is added.
