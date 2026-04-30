---
status: accepted
adr_type: implementation
date: 2026-04-26
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0003: Use Qdrant and Oxigraph as default storage backends

## Context and Problem Statement

The library needs both semantic recall and explicit relationship traversal. Qdrant and Oxigraph provide practical default backends for those roles. The old roadmap already assumed this split, with Qdrant for vectors and payload filtering and Oxigraph for RDF/SPARQL relationships.

## Decision Drivers

- Provide concrete defaults so v0.1 can be implemented and tested.
- Support vector recall and graph relationships as separate capabilities.
- Avoid turning the domain model into a backend-specific schema.
- Preserve the ability to test storage behavior with fixtures.

## Decision

Use:

```text
Qdrant   → vector candidate recall and payload filtering
Oxigraph → embedded graph-authoritative memory objects, relationships, provenance, lifecycle state, and expansion
```

as the default v0.1 storage backends. The current v0.1 public constructor uses embedded in-memory Oxigraph; persistent Oxigraph storage configuration is future work.

The domain objects should not expose backend client types as their public shape. Backend-specific mappings live in storage modules.

## Implementation Impact

- Implement Qdrant candidate/payload mapping for `Episode`, `Observation`, `DerivedMemory`, `MemoryThread`, and `Entity` records that are indexed.
- Implement Oxigraph/RDF mapping for domain objects and `MemoryLink` relations.
- Public construction and facades should compose the embedder, Qdrant candidate store, and embedded Oxigraph graph authority.
- Provide tests for store contracts, graph expansion, lifecycle filtering, and cross-store joins.

## Considered Options

1. Use only a vector database.
2. Use only a graph database.
3. Use Qdrant and Oxigraph as default complementary backends.
4. Avoid choosing default backends in v0.1.

## Decision Outcome

Chosen option: **3. Use Qdrant and Oxigraph as default complementary backends**.

This best supports the hybrid retrieval approach while staying concrete enough for implementation.

## Consequences

### Positive

- Clear storage responsibilities.
- Supports semantic search and structured graph traversal.
- Matches existing repository assumptions and test direction.

### Negative / Tradeoffs

- Requires operating two storage systems.
- Cross-store consistency must be handled explicitly.
- Some applications may eventually want different backends.

## Validation

- Integration tests should upsert objects into both stores and retrieve them via hybrid lookup.
- Tests should verify stable cross-store IDs.
- Backend mapping code should be isolated from core domain types.

## Revisit When

Revisit if operating two stores becomes too heavy for target users, or if a single backend becomes sufficient for both vector and graph behavior.
