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

The service indexes `object_id` and `object_type`. The remaining fields describe the record; they are not prefilter columns. Readable result content, graph URIs, assertion and grounding data, relationships, lifecycle values, timestamps and raw references are hydrated from Oxigraph.

The [payload writer](../../../src/adapters/qdrant/payload.rs) enforces the supported schema marker and emits the five fields. The [candidate reader](../../../src/adapters/qdrant/payload.rs) reads `object_id`, `object_type` and `surface`, then combines them with the vector score. It does not read `schema_version`, `embedding_text` or extra payload fields to construct a candidate. Unknown type/surface tokens and malformed IDs fail decoding.

## Indexed Objects And Surfaces

| Object type | Surface | Embedding text |
|---|---|---|
| `episode` | `summary` | `Episode summary: ` followed by the summary, then labelled scene words when supplied |
| `observation` | `text` | `Observation excerpt: ` followed by observation text |
| `memory_thread` | `summary` | `Thread summary: ` followed by title and summary |
| `derived_memory` | `derived_text` | A category label followed by interpreted-memory text |

Each indexed object has at most one surface. `max_embedding_surfaces` returns zero for `entity` and `memory_link`; neither has an object vector builder. The [embedding builders](../../../src/policy/embedding_surface.rs) define these limits and fold whitespace in the natural-language input. The `query` surface identifies query embeddings rather than stored memory content.

The [episode builder](../../../src/policy/embedding_surface.rs) appends nonblank setting words on a new `Setting:` line, followed by one `With:` line per participant joining its nonblank name and description with a comma, in authored order, even when the participant also has an identity key. It folds whitespace just as it does for the summary, while Oxigraph retains the original scene values. Keys and custom values are excluded. With no scene words the embedding text is byte-identical to the summary-only form. These words share the episode's existing `summary` surface; there is no second vector or scene surface.

The closed surface tokens are `summary`, `text`, `derived_text` and `query`. The domain enums own their persisted spelling and parsing. Graph object kinds still include notions and links even though those kinds have no emitted embedding surface.

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
