# Database Schema Cheat Sheet

Oxigraph holds memory truth. Qdrant recalls content candidates. Retrieval statistics guide bounded expansion. An `Entity` is a notion identity; names and other commitments belong to interpreted memories about it.

## Stores And Join Keys

| Store | Responsibility |
|---|---|
| Oxigraph | Objects, links, provenance, suppression, supersession and expansion context |
| Qdrant Edge or Qdrant service | Vector candidates and object-type/surface prefiltering |
| Retrieval stats | Derived entity/relation/object and global counters, health and fanout inputs |
| Caller storage | Raw transcripts and other source content behind pointers |

Vector payload `object_id` joins the graph's UUID `objectId`. RDF resources also carry `objectType`, `graphUri` and `schemaVersion`. Vector records carry `schema_version`; version admission is enforced when records are written.

## Vector Records

| Payload field | Shape | Purpose |
|---|---|---|
| `object_id` | UUID keyword | Graph join identity |
| `object_type` | Closed keyword enum | Object kind and prefilter scope |
| `surface` | Closed keyword enum | Embedded semantic surface |
| `schema_version` | String | Write-side record compatibility marker |
| `embedding_text` | Text | Exact embedding input for audit |

These are the five emitted payload fields. Both adapters index `object_id`, `object_type` and `surface`. Returned candidates contain object identity, surface and score; content and lifecycle come from graph hydration. The [candidate reader and payload writer](../../../src/adapters/qdrant/payload.rs) decode identity/type/surface and check the write-side schema marker.

| Object type | Surface | Text source | Maximum surfaces |
|---|---|---|---|
| `episode` | `summary`, `scene_setting`, `scene_participants` | Summary; optional setting words; optional joined participant names/descriptions. Keys and custom values excluded | 3 |
| `observation` | `text` | Observation text | 1 |
| `memory_thread` | `summary` | Thread title and summary | 1 |
| `derived_memory` | `derived_text` | Interpreted-memory text | 1 |
| `entity` | — | Graph notion identity | 0 |
| `memory_link` | — | Graph relationship | 0 |

The `query` surface represents search input. It is not emitted for a stored memory object. A topic searches content surfaces; scene words search their respective episode surfaces and are not part of the summary text. Exact names also resolve through graph assertions.

Descriptions contribute their most recent recallable occasions from the fetched matches, as many as the kind's floor and at least one; the rest of their matches contribute nothing. Trace-only `scene_cue_searches` lists a best score and the references sharing each search, once for the setting and once for all participant words; no returned match means `null`. Each search entry also carries its `cue_kind` and `omitted_count`, the number of recallable matches excluded by the occasion limit. These are retrieval diagnostics, not stored vector payload fields. See [description recall and trace](vector_payload_design.md#description-recall-and-trace) for score, admission and identity boundaries.

## Graph Resources

| Class suffix under `urn:cmem:vocab:` | Resource URI |
|---|---|
| `Episode` | `urn:cmem:episode:<uuid>` |
| `Observation` | `urn:cmem:observation:<uuid>` |
| `Entity` | `urn:cmem:entity:<uuid>` |
| `MemoryThread` | `urn:cmem:thread:<uuid>` |
| `DerivedMemory` | `urn:cmem:derived-memory:<uuid>` |
| `MemoryLink` | `urn:cmem:link:<uuid>` |

### Identity And Experience Predicates

Predicate names in these tables are suffixes under `urn:cmem:vocab:`.

| Predicate | Meaning |
|---|---|
| `objectId`, `objectType`, `graphUri`, `schemaVersion` | Common identity and schema literals |
| `createdAt`, `updatedAt` | Timestamps on the types that declare them |
| `modality`, `endedAt` | Episode modality and optional interval end |
| `sceneTime` | Required experience instant, stored in UTC without precision loss |
| `sceneOffsetSeconds` | Required supplied offset in seconds east of UTC, restricted to whole minutes within ±23:59; hydration restores it and replay equality compares it |
| `sceneLocalYear`, `sceneMonthDay` | Anniversary index derived from `scene.time`'s local date |
| `sceneParticipants` | Lossless JSON array of participants, each with optional key, name and description; a key or nonblank words required |
| `settingKey`, `settingWords` | Independently optional context key and words; the key is directly queryable |
| `sceneCustomValues` | Lossless JSON object of string custom values |
| `summary` | Episode or thread summary |
| `rawRef` | Episode or observation source pointer |
| `episode`, `speakerEntity`, `observedAt` | Observation source, optional speaker and time |
| `text` | Observation or interpreted-memory content |
| `salienceScore` | Episode, observation, thread or interpreted-memory salience |
| `retentionState` | `active` or `suppressed` for episodes, observations and interpreted memories |

### Notions And Threads

Notions carry only common identity/schema literals and `createdAt`. The following descriptive predicates belong to threads:

| Predicate | Meaning |
|---|---|
| `title`, `summary` | Thread content |
| `threadStatus` | `active`, `dormant` or `resolved` |
| `lastTouchedAt` | Thread recency |
| `canonicalKey` | Optional thread key |

### Interpreted Memories And Assertions

