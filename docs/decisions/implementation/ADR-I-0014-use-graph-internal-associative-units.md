---
status: accepted
adr_type: implementation
date: 2026-05-10
deciders: []
consulted: []
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0014: Use graph-internal associative units instead of a separate weak hint store

## Context and Problem Statement

One possible way to support weak serendipitous recall is to maintain a separate store of weak associative hints. That would reduce graph clutter, but it risks drift between hint state and graph authority.

Character Memory already uses Oxigraph as graph authority for object existence, relationships, provenance, lifecycle, currentness, and expansion context.

## Decision Drivers

- Keep associative recall under graph lifecycle and provenance authority.
- Avoid introducing a third memory truth surface.
- Make weak association evidence inspectable and queryable through graph tooling.

## Decision

Weak associative evidence should be represented inside the graph as explicit associative structures, not in a separate weak hint store.

The implementation should use:

```text
AssociativeUnit
AssociativeMembership
AssociationSupport
```

rather than free-floating hints or ordinary low-value pairwise association edges.

## Implementation Impact

Associative recall remains under graph lifecycle, provenance, and currentness validation.

Retrieval can still treat candidate/peripheral memberships differently from core/exemplar memberships.

## Considered Options

1. Store weak associative hints outside the graph.
2. Use ordinary low-value pairwise association edges.
3. Represent associative evidence as graph-internal units, memberships, and support records.

## Decision Outcome

Chosen option: **Represent associative evidence as graph-internal units, memberships, and support records**.

This preserves graph authority while avoiding broad pairwise edge pollution and separate weak-hint drift.

## Consequences

### Positive

- Reduces drift between associative recall evidence and graph authority.
- Keeps associative structures inspectable through graph queries.
- Supports lifecycle and provenance validation.
- Avoids creating a third memory truth surface.

### Negative / Tradeoffs

- Adds graph schema complexity.
- Requires bounded expansion and careful retrieval policy.
- Requires maintenance of member-level lifecycle.

## Validation

- `AssociativeUnit` and `AssociativeMembership` records must be graph-queryable.
- Suppressed, deleted, or superseded memories must be excluded even if they remain memberships.
- Retrieval must not use associative membership to bypass graph authority.
- No separate weak hint store should be required for core associative recall.

## Revisit When

Revisit if graph storage or query performance cannot support associative units at required scale. Prefer bounded expansion and lifecycle filtering before introducing a separate hint store.
