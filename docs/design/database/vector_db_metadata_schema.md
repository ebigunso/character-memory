# Vector Database Payload Design

This document describes the v0.1 Qdrant payload design for Character Memory.
It is intentionally a design note, not a field-by-field copy of the Rust
mapping code.

Qdrant is the semantic candidate index. It is not the memory database of record.
The authoritative memory state lives in the graph store. A Qdrant hit means
"this object may be relevant"; it does not mean "this object is current,
related, or safe to include." Retrieval must verify candidates through the
graph authority before returning them in a continuity context pack.

## Design Goal

The vector payload exists to make candidate recall cheap without duplicating
the graph model.

That leads to three rules:

1. Store natural-language embedding surfaces in vectors.
2. Store only enough metadata to prefilter and join candidates.
3. Treat relationship and lifecycle payload fields as hints until Oxigraph
   verifies them.

This split is deliberate. A vector database is excellent at finding nearby text
and applying coarse filters. It is not the right place to decide graph truth,
supersession, provenance, or lifecycle policy.

## Record Shape

Each indexed Qdrant point represents one vector surface for one canonical memory
object. The point id may be adapter-specific, so the stable identity is carried
in payload:

```text
object_id  stable UUID for the memory object
graph_uri  deterministic graph URI for joining to Oxigraph
surface    semantic surface that was embedded
```

The same object can have more than one surface over time, but the v0.1 mapping
keeps the join key stable and explicit. This lets retrieval collect vector
candidates, deduplicate by object identity, and ask the graph store for the
authoritative object.

Indexed object types are:

```text
episode
observation
entity
memory_thread
derived_memory
```

`memory_link` is graph-authoritative relationship data and is not indexed as a
semantic memory object by default.

## Why Natural-Language Surfaces

Embedding text should describe the memory in language a model or user might use
later. It should not be a serialized metadata template.

Good embedding surface:

```text
The user prefers deterministic public facade tests.
```

Poor embedding surface:

```text
object_type=derived_memory; retention_state=active; confidence=0.82
```

The first supports semantic recall. The second teaches the embedding model
about field names rather than memory meaning. Metadata belongs in payload
filters, not in the embedded text.

## Payload Categories

The implemented payload fields fall into a few design categories.

### Identity And Versioning

```text
object_id
graph_uri
object_type
record_type
schema_version
surface
```

These fields make vector-to-graph joins deterministic and make future migration
auditable. `object_type` and `record_type` are currently the same for v0.1, but
both are retained so future vector records can diverge from domain object
classes without changing the join contract.

### Text Surfaces

```text
embedding_text
content_text
```

`embedding_text` is the exact text used to generate the vector. `content_text`
is a compact readable payload for debugging, inspection, and possible
re-indexing workflows. Neither field is a raw transcript store.

Raw interaction material should be addressed through `raw_ref` pointers owned
by a raw store or caller-managed transcript system.

### Object-Specific Filter Hints

```text
derived_type
entity_type
thread_status
modality
source_conversation_id
canonical_key
```

These fields let Qdrant avoid returning obviously irrelevant records. They are
not a substitute for domain validation. For example, `thread_status` may help
avoid archived threads during candidate recall, but the graph store still
decides whether a thread belongs in the final context pack.

### Relationship Hints

```text
episode_ids
observation_ids
thread_ids
entity_ids
participant_entity_ids
speaker_entity_id
supersedes
```

These are denormalized hints. They exist because filters like "memories about
this entity" or "memories in this thread" are common and should not require a
large vector search before graph expansion.

They are intentionally called hints because relationships are graph facts. If a
payload says a memory is connected to an entity but the graph no longer agrees,
retrieval must follow the graph.

### Lifecycle And Ranking Hints

```text
retention_state
is_current
is_superseded
salience_score
confidence
stability
```

These fields reduce work before graph verification. They also make lifecycle
cleanup more visible during operational inspection.

They are not sufficient for final inclusion. v0.1 explicitly supports the case
where vector cleanup fails after graph mutation: retrieval should still exclude
stale graph records even if Qdrant still returns them.

### Time Hints

```text
created_at
updated_at
started_at
ended_at
observed_at
last_touched_at
```

Time filters support recency and episode/thread constraints without embedding
time into the semantic vector. This keeps time an explicit retrieval dimension
instead of hoping the embedding captures it.

### Source Pointer

```text
raw_ref
```

`raw_ref` preserves a pointer to source material without storing the full raw
chat or voice transcript in Qdrant. This keeps v0.1 chat-native while avoiding a
premature raw-storage policy.

## Indexing Policy

High-value Qdrant payload indexes are the fields that either reduce candidate
set size or protect lifecycle correctness:

```text
object_id
graph_uri
object_type
record_type
derived_type
entity_type
thread_status
schema_version
retention_state
episode_ids
observation_ids
thread_ids
entity_ids
participant_entity_ids
speaker_entity_id
supersedes
modality
source_conversation_id
canonical_key
created_at
updated_at
started_at
ended_at
observed_at
last_touched_at
is_current
is_superseded
salience_score
confidence
stability
raw_ref
```

The intent is not to index every interesting fact. The intent is to index facts
that are useful before graph expansion. Rich relationship traversal belongs in
the graph store.

## Consistency Model

Writes that affect indexed memory normally update both stores:

```text
remember     graph upsert, then vector upsert
correct      graph supersession, then vector delete/upsert maintenance
forget       graph lifecycle mutation, then vector delete maintenance
retrieve     vector candidates, then graph verification
```

This is not a distributed transaction. The design assumes partial vector
maintenance failure can happen. The correctness rule is therefore:

```text
Graph state wins over Qdrant payload state.
```

If Qdrant is stale, retrieval may do extra work, but it should not return stale
or suppressed memories as current context after graph verification.

## What Changed From The Old Schema

The previous schema described flat `episodic` and `semantic` memory entries with
fields such as `memory_type`, `content`, `timestamp`, `location_text`, and
`participants`.

That no longer matches v0.1.

v0.1 uses typed memory objects:

```text
Episode
Observation
Entity
MemoryThread
DerivedMemory
MemoryLink
```

The vector schema indexes semantic surfaces for the object types that benefit
from recall. It does not preserve the old flat `memory_type` split, and it does
not treat location or participant text as required episodic fields. Participants
are entity relationships, and relationships are graph-authoritative.

## Future Revisit Points

Revisit this design when:

- persistent Oxigraph configuration lands and cross-process graph/vector
  consistency needs stronger operational guarantees
- multiple vector surfaces per object become common enough to require public
  surface policy
- spatial/location retrieval becomes a real product requirement
- a belief/claim subsystem introduces new indexable object types
