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

# ADR-I-0004: Keep graph relationships as typed MemoryLink records in the domain model

## Context and Problem Statement

The graph layer will store many relationship types: episode-to-observation, episode-to-entity, episode-to-thread, derived-memory provenance, supersession, contradiction, association, and more. If these relations exist only as backend-specific triples, the domain layer loses an inspectable representation of memory structure.

## Decision Drivers

- Keep the v0.1 domain model simple but graph-aware.
- Allow relationship confidence and rationale where useful.
- Avoid hard-coding every relationship as a separate top-level field.
- Preserve a path toward RDF/SPARQL mapping without making RDF the only public representation.

## Decision

Use a typed `MemoryLink` or equivalent domain edge object for graph relationships.

A link may include:

```text
id
from_id
to_id
relation
confidence
rationale
created_at
metadata
```

Starter relation types include:

```text
has_observation
mentions
involves
about
derived_from
part_of_thread
supports
contradicts
supersedes
resolves
creates_open_loop
fulfills_commitment
associated_with
```

## Implementation Impact

- `MemoryLink` should map to graph triples in Oxigraph.
- Reified links may be needed when confidence/rationale is present.
- Retrieval can use links to assemble graph context and rationale.

## Considered Options

1. Store relationships only as backend-specific RDF triples.
2. Store relationships only as direct fields on objects.
3. Use typed `MemoryLink` edges in the domain model and map them to graph storage.

## Decision Outcome

Chosen option: **3. Use typed `MemoryLink` edges in the domain model and map them to graph storage**.

This keeps the domain model extensible without forcing every relationship into a special class.

## Consequences

### Positive

- Easier to add new relation types without schema churn.
- Supports soft thread membership and provenance links.
- Makes retrieval rationale easier to inspect.

### Negative / Tradeoffs

- Requires link validation to prevent arbitrary relation sprawl.
- Some relations may later deserve specialized first-class objects.

## Validation

- Tests should cover creation, persistence, and retrieval of core relation types.
- Invalid or unknown relation types should be rejected or namespaced explicitly.
- Graph round-trip tests should verify link mapping.

## Revisit When

Revisit when certain links become semantically rich enough to require dedicated object types.
