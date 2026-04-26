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

# ADR-D-0008: Preserve source references because summaries are not source material

## Context and Problem Statement

Episode summaries and derived memories are useful, but they are lossy. If the system stores only summaries, later correction, inspection, reflection, and provenance become weaker because the original conversational context is unavailable.

## Decision Drivers

- Preserve auditability of derived memories.
- Allow later reflection to revisit source context.
- Support user correction and developer inspection.
- Avoid making summaries the only evidence for behavior-influencing memories.

## Decision

Episodes should include `raw_ref` or an equivalent pointer to the source conversation, transcript, or stored source material when available.

The graph/vector layer may store summaries, excerpts, and derived memories, but these are not substitutes for source material.

## Character Memory Relevance

Character continuity depends on memory shaped by actual past interaction. If the original interaction is lost and only a summary remains, the system may preserve a distorted version of the relationship history.

## Implementation Impact

- `Episode` should support a `raw_ref` or equivalent source pointer.
- `Observation` may store excerpts, but should still be traceable to the episode and source context.
- Retrieval can return summaries by default while keeping source references available for inspection.

## Considered Options

1. Store only summaries.
2. Store full raw logs directly in the graph/vector stores.
3. Store summaries/excerpts in memory stores and keep source references to raw material.

## Decision Outcome

Chosen option: **3. Store summaries/excerpts in memory stores and keep source references to raw material**.

This balances auditability with storage practicality.

## Consequences

### Positive

- Supports provenance and later correction.
- Avoids bloating vector or graph stores with full raw logs.
- Keeps retrieval concise while preserving inspection paths.

### Negative / Tradeoffs

- Requires the application or storage layer to manage raw source retention.
- Source references may become stale if upstream logs are deleted or moved.

## Validation

- Episode fixtures should include `raw_ref` when source material exists.
- Derived memory provenance tests should be able to trace from derived memory to episode and source reference.
- Documentation should distinguish summary, excerpt, and source material.

## Revisit When

Revisit if applications cannot reliably preserve raw source material or if privacy requirements require short source retention windows.
