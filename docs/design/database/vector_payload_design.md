# Vector Database Payload Design

Qdrant stores semantic candidates for memory content. Oxigraph supplies authoritative content, relationships, provenance, suppression and supersession before a candidate can enter a continuity context pack. The embedded Qdrant Edge adapter and Qdrant service adapter share the same record contract.

Notions (`Entity`) are graph identities. Interpreted memories carry their names, descriptions and assertions, so semantic recall indexes belief text and reaches notions through graph relationships.

## Record Fields

Each emitted point has exactly five payload fields:

| Field | Shape | Purpose |
|---|---|---|
| `object_id` | UUID keyword | Stable vector-to-graph join identity |
| `object_type` | Closed keyword enum | Canonical memory object kind |
| `surface` | Closed keyword enum | Semantic surface represented by the vector |
| `schema_version` | String | Write-side record compatibility marker |
| `embedding_text` | Text | Exact input used to create the vector |

Both adapters index `object_id`, `object_type` and `surface`. The remaining fields describe the record; they are not prefilter columns. Readable result content, graph URIs, assertion and grounding data, relationships, lifecycle values, timestamps and raw references are hydrated from Oxigraph.

The [payload writer](../../../src/adapters/qdrant/payload.rs) enforces the supported schema marker and emits the five fields. The [candidate reader](../../../src/adapters/qdrant/payload.rs) reads `object_id`, `object_type` and `surface`, then combines them with the vector score. It does not read `schema_version`, `embedding_text` or extra payload fields to construct a candidate. Unknown type/surface tokens and malformed IDs fail decoding.

## Indexed Objects And Surfaces

| Object type | Surface | Embedding text |
|---|---|---|
| `episode` | `summary` | `Episode summary: ` followed by the summary |
| `episode` | `scene_setting` | Nonblank setting words, without a label |
| `episode` | `scene_participants` | Nonblank participant names and descriptions, one participant per line |
| `observation` | `text` | `Observation excerpt: ` followed by observation text |
| `memory_thread` | `summary` | `Thread summary: ` followed by title and summary |
| `derived_memory` | `derived_text` | A category label followed by interpreted-memory text |

An episode has up to three surfaces; each other indexed object has one. `max_embedding_surfaces` returns zero for `entity` and `memory_link`; neither has an object vector builder. The [embedding builders](../../../src/policy/embedding_surface.rs) define these limits and fold whitespace in the natural-language input. The `query` surface identifies query embeddings rather than stored memory content.

The [plural builder](../../../src/policy/embedding_surface.rs) emits the unchanged summary record and a record for each nonempty scene surface. Participants retain authored order, joining each nonblank name and description with a comma even when a key is present. Fields have whitespace folded; participant lines are joined with newlines, without labels. Oxigraph retains the original scene values. Keys and custom values are excluded. Summary bytes are identical with and without scene words. All records are embedded in one batch before the write turn; outcome lists still contain one entry per object. Deletion by object id removes all its surfaces.

The closed surface tokens are `summary`, `text`, `derived_text`, `scene_setting`, `scene_participants` and `query`. The domain enums own their persisted spelling and parsing. Graph object kinds still include notions and links even though those kinds have no emitted embedding surface.

Topic searches use only the three content surfaces. Setting words search `scene_setting`; participant words are joined with the same composition as the write surface and searched once on `scene_participants`. Query embeddings may be reused for equal text, but results are cached by text and surface scope. Nearest search, zero-norm scroll and completeness counts share that scope in both adapters. An index built without scene surfaces returns no description matches until rebuilt; no migration is provided.

## Description Recall And Trace

A key or a known name brings current beliefs about the person or context it identifies. A description brings recent occasions with similar recorded words, without establishing an identity. Each description search contributes only its most recent recallable episodes at or before the retrieval scene time, as many as its cue kind's floor and never fewer than one. Selection uses the fetched search pool, including the fetched tie at the cutoff; an open search boundary limits this claim to the fetched matches. Other matches do not enter the candidate set through that description. Each trace entry in `scene_cue_searches` records in `omitted_count` the number of recallable matches excluded by this occasion limit, once for the setting and once for the joined participants.

