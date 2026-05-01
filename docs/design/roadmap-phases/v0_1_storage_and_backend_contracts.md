# v0.1 Backend and Storage Contract Draft

## Version intent

This draft records the backend discipline needed for the Character Memory data model.

Default stack:

```text
Qdrant   = vector candidate recall + payload filtering
Oxigraph = embedded in-memory RDF/SPARQL graph authority
Raw refs = stable pointers to caller-owned source material
```

The implementation should remain backend-abstract where practical.

Public construction and facades compose an embedder, a Qdrant vector candidate store, and an embedded in-memory Oxigraph graph authority store. Qdrant results are candidates only; Oxigraph is authoritative for memory objects and relationships within the running process. Persistent Oxigraph storage configuration remains v0.1.1 future work.

---

# 1. Store responsibilities

## 1.1 Raw source material

Production raw storage is caller-owned and deferred in v0.1. The graph/vector memory stores preserve source pointers, but they do not own the original interaction material.

Examples:

```text
chat transcript
voice transcript
source conversation log
selected excerpts
```

The graph/vector layer does not store raw transcripts as v0.1 memory content. It stores summaries, excerpts, derived memories, and stable `raw_ref` pointers. A `raw_ref` identifies where source material can be found by the caller's transcript system; it is not the transcript content itself.

```json
{
  "raw_ref": "raw://conversation/chat_123#turn_42"
}
```

## 1.2 Graph store

Authoritative for:

```text
memory object existence
object types
entity links
thread links
provenance links
correction/supersession links
retention state
currentness
```

Default: embedded in-memory Oxigraph RDF/SPARQL. Persistent Oxigraph storage configuration remains v0.1.1 future work.

## 1.3 Vector store

Responsible only for:

```text
candidate recall
semantic similarity
coarse payload filtering
```

Default: Qdrant.

The vector store is not the source of truth for memory existence, relationships, provenance, currentness, or correction.

---

# 2. Stable ID and IRI strategy

Every durable object has an ID.

Recommended prefixes:

```text
ep_       Episode
obs_      Observation
ent_      Entity
thread_   MemoryThread
dm_       DerivedMemory
link_     MemoryLink
ret_      RetentionAssessment, future
trace_    RetrievalTrace, future
```

Graph IRI generation should be deterministic:

```text
urn:cmem:episode:<id>
urn:cmem:observation:<id>
urn:cmem:entity:<id>
urn:cmem:thread:<id>
urn:cmem:derived-memory:<id>
urn:cmem:link:<id>
```

Use one canonical function per object type:

```rust
fn graph_uri(object_type: MemoryObjectType, id: &str) -> Result<String, MemoryError>;
```

Acceptance criteria:

```text
same object always maps to same graph URI
vector payload stores graph_uri and object id
raw_ref pointers are preserved and unresolved refs remain representable
```

---

# 3. Qdrant collection design

## 3.1 Record types

v0.1 indexed record types:

```text
episode
observation
derived_memory
memory_thread
entity
```

Do not index every raw turn. Index only records useful for recall.

## 3.2 Embedding text policy

Embed natural-language semantic surfaces, not structured metadata templates.

Good:

```text
The user wanted the starter Character Memory implementation to stay chat-native and avoid overbuilding future embodied features.
```

Bad:

```text
record_type: derived_memory; salience_score: 0.91; thread_id: thread_character_memory_design; ...
```

Keep metadata in payload.

## 3.3 Generic payload

```json
{
  "id": "dm_01h...",
  "graph_uri": "urn:cmem:derived-memory:dm_01h...",
  "object_type": "derived_memory",
  "record_type": "derived_memory",
  "derived_type": "reflection",

  "embedding_text": "The starter Character Memory implementation should be chat-native and episodic-first.",
  "content_text": "The starter implementation should avoid overbuilding multimodal or embodied support.",

  "episode_ids": ["ep_01h..."],
  "observation_ids": ["obs_01h..."],
  "thread_ids": ["thread_character_memory_design"],
  "entity_ids": ["ent_project_character_memory"],

  "modality": "chat",
  "scope": "project",

  "created_at": "2026-04-26T10:46:00+09:00",
  "observed_at": "2026-04-26T10:12:00+09:00",
  "last_touched_at": "2026-04-26T10:46:00+09:00",

  "salience_score": 0.92,
  "confidence": 0.88,
  "stability": "medium",
  "is_current": true,
  "is_superseded": false,
  "retention_state": "active",

  "schema_version": "episodic_memory_initial"
}
```

## 3.4 Suggested payload indexes

High-value indexes:

```text
object_type / record_type
derived_type
entity_ids
thread_ids
episode_ids
modality
created_at / observed_at / last_touched_at
is_current
is_superseded
retention_state
salience_score
```

