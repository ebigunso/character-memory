---
status: proposed
adr_type: implementation
date: 2026-04-26
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0002: Embed natural-language semantic surfaces, not structured metadata templates

## Context and Problem Statement

Vector search should retrieve records by semantic meaning. If the system embeds serialized metadata templates, boilerplate field names, IDs, and repeated schema text may distort the vector space and reduce retrieval quality.

## Decision Drivers

- Keep embeddings aligned with natural-language user queries.
- Avoid injecting repeated metadata boilerplate into vectors.
- Preserve structured metadata for filtering and graph joins.
- Support embedding text that reflects what the memory means, not how it is stored.

## Decision

Embedding input should be a concise natural-language semantic surface.

Good:

```text
The user prefers natural-language embedding surfaces over structured metadata templates.
```

Avoid:

```text
record_type: derived_memory
confidence: 0.86
thread_id: thread_character_memory_design
retention_state: active
```

Structured fields belong in Qdrant payloads and graph triples, not in the embedding string.

## Character Memory Relevance

This protects continuity retrieval from being dominated by implementation artifacts. Memories should be recalled because they are meaningfully related to the current context, not because they share schema boilerplate.

## Implementation Impact

- Persist both `embedding_text` and `content_text` where useful.
- Exclude IDs, schema versions, booleans, retention states, scores, and backend fields from `embedding_text`.
- Include entity names, source names, places, and thread titles only when they are natural recall cues.

## Considered Options

1. Embed raw JSON or serialized records.
2. Embed structured metadata templates.
3. Embed natural-language semantic surfaces and keep metadata separate.

## Decision Outcome

Chosen option: **3. Embed natural-language semantic surfaces and keep metadata separate**.

This gives the vector layer the most query-aligned representation while preserving precise filters elsewhere.

## Consequences

### Positive

- Better alignment with user queries and ordinary text embeddings.
- Cleaner separation between semantic search and exact filtering.
- Easier to test generated embedding text.

### Negative / Tradeoffs

- Requires surface-generation logic per record type.
- Some metadata-heavy queries may require payload filters or graph lookup instead of dense retrieval alone.

## Validation

- Snapshot tests for generated `embedding_text`.
- Tests should assert that IDs, schema versions, retention states, and numeric scores are not included in embedding text by default.
- Retrieval fixtures should compare natural-language queries against episode, thread, and derived memory records.

## Revisit When

Revisit if empirical retrieval evaluation shows that selected metadata phrases improve recall without harming precision.
