# Database Schema Cheat Sheet

This is the compact schema reference. The companion design notes explain why the schema is shaped this way:

- [Vector Database Payload Design](vector_db_metadata_schema.md)
- [Graph Database Schema Design](graph_db_schema.md)

## Authority Split

| Store | Role | Authoritative For | Not Authoritative For |
|---|---|---|---|
| Qdrant | Vector candidate recall and coarse payload filtering | Vector points, embedding surfaces, payload hints | Memory existence, relationships, provenance, lifecycle, currentness |
| Oxigraph | Graph authority | Memory objects, typed links, provenance, lifecycle, currentness, expansion context | Semantic nearest-neighbor ranking |
| Raw store / caller storage | Source material | Raw transcript or source content behind `raw_ref` | Canonical memory state |

## Cross-Store Join Keys

| Field | Stored In | Purpose |
|---|---|---|
| `object_id` / `objectId` | Qdrant payload and graph literal | Stable object UUID |
| `graph_uri` / `graphUri` | Qdrant payload and graph literal | Stable graph resource pointer |
| `schema_version` / `schemaVersion` | Qdrant payload and graph literal | Persistence and migration marker |

Graph URI pattern:

```text
urn:cmem:episode:<uuid>
urn:cmem:observation:<uuid>
urn:cmem:entity:<uuid>
urn:cmem:thread:<uuid>
urn:cmem:derived-memory:<uuid>
urn:cmem:link:<uuid>
```

## Qdrant Payload Fields

### Identity And Surface

| Field | Type / Shape | Notes |
|---|---|---|
| `object_id` | keyword UUID string | Stable vector-to-graph join id |
| `graph_uri` | keyword URI string | Stable graph resource pointer |
| `object_type` | keyword enum | Canonical memory object type |
| `record_type` | keyword enum | Indexed vector record kind |
| `schema_version` | keyword string | Payload migration marker |
| `surface` | keyword enum | Embedded semantic surface |
| `embedding_text` | string | Text used to generate the vector |
| `content_text` | string | Compact readable/debug text |

### Object-Specific Hints

| Field | Type / Shape | Notes |
|---|---|---|
| `derived_type` | keyword enum | Derived memory subtype |
| `entity_type` | keyword enum | Entity subtype |
| `thread_status` | keyword enum | Thread lifecycle/status hint |
| `modality` | keyword enum | Source modality |
| `source_conversation_id` | keyword string | Source conversation filter |
| `canonical_key` | keyword string | Stable caller/domain key |

### Relationship Hints

Relationship fields in Qdrant are filter hints only. Oxigraph remains authoritative.

| Field | Type / Shape | Notes |
|---|---|---|
| `episode_ids` | keyword array | Related episode ids |
| `observation_ids` | keyword array | Related observation ids |
| `thread_ids` | keyword array | Related thread ids |
| `entity_ids` | keyword array | Related entity ids |
| `participant_entity_ids` | keyword array | Episode participant ids |
| `speaker_entity_id` | keyword UUID string | Observation speaker id |
| `supersedes` | keyword array | Supersession hint |

### Lifecycle, Ranking, And Time Hints

| Field | Type / Shape | Notes |
|---|---|---|
| `retention_state` | keyword enum | Lifecycle filter hint |
| `is_current` | bool | Currentness hint |
| `is_superseded` | bool | Supersession/currentness hint |
| `salience_score` | float | Ranking/filter hint |
| `confidence` | float | Ranking/filter hint |
| `stability` | keyword enum | Derived memory stability |
| `created_at` | datetime | Creation time |
| `updated_at` | datetime | Update time |
| `started_at` | datetime | Episode start time |
| `ended_at` | datetime | Episode end time |
| `observed_at` | datetime | Observation time |
| `last_touched_at` | datetime | Thread recency |
| `raw_ref` | keyword string | Source pointer, not raw transcript content |

## Qdrant Indexed Object Types

```text
episode
observation
entity
memory_thread
derived_memory
```

`memory_link` is not indexed as a semantic memory object by default. Links are graph-authoritative relationship records.

## Oxigraph Classes

