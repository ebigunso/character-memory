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

# ADR-I-0016: Use retrieval intent as query-time policy

## Context and Problem Statement

Retrieval governance needs different behavior for continuity, current-state reads, correction review, source audit, and associative diagnostics. Persisting retrieval eligibility metadata on memory objects would make retrieval policy durable object state instead of query-time policy.

## Decision

Character Memory retrieval APIs support query-time retrieval intent.

```rust
enum RetrievalIntent {
    Continuity,
    CurrentState,
    CorrectionReview,
    SourceAudit,
    AssociativeProbe,
}
```

`RetrievalIntent` is an input to retrieval policy. It is not persisted on memory objects as retrieval eligibility metadata.

`Continuity` is the default retrieval intent.

## Consequences

- Retrieval behavior can be explicit without mutating memory objects.
- `SourceAudit` can return provenance paths and source-reference metadata without resolving raw logs.
- `AssociativeProbe` can expose weak activation diagnostics without promoting weak associations.

## Validation

- Retrieval defaults to `Continuity` when no intent is supplied.
- `SourceAudit` does not add raw-log search or public raw-reference resolution.
- Retrieval traces record the applied intent.
