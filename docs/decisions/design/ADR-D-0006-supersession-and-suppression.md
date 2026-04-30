---
status: accepted
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
- Leave room for later user-controlled deletion or redaction when required.
- Avoid rewriting historical episodes as if they never happened.

## Decision

The default lifecycle operations are:

```text
supersede
suppress
archive
defer destructive redaction/delete until explicit production policy support exists
```

Corrections should normally create new records and link them to old records via `supersedes` or equivalent relations. Forgetting should normally suppress or archive records from default retrieval. Physical redaction/delete remains outside v0.1 production support until explicit policy and storage behavior are implemented.

## Character Memory Relevance

Character continuity includes mistakes, corrections, and changed understanding. The assistant should be able to stop relying on an outdated memory while still preserving the fact that it once had that memory, unless the user or policy requires deletion.

## Implementation Impact

- Add lifecycle fields such as `retention_state`, `is_current`, `supersedes`, and/or `superseded_by`.
- Default retrieval filters must exclude suppressed, archived, and superseded records unless explicitly requested.
- Correction APIs should avoid overwriting records in place when supersession is more appropriate.

## Considered Options

1. Overwrite memories on correction.
2. Hard-delete by default when corrected or forgotten.
3. Supersede or suppress by default, with destructive redaction/delete deferred to explicit later policy support.

## Decision Outcome

Chosen option: **3. Supersede or suppress by default, with destructive redaction/delete deferred to explicit later policy support**.

This preserves continuity and auditability while respecting forgetting and correction needs.

## Consequences

### Positive

- Maintains history of how memories changed.
- Allows developers to debug behavior changes.
- Prevents outdated or unwanted memories from being retrieved by default.

### Negative / Tradeoffs

- Requires lifecycle filtering in retrieval.
- Storage grows unless retention policies later archive or delete old material.
- User-facing deletion semantics must remain clear that v0.1 lifecycle forgetting is non-destructive.

## Validation

- Retrieval tests should verify that suppressed and superseded records are excluded by default.
- Correction tests should verify supersession links.
- Forget tests should cover suppress and archive modes; redaction/delete require separate production policy support before being documented as implemented.

## Revisit When

Revisit when privacy, legal, or user-control requirements demand stronger deletion semantics by default.