| Class URI | Domain Object |
|---|---|
| `urn:cmem:vocab:Episode` | `Episode` |
| `urn:cmem:vocab:Observation` | `Observation` |
| `urn:cmem:vocab:Entity` | `Entity` |
| `urn:cmem:vocab:MemoryThread` | `MemoryThread` |
| `urn:cmem:vocab:DerivedMemory` | `DerivedMemory` |
| `urn:cmem:vocab:MemoryLink` | `MemoryLink` |

## Oxigraph Predicates

### Common Object Predicates

| Predicate URI | Purpose |
|---|---|
| `urn:cmem:vocab:objectId` | Stable UUID literal |
| `urn:cmem:vocab:objectType` | Canonical object type literal |
| `urn:cmem:vocab:graphUri` | Stable graph URI literal |
| `urn:cmem:vocab:schemaVersion` | Schema migration marker |
| `urn:cmem:vocab:createdAt` | Creation timestamp |
| `urn:cmem:vocab:updatedAt` | Update timestamp |

### Episode And Observation Predicates

| Predicate URI | Purpose |
|---|---|
| `urn:cmem:vocab:modality` | Source modality |
| `urn:cmem:vocab:sourceConversationId` | Source conversation id |
| `urn:cmem:vocab:startedAt` | Episode start time |
| `urn:cmem:vocab:endedAt` | Episode end time |
| `urn:cmem:vocab:participantEntity` | Episode participant entity edge |
| `urn:cmem:vocab:summary` | Episode/thread/entity summary |
| `urn:cmem:vocab:rawRef` | Source pointer |
| `urn:cmem:vocab:salienceScore` | Salience literal |
| `urn:cmem:vocab:retentionState` | Lifecycle state |
| `urn:cmem:vocab:episode` | Observation-to-episode edge |
| `urn:cmem:vocab:speakerEntity` | Observation speaker entity edge |
| `urn:cmem:vocab:observedAt` | Observation time |
| `urn:cmem:vocab:text` | Observation or derived memory text |

### Entity And Thread Predicates

| Predicate URI | Purpose |
|---|---|
| `urn:cmem:vocab:entityType` | Entity subtype |
| `urn:cmem:vocab:name` | Entity display name |
| `urn:cmem:vocab:alias` | Entity alias |
| `urn:cmem:vocab:canonicalKey` | Stable caller/domain key |
| `urn:cmem:vocab:title` | Thread title |
| `urn:cmem:vocab:threadStatus` | Thread status |
| `urn:cmem:vocab:lastTouchedAt` | Thread recency |

### Derived Memory Predicates

| Predicate URI | Purpose |
|---|---|
| `urn:cmem:vocab:derivedType` | Derived memory subtype |
| `urn:cmem:vocab:derivedFromEpisode` | Provenance edge to episode |
| `urn:cmem:vocab:derivedFromObservation` | Provenance edge to observation |
| `urn:cmem:vocab:partOfThread` | Thread membership edge |
| `urn:cmem:vocab:aboutEntity` | Entity/topic edge |
| `urn:cmem:vocab:confidence` | Confidence literal |
| `urn:cmem:vocab:stability` | Stability literal |
| `urn:cmem:vocab:isCurrent` | Currentness literal |
| `urn:cmem:vocab:supersedes` | Supersession edge |

### MemoryLink Predicates

| Predicate URI | Purpose |
|---|---|
| `urn:cmem:vocab:from` | Link source resource |
| `urn:cmem:vocab:fromType` | Link source object type |
| `urn:cmem:vocab:to` | Link target resource |
| `urn:cmem:vocab:toType` | Link target object type |
| `urn:cmem:vocab:relation` | Relation enum literal |
| `urn:cmem:vocab:rationale` | Optional relationship rationale |
| `urn:cmem:vocab:confidence` | Link confidence literal |
| `urn:cmem:vocab:createdAt` | Link creation timestamp |
| `urn:cmem:relation:<relation_name>` | Direct traversal predicate emitted for typed links |

## Retrieval Rule Of Thumb

```text
Qdrant narrows candidates.
Oxigraph verifies graph truth.
The final context pack follows Oxigraph state.
```
