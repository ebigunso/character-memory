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

Episodes record interaction spans with `summary`, `modality`, one `Scene`, optional `endedAt` and `rawRef`, salience and retention. The scene holds the required experience time, participants with any combination of notion key, supplied name and description, a setting key and/or words, and flat string custom values. Missing scene parts stay unspecified. Observations record `text` tied to an `episode`, with an optional `speakerEntity` and raw reference. Preparation defaults observation time to scene time while preserving an explicit observation time. Involved notions, participants, speakers, thread memberships and the episode end time remain independent.

The episode owns the scene. `sceneTime` stores its UTC instant losslessly; `sceneParticipants` and `sceneCustomValues` store JSON literals, preserving participant order, duplicates, spelling and custom string values. `settingKey` is a directly queryable literal, separate from `settingWords`. Keys identify application contexts, not notions; sessions may be recorded in custom values. Embedding construction is defined in the [vector payload design](vector_payload_design.md).

`Scene.time` accepts an offset-carrying instant. Episode construction derives `scene_local_date` from the supplied offset, and RDF stores it as the `sceneLocalYear` and `sceneMonthDay` literals. It is not a draft input. A UTC calendar day would invent the experienced day when the supplied offset crosses midnight. Hydration reconstructs the date for replay equality while retaining the existing UTC `sceneTime` representation. The original offset is not reconstructed from the date. An episode without these literals has no local date and never matches an anniversary; its existing instant stays readable, with no migration.

Anniversary selection matches `sceneMonthDay` exactly and compares `sceneLocalYear` with the reference scene's local year. It reads eligible episode IDs with a shared-occasion flag, ordered shared first, then recorded instant descending and ID ascending, and fetches one beyond the contribution limit for the trace marker. Sharing uses both orientations of `Involves` episode links and `Mentions` observation-to-parent-episode links, with the existing retention checks before the cut. Only a resolved scene identity supplies a sharing key; ambiguous names retain their other recall behavior but grant no anniversary reservation. Full episode hydration is bounded to selected roots and their graph expansion. Unshared matches retain date-match provenance while remaining outside that kind's reservation queue.

Commit never substitutes a clock reading for a scene time.

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

The reference lists have set semantics: IDs are sorted and deduplicated at draft conversion for stable persistence and replay. This applies to the source, thread, subject and predecessor lists on ordinary and replacement interpreted-memory drafts. Scene participants retain authored order and repeated values; generated participant links and retrieval-stat edges count each keyed participant once.

### Derived Context Keys

An interpreted memory also stores zero or more internal `scopeKey` literals. Each literal is lossless JSON: `{"setting":"cafe"}` or `{"custom":{"name":"project","value":"42"}}`. Setting keys and custom keys occupy separate namespaces, and a custom key includes both its name and value. Strings match exactly, without normalization. Keys have set semantics and never appear as caller-discoverable scope identifiers or in serialized memory output.

Commit derives the keys inside the write turn, after validation and before replay collision checks and graph persistence. It reads the recorded scenes of all source episodes, including the parent episode of each source observation, and retains their union. Sources in the write plan are read from that plan; missing sources and observation parents are looked up by object reference. A memory with no source experience has no context keys, and setting words create none. A correction derives keys from its replacement's final source list rather than copying its predecessor's keys. Keys add recall paths rather than restrict disclosure, so a belief can be recalled in any place where its sources occurred; participants are not stored as context keys because notion and thread scopes are read from the memory's own `entity_ids` and `thread_ids`. Drafts have no context-key field, and commit overwrites the internal keys with the derived set. Existing records without the predicate have no keys; no backfill is performed.

Scene-key retrieval uses the indexed predicate and scalar lifecycle/salience/creation metadata to select memory roots before hydrating payloads. It shares the subject-state selector's currentness checks and priority order and the existing root and section limits. Participant scopes precede setting, custom names in map key order, and activity. One memory shared by scopes uses one slot and credits each scope.

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

Source lookup, thread lookup, name lookup, scope-key lookup and bounded expansion query the named graphs they need. Object hydration currently reads every stored quad into a subject map before picking the requested objects, so its cost grows with the store and not with the request; a targeted read is the known improvement. Expansion is bounded by depth, object and relation scope, lifecycle policy and fanout caps.

The retrieval stats store maintains derived entity/relation/object and global counters. Its `total_count` includes all indexed edges, `active_count` restricts retention to active, and `current_count` additionally excludes superseded interpreted-memory endpoints. Its cached `is_current` value is a projection input rather than a persisted memory field or a source of graph authority.