| Predicate | Meaning |
|---|---|
| `derivedType` | Interpreted-memory category |
| `derivedFromEpisode`, `derivedFromObservation` | Experience provenance |
| `partOfThread` | Thread membership |
| `aboutEntity` | Notion subject from `entity_ids` |
| `scopeKey` | Internal setting or named custom value from any source scene; derived inside the write turn, never authored |
| `supersedes` | Predecessor reference from the memory's list |
| `givenByApplication` | Source-free application grounding; requires a subject and excludes experience sources |
| `assertion` | Reference to an ordinal assertion resource |
| `assertionSubject` | Notion subject, present in the containing memory's subject list |
| `assertionPredicate` | `known_as` |
| `assertionName` | Supplied name spelling |
| `normalizedName` | NFKC, lowercase and whitespace-folded lookup spelling |

Assertion resources use `<memory-uri>:assertion:<zero-padded ordinal>`. Their ordering and repeated values are preserved by the [assertion reader (`shared.rs:389`)](../../../src/adapters/oxigraph/shared.rs#L389). Interpreted-memory source, thread, subject and supersedes ID lists use set semantics: conversion sorts IDs and removes duplicates, including on replacement drafts. Scene participants preserve authored order and duplicates.

Name lookup reads active beliefs with no incoming `Supersedes` link. It can resolve the same normalized name to several notion IDs; see the [name selector (`sparql_selectors.rs:104`)](../../../src/adapters/oxigraph/sparql_selectors.rs#L104).

### Links And Currency

| Predicate | Meaning |
|---|---|
| `from`, `fromType`, `to`, `toType` | Typed link endpoints |
| `relation` | Relation token |
| `rationale` | Optional explanation |
| `createdAt` | Link creation time |
| `urn:cmem:relation:<relation_name>` | Direct traversal predicate emitted with the reified link |

| Authoritative reference | Derived link | Direction |
|---|---|---|
| `entity_ids` | `About` / `about` | Interpreted memory → notion |
| `supersedes` | `Supersedes` / `supersedes` | Successor → predecessor |
| Observation `episode_id` | `ObservedIn` / `observed_in` | Observation → episode |
| Episode `scene.participants[].key` | `Involves` / `involves` | Episode → notion |

Commit persists deterministic derived links with their memory objects. `ObservedIn` and `Involves` reuse an authored link between the required endpoints in either orientation; presence creates no `Mentions` links. An observation's parent episode must exist in the same plan or store. Admission rejects authored `Supersedes`, authored `About` between interpreted memories and notions in either direction, duplicate generated/authored link IDs, and supersession predecessors that do not already exist in the graph.

Currency is determined by incoming interpreted-memory `Supersedes` links. Predecessor content and retention are preserved; a suppressed successor still supplies supersession evidence. Default retrieval excludes suppressed and superseded memories. `include_suppressed` and `include_superseded` independently opt into those histories. Notions and threads remain graph anchors.

## Retrieval Stats

The internal SQLite projection uses `entity_edge_index`, `entity_relation_counts`, `global_relation_counts`, `episode_state_index`, `episode_counts`, `episode_presence_index` and `stats_meta`. The in-memory implementation follows the same counter contract.

The edge index deduplicates counted `(entity, relation, object type, object)` tuples. `episode_presence_index` counts each notion once per episode from scene participant keys and `Involves` episode links in either orientation, following episode retention. `Mentions` observations are aboutness, not presence; forgetting an observation does not remove the episode's presence. `episode_state_index` stores every episode, including those without keyed participants, with its retention state. The presence route uses the episode total as its frequency denominator. `About` and `Mentions` share a bounded aboutness list and maintain no relation counters; counted relations such as `PartOfThread` retain their relation/object edge-count denominators.

`episode_counts` caches total and active episode counts in one row, updated in the same transaction as episode state changes. Retrieval reads that row without scanning episodes. The in-memory adapter refreshes episode counts with its existing dirty-counter cache.

| Counter | Scope |
|---|---|
| `total_count` | All indexed edges for a relation/object counter; all episodes for the episode counter |
| `active_count` | Active retention, including superseded interpreted memories |
| `current_count` | Active retention and no graph supersession evidence for interpreted-memory endpoints |

The edge cache's `is_current` value is derived from graph state. Episodes have no supersession, so their active and current counts are equal. Including suppressed but excluding superseded memories uses total relation/object counts as an approximation, with possible fanout distortion in either direction; graph filtering still enforces eligibility.

SQLite schema initialization is transactional, including the episode tables. The library provides no statistics schema migration or graph-to-statistics rebuild; stores using a superseded schema must be recreated and populated through writes.

Statistics cannot establish object existence, provenance, links or retrieval eligibility. An unhealthy stats store causes conservative selectivity fallback and requires caller-managed recovery.

## Failure Boundaries

- Graph failure rejects the authoritative write. Later vector or stats failure is reported as a typed repair outcome while the graph commit stands.
- Malformed candidate IDs or unknown object-type/surface tokens fail decoding. Graph hydration rejects absent or lifecycle-ineligible objects before context inclusion.
- A graph-only object has reduced semantic recall until indexed. Superseded interpreted memories are not re-indexed by replaying old plans.
- Cross-store reconciliation and stats rebuild are caller/operator responsibilities; the library exposes no such operation.

The [graph schema design](graph_schema_design.md) explains the authority and belief model. The [vector payload design](vector_payload_design.md) describes the recall record contract.
