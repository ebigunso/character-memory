# Database Schema Cheat Sheet

This is the compact schema reference. The companion design notes explain why the schema is shaped this way:

- [Vector Database Payload Design](vector_payload_design.md)
- [Graph Database Schema Design](graph_schema_design.md)

## Authority Split

| Store | Role | Authoritative For | Not Authoritative For |
|---|---|---|---|
| Qdrant (service or embedded Qdrant Edge, one record contract) | Vector candidate recall and object-type prefiltering | Vector points and embedding-surface provenance | Memory content, existence, relationships, provenance, lifecycle, currentness, entity selectivity |
| Oxigraph | Graph authority | Memory objects, typed links, provenance, lifecycle, currentness, expansion context | Semantic nearest-neighbor ranking, derived selectivity counters |
| RetrievalStatsStore | Derived retrieval-policy statistics | Entity/relation counters, global counters, selectivity inputs, fanout diagnostics | Memory existence, relationships, provenance, lifecycle, currentness, semantic ranking |
| Raw store / caller storage | Source material | Raw transcript or source content behind `raw_ref` | Canonical memory state |

Rule of thumb:

```text
Qdrant suggests.
Stats guide fanout.
Oxigraph decides.
```

## Cross-Store Join Keys

| Field | Stored In | Purpose |
|---|---|---|
| `object_id` / `objectId` | Qdrant payload and graph literal | Stable object UUID |
| `graphUri` | Graph literal | Stable graph resource pointer |
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

Retrieval stats store keys refer to the same `object_id` / entity ID values, but stats are derived and rebuildable.

## Qdrant Payload Fields

| Field | Type / Shape | Notes |
|---|---|---|
| `object_id` | indexed keyword UUID string | Stable vector-to-graph join id |
| `object_type` | indexed keyword enum | Canonical memory object type |
| `surface` | keyword enum | Embedded semantic surface |
| `schema_version` | keyword string | Record compatibility marker |
| `embedding_text` | text | Exact text used to generate the vector; not read-out content |

These are the only Qdrant payload fields. Readable content, graph URI, object-specific state, relationships, lifecycle/currentness, ranking, timestamps, provenance, and raw references are hydrated from Oxigraph by `object_id`. Existing obsolete extra fields may remain on old points but readers ignore them.

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

## Future Associative Recall Concepts

| Concept | Purpose | Important distinction |
|---|---|---|
| AssociativeUnit | Represents a pair, cue bundle, cluster, or scope pattern used for associative recall. | Unit lifecycle says whether the associative structure is candidate, active, retired, or rejected. |
| AssociativeMembership | Represents a specific memory's membership in an associative unit. | Membership status says whether that memory is candidate, active, retired, or rejected; member role says whether it is core, exemplar, peripheral, bridge, or outlier. |
| AssociationSupport | Records why a unit or membership exists. | Support explains association evidence; it is not ordinary relationship truth. |
| QueryTimeActivation | Activates memories through semantic/entity/thread/scope/time/salience cues during retrieval. | Supports serendipitous recall before durable association is promoted. |

Design rule:

```text
Weak association evidence belongs in the graph,
but not as ordinary durable pairwise association.
```

Retrieval quality rule:

```text
An active cluster may contain tentative members.
Candidate-status members are considered, not trusted.
```

## Retrieval Stats Store

The retrieval stats store is an internal derived index used for selectivity scoring and fanout policy. It is not a memory store of record. It can be rebuilt from graph authority and must not override Oxigraph lifecycle/currentness/provenance decisions.

Core stats tables:

```text
entity_edge_index
entity_relation_counts
global_relation_counts
stats_meta
```

The stats store answers retrieval-policy questions such as:

```text
How selective is entity E under relation R for object type O?
What fanout budget should this relation expansion receive?
Which broad entity expansions were rejected as low-information?
```

It does not answer:

```text
Does this memory exist?
Is this memory current?
Is this relationship true?
Is this memory suppressed?
Should this memory enter final context?
```

Those remain graph-authoritative Oxigraph questions.

## Retrieval Rule Of Thumb

```text
Qdrant narrows candidates.
Stats guide bounded expansion.
Oxigraph verifies graph truth.
The final context pack follows Oxigraph state.
```

## Drift Handling

There is no reconciliation pass between the stores. Drift is surfaced at write time and neutralised at read time:

- A vector write that fails after the graph commit is reported as a typed vector-indexing failure in the public write outcome, naming the affected objects and the cause; the graph commit stands, so a graph-only record exists and semantic recall of it is degraded until the caller re-indexes it.
- A candidate whose payload carries a malformed object id or an unknown object-type or surface token fails decoding and never becomes a candidate. The schema version is enforced when a record is written, not when a point is read.
- Every surviving candidate is hydrated and verified through graph authority before it can enter a context pack, so a vector point whose object is absent from the graph, or no longer current, is omitted there.
- The retrieval stats store records its own health; after an internal failure it reports unhealthy and retrieval falls back to conservative selectivity. The unhealthy state is sticky for that store: the library has no rebuild or restore operation, so recovery is an operator action (a fresh stats store rebuilt by replaying writes).

Cross-store census operations (vector points without a graph object, graph objects without a vector point) are not part of the library; an operator performs them against the stores directly if needed.
