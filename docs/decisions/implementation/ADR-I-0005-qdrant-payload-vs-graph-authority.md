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

# ADR-I-0005: Keep Qdrant metadata filterable while graph relationships remain authoritative

## Context and Problem Statement

Qdrant payloads are useful for fast filtering, but they should not become a second, divergent graph model. The graph store is the authoritative source for relationships and provenance. Qdrant payloads should contain enough duplicated metadata for efficient recall and filtering, while graph expansion resolves authoritative relationship context.

## Decision Drivers

- Keep vector retrieval fast.
- Avoid duplicate relationship logic drifting between Qdrant and Oxigraph.
- Support filtering by type, time, retention state, salience, thread, entity, and current/superseded status.
- Preserve graph authority for provenance, links, and relationship traversal.

## Decision

Qdrant payloads store searchable/filterable metadata and graph pointers, including fields such as:

```text
object_id
graph_uri
record_type
derived_type
embedding_text
content_text
episode_ids
observation_ids
thread_ids
entity_ids
created_at
observed_at
salience_score
confidence
is_current
retention_state
modality
schema_version
```

The graph remains authoritative for relationship structure, provenance paths, supersession, contradiction, and thread/entity linkage.

## Implementation Impact

- Payload fields used for filters must be kept in sync with graph state where duplicated.
- Retrieval should treat payload metadata as a candidate filter, then expand/verify through graph lookup.
- Relationship updates may require Qdrant payload updates for indexed records.

## Considered Options

1. Store all relationships only in Qdrant payloads.
2. Store no relationship metadata in Qdrant payloads.
3. Store filterable relationship hints in Qdrant while keeping graph relationships authoritative.

## Decision Outcome

Chosen option: **3. Store filterable relationship hints in Qdrant while keeping graph relationships authoritative**.

This balances performance with correctness.

## Consequences

### Positive

- Enables efficient candidate retrieval and filtering.
- Keeps graph traversal as the source of truth.
- Supports hybrid retrieval without overloading either backend.

### Negative / Tradeoffs

- Requires sync discipline between graph and Qdrant payloads.
- Some relationship updates need multi-store writes.
- Payload fields may be stale if write failures are not handled carefully.

## Validation

- Tests should verify that Qdrant candidates can be expanded to graph context using `graph_uri` or object ID.
- Tests should cover retention filtering before graph expansion.
- Consistency tests should detect payload graph pointers that do not resolve.

## Revisit When

Revisit if graph lookups become fast enough to avoid duplicated relationship hints, or if payload sync becomes too error-prone.
