# Graph Database Schema Design

Oxigraph is the authority for memory identity, content, relationships, provenance, suppression and supersession. Vector storage recalls content candidates; graph hydration and bounded expansion decide which memories enter retrieved context.

An `Entity` represents a notion: an identity that memories can be about. A `DerivedMemory` is an interpreted memory or belief whose text, subjects and optional assertions carry the character's commitments. These are separate graph objects so names and other beliefs can change while the notion keeps its identity.

## Identity And Durable Storage

Each canonical object or link has a UUID and a deterministic resource URI:

| Domain object | Resource URI | RDF class |
|---|---|---|
| `Episode` | `urn:cmem:episode:<uuid>` | `urn:cmem:vocab:Episode` |
| `Observation` | `urn:cmem:observation:<uuid>` | `urn:cmem:vocab:Observation` |
| `Entity` | `urn:cmem:entity:<uuid>` | `urn:cmem:vocab:Entity` |
| `MemoryThread` | `urn:cmem:thread:<uuid>` | `urn:cmem:vocab:MemoryThread` |
| `DerivedMemory` | `urn:cmem:derived-memory:<uuid>` | `urn:cmem:vocab:DerivedMemory` |
| `MemoryLink` | `urn:cmem:link:<uuid>` | `urn:cmem:vocab:MemoryLink` |

RDF named graphs store the authoritative representation. Common identity literals are `objectId`, `objectType`, `graphUri` and `schemaVersion`; timestamps belong to the object types that declare them. All predicate names below use the `urn:cmem:vocab:` namespace unless a relation URI is shown explicitly. The [RDF vocabulary](../../../src/adapters/oxigraph/vocabulary.rs) and [mapping](../../../src/adapters/oxigraph/rdf_mapping.rs) define the persisted spellings.

Hydration reconstructs domain objects and links from the selected RDF named graphs. It does not require a process-local object cache or vector payload content. Raw transcripts remain in caller-owned storage; `rawRef` is a pointer to source material.

## Experiences, Notions And Threads

Episodes record interaction spans with `summary`, `modality`, optional source and time metadata, `participantEntity` references, salience and retention. Observations record `text` tied to an `episode`, with observation time, an optional `speakerEntity` and an optional raw reference. Dedicated source references keep experience-based provenance queryable.

A notion stores only its identity, `createdAt` and schema metadata. Its names and descriptions are carried by interpreted memories about it. Multiple notions may share a name, and one notion may have multiple naming beliefs.

Threads are continuity overlays with `title`, `summary`, `threadStatus`, `lastTouchedAt`, `salienceScore`, optional `canonicalKey`, and creation/update timestamps. The status vocabulary is `active`, `dormant` and `resolved`. Memory membership is represented by `partOfThread` references or typed links. Suppressing thread members preserves the thread object and its retrieval surface.

## Beliefs And Grounding

A `DerivedMemory` stores `derivedType`, `text`, `salienceScore`, `retentionState`, creation/update timestamps, and these reference lists:

| Domain field | RDF predicate | Meaning |
|---|---|---|
| `derived_from_episode_ids` | `derivedFromEpisode` | Source episodes |
| `derived_from_observation_ids` | `derivedFromObservation` | Source observations |
| `thread_ids` | `partOfThread` | Continuity threads |
| `entity_ids` | `aboutEntity` | Notion subjects |
| `supersedes` | `supersedes` | Older interpreted memories replaced by this memory |

An interpreted memory either cites at least one episode or observation, or declares `given_by_application=true`. The latter is persisted as `givenByApplication`, requires at least one notion subject, and excludes experience source references. Application-given beliefs carry application-supplied grounding without inventing an experience. Corrections of such beliefs require an explicit replacement with its grounding declared.

The reference lists have set semantics: IDs are sorted and deduplicated at draft conversion for stable persistence and replay. This applies to episode participant IDs and to the source, thread, subject and predecessor lists on ordinary and replacement interpreted-memory drafts.

### Assertions And Name Lookup

Assertions record commitments the character holds about a memory's notion subjects. Reported claims and doubts can remain text without assertions. Every assertion subject must appear in the containing memory's `entity_ids`.

The supported assertion predicate is `KnownAs { name }`, persisted as `known_as`. An assertion is represented by a resource beneath its containing memory:

```text
<memory> assertion <memory>:assertion:<zero-padded ordinal>
<assertion> assertionSubject <notion>
<assertion> assertionPredicate "known_as"
<assertion> assertionName "Alice"
<assertion> normalizedName "alice"
```

