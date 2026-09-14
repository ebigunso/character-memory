# v0.1 Design Draft: Starter Episodic Memory

## Version intent

v0.1 should be the smallest useful Character Memory implementation.

It should feel like:

```text
long-term episodic memory for a persistent assistant
```

not:

```text
generic vector/RDF search over memory records
```

The system should remember chat-native interactions, extract what matters, link those memories to entities and soft threads, and retrieve continuity context for future conversations.

The public v0.1 architecture is graph-authoritative: Qdrant supplies vector candidates, while embedded Oxigraph owns memory objects, links, provenance, currentness, and lifecycle state within the running process. Persistent Oxigraph storage configuration is future work.

---

# 1. Why this version comes first

Character continuity starts with remembered episodes.

The system cannot form stable character signals, relationship context, or meaningful reflection if it cannot first preserve:

```text
what happened
when it happened
who/what was involved
why it mattered
what was derived from it
```

v0.1 therefore focuses on the episodic substrate and minimal derived memory.

---

# 2. Non-goals

Do not implement in v0.1:

```text
full factual truth maintenance
separate Assertion / Claim / EvidenceLink / BeliefAssessment classes
domain-scoped source credibility
full relationship-state model
sleep-like consolidation scheduler
raw transcript storage in graph/vector stores
multimodal event segmentation
robotic situation frames
heavy ontology reasoning
complex spreading activation
learned admission control
physical redaction/delete as the default lifecycle path
```

These are future layers.

---

# 3. Core concepts

## 3.1 Episode

A bounded remembered interaction or meaningful segment.

For v0.1, an episode is usually:

```text
a chat session
a segment of a chat session
a voice transcript segment
a meaningful interaction chunk
```

Example:

```json
{
  "id": "ep_01h...",
  "object_type": "episode",
  "modality": "chat",
  "source_conversation_id": "chat_123",
  "started_at": "2026-04-26T10:00:00+09:00",
  "ended_at": "2026-04-26T10:45:00+09:00",
  "participants": ["ent_user_primary", "ent_assistant_self"],
  "summary": "The user asked for a YAGNI starter architecture for Character Memory.",
  "raw_ref": "raw://conversation/chat_123",
  "salience_score": 0.86,
  "retention_state": "active",
  "schema_version": "episodic_memory_initial"
}
```

Notes:

- `raw_ref` should point to the source conversation or transcript when available.
- `location_text` is optional. Do not force `unknown` placeholders as public data.
- `modality` starts with `chat` and `voice_transcript`.

## 3.2 Observation

A salient excerpt or observation inside an episode.

For chat, this may be a user message, assistant message, or condensed excerpt.

```json
{
  "id": "obs_01h...",
  "object_type": "observation",
  "episode_id": "ep_01h...",
  "speaker_entity_id": "ent_user_primary",
  "observed_at": "2026-04-26T10:12:00+09:00",
  "modality": "chat",
  "text": "I want to leave room for evolution without major breaking changes, but not have everything at the start.",
  "raw_ref": "raw://conversation/chat_123#turn_42",
  "salience_score": 0.91,
  "schema_version": "episodic_memory_initial"
}
```

Do not store every turn as an observation by default. Store observations when they are salient.

## 3.3 Entity

A recurring person, project, tool, place, document, concept, or other memory anchor.

```json
{
  "id": "ent_project_character_memory",
  "object_type": "entity",
  "entity_type": "project",
  "name": "Character Memory",
  "aliases": ["character-memory", "memory library"],
  "summary": "A library for long-term episodic memory and character continuity.",
  "schema_version": "episodic_memory_initial"
}
```

Starter entity types:

```text
person
project
concept
tool
document
place
organization
assistant
user
other
```

## 3.4 MemoryThread

A soft persistent continuity pattern.

A thread is not the same thing as a chat thread. It is a continuity structure across episodes.

```json
{
  "id": "thread_character_memory_design",
  "object_type": "memory_thread",
  "title": "Character Memory design",
  "summary": "Ongoing design discussion about an episodic memory substrate for persistent AI character.",
  "status": "active",
  "last_touched_at": "2026-04-26T10:45:00+09:00",
  "salience_score": 0.94,
  "schema_version": "episodic_memory_initial"
}
```

Thread membership is a link with confidence:

```json
{
  "id": "link_01h...",
  "from_id": "ep_01h...",
  "to_id": "thread_character_memory_design",
  "relation": "part_of_thread",
  "confidence": 0.88,
  "rationale": "The episode continues the Character Memory architecture discussion."
}
```

## 3.5 DerivedMemory

A memory derived from one or more episodes or observations.

This is the v0.1 simplification that avoids premature schema explosion.

```json
{
  "id": "dm_01h...",
  "object_type": "derived_memory",
  "derived_type": "reflection",
  "text": "The starter implementation should be chat-native and episodic-first, with extensible hooks for future modalities.",
  "derived_from_episode_ids": ["ep_01h..."],
  "derived_from_observation_ids": ["obs_01h..."],
  "thread_ids": ["thread_character_memory_design"],
  "entity_ids": ["ent_project_character_memory"],
  "confidence": 0.88,
  "salience_score": 0.92,
  "stability": "medium",
  "is_current": true,
  "supersedes": [],
  "retention_state": "active",
  "created_at": "2026-04-26T10:46:00+09:00",
  "schema_version": "episodic_memory_initial"
}
```

