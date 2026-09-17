---
status: proposed
adr_type: implementation
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-I-0002-natural-language-embedding-surfaces--superseded-by-ADR-I-0032.md]
superseded_by: null
depends_on: [ADR-I-0025-vector-record-is-a-read-contract.md]
---

# ADR-I-0032: The text a vector embeds is a natural-language semantic surface, never a metadata template

## Context and Problem Statement

Vector search should retrieve records by meaning. Embedding serialized records or metadata templates puts field names, IDs, and repeated schema text into the vector space, where they distort similarity and pull unrelated records together on boilerplate. This record restates the decision of ADR-I-0002 as it stands after ADR-I-0025 settled what the vector record stores.

## Decision

The text a vector embeds is a concise natural-language surface expressing what the memory means. IDs, schema versions, flags, retention states, scores, and backend fields are never part of it. Entity names, source names, places, and thread titles appear only when they are natural recall cues. Structured fields live in graph authority. The vector record stores the embedded surface, and only that text, as provenance of what was ranked; readable content is hydrated from graph authority by object ID.

## Why

Queries arrive as natural language, and a vector space that mixes meaning with implementation artifacts recalls memories for sharing a schema rather than for being related. Keeping the surface natural and the metadata elsewhere gives the vector layer the most query-aligned representation while precise filtering stays where it is authoritative.

## Rejected Alternatives

- Embedding raw JSON or serialized records: rejected outright; boilerplate dominates the vector.
- Embedding structured metadata templates: rejected outright, for the same reason.
- Persisting a readable content column beside the surface: rejected by ADR-I-0025 because it is a deterministic function of graph fields; reopen only with a re-indexing workflow that cannot rebuild from graph authority.

## Decision Boundary

Invariant: the embedded text is natural language free of identifiers, versions, flags, and scores, and the vector record stores that surface and no other text.

Not covered: the surface-building rules per object type, which are embedding-surface policy and change by measurement.

## Validation

- Snapshot tests on generated surfaces per object type.
- Tests assert that IDs, schema versions, retention states, and numeric scores never appear in a surface.
- Retrieval fixtures compare natural-language queries against episode, thread, and derived-memory surfaces.

## Revisit When

Measured retrieval evaluation shows that selected metadata phrases improve recall without harming precision.

## More Information

- ADR-I-0025 defines the vector record and its single text column.
- ADR-I-0002 in `superseded/` carries the original reasoning of 2026-04-26.
