---
status: proposed
adr_type: design
date: 2026-04-26
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0006: Use supersession and suppression as default correction/forgetting mechanisms

## Context and Problem Statement

Persistent memory must support correction and forgetting. If old memories are overwritten destructively, the system loses the ability to explain how its behavior changed. If forgotten or corrected memories continue to influence generation, user trust is damaged.

## Decision Drivers

- Preserve correction history where appropriate.
- Prevent suppressed or forgotten memories from influencing default retrieval.
- Support user-controlled deletion or redaction when required.
- Avoid rewriting historical episodes as if they never happened.

## Decision

The default lifecycle operations are:

```text
supersede
suppress
archive
redact
delete only when explicitly required
```

Corrections should normally create new records and link them to old records via `supersedes` or equivalent relations. Forgetting should normally suppress or archive records from default retrieval unless policy or user request requires deletion or redaction.

## Character Memory Relevance

Character continuity includes mistakes, corrections, and changed understanding. The assistant should be able to stop relying on an outdated memory while still preserving the fact that it once had that memory, unless the user or policy requires deletion.

## Implementation Impact

- Add lifecycle fields such as `retention_state`, `is_current`, `supersedes`, and/or `superseded_by`.
- Default retrieval filters must exclude suppressed, redacted, deleted, and superseded records unless explicitly requested.
- Correction APIs should avoid overwriting records in place when supersession is more appropriate.

## Considered Options

1. Overwrite memories on correction.
2. Hard-delete by default when corrected or forgotten.
3. Supersede or suppress by default, with deletion/redaction as explicit operations.

## Decision Outcome

Chosen option: **3. Supersede or suppress by default, with deletion/redaction as explicit operations**.

This preserves continuity and auditability while respecting forgetting and correction needs.

## Consequences

### Positive

- Maintains history of how memories changed.
- Allows developers to debug behavior changes.
- Prevents outdated or unwanted memories from being retrieved by default.

### Negative / Tradeoffs

- Requires lifecycle filtering in retrieval.
- Storage grows unless retention policies later archive or delete old material.
- User-facing deletion semantics must be clear.

## Validation

- Retrieval tests should verify that suppressed and superseded records are excluded by default.
- Correction tests should verify supersession links.
- Forget tests should cover suppress, archive, redact, and delete modes where implemented.

## Revisit When

Revisit when privacy, legal, or user-control requirements demand stronger deletion semantics by default.
