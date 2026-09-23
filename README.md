# Character Memory

Character Memory is a Rust library for giving LLM assistants memory that shapes behavior over time.

A recognized key or name brings back where things stand with that person or in that place. A description such as "a man with a deep voice" or "at my desk" brings recent occasions like this to mind, without deciding who the person is or treating the description as a place's identity.

It is built for persistent AI assistants and companions that should remember past interactions, recognize recurring entities, and maintain character continuity across sessions.

Instead of treating memory as a chat log or simple vector search, Character Memory stores episodic memories with temporal and relational structure.

It helps an assistant remember:

- what happened
- when it happened
- who or what was involved
- how past events relate to each other

To aid in character continuity.

A character is not just a prompt. A character is shaped by remembered experience.

## Why Character Memory?

Modern LLM agents are usually built around task execution.

They can plan, call tools, and complete workflows, but they often lack persistent memory of lived interaction. This makes them feel stateless: each conversation may be useful, but the assistant does not develop a stable sense of continuity.

Character Memory is designed for systems where past interactions matter.

Use it when you want an assistant to remember things like:

- recurring people, projects, places, and topics
- past conversations with the user
- important events and decisions
- relationship history
- preferences that emerged over time
- memories that are relevant because of time, entities, or meaning

## Core idea

Character Memory stores memories as episodes.

An episode is a remembered event or interaction. Each episode can be connected to time, entities, and related memories.

Retrieval is graph-authoritative and hybrid:

- **Vector candidate recall:** uses embedded Qdrant Edge by default, or an explicit Qdrant service, to find semantically similar memory objects
- **Graph expansion:** uses Oxigraph as the authority for entities, threads, provenance, lifecycle state, and links
- **Temporal structure:** recent occasions, an application-supplied time range and anniversaries can recall experiences by their recorded scene time
- **Entity-based retrieval:** includes memories involving the same people, projects, places, or concepts
- **Continuity retrieval:** returns a structured `ContinuityContextPack` rather than a generic ranked list

`RetrievalContext` carries a `Scene` and an optional topic; its time defaults to now. The topic recalls what happened or was learned, while descriptions recall occasions with similar surroundings or people. A setting key or custom value brings what was formed there as a reminder, newest first. It reserves the place floor, then competes for unclaimed room at a cue score of zero, without opening history or taking turns from the people present. Beliefs with several sources can return in any of their recorded places or named contexts. The result returns the present scene, its reference resolutions, and each admitted memory's recorded source scenes even without a trace. Forgotten sources are explicit; forgotten scenes follow `include_suppressed`. Scene differences never exclude a memory or determine who may hear it.

`memory_scenes[].admitted_by` reports the `AdmissionRoad` values that brought each admitted memory to mind, with tracing on or off. When several roads reach an admitted memory, it reports all of them. The consumer chooses the words used to present this information. Reporting does not change selection, order or scores.

| Admission road | What reached the memory |
| --- | --- |
| `Participant` | Someone present, supplied by key or name, reached occasions they were at or what is held about them. |
| `Place` | A setting key or custom value reached a memory formed there. |
| `Activity` | A supplied thread or open loop reached its members, sources or linked memories. |
| `Topic` | Content matched the supplied topic, even faintly, and could bring connected memories. |
| `PersonDescription` | A resemblance to how someone present was described reached an occasion and memories resting on it; the person may be a stranger. |
| `SettingWords` | A resemblance to the described surroundings reached an occasion and memories resting on it. |
| `Range` | A caller-supplied span reached an occasion within it and memories resting on that occasion. |
| `Anniversary` | A recorded occasion's local month and day recurred in a later year, reaching that occasion and memories resting on it. |
| `Recency` | With no supplied range, a recent occasion at or before the scene time reached that occasion and memories resting on it. |

For an admitted interpreted memory, `memory_scenes[].seconds_since_support` reports whole seconds since its latest supporting experience at or before the reference scene time, even with tracing off. An observation uses its own observed time, falling back to its parent scene time. Suppressed sources count only when `include_suppressed` is enabled; an active observation still counts after its parent episode is forgotten. No eligible support, including application-given beliefs without experiences and future-only support, reports `None` (`null` in JSON). A correction uses its replacement's own sources. This age never changes selection, order or scores.

