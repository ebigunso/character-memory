# Character Memory Decisions

This directory contains decision records (ADRs) for Character Memory.

The decision records are split into two tracks so high-level philosophy/design decisions do not get mixed with lower-level implementation choices.

## Directory layout

```text
docs/decisions/
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

A design ADR should be used when overlooking the decision would risk violating the core Character Memory philosophy: episode-backed continuity, provenance, correction, reflection, scoped continuity, or entity-neutral recall.

An implementation ADR should be used when the decision is primarily about how the library is built: storage contracts, indexing, IDs, schema versions, retrieval bounds, fanout policy, derived stats, and integration behavior.

## Status values

Recommended statuses:

```text
accepted
rejected
superseded
deprecated
```

Records in this repository are expected to capture decisions, not undecided proposals. Use `accepted` when the decision is current, `rejected` when a considered decision is not adopted, `superseded` when a later decision replaces it, and `deprecated` when the decision remains historical but should no longer guide new work.

## Current Decision Set

### Design Decisions

- [ADR-D-0001: Use an episode-backed object model](design/ADR-D-0001-episode-backed-object-model.md)
- [ADR-D-0002: Require provenance for behavior-influencing derived memories](design/ADR-D-0002-derived-memory-provenance.md)
- [ADR-D-0003: Treat memory threads as soft continuity overlays](design/ADR-D-0003-soft-memory-threads.md)
- [ADR-D-0004: Return a ContinuityContextPack instead of a flat retrieval list](design/ADR-D-0004-continuity-context-pack.md)
- [ADR-D-0005: Use DerivedMemory subtypes in v0.1 before a normalized belief ontology](design/ADR-D-0005-derived-memory-subtypes-before-belief-ontology.md)
- [ADR-D-0006: Use supersession and suppression as default correction/forgetting mechanisms](design/ADR-D-0006-supersession-and-suppression.md)
- [ADR-D-0007: Start chat-native and transcript-compatible, not multimodal-native](design/ADR-D-0007-chat-native-transcript-compatible-start.md)
- [ADR-D-0008: Preserve source references because summaries are not source material](design/ADR-D-0008-preserve-source-references.md)
- [ADR-D-0009: Keep core retrieval policy entity-neutral](design/ADR-D-0009-entity-neutral-retrieval-policy.md)
- [ADR-D-0010: Treat recurring entities as continuity anchors, not traversal invitations](design/ADR-D-0010-recurring-entities-are-anchors-not-traversal-invitations.md)
- [ADR-D-0011: Scope continuity around arbitrary entities and contexts](design/ADR-D-0011-scope-continuity-around-arbitrary-entities-and-contexts.md)
- [ADR-D-0012: Separate memory candidates from committed memory](design/ADR-D-0012-separate-memory-candidates-from-committed-memory.md)
- [ADR-D-0013: Support controlled serendipitous recall without weak pairwise durable links](design/ADR-D-0013-controlled-serendipitous-recall-without-weak-pairwise-links.md)
- [ADR-D-0014: Represent associative membership lifecycle separately from associative unit lifecycle](design/ADR-D-0014-represent-associative-membership-lifecycle-separately.md)
- [ADR-D-0015: Keep raw source storage outside Character Memory core](design/ADR-D-0015-keep-raw-source-storage-outside-core.md)
- [ADR-D-0016: Do not add a generic MetaMemory plane to core](design/ADR-D-0016-do-not-add-generic-metamemory-plane.md)
- [ADR-D-0017: Keep the memory record append-only, with erasure as an out-of-band operational action](design/ADR-D-0017-append-only-memory-record-with-out-of-band-purge.md)

### Implementation Decisions

- [ADR-I-0001: Use stable cross-store IDs and deterministic graph IRIs](implementation/ADR-I-0001-stable-cross-store-ids.md)
- [ADR-I-0002: Embed natural-language semantic surfaces, not structured metadata templates](implementation/ADR-I-0002-natural-language-embedding-surfaces.md)
- [ADR-I-0003: Use Qdrant and Oxigraph as default storage backends](implementation/ADR-I-0003-qdrant-oxigraph-defaults.md)
- [ADR-I-0004: Keep graph relationships as typed MemoryLink records in the domain model](implementation/ADR-I-0004-typed-memory-links.md)
- [ADR-I-0005: Keep Qdrant metadata filterable while graph relationships remain authoritative](implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md)
- [ADR-I-0006: Bound graph expansion and expose retrieval rationale](implementation/ADR-I-0006-bounded-graph-expansion.md)
- [ADR-I-0007: Version persisted schemas from the first implementation](implementation/ADR-I-0007-schema-versioning.md)
- [ADR-I-0008: Treat retrieval stats as derived policy metadata, not graph truth](implementation/ADR-I-0008-retrieval-stats-are-derived-policy-metadata.md)
- [ADR-I-0009: Use SQLite as the default retrieval stats store](implementation/ADR-I-0009-use-sqlite-as-default-retrieval-stats-store.md)
- [ADR-I-0010: Use continuous selectivity and smooth fanout](implementation/ADR-I-0010-use-continuous-selectivity-and-smooth-fanout.md)
- [ADR-I-0011: Guard against durable links from low-information co-occurrence](implementation/ADR-I-0011-guard-against-low-information-co-occurrence-links.md)
- [ADR-I-0012: Use prepare / validate / commit for the write workflow](implementation/ADR-I-0012-use-prepare-validate-commit-write-workflow.md)
- [ADR-I-0013: Deterministic write-planning helpers do not infer high-level meaning](implementation/ADR-I-0013-deterministic-helpers-do-not-infer-high-level-meaning.md)
- [ADR-I-0014: Use graph-internal associative units instead of a separate weak hint store](implementation/ADR-I-0014-use-graph-internal-associative-units.md)
- [ADR-I-0015: Record producer and rationale origin in candidate provenance](implementation/ADR-I-0015-record-producer-and-rationale-origin-in-candidate-provenance.md)
- [ADR-I-0016: Use retrieval intent as query-time policy](implementation/ADR-I-0016-use-retrieval-intent-as-query-time-policy.md)
- [ADR-I-0017: Persist association support, not derived association scores](implementation/ADR-I-0017-persist-association-support-not-derived-association-scores.md)
- [ADR-I-0018: Organize the crate into responsibility-boundary modules with enforced dependency direction](implementation/ADR-I-0018-responsibility-boundary-modules-with-enforced-dependency-direction.md)