The policy combination that includes suppressed memories while excluding superseded ones uses total counters as an approximation; the extra edges can skew fanout estimates in either direction. Graph eligibility filtering still applies independently. Missing or unhealthy statistics use conservative selectivity fallback.

### State And Last Interaction

Participants resolved from keys or known names select beliefs through their own subject lists, independently of source-scene participants. The [state selector (`sparql_selectors.rs:247`)](../../../src/adapters/oxigraph/sparql_selectors.rs#L247) orders eligible beliefs by descending salience, descending creation time, then ID, with current beliefs before superseded history when `include_superseded` is enabled. Existing graph limits still apply. Section selection takes state-bearing memories in scene-scope rounds while preserving each scope's final relevance order; a shared memory uses one slot and credits each scope. The [state ordering helper (`state.rs:74`)](../../../src/usecases/retrieve/state.rs#L74) changes only state-bearing positions. Topic-only retrieval and content-derived entity expansion retain their selection rules.

Incoming `Resolves` and `FulfillsCommitment` links exclude targets from named-subject state, activity thread membership and setting/custom-key state. Trace omissions use `ResolvedOmitted`; ordinary recall remains eligible. Resolver retention does not change this evidence, and resolution changes neither target content nor retention. Admitted interpreted memories carry sorted, deduplicated resolver IDs in `resolved_by`, including with tracing disabled ([evidence merge, `retrieve.rs:553`](../../../src/usecases/retrieve.rs#L553)). The [section classifier (`retrieve.rs:1431`)](../../../src/usecases/retrieve.rs#L1431) assigns resolved open loops and fulfilled commitments to `derived_memories`, so they use that section's cap. Stored subtypes and scores stay intact.

Each resolved scene reference reports a `last_interactions` map keyed by notion ID, including every candidate of an ambiguous name. Values contain the last episode ID, recorded scene time and whole seconds elapsed relative to the retrieval scene; `null` means no eligible encounter at or before that time. Descriptions and unknown references have an empty map. [Scene resolution (`scene.rs:98`)](../../../src/usecases/retrieve/scene.rs#L98) collects these facts independently of pack caps and tracing. The [last-interaction selector (`sparql_selectors.rs:414`)](../../../src/adapters/oxigraph/sparql_selectors.rs#L414) uses one SPARQL query over `Involves` episode links and `Mentions` observation-to-parent-episode links, in either orientation. Both neighbor and parent retention checks, controlled by `include_suppressed`, and the scene-time cutoff precede `ORDER BY` descending timestamp, ascending episode ID and `LIMIT 1`. Scene-time strings are cast to timestamps for comparison, preserving fractional-time ordering. One result row crosses into Rust; the query engine may still examine all matching candidates.

Pack expansion reads bounded occasion prefixes and exclusion evidence separately. The [participant limiter](../../../src/policy/graph_expansion.rs) shares one episode budget across direct episode and observation routes, with at most one observation per selected episode. Compact occasion metadata is read first; Rust applies eligibility and distinct-episode limits before full RDF hydration. Frequency counts distinct episodes, and selection prefers recorded scene time, latest first. Only participants recognized by key or name apply the retrieval scene-time cutoff here; beliefs and other retrieval paths have no new as-of rule.

State reads scan keyed IDs and rank/lifecycle columns, then apply eligibility, deterministic ordering and the caller's budget in Rust before full RDF hydration; subject state additionally requires a traversable About link. Exclusion evidence has a separate latest-first prefix of the same budget. Fanout omission telemetry counts omissions within fetched prefixes, without a whole-store count query. Thread expansion excludes resolved members independently of that bounded evidence.

The hub limit counts the fetched, eligibility-pruned traversal prefix, excluding trace-only omission evidence, so a larger stored history alone does not cause a hub failure.

## Cross-Store Failures

Graph writes are critical. Vector and stats updates are repairable parts of the write outcome, with typed failures and affected IDs. A graph-only memory loses semantic recall until vector indexing is repaired; stale candidates are checked against graph existence and lifecycle before inclusion. Replaying an old plan does not re-index an interpreted memory that graph authority identifies as superseded.

A failed graph read feeding the stats projection is reported as `StatsUpdateCause::GraphRead` and retained in unhealthy stats as `RetrievalStatsHealthCause::GraphRead`. Both carry the underlying `GraphQueryError`, whether the read was endpoint hydration or currency lookup.

The library has no cross-store reconciliation or stats-rebuild operation. Stats health remains unhealthy after an internal failure; recovery requires a fresh store and caller-managed replay. Raw-source resolution and cross-store census operations belong to callers or operators.

See the [schema cheat sheet](schema_cheat_sheet.md) for predicate tables.