`assertionName` preserves the supplied spelling. `normalizedName` is derived by Unicode NFKC normalization, lowercase conversion and whitespace folding. Lowercasing keeps `ß` and `ss` distinct, as implemented by [name normalization](../../../src/domain/belief.rs#L68).

Assertions are ordered payloads: ordinal assertion resources preserve their order and repeated values. The [assertion reader (`shared.rs:389`)](../../../src/adapters/oxigraph/shared.rs#L389) sorts those resources before reconstructing the list. They do not use the ID-list set semantics.

Name lookup selects notions named by active, non-superseded beliefs. An incoming `Supersedes` link excludes a naming belief even when its successor is suppressed; lookup can return multiple notion IDs for the same normalized name. The [name selector (`sparql_selectors.rs:104`)](../../../src/adapters/oxigraph/sparql_selectors.rs#L104) reads the assertion predicates and checks incoming supersession evidence.

## Relationships And Derived Links

Object-reference predicates preserve authored domain fields. Typed `MemoryLink` records provide inspectable relation identity and direct traversal triples. Each link stores `from`, `fromType`, `to`, `toType`, `relation`, optional `rationale`, `createdAt`, and common identity/schema metadata. Its direct traversal predicate is `urn:cmem:relation:<relation_name>`.

Remember and correction derive two link families from interpreted-memory lists:

| Authoritative list | Derived link | Direct traversal triple |
|---|---|---|
| `entity_ids` | `About`, memory to notion | `<memory> urn:cmem:relation:about <notion>` |
| `supersedes` | `Supersedes`, successor to predecessor | `<successor> urn:cmem:relation:supersedes <predecessor>` |

Generated link IDs are deterministic for their endpoints and relation family. Objects and derived links are written in one graph batch. The [derived-link builder](../../../src/usecases/write_planning.rs#L467) preserves this relationship between object lists and graph traversal.

Authored `Supersedes` links are rejected. Authored `About` links between interpreted memories and notions are rejected in either orientation; the memory's subject list owns those links. Other admitted link kinds remain caller-authored. Commit rejects duplicate link IDs, including collisions with generated links. Supersession predecessors must already exist in the graph; creating a predecessor in the same plan does not satisfy the reference.

## Suppression And Supersession

Episode, observation and interpreted-memory retention uses `active` and `suppressed`. Default retrieval omits suppressed memories. The retrieval policy can include them explicitly.

Interpreted-memory currency is derived from incoming `Supersedes` links between interpreted memories. A memory's own `supersedes` list names its predecessors; it does not determine whether that memory has a successor. A replacement leaves predecessor content and retention unchanged. Suppressing the replacement does not erase its supersession evidence. Default retrieval omits superseded interpreted memories, with an independent policy option for historical inclusion.

These graph checks remain decisive when vector deletion or indexing fails. A stale vector cannot make a suppressed or superseded memory eligible for default context. Notions remain graph traversal anchors, while semantic recall reaches them through interpreted-memory content and derived `About` links.

## Retrieval And Derived Statistics

Source lookup, thread lookup, name lookup and bounded expansion query the named graphs they need. Object hydration currently reads every stored quad into a subject map before picking the requested objects, so its cost grows with the store and not with the request; a targeted read is the known improvement. Expansion is bounded by depth, object and relation scope, lifecycle policy and fanout caps.

The retrieval stats store maintains derived entity/relation/object and global counters. Its `total_count` includes all indexed edges, `active_count` restricts retention to active, and `current_count` additionally excludes superseded interpreted-memory endpoints. Its cached `is_current` value is a projection input rather than a persisted memory field or a source of graph authority.

The policy combination that includes suppressed memories while excluding superseded ones uses total counters as an approximation; the extra edges can skew fanout estimates in either direction. Graph eligibility filtering still applies independently. Missing or unhealthy statistics use conservative selectivity fallback.

## Cross-Store Failures

Graph writes are critical. Vector and stats updates are repairable parts of the write outcome, with typed failures and affected IDs. A graph-only memory loses semantic recall until vector indexing is repaired; stale candidates are checked against graph existence and lifecycle before inclusion. Replaying an old plan does not re-index an interpreted memory that graph authority identifies as superseded.

A failed graph read feeding the stats projection is reported as `StatsUpdateCause::GraphRead` and retained in unhealthy stats as `RetrievalStatsHealthCause::GraphRead`. Both carry the underlying `GraphQueryError`, whether the read was endpoint hydration or currency lookup.

The library has no cross-store reconciliation or stats-rebuild operation. Stats health remains unhealthy after an internal failure; recovery requires a fresh store and caller-managed replay. Raw-source resolution and cross-store census operations belong to callers or operators.

See the [schema cheat sheet](schema_cheat_sheet.md) for predicate tables and the [vector payload contract](vector_payload_design.md) for recall record fields.