An episode has a content search surface for its summary and up to two separate scene surfaces: the setting words and the participants' words. Scene words are not appended to the content text. All participants' names and descriptions are searched together, as they were recorded, so any similarity score for them belongs to that shared search. See the [vector design](docs/design/database/vector_payload_design.md#indexed-objects-and-surfaces) for the stored forms.

For each recognized participant, `last_interactions` says when the character last met them and how much time has passed. If a name could mean several participants, each has its own answer. No eligible encounter at or before the scene time means never met. These facts remain available even when no memories fit the requested amount. Forgotten encounters count only when `include_suppressed` is enabled.

Use `.with_activity(ActivityRef::Thread(thread_id))` or `.with_activity(ActivityRef::OpenLoop(open_loop_id))` to recall ongoing work without a topic. Give an ongoing project as the activity; custom values describe the context in which memories were formed. Thread membership, open-loop sources and linked memories supply candidates within the retrieval limits. The result echoes the activity with `Found` or `Unknown`; finding an activity does not guarantee an admitted memory. With tracing enabled, each section assignment reports its set of `CueKind` values used for floors: `Topic`, `Participant`, `Place`, `Activity`, `DateMatch` and `Recency`.

Each way of reaching a memory is a **road**. One memory can be reached by several roads. The same table controls what each road reports, whether it opens history, its reservation, and how many roots it contributes:

| Road | Floor kind | Admitted by | Opens history | Reserves room | Contributes |
| --- | --- | --- | --- | --- | --- |
| Participant key or name | Participant | Participant | Yes | Yes | The recognized notion, expanded |
| Setting key or custom value | Place | Place | No | Yes | Up to the root cap, newest first by memory time |
| Activity | Activity | Activity | Yes | Yes | The thread's members |
| Topic | Topic | Topic | Yes | Yes | Up to the candidate cap |
| Participant description | Participant | PersonDescription | No | Yes | The larger of one and the participant floor |
| Setting words | Place | SettingWords | No | Yes | The larger of one and the place floor |
| Supplied range | DateMatch | Range | No | Yes | Up to the room, plus one read to report whether more exists |
| Anniversary shared with someone resolved in the scene | DateMatch | Anniversary | No | Yes | Up to the room |
| Unshared anniversary | DateMatch | Anniversary | No | No | Up to the room |
| Recency, when no range is supplied | Recency | Recency | No | Caller floor, default zero | Up to the room |

The room is the largest requested section cap. Five principles govern these roads:

1. **Reserve from each road's own order.** Within a kind, roads that open history come first. A description sharing that kind does not jump ahead because another road also reached its memory.
2. **Contribute only the table's amount.** Every search score enters as the larger of zero and its similarity. Contributing a candidate does not guarantee a place in the result.
3. **A range is recency's window.** One time read uses the supplied span, or otherwise ends at the scene time. A supplied range reports date matches and contributes no recency outside it.
4. **Recall the strongest matches first.** Among memories that nothing matched, the more significant come first, then the newer. When a section fills, equally strong memories favor the newer one. After reservations, only roads that open history share spare turns as roots are chosen; other stages fill the remaining room in order. Several people present retain rounds among their own memories.
5. **A road determines what opens.** A memory keeps its best score from all roads. Beyond a reminder's leaf reach, only expanding roads pass on their score and kind. Reminder proximity applies where no expanding road reached; a memory that expands as a root retains its own best strength and proximity.

Memory time is an episode's scene time, an observation's observed time or parent scene time, and creation time for other objects. Root scores of one from a participant key, name or activity stay ahead of zero-score place and time reminders. Among zero-score place and time roots, stored salience comes first, then the newest memory time.

Episodes and observations recalled through any road must be at or before the retrieval scene time, using those experience times. The exception is what an application-supplied range contributes: the scene time does not shorten that span. Interpreted memories and threads are not cut by their write times, and notions remain resolvable.

For a question about a span of time, the application supplies both endpoints with `.with_time_range(start, end)`. The library does not parse dates from the topic or call a model to determine the span. The range can accompany a topic and an activity:

```rust
use character_memory::{ActivityRef, MemoryId, RetrievalContext};
use chrono::{DateTime, Utc};

let start: DateTime<Utc> = "2026-09-15T00:00:00Z".parse()?;
let end: DateTime<Utc> = "2026-09-15T23:59:59.999999999Z".parse()?;
let thread_id: MemoryId = "00000000-0000-0000-0000-000000000007".parse()?;
let context = RetrievalContext::new("What happened last Tuesday?")
    .with_activity(ActivityRef::Thread(thread_id))
    .with_time_range(start, end);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Date match contributes the newest recallable episodes inside the range, including both endpoints, up to the largest requested section cap. All-zero section caps contribute none. Its provisional floor is one and only sets the reservation at roots and sections; it takes no spare root turn. Other cues can still recall episodes outside the range, and a shared hit keeps its score and standing. The reference scene time does not shorten the span. An inverted range matches nothing and is echoed unchanged, as every supplied range is, even with tracing off. With tracing enabled, `time_range_has_more` says whether the range contained more eligible occasions than this source contributed; later caps can still omit those contributions. It reads one extra ID and performs no count query. A crowded range is not a filtered topic search: an older topical memory inside it can still be missed when the unrestricted topic search ranks it below its cap.

A scene time reads back with the instant, fractional precision and offset supplied by the application. Supported offsets are whole minutes from -23:59 through +23:59, so returned scenes also round-trip through RFC 3339 JSON losslessly. `Scene::at` and `prepare` accept a scene whose offset has a seconds part; plan validation marks it invalid, and commit and retrieval reject it with a typed error carrying the offset in seconds. Anniversaries match the local calendar day of the offset each experience was written with against the retrieval scene's local day; library-generated scene times use UTC. Replaying the same ID and instant with a different offset is a collision, even when the local day is unchanged.

The same local month and day in an earlier local year contributes anniversary occasions as date matches, with February 29 matching only February 29. Shared and unshared anniversaries each contribute their own newest-first list, up to the room. An anniversary reserves room only when it was shared with a notion that the present scene resolves to one identity; an ambiguous name grants no reservation. The range and shared anniversaries share one date-match floor. Unshared anniversaries compete for unclaimed room by the root key: among zero-score roots, salience first, then newest. An ordinary anniversary therefore loses to a recent occasion of equal salience; a more salient one can come to mind.

Without a supplied range, recent recallable episodes at or before the scene time can come to mind without any other cue, together with memories resting on those occasions. Recorded scene time determines recency, not creation time. Recency contributes the newest occasions up to the largest requested section cap, in newest-first selector order; all-zero section caps contribute none. Its floor ships at zero until measured. It takes no spare turn at root selection and competes for room remaining after reservations and expanding-road turns. The root and section caps still bound what is recalled. Every reminder road, including scene descriptions, stays on its occasion: a person, thread or interpreted memory reached through it is a leaf. Another occasion does not open or inherit the reminder kind through shared work. An object that actually expands as a root keeps the best proximity of every road that reached it; objects reached only through roots inherit proximity from full-standing roads when present.

Once a recent occasion has root room, section selection uses the ordinary score: a low-scoring Topic descendant can yield to a salient recent occasion under a section cap. Recency has zero cue score, so even its most salient root scores at most 0.35; stronger Topic roots keep their places. A raised recency floor reserves the latest contributions at roots and sections. Remaining section room uses score, then memory time, type and ID for every memory. With a zero floor, an older, more salient contribution may survive when the newest does not. Chronological presentation belongs to the renderer: admitted memories already report their recorded source scenes.

Thread members are taken most recent first by creation time; an open loop's source episodes are taken most recent first by recorded scene time.

A description brings the most recent few recallable occasions among its fetched matches, at or before the scene time: as many as its cue kind's reserved amount, called its floor, and at least the latest one. Its other matches contribute nothing; the trace counts those left out by this limit. What was brought then competes by relevance. Keys and names take their kind's reserved room first, so adding a description does not displace what is already known merely to fill that reservation. Descriptions take no spare root turn. At roots, only expanding roads share turns; elsewhere, spare room follows the section rank order.

An occasion recalled by a description can bring what was observed and concluded there. It does not by itself bring the whole history of a person or thread mentioned there. A topic, key or name can still recall that history on its own strength.

Each cue kind has a floor at the caps it encounters. The measured defaults reserve one slot each for participant, place, activity and topic; date match provisionally reserves one and recency defaults to zero. Floors apply per kind, not per person or place: five people share one participant floor. `context.cue_floors` is a calibration knob for measured defaults; applications are not expected to set it. Slots are reserved in successive rounds: participant, place, activity, date match, topic, recency. Unused room returns to the common pool, and selected memories keep their ranked order. A zero floor reserves nothing for that kind; a zero cap admits nothing. A single kind also receives its floor from its own queue, then fills remaining room in final-ranked order. Calibration uses the public companion evaluation repository, a development aid outside the core library.

With tracing enabled, `floor_admissions` identifies the object, stage and cue kind when a reserved or spare turn admitted an object outside that stage's original capped prefix. Earlier-stage evidence remains even when a later stage omits the object.

State and participant-occasion reads bound eligible results and exclusion evidence separately. Exclusion evidence is a latest-first prefix within the caller's budget, not a complete audit. Fanout omission telemetry counts only omissions seen within fetched prefixes; it does not count the entire matching store.

The trace's `scene_cue_searches` reports the best similarity found for the setting and for the joined participant words, with the references sharing each search; a search with no matches has no score. Each entry identifies its cue kind and counts recallable matches left out by the description's occasion limit in `omitted_count`. There is no minimum similarity: whether a description is too weak to remind the character of anything remains a question for measurement, not a decision made by this score.

A participant present in most experiences brings fewer past encounters to mind, while retaining the latest eligible one within the requested limits. A participant recognized by key or name brings only encounters at or before the scene time. Several remarks or participants in one encounter do not make it count more than once. Familiarity limits recalled encounters, not beliefs about that participant. This applies to every notion, including whichever one the application regards as the character.

Meeting a participant recognized by key or name brings what is held about them, even if it was learned elsewhere. Beliefs and observations about that person share one bounded list, with current memories first, then higher salience, newer memory time and ID. Replaced beliefs appear only when `include_superseded` is enabled. The participant's shared occasions come before this aboutness list during expansion. Being mentioned does not count as being present or as an encounter. When several participants bring beliefs to mind, they take turns in scene order within the participant kind; a shared belief counts for each participant. The scene time limits episodes and observations, not interpreted beliefs. See the [retrieval design](docs/design/database/graph_schema_design.md#state-and-last-interaction) for selection details.

Retrieval statistics are derived from writes. Missing or unhealthy statistics use conservative fanout, normally one neighbour per covered relation/object bucket; the participant routes share the latest occasion, just as a ubiquitous participant does with healthy statistics. The library provides no statistics schema migration or graph-to-statistics rebuild; stores using a superseded schema must be recreated and populated through writes.

## What this is not

Character Memory is not:

- a generic vector database wrapper
- a chat history dump
- a simple user profile store
- a task-agent framework
- a replacement for an LLM

It is a memory layer for persistent AI assistants and companions.

## Memory permanence and data erasure

Character Memory treats the memory record as append-only. Forgetting suppresses a memory; correction supersedes it. Both remove influence while preserving history. Nothing fades on its own, and what is over leaves current views through a change of currency while staying fully recallable. There is no destructive deletion in the memory operations, because deleting memory rewrites a character's perceived history and breaks continuity.

Suppression and supersession are read from retention and incoming Supersedes links. Author a memory's `supersedes` list; commit derives the links and writes them with the objects in one graph batch. Caller-authored Supersedes links are rejected. Existing link IDs reject different content, including attempts to replace a generated Supersedes link with another relation. Correction uses the same derivation and leaves each predecessor unchanged, Active and superseded. Even a suppressed successor still supersedes its predecessor, so forgetting a correction never restores the older belief.

Settling a question or fulfilling a promise stops it appearing among current matters. It can still be remembered, and `resolved_by` identifies the memories that settled it. Resolved open loops and fulfilled commitments appear alongside ordinary interpreted memories rather than unfinished matters. Forgetting what settled a matter does not reopen it, and the original memory stays intact.

Superseded memories leave the content index and remain reachable from successors through graph expansion with `include_superseded`. Independently suppressed memories require `include_suppressed`. Default retrieval reports a superseded predecessor as superseded; when it is also suppressed, suppression takes precedence. Vector-delete failures are reported through a `RepairMarker::VectorMaintenance` containing the failed `Delete` operation; replaying the same commit repairs the delete while graph filtering excludes stale candidates. Retrieval-stat current counters cache currency derived from graph links and retention. Replaying an older plan or correction consults graph currency before indexing, so superseded memories stay out of candidate recall. A failed currency lookup is reported as a vector-indexing repair cause.

Forgetting a thread leaves its status and vector unchanged, so it remains reachable. Set `apply_to_thread_members` to suppress its interpreted members and remove their vectors from candidate recall. Source objects and opaque raw references remain preserved.

Applications with personal-data erasure obligations (for example, GDPR/CCPA deletion requests) own that compliance policy themselves. Erasure is an out-of-band operational action against the backing stores, not a memory operation exposed by the API, and no purge tooling ships with the library today. If you implement one, it must cover every store your deployment uses — the graph authority, the vector index, and retrieval statistics — and must tombstone or repair provenance references that would otherwise dangle, or the remaining record becomes inconsistent.

## Typical usage

Writes through one `CharacterMemory` value apply one at a time, so use one value per store.

A typical assistant loop looks like this:

1. The user says something.
2. The assistant retrieves relevant memories.
3. The retrieved memories are added to the LLM context.
4. The assistant responds.
5. Important parts of the interaction are stored as new memories.
6. Over time, memories reinforce character continuity.

Conceptually:

```text
user message
    ↓
retrieve relevant memories
    ↓
LLM prompt with memory context
    ↓
assistant response
    ↓
store new episode
    ↓
future interactions become more continuous
```

## Construction

`CharacterMemory::new(settings, collection_name).await?` constructs the default memory system.

By default, this uses:

- OpenAI for embeddings
- Embedded Qdrant Edge for local vector candidate recall with object-type scope filtering
- Embedded persistent Oxigraph for graph-authoritative memory objects, relationships, provenance, and lifecycle state

```rust
let memory = CharacterMemory::new(settings, "my-assistant-memory".to_owned()).await?;
```

For deterministic tests or custom embedding backends, use:

```rust
let memory = CharacterMemory::new_with_embedding_provider(
    settings,
    "my-assistant-memory".to_owned(),
    embed_provider,
).await?;
```

Your custom provider must implement `EmbeddingProvider`.

This is useful when you want to:

- use a local embedding model
- avoid embedding-provider network calls in tests
- make tests deterministic
- integrate another embedding API

## Write path

Character Memory separates planning from persistence. Use `prepare` to build an inspectable `RememberWritePlan`, `validate_plan` to check it against the current graph, and `commit` to persist it. `prepare` and `validate_plan` do not write graph objects, vector entries, retrieval statistics, or raw source data.

```rust
use character_memory::{Scene, SceneParticipant};

let mut scene = Scene::now(); // UTC; use Scene::at(application_time) to supply a local offset.
scene.participants.push(SceneParticipant {
    description: Some("a visitor".to_owned()),
    ..Default::default()
});
scene.setting.words = Some("a quiet room".to_owned());
let input = RememberInput::new("caller-provided note or transcript reference").with_scene(scene);

let plan = memory.prepare(input, PrepareOptions::default()).await?;
let validation = memory.validate_plan(&plan).await?;

if validation.iter().all(|candidate| candidate.status == CandidateValidationStatus::Valid) {
    let outcome = memory.commit(plan, CommitOptions::default()).await?;
}
```

`commit` revalidates the plan before writing. Graph-authoritative objects, links, provenance, lifecycle, and currentness are critical writes; vector indexing and retrieval-stat updates are repairable and are reported in `RememberOutcome`. Failed graph reads feeding the stats projection, including endpoint hydration and currency lookup, report `StatsUpdateCause::GraphRead`; the nested `GraphQueryError` retains the underlying failure.

For callers that want the standard write lifecycle in one call, `remember(RememberInput, RememberOptions)` composes `prepare`, `validate_plan`, and `commit` over the same graph-authoritative machinery.

An episode stores one `Scene` with its time, participants, setting and custom values. Setting and participant words have separate optional embeddings beside its summary; keys and custom values are excluded. If the input and episode draft omit the scene, `prepare` fixes the current UTC time once. `Scene::now()` and the default retrieval scene also use UTC. For anniversaries on the character's local day, the application supplies that offset in both write and retrieval scenes. A caller-built episode must supply its scene. Participant keys must identify existing or same-plan notions.

The write path is deliberately not an extraction system. Character Memory core does not infer preferences, commitments, corrections, character signals, thread membership, or entity identity from raw text. It does not store raw logs, and `raw_ref` values remain opaque caller-managed provenance pointers. Candidates in a `RememberWritePlan` are not memory until a valid plan is committed.

### Write warnings

A valid write can carry warnings. Read each candidate's `warnings` in the validations returned by `validate_plan`, or in `RememberOutcome.diagnostics.validations` after `remember` or `commit`. Each warning has a `kind` tag and identifies the affected memories or participant key; `candidate_index` and `candidate_kind` identify the candidate. The same warnings are also projected into `diagnostics.messages` with warning severity and the code `WritePlanValidationWarning`. Warnings neither refuse a write nor change what is stored.

- `duplicate_observation_echo`: an observation or interpreted memory repeats its source episode's summary exactly, so that wording is stored and indexed again.
- `resolver_without_shared_subject_or_thread`: a `Resolves` or `FulfillsCommitment` link joins interpreted memories with no shared subject and no shared thread. The matter leaves where things stand, but meeting the person alone will not bring the memory that says it was settled. The warning belongs to the link candidate and names the resolver and target, including when either is already stored.
- `repeated_scene_participant`: the same participant key occurs more than once in an episode's scene. Every entry is kept and reported, and its words contribute again to the participant search text; the write still creates only one participant link per distinct key. There is one warning per repeated key. Identical descriptions without keys do not establish that two participants are the same person.

`remember(RememberInput::new("Alice asked about the trip"), RememberOptions::default())` uses the same text for the episode summary and observation, so it carries the echo warning. Give the episode a distinct summary with `.with_episode(EpisodeDraft::new("Planning a trip with Alice"))`, or supply a distinct observation, to keep the two texts different. A resolution written through the separate `link` method carries no validation warnings; a link-only `RememberWritePlan` passed to `validate_plan` or `commit` does.

Recency and last met see an experience only at reference times at or after its scene time. A later scene time is stored as given and does not itself produce a warning.

## Notions and naming beliefs

An `Entity` represents a notion the character holds: an id, object type, creation time and schema version. An entity has no name. Construct its draft with `EntityDraft::new()` and supply or retain its id. Names and other descriptions belong to ordinary `DerivedMemory` beliefs about that id. A belief's `assertions: Vec<BeliefAssertion>` can carry `BeliefPredicate::KnownAs { name }` for a subject in its `entity_ids`. An assertion is the character's own commitment; hearsay, doubt and aspects without a mechanical reader remain plain text.

Set `given_by_application = true` when the application gives a belief about a notion without source episodes or observations. That declaration requires a notion subject and cannot coexist with source experiences. Without source experiences, the belief has no setting or custom context to recall it by; its subjects and the topic can still bring it to mind. With neither sources nor the declaration, admission fails. To initialize a notion and a given belief before any experience, author their candidates in a `RememberWritePlan` with explicit ids, timestamps and schema versions; validate and commit the plan. The `RememberInput` convenience path also creates an episode and observation for its input, but does not attach those as sources to a given belief.

A rename is an ordinary supersession. With caller-retained `notion_id`, `old_belief_id` and a fresh `new_belief_id`, prepare the new belief, inspect validation and commit:

```rust
use character_memory::{
    BeliefAssertion, BeliefPredicate, CandidateValidationStatus, CommitOptions,
    DerivedMemoryDraft, DerivedType, PrepareOptions, RememberInput,
};

let mut renamed = DerivedMemoryDraft::new(DerivedType::Reflection, "I know this notion as Bob.");
renamed.id = Some(new_belief_id);
renamed.entity_ids = vec![notion_id];
renamed.assertions = vec![BeliefAssertion {
    subject: notion_id,
    predicate: BeliefPredicate::KnownAs { name: "Bob".to_owned() },
}];
renamed.given_by_application = true;
renamed.supersedes = vec![old_belief_id];
let plan = memory.prepare(
    RememberInput::new("The application gives the name Bob.").with_derived_memory(renamed),
    PrepareOptions::default(),
).await?;
let validation = memory.validate_plan(&plan).await?;
assert!(validation.iter().all(|item| item.status == CandidateValidationStatus::Valid));
let outcome = memory.commit(plan, CommitOptions::default()).await?;
```

The old belief remains history. To change that given belief again through `correct`, explicitly declare the replacement's assertions and grounding; a rationale-only correction with no source experiences returns `LifecycleDtoValidationError::MissingGivenReplacement`. Neither the given marker nor old assertions are copied into a default replacement.

```rust
use character_memory::{
    BeliefAssertion, BeliefPredicate, CorrectMemoryDraft, CorrectionTarget, DerivedType,
    ExternalSourceReference, ReplacementDerivedMemoryDraft, SourceProvenanceReference,
};

let origin = SourceProvenanceReference {
    episode_ids: vec![],
    observation_ids: vec![],
    external_refs: vec![ExternalSourceReference::source("application:name-change")],
};
let mut replacement = ReplacementDerivedMemoryDraft::new(DerivedType::Correction, "The name is Carol.");
replacement.entity_ids = vec![notion_id];
replacement.assertions = vec![BeliefAssertion {
    subject: notion_id,
    predicate: BeliefPredicate::KnownAs { name: "Carol".to_owned() },
}];
replacement.given_by_application = true;
replacement.correction_origin_provenance = origin.clone();
let mut correction = CorrectMemoryDraft::new(
    CorrectionTarget::derived_memory(new_belief_id), "Update the given name.",
).with_replacement(replacement);
correction.correction_origin = origin;
let outcome = memory.correct(correction).await?;
// Persist outcome.graph_mutated_object_ids with the caller's external identifiers.
```

Notions have no vector surface. Recall starts from belief content and follows derived `About` links to its notions; commit creates one for every notion in the memory's `entity_ids`, and validates that every subject exists. Callers author that subject list and do not need to author the same `About` links. `RetrievalContext::object_type_defaults` scopes vector candidates, while `graph_limits.allowed_object_types` separately scopes traversal and includes entities by default.

Names retain their original spelling and match ignoring case, width and spacing; `Straße` with sharp s and `STRASSE` with double s still differ. There is no lookup by name; retain ids across restarts as described below.

## Memory identity across restarts

Lifecycle operations (`correct`, `forget`, `link`) address memories by `MemoryId`. Every operation that creates memory reports the resulting ids: `RememberOutcome` carries persisted object and link ids, `link` returns a `LinkOutcome` containing the created link, and `correct` reports generated replacement ids through `LifecycleMutationOutcome`. Retrieval packs also carry the ids of returned objects, and drafts (including replacement drafts in corrections) accept caller-supplied ids.

The public API deliberately provides no lookup by external id, no enumeration, and no query by source reference. Callers that need to reference memories across process or instance restarts own that mapping: either supply deterministic `MemoryId`s in drafts, or durably persist every id the API returns — including replacement ids from corrections — keyed by your own external identifiers. Retrieval verifies that memories survived a restart; it is not an identity-recovery mechanism.

Supplying deterministic ids gives you stable identity across retries. Replaying the same prepared plan accepts objects and links already stored with equal content and reapplies derived-store writes; reusing an id with different content is rejected. Preparing a draft again regenerates defaulted timestamps, so retain the prepared plan for an exact replay. Corrections also derive deterministic replacement ids when callers omit them.

## Backends

The default implementation is backed by embedded Qdrant Edge and embedded persistent Oxigraph. Configure one local vector root; each `collection_name` passed to the constructor owns one shard directory beneath it:

```text
VECTOR_STORE_MODE=embedded
VECTOR_STORE_PATH=./data/vectors
```

Embedded vector storage is single-process. It ships with Qdrant Edge's indexing threshold at zero, so candidate recall is an exhaustive scan and reports `VectorRecallCompleteness::Exhaustive`; index tuning is deliberately deferred. The vector shard is a rebuildable candidate index over graph authority, so moving between embedded and service modes means rebuilding vectors from the graph-authoritative objects rather than copying shard files.

On the 2026-09-04 Windows x86-64 development run at 1,536 dimensions, the reproducible ignored benchmark measured exhaustive query latency of 3 ms for 100 records, 11 ms for 1,000 records, and 48 ms for 5,000 records. Treat these as local guidance, not a performance guarantee; run `cargo test benchmark_configured_dimension_and_owner_responsiveness -- --ignored --nocapture --test-threads=1` on the deployment target before setting corpus expectations.

Call `memory.close().await?` before deleting the store directory to await resource release; acknowledged writes are already durable, and ordinary drop remains valid without waiting for the embedded owner.

The embedded engine (`qdrant-edge`) enables `serde_json::preserve_order` through Cargo feature unification, so consumers sharing that dependency get insertion-ordered `serde_json::Value` maps and must explicitly sort keys when producing canonical bytes.

To use a Qdrant service instead, select it explicitly and supply its gRPC endpoint:

```text
VECTOR_STORE_MODE=service
QDRANT_CONNECTION_STRING=http://127.0.0.1:6334
```

Both vector modes are candidate recall only. Oxigraph is the graph authority for memory objects, links, provenance, currentness, and lifecycle filtering. Local application construction defaults to `GRAPH_STORE_MODE=persistent` with `OXIGRAPH_PATH` set to a local filesystem path such as `./data/oxigraph`; deterministic tests and fixtures can use `GRAPH_STORE_MODE=in_memory`.

Build `Settings` from a caller-supplied `config::Config`; required values follow the selected backend. In-memory graphs need no `OXIGRAPH_PATH`. `CharacterMemory::new` requires `OPENAI_API_KEY` and `EMBEDDING_MODEL`, while `new_with_embedding_provider` ignores both and uses `EmbeddingProvider::vector_size()`; existing vector storage must match that dimension. Set `RETRIEVAL_STATS_STORE_MODE=in_memory` to keep statistics in memory without a path or file; SQLite uses `RETRIEVAL_STATS_PATH`, defaulting to `./data/retrieval-stats.sqlite3`. No placeholder values are needed for unused settings.

Raw source storage is outside Character Memory core. The library may preserve opaque `raw_ref` pointers for provenance, but raw logs are not stored by core graph/vector backends and no public raw-reference resolution API is part of v0.1.

## Running tests

By default, `cargo test` runs against embedded stores with deterministic embedding providers and needs neither a Qdrant service nor a `.env` file. The only integration tests that connect to a Qdrant service are the two service parity tests in `tests/vector_port_contract_tests.rs`; opt in with `REQUIRE_QDRANT_TESTS=1` and set `QDRANT_CONNECTION_STRING` to the service's gRPC endpoint.

```sh
cargo test
```

### Start Qdrant for service parity tests

The default gRPC port is `6334`.

Using Docker:

```sh
docker run -d \
  --name charactermemory-qdrant \
  -p 6333:6333 \
  -p 6334:6334 \
  qdrant/qdrant:v1.19.0
```

Or using Docker Compose:

```sh
docker compose -f docker-compose.qdrant.yml up -d
```

Then run the two service parity tests:

```sh
REQUIRE_QDRANT_TESTS=1 QDRANT_CONNECTION_STRING=http://127.0.0.1:6334 cargo test --test vector_port_contract_tests service_and_embedded_
```

## Status

Character Memory is under active development.

The v0.1 public architecture is graph-authoritative episodic continuity memory: public construction and facades compose an embedder, Qdrant candidate recall, and Oxigraph graph authority.

v0.1 does not store raw transcripts directly in graph/vector storage, run a reflection scheduler, implement a normalized belief ontology, support multimodal memory, or perform physical redaction/delete as a default lifecycle operation.

Production raw transcript storage is caller-owned and deferred. No public raw-reference resolution API is part of v0.1.
