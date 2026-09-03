# Vector Database Payload Design

> Current contract: [ADR-I-0025](../../decisions/implementation/ADR-I-0025-vector-record-is-a-read-contract.md) supersedes the former denormalized payload-hint inventory with the five-field read contract documented here. [ADR-I-0024](../../decisions/implementation/ADR-I-0024-vector-candidate-recall-reports-completeness-and-prefilters-never-match-unknown.md) governs any future prefilter re-entry.

This document describes the Qdrant record contract for Character Memory. Qdrant is the semantic candidate index, while Oxigraph is the authority for memory content, relationships, provenance, lifecycle state, and currentness.

A Qdrant hit means that an object may be relevant. Retrieval must hydrate and verify that object through graph authority before it can enter a continuity context pack.

## Record Contract

Each Qdrant point carries exactly five payload fields:

| Field | Shape | Purpose |
|---|---|---|
| object_id | UUID keyword | Stable vector-to-graph join identity |
| object_type | closed keyword enum | Canonical memory object kind |
| surface | closed keyword enum | Semantic surface represented by the vector |
| schema_version | keyword string | Record compatibility marker |
| embedding_text | text | Exact natural-language input used to create the vector |

The service indexes only object_id and object_type. The other three fields are stored provenance, not prefilter columns.

The record intentionally excludes readable result text, graph URIs, relationships, provenance links, lifecycle/currentness values, ranking values, timestamps, and raw references. Retrieval obtains those values from the graph-authoritative object identified by object_id.

## Indexed Object Types And Surfaces

The vector-indexed object kinds are:

~~~text
episode
observation
entity
memory_thread
derived_memory
~~~

memory_link remains graph-authoritative relationship data and has no embedding surface.

The public maximum-surfaces policy is colocated with the builders. This release emits at most one surface for each vector-indexed object and zero for memory_link. Publishing that limit lets callers bound recall expansion without guessing from implementation details.

## Natural-Language Embedding Text

Embedding text should describe the memory in language a model or user might use later. It must not serialize record metadata.

Good:

~~~text
The user prefers deterministic public facade tests.
~~~

Bad:

~~~text
object_type=derived_memory; retention_state=active; confidence=0.82
~~~

The first supports semantic recall. The second trains similarity on storage vocabulary rather than memory meaning.

embedding_text is retained so an operator can audit what produced a vector. It is not read-out content. Prompt-ready content is hydrated from graph authority.

## Typed Tokens

object_type and surface are closed vocabularies. Their persisted spellings are owned by the domain enums through one Display and one FromStr implementation per enum. Adapters must not maintain independent token tables.

Unknown tokens fail candidate decoding. This prevents a new producer variant from being silently accepted or mapped to the wrong meaning.

## Consistency And Migration

Graph writes may succeed while vector maintenance fails. Public outcomes therefore report typed vector-indexing failures, and retrieval always verifies vector candidates against current graph state.

The five-field change does not bump schema_version. Existing points may still contain obsolete extra fields; readers ignore those fields, and new writes emit only the five-field contract. No in-place payload migration is required. A rebuild from graph authority removes old extras naturally.

A future change that alters the meaning or required interpretation of the five fields must use the repository's schema-version policy. Adding graph-derived prefilter columns also requires an explicit consistency design: every write path must synchronize them, or the values must be immutable, and unknown values must never match.

## Indexing Admission

The write-side indexing service rejects a zero-norm record embedding before calling the Qdrant adapter. The failure identifies the affected memory object through the public typed indexing-cause contract. This mirrors the query-side rule that cosine search must not receive a zero-norm query.

## Operational Checks

Useful checks are:

- graph objects with no vector point
- vector points whose graph object no longer exists
- unsupported record schema versions
- malformed or unknown object_type and surface tokens
- zero-norm embeddings rejected before adapter dispatch

Obsolete extra payload fields are not read and are not treated as authority.