Starter `derived_type` values:

```text
reflection
user_preference
assistant_behavior_note
commitment
open_loop
character_signal
relationship_note
project_note
claim
correction
```

These can later become specialized classes if needed.

## 3.6 MemoryLink

A typed relation between memory objects.

```json
{
  "id": "link_01h...",
  "object_type": "memory_link",
  "from_id": "dm_01h...",
  "to_id": "ep_01h...",
  "relation": "derived_from",
  "confidence": 1.0,
  "rationale": "The reflection was generated from the episode.",
  "created_at": "2026-04-26T10:46:00+09:00",
  "schema_version": "episodic_memory_initial"
}
```

Starter relation types:

```text
has_observation
mentions
involves
about
derived_from
part_of_thread
supports
contradicts
supersedes
resolves
creates_open_loop
fulfills_commitment
associated_with
```

Admission note: `associated_with`-style links are admissible only with stronger evidence or explicit application intent until v0.5 associative structures exist. Shared low-selectivity co-occurrence alone must not create durable pairwise links; see [ADR-I-0011](../../decisions/implementation/ADR-I-0011-guard-against-low-information-co-occurrence-links.md).

---

# 4. Starter write pipeline

## 4.1 Working buffer

During interaction, maintain non-durable working state:

```text
recent turns
active entities
candidate thread hints
candidate preferences
candidate corrections
candidate commitments
candidate open loops
```

This is not long-term memory yet.

## 4.2 Event/session boundary commit

At session end or a meaningful segment boundary:

```text
create Episode
extract salient Observations
extract/link Entities
score salience
link to existing MemoryThreads or create candidate thread
create obvious DerivedMemories
write authoritative objects and graph links
index selected records in vector candidate store
```

## 4.3 Immediate persistence cases

Some things should be persisted immediately or near-immediately:

```text
explicit correction
explicit “remember this” request
explicit “forget this” request
assistant commitment
user preference stated clearly
safety/privacy instruction
```

---

# 5. Starter retrieval pipeline

Public retrieval should return a `ContinuityContextPack`.

```json
{
  "query": "What should the starter implementation include?",
  "active_threads": [],
  "relevant_episodes": [],
  "salient_observations": [],
  "derived_memories": [],
  "preferences": [],
  "open_loops": [],
  "commitments": [],
  "character_signals": [],
  "retrieval_rationale": []
}
```

Internal steps:

```text
1. Build natural-language query surface from current context.
2. Use Qdrant to find candidate episode, observation, derived_memory, thread, and entity records.
3. Resolve and expand candidates through Oxigraph by entity, thread, lifecycle state, and provenance.
4. Filter suppressed/archived/non-current records.
5. Rerank by semantic similarity, thread match, entity overlap, recency, salience, and open-loop priority.
6. Format a `RetrieveOutcome` containing a compact continuity context pack in `pack`.
```

---

# 6. Starter public API

```rust
async fn remember(&self, draft: RememberDraft) -> Result<RememberOutcome, CustomError>;
async fn retrieve(&self, context: RetrievalContext) -> Result<RetrieveOutcome, CustomError>;
async fn correct(&self, draft: CorrectMemoryDraft) -> Result<LifecycleMutationOutcome, CustomError>;
async fn forget(&self, draft: ForgetMemoryDraft) -> Result<LifecycleMutationOutcome, CustomError>;
async fn link(&self, draft: MemoryLinkDraft) -> Result<MemoryLink, CustomError>;
```

Optional low-level diagnostics:

```text
RetrieveOutcome may include light RetrievalRationale or RetrievalTelemetry when the caller enables diagnostic output.
```

Low-level graph/vector APIs are internal in v0.1; public diagnostics are limited to optional per-retrieval rationale or telemetry. Durable first-class `RetrievalTrace` objects are deferred to v0.4.

---

# 7. Acceptance criteria

## Storage

```text
Episode can be created with stable ID.
Observation can point to Episode.
DerivedMemory can point to Episode/Observation.
Entity links can be created and queried.
Thread links can be created, confidence-scored, and revised.
```

## Retrieval

```text
retrieve() returns RetrieveOutcome.
RetrieveOutcome.pack is the ContinuityContextPack and includes rationale for included memories.
Suppressed/archived memories are excluded.
Current derived memories outrank superseded ones.
Thread and entity matches influence ranking.
```

## Correction

```text
Correction creates new DerivedMemory or updates lifecycle state.
Old memory can be superseded without deletion.
Provenance to correction episode is preserved.
```

## YAGNI

```text
No separate claim/evidence subsystem required.
No full reflection scheduler required.
No multimodal event segmentation required.
No heavy ontology machinery required.
```

---

# 8. Design Commitments

v0.1 keeps:

```text
stable IDs
Qdrant vector recall
Oxigraph/RDF graph relationships
hybrid vector + graph retrieval
schema versioning
regression tests
bounded graph expansion
```

v0.1 uses:

```text
Episode/Observation/DerivedMemory
ContinuityContextPack
object_type + derived_type
optional context fields
```
