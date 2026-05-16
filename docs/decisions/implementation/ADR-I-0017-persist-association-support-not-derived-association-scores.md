---
status: accepted
adr_type: implementation
date: 2026-05-16
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0017: Persist association support, not derived association scores

## Context and Problem Statement

v0.5 adds controlled associative recall and clustering. Association retrieval needs scores such as strength, confidence, salience, activation, and review priority, but persisting those derived values as durable graph truth would make policy outputs compete with evidence.

## Decision

v0.5 persists associative structure and support evidence.

Persisted graph concepts:

```text
AssociativeUnit
AssociativeMembership
AssociativeMembership.status
AssociativeMembership.role, when needed
AssociationSupport
AssociationSupport.support_type
AssociationSupport.support_source_id
AssociationSupport.created_at
```

The following are derived or rebuildable policy/cache values, not durable graph truth by default:

```text
membership_strength
membership_confidence
membership_salience
supporting_signal_count
last_reinforced_at
activation score
review priority
```

## Consequences

- The graph stores associative evidence and lifecycle, not policy snapshots.
- Retrieval-time and maintenance-time policy can evolve without rewriting graph truth.
- Diagnostics can still expose derived scores as traces or rebuildable cache values.

## Validation

- Association fixtures distinguish support evidence from derived policy values.
- Retrieval tests can rebuild activation and review priority from persisted support.
- Durable graph assertions treat unit, membership lifecycle, and support evidence as authoritative.
