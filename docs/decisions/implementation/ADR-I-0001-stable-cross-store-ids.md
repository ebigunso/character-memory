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

# ADR-I-0001: Use stable cross-store IDs and deterministic graph IRIs

## Context and Problem Statement

Character Memory uses both vector and graph storage. If Qdrant records and graph resources cannot be joined reliably, provenance, correction, retrieval expansion, and debugging become fragile.

## Decision Drivers

- Support vector-to-graph and graph-to-vector joins.
- Preserve stable provenance paths across migrations and updates.
- Make idempotent upserts possible.
- Avoid generating IDs from mutable summaries or extracted text.

## Decision

Every persisted domain object has a stable ID. Graph IRIs are deterministically derived from those IDs.

Example shape:

```text
Episode ID: ep_...
Graph IRI: urn:cmem:episode:<ep_id> or equivalent deterministic IRI
Qdrant payload graph_uri: same graph IRI
Qdrant payload object_id: same logical object ID
```

The exact IRI scheme may be adjusted, but it must be deterministic and stable.

## Implementation Impact

- Qdrant payloads must include the logical object ID and graph URI.
- Oxigraph resources must be generated from stable IDs, not from mutable text.
- Upsert should be idempotent for the same logical object ID.

## Considered Options

1. Let each backend generate independent IDs.
2. Generate graph IRIs from normalized text content.
3. Use stable domain IDs and deterministic graph IRIs across stores.

## Decision Outcome

Chosen option: **3. Use stable domain IDs and deterministic graph IRIs across stores**.

This is necessary for reliable hybrid retrieval and provenance.

## Consequences

### Positive

- Makes cross-store joins predictable.
- Supports correction and supersession links.
- Simplifies tests and debugging.

### Negative / Tradeoffs

- Requires explicit ID generation and validation logic.
- ID scheme changes require migrations.

## Validation

- Round-trip tests: domain ID → graph IRI → Qdrant payload → domain ID.
- Idempotent upsert tests.
- Tests should fail if graph resources are generated from mutable text summaries.

## Revisit When

Revisit only if the storage architecture changes to a single unified backend or if ID scheme requirements change substantially.
