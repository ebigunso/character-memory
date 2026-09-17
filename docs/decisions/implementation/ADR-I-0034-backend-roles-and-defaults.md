---
status: accepted
adr_type: implementation
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-I-0003-qdrant-oxigraph-defaults--superseded-by-ADR-I-0034.md]
superseded_by: null
depends_on: [ADR-I-0021-embedded-persistent-oxigraph-default.md, ADR-I-0023-embedded-qdrant-edge-vector-candidate-store.md, ADR-I-0033-graph-authority-owns-relationships-and-lifecycle.md]
---

# ADR-I-0034: Vector candidate recall and graph authority are separate backend roles, each with an embedded default

## Context and Problem Statement

The library needs semantic recall and explicit relationship traversal, which no single store does well. ADR-I-0003 chose Qdrant and Oxigraph as the default backends for those roles when both ran as services and the graph store was in-memory. Both defaults have since moved to embedded persistent modes by their own records, and ADR-I-0003 described a state that no longer exists. This record restates the surviving decision: the roles, and where each default is decided.

## Decision

Storage is split into two backend roles that the library composes and never merges: vector candidate recall and graph authority. Each role has a port, and domain objects expose no backend client types; backend-specific mappings live in adapter modules behind the port.

The vector candidate role is served by the Qdrant engine family, with the embedded in-process engine as the default mode and the service as the explicit alternative (ADR-I-0023). The graph authority role is served by Oxigraph, with embedded persistent storage as the default (ADR-I-0021). Retrieval statistics are a third, derived store outside both roles (ADR-I-0008, ADR-I-0009).

## Why

Recall by meaning and traversal by relationship are different capabilities with different failure modes, and keeping them as separate roles behind ports lets either backend change, including to a service mode, without the domain model or the other role noticing.

## Rejected Alternatives

- A vector database alone: rejected outright; relationships, provenance, and lifecycle need traversal and authority.
- A graph database alone: rejected outright; semantic recall over natural-language surfaces needs a vector index.
- Leaving backends unchosen: rejected outright; the library ships a validated default path, and each default is licensed by the evidence its own record cites.
- A single backend serving both roles: reopen if one engine becomes sufficient for both semantic recall and graph authority at the decade scale ADR-I-0023 names.

## Decision Boundary

Invariant: the two roles stay separate behind their ports; domain types carry no backend types; each role's default is set by a measured record, never by this one.

Not covered: the defaults themselves and their modes (ADR-I-0021, ADR-I-0023), tuning values, and the statistics store's engine (ADR-I-0009).

## Validation

- Integration tests write through both roles and retrieve through the hybrid path on the default modes without any external service.
- Adapter mapping code is isolated from domain types, checked by the dependency-direction rule of ADR-I-0018.

## Revisit When

Operating two roles becomes too heavy for a target deployment even in embedded modes, or one engine serves both roles.

## More Information

- ADR-I-0033 fixes which role is authoritative.
- ADR-I-0003 in `superseded/` carries the original reasoning of 2026-04-26.
