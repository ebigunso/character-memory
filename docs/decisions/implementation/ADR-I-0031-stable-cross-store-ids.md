---
status: proposed
adr_type: implementation
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-I-0001-stable-cross-store-ids--superseded-by-ADR-I-0031.md]
superseded_by: null
depends_on: [ADR-I-0025-vector-record-is-a-read-contract.md]
---

# ADR-I-0031: Every persisted memory object has a stable ID, and its graph IRI is derived from that ID deterministically

## Context and Problem Statement

Memory lives in a vector store and a graph store, and provenance, correction, retrieval expansion, and debugging all depend on joining the two reliably. IDs generated independently by each backend, or derived from mutable text such as a summary, would make those joins fragile and every update a migration. This record restates the decision of ADR-I-0001 as it stands after ADR-I-0025 removed the graph URI from the vector record.

## Decision

Every persisted memory object has a stable ID assigned once and never derived from its content. The object's graph IRI is a deterministic function of that ID and its object type. The vector record carries the stable object ID as its cross-store identity, and graph authority derives the IRI from it; no store carries a second copy of the identity. Upserts are idempotent for the same object ID.

## Why

A join that depends on a value that can change is not a join, and hybrid retrieval, supersession links, and provenance chains are all joins across the two stores. One stable identity that every store carries, with everything else derived from it, keeps those joins predictable across migrations, re-embeddings, and corrections.

## Rejected Alternatives

- Independent IDs per backend: rejected outright; cross-store joins would need a mapping table that is itself mutable state.
- IRIs generated from normalized content: rejected outright; any edit or correction would change the identity of the thing corrected.
- Carrying the derived graph IRI in the vector record as well: rejected by ADR-I-0025 as a redundant copy of the ID; reopen only if a vector-layer reader needs the IRI without access to graph authority.

## Decision Boundary

Invariant: object identity is a stable ID assigned once; graph IRIs derive from it deterministically; upserts on the same ID are idempotent; no store derives identity from mutable content.

Not covered: the IRI scheme's exact form, the ID format, and how a caller supplies or receives IDs (ADR-I-0020).

## Validation

- Round-trip tests: object ID to graph IRI to vector record to object ID.
- Idempotent upsert tests on both stores.
- A test fails if a graph resource is generated from a summary or other mutable text.

## Revisit When

The storage architecture collapses to a single backend, or a requirement appears that stable IDs cannot satisfy, such as content-addressed deduplication.

## More Information

- ADR-I-0025 defines the vector record, which carries the object ID and not the IRI.
- ADR-I-0020 defines how identity survives restarts through caller-supplied IDs.
- ADR-I-0001 in `superseded/` carries the original reasoning of 2026-04-26.
