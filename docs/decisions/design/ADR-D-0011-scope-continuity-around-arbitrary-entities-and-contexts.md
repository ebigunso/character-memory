---
status: accepted
adr_type: design
date: 2026-05-08
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0011: Scope continuity around arbitrary entities and contexts

## Context and Problem Statement

The v0.2 roadmap originally emphasized continuity and reflection using concepts like relationship state, character signals, commitments, and open loops. If those concepts implicitly center on a single user-assistant relationship, they will not generalize well to companions, simulations, games, or research systems.

The problem: how should current continuity state be attached so it remains useful across application domains?

## Decision Drivers

- Avoid assuming that continuity is globally user-centered.
- Support relationship state between arbitrary entities.
- Allow character signals for any continuing entity or scope.
- Make reflection targeted instead of all-history scanning.
- Preserve the ability for applications to define custom scope boundaries.

## Decision

Introduce `ContinuityScope` as the organizing concept for v0.2 continuity and reflection.

A `ContinuityScope` may represent:

```text
entity
entity pair
thread
project
place
source conversation
character
application-provided custom scope
```

Existing v0.2 concepts become scope-aware:

```text
RelationshipState:
  relationship between arbitrary entities or within a relationship-like scope

CharacterSignal:
  signal for any continuing entity or scoped continuity subject

OpenLoop:
  unresolved question, tension, commitment, or pending matter in a scope

Commitment:
  obligation or intent attributed to an entity or relationship scope

CurrentContinuityView:
  current usable context for a scope, not a global user profile
```

## Character Memory Relevance

Character continuity is accumulated, but it is not always global. A project, relationship, scene, recurring place, or simulated character may each have different continuity context.

Scoping prevents a memory signal that is valid in one context from becoming a brittle global persona patch.

## Implementation Impact

- Reflection jobs should require explicit or inferred `ContinuityScope`.
- Current continuity views should be generated for a scope.
- Relationship state should support arbitrary entity relationships.
- Character signals should attach to a continuing entity or scope.
- Open loops and commitments should be retrievable by scope.
- Reflection should avoid all-history scans through broad entities.

## Considered Options

1. Keep continuity global.
2. Center continuity around a user-assistant relationship.
3. Make continuity scope explicit and entity-neutral.

## Decision Outcome

Chosen option: **3. Make continuity scope explicit and entity-neutral**.

This supports the broad set of intended use cases while preserving precise current continuity views.

## Consequences

### Positive

- Avoids global persona overwrites.
- Supports games, simulations, companions, and research systems.
- Makes reflection more targeted and inspectable.
- Works naturally with v0.1.2 selectivity and fanout guardrails.

### Negative / Tradeoffs

- Adds a scope model that applications must understand.
- Scope inference may be ambiguous.
- Some simple assistant applications may need convenience defaults.

## Validation

- Tests should show `RelationshipState` between arbitrary entities.
- Tests should show `CharacterSignal` attached to non-user scopes.
- Reflection jobs should require explicit or inferred scope.
- Current continuity views should not require scanning all history.
- Open loops and commitments should be retrievable by scope.

## Revisit When

Revisit if scope modeling becomes too complex for v0.2. If necessary, keep `ContinuityScope` simple initially and let applications provide custom scope IDs. Do not revert to user/assistant assumptions.
