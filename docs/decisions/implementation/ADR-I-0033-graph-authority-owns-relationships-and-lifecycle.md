---
status: proposed
adr_type: implementation
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-I-0005-qdrant-payload-vs-graph-authority--superseded-by-ADR-I-0033.md]
superseded_by: null
depends_on: [ADR-I-0025-vector-record-is-a-read-contract.md, ADR-I-0028-vector-prefilters-require-fully-populated-current-columns-and-never-match-unknown.md]
---

# ADR-I-0033: Graph authority owns relationships, provenance, and lifecycle, and the vector layer is candidate recall only

## Context and Problem Statement

Two stores hold memory, and any fact represented in both can drift. ADR-I-0005 answered this by letting the vector payload carry relationship and lifecycle hints for filtering while the graph stayed authoritative; ADR-I-0025 then removed those hints because nothing read them and nothing kept them in sync. What survives, and needs its own home, is the authority split itself: which store decides.

## Decision

The graph store is the single authority for memory objects, relationships, provenance paths, supersession, contradiction, thread and entity linkage, currency, and retention state. Final inclusion of any memory in a retrieval result is decided by graph authority.

The vector layer is candidate recall only. It suggests candidates by semantic similarity within the scope its record supports; it never decides inclusion, never describes a memory, and is rebuildable from graph authority at any time. A vector-layer predicate exists only under the admission rule of ADR-I-0028.

## Why

Two authorities for one fact is a drift waiting to happen, and every drift in lifecycle or linkage shows up as a memory the character wrongly has or wrongly lacks. One authority for truth and one suggester for recall keeps retrieval fast without letting a stale hint become the reason a memory is unreachable.

## Rejected Alternatives

- Relationships only in vector payloads: rejected outright; the vector store cannot traverse or verify them.
- Relationship and lifecycle hints mirrored in the vector payload for prefiltering: rejected by ADR-I-0025 because unread and unsynchronised; a column returns only under ADR-I-0028.
- A vector store that decides inclusion for speed: rejected outright; candidate recall is non-authoritative by design so that a determinism caveat never becomes a retrieval failure (ADR-I-0024).

## Decision Boundary

Invariant: graph authority decides inclusion, relationships, provenance, currency, and retention; the vector layer only proposes candidates and is rebuildable from graph authority.

Not covered: the graph store implementation and its default (ADR-I-0021), the vector record's fields (ADR-I-0025), and the expansion bounds applied after candidates are verified (ADR-I-0006).

## Validation

- Retrieval tests show every vector candidate is verified through graph authority before inclusion and that lifecycle filtering happens on graph state.
- Consistency tests detect vector records whose object ID does not resolve in graph authority.
- A rebuild test reconstructs the vector store from graph authority alone.

## Revisit When

Graph lookups become fast enough that candidate recall itself moves into graph authority, or a deployment requires a vector store that is not rebuildable from the graph.

## More Information

- ADR-I-0024 makes candidate recall report its own completeness rather than fail.
- ADR-I-0021 and ADR-I-0023 set the default stores for each role.
- ADR-I-0005 in `superseded/` carries the original reasoning of 2026-04-26.
