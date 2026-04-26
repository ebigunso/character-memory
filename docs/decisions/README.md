# Character Memory ADRs

This directory contains Architecture Decision Records for Character Memory.

The ADRs are split into two tracks so high-level philosophy/design decisions do not get mixed with lower-level implementation choices.

## Directory layout

```text
docs/adrs/
  template.md
  design/
    ADR-D-0001-...
  implementation/
    ADR-I-0001-...
```

## Numbering

Use separate numbering per track:

```text
ADR-D-0001, ADR-D-0002, ...  High-level design and product-philosophy decisions.
ADR-I-0001, ADR-I-0002, ...  Implementation, storage, API, and operational decisions.
```

A design ADR should be used when overlooking the decision would risk violating the core Character Memory philosophy: episode-backed continuity, provenance, correction, or reflection.

An implementation ADR should be used when the decision is primarily about how the library is built: storage contracts, indexing, IDs, schema versions, retrieval bounds, and integration behavior.

## Status values

Recommended statuses:

```text
proposed
accepted
rejected
superseded
deprecated
```

Use `proposed` while the design is still being implemented. Move to `accepted` after the corresponding behavior exists and has tests.

## Current ADR set

### Design ADRs

- [ADR-D-0001: Use an episode-backed object model](design/ADR-D-0001-episode-backed-object-model.md)
- [ADR-D-0002: Require provenance for behavior-influencing derived memories](design/ADR-D-0002-derived-memory-provenance.md)
- [ADR-D-0003: Treat memory threads as soft continuity overlays](design/ADR-D-0003-soft-memory-threads.md)
- [ADR-D-0004: Return a ContinuityContextPack instead of a flat retrieval list](design/ADR-D-0004-continuity-context-pack.md)
- [ADR-D-0005: Use DerivedMemory subtypes in v0.1 before a normalized belief ontology](design/ADR-D-0005-derived-memory-subtypes-before-belief-ontology.md)
- [ADR-D-0006: Use supersession and suppression as default correction/forgetting mechanisms](design/ADR-D-0006-supersession-and-suppression.md)
- [ADR-D-0007: Start chat-native and transcript-compatible, not multimodal-native](design/ADR-D-0007-chat-native-transcript-compatible-start.md)
- [ADR-D-0008: Preserve source references because summaries are not source material](design/ADR-D-0008-preserve-source-references.md)

### Implementation ADRs

- [ADR-I-0001: Use stable cross-store IDs and deterministic graph IRIs](implementation/ADR-I-0001-stable-cross-store-ids.md)
- [ADR-I-0002: Embed natural-language semantic surfaces, not structured metadata templates](implementation/ADR-I-0002-natural-language-embedding-surfaces.md)
- [ADR-I-0003: Use Qdrant and Oxigraph as default storage backends](implementation/ADR-I-0003-qdrant-oxigraph-defaults.md)
- [ADR-I-0004: Keep graph relationships as typed MemoryLink records in the domain model](implementation/ADR-I-0004-typed-memory-links.md)
- [ADR-I-0005: Keep Qdrant metadata filterable while graph relationships remain authoritative](implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md)
- [ADR-I-0006: Bound graph expansion and expose retrieval rationale](implementation/ADR-I-0006-bounded-graph-expansion.md)
- [ADR-I-0007: Version persisted schemas from the first implementation](implementation/ADR-I-0007-schema-versioning.md)