The contributed occasions compete by their similarity scores within the existing relevance ranking. Keys, names and current state take their kind's floor before description matches. Spare room follows score order when candidates and context-pack sections are selected; only graph-root selection shares spare room by turns among kinds. An occasion recalled through words can bring its observations and conclusions, but does not by itself open the other occasions of a person or thread reached from it. When the topic or a known reference also reaches it, linked history inherits that source's score; a strong description match cannot lend its score to a weak topic match.

With tracing enabled, [scene search trace emission](../../../src/usecases/retrieve/scene.rs) emits one `scene_cue_searches` entry for each scene search. Each [entry](../../../src/api/types/retrieval.rs) lists its `cue_kind`, the `SceneReference` values sharing that search, its `omitted_count`, and its `best_score`: the highest fetched scene-surface score before graph eligibility and occasion selection, or `null` if no scene-surface match was returned. The setting entry refers to `SettingWords`; the participants entry lists all nonblank names and descriptions that contributed to the single joined search, including names whose identity lookup was unknown. Its score is shared, never a separate score for each person. Keys do not contribute words. An empty scene omits the scene-search field; readers treat an absent field as an empty list. Tracing disabled produces no trace.

This score describes the search, not whether the best match was admitted or whether the description identifies someone. No minimum similarity is applied. These scores support measuring whether a useful boundary for descriptions that remind of nothing exists. That measurement belongs in the public companion evaluation repository, `CharacterMemoryEvals`, whose evaluation tooling is a development aid outside the core library. A decision to apply such a boundary requires that evidence.

## Belief Content And Graph Authority

A derived-memory vector represents its `text`. Structured assertions, including `KnownAs`, belong to the graph and are not independently embedded or appended to that text. Their normalized names support graph lookup. The interpreted memory's subject list generates `About` links that allow bounded expansion from recalled content to notion identities.

Embedding text uses natural language, for example `User preference: Prefer deterministic public facade tests.` Metadata such as `object_type=derived_memory; retention_state=active` belongs outside the semantic input. Retained `embedding_text` supports auditing of vector generation; context content comes from graph hydration.

## Indexing And Currency

Embedding happens before a write takes its turn, so no turn holds a model call; inside the turn, indexing consults incoming graph supersession evidence before writing any interpreted memory's vector. Already-superseded memories are excluded, including when an older plan is replayed. A graph-query failure is reported through the typed vector-indexing repair outcome; it does not permit unverified re-indexing. The [indexing service](../../../src/usecases/vector_indexing.rs) owns this admission check.

Record embeddings with zero norm are rejected before the adapter call, and dimensions must match the configured store. Query embeddings must satisfy the same cosine-search constraints. A successor derives `Supersedes` links from its predecessor list and schedules predecessor-vector deletion. If deletion fails, graph supersession still excludes those predecessors from default retrieval and the write outcome reports the maintenance failure.

Retention has two values, `active` and `suppressed`. Retrieval independently controls inclusion of suppressed and superseded memories through `include_suppressed` and `include_superseded`; vector payloads carry neither decision. Historical graph expansion may reach a superseded memory even when its vector has been removed.

## Failure And Recovery Boundaries

Graph commits remain authoritative when subsequent vector or stats maintenance fails. Typed outcomes identify affected objects and repair causes. A graph-only object has reduced semantic recall until indexing is repaired; a stale or orphaned point must pass graph existence and lifecycle checks before context inclusion.

The library does not run cross-store reconciliation or provide a stats-rebuild operation. Callers manage repair and recovery. A graph-derived prefilter may be introduced only under the completeness and synchronization requirements in [ADR-I-0028](../../decisions/implementation/ADR-I-0028-vector-prefilters-require-fully-populated-current-columns-and-never-match-unknown.md).

See [ADR-I-0025](../../decisions/implementation/ADR-I-0025-vector-record-is-a-read-contract.md) for the vector-record decision, the [graph schema design](graph_schema_design.md) for belief and lifecycle authority, and the [schema cheat sheet](schema_cheat_sheet.md) for a compact field inventory.