Secondary indexes:

```text
scope
stability
source_conversation_id
participant_ids
```

Additional caller-specific payload fields can be added when they serve filtering, but they are not core graph authority.

---

# 4. Oxigraph/RDF model

## 4.1 Minimal RDF classes

```turtle
cmem:Episode a rdfs:Class .
cmem:Observation a rdfs:Class .
cmem:Entity a rdfs:Class .
cmem:MemoryThread a rdfs:Class .
cmem:DerivedMemory a rdfs:Class .
cmem:MemoryLink a rdfs:Class .
```

## 4.2 Minimal predicates

```turtle
cmem:hasObservation
cmem:observedIn
cmem:mentions
cmem:involves
cmem:about
cmem:partOfThread
cmem:derivedFrom
cmem:supersedes
cmem:contradicts
cmem:resolves
cmem:createdAt
cmem:startedAt
cmem:endedAt
cmem:observedAt
cmem:hasRetentionState
cmem:isCurrent
cmem:salienceScore
```

## 4.3 Example triples

```turtle
<urn:cmem:episode:ep_001> a cmem:Episode ;
    cmem:startedAt "2026-04-26T10:00:00+09:00"^^xsd:dateTime ;
    cmem:endedAt "2026-04-26T10:45:00+09:00"^^xsd:dateTime ;
    cmem:hasObservation <urn:cmem:observation:obs_001> ;
    cmem:partOfThread <urn:cmem:thread:character_memory_design> ;
    cmem:mentions <urn:cmem:entity:project_character_memory> .

<urn:cmem:derived-memory:dm_001> a cmem:DerivedMemory ;
    cmem:derivedFrom <urn:cmem:episode:ep_001> ;
    cmem:about <urn:cmem:entity:project_character_memory> ;
    cmem:partOfThread <urn:cmem:thread:character_memory_design> ;
    cmem:isCurrent true ;
    cmem:salienceScore "0.92"^^xsd:decimal .
```

## 4.4 Core SPARQL helpers

v0.1 should support:

```text
context by object id
episodes by entity
episodes by thread
observations by episode
derived memories by episode/provenance
derived memories by thread/entity
active threads by last_touched_at
current derived memories only
suppressed/archived filtering
```

Example query intents:

```text
get context around dm_123
get all current memories derived from ep_123
get active threads involving entity X
get relevant observations from thread Y in time range
```

---

# 5. Hybrid retrieval contract

Internal flow:

```text
1. Vector search over natural-language surfaces.
2. Collect candidate graph IDs.
3. Resolve and expand candidates through the graph authority.
4. Add thread/entity/provenance context.
5. Filter by lifecycle state.
6. Rerank.
7. Return RetrieveOutcome with the assembled ContinuityContextPack plus rationale/trace metadata.
```

Public API should expose:

```rust
async fn retrieve(&self, context: RetrievalContext) -> Result<RetrieveOutcome, CustomError>;
```

Optional debug trace:

```json
{
  "vector_candidates": [],
  "graph_expansions": [],
  "filters_applied": [],
  "ranking_features": [],
  "final_selection_reason": []
}
```

---

# 6. Performance controls

Keep graph expansion bounded and predictable.

```json
{
  "max_vector_candidates": 40,
  "max_graph_depth": 2,
  "max_neighbors_per_node": 25,
  "max_thread_episodes": 10,
  "max_entity_fanout": 20,
  "timeout_ms": 500
}
```

Hub handling:

```text
If an entity appears in too many memories, expand through recent/high-salience/current links first.
Do not blindly fetch every memory involving the user or assistant.
```

---

# 7. Test harness

## 7.1 Golden fixtures

Create fixtures for:

```text
simple episode
episode with observation
episode with entity links
episode with thread link
derived memory from episode
correction superseding derived memory
suppressed memory
hub entity scenario
```

## 7.2 Required tests

```text
ID/IRI determinism
Qdrant payload validation
Qdrant filter correctness
RDF triple generation
SPARQL query regression
hybrid retrieval deterministic output under fixed fixtures
retention filtering
supersession filtering
bounded graph expansion
```

## 7.3 Migration tests

Every schema version should include:

```text
forward migration test
backward compatibility expectation
clear failure behavior if migration is required
```

---

# 8. Design Summary

## Core backend commitments

```text
Qdrant/Oxigraph defaults
shared IDs and deterministic IRIs
payload filters
SPARQL regression tests
hybrid retrieval
bounded graph expansion
non-core examples
```

## Current data model direction

```text
typed memory objects
object_type and derived_type payload/schema markers
ContinuityContextPack retrieval output
optional contextual fields instead of mandatory placeholders
DerivedMemory now, richer Claim/Belief subsystem later
```
