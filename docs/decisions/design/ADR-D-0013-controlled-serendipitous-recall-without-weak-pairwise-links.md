---
status: accepted
adr_type: design
date: 2026-05-10
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0013: Support controlled serendipitous recall without weak pairwise durable links

## Context and Problem Statement

Continuous entity selectivity and retrieval guardrails prevent durable pairwise links from weak low-selectivity co-occurrence. This protects the graph from broad-entity clique growth and false continuity.

However, human-like recall includes weak serendipitous associations: one memory can remind the assistant of another through partial cues, repeated coactivation, or a shared context that is not yet strong enough to be a durable relationship.

## Decision Drivers

- Preserve long-term graph quality while allowing richer recall later.
- Avoid treating broad recurring entities as evidence that all incident memories are meaningfully related.
- Keep future associative recall bounded, explainable, and compatible with graph authority.

## Decision

Character Memory will not create ordinary durable pairwise association links solely from weak low-selectivity co-occurrence.

The project will still support serendipitous associative recall later through controlled mechanisms:

```text
query-time activation
AssociativeUnit
AssociativeMembership
AssociationSupport
promotion/decay policy
bounded cluster expansion
```

Weak association evidence may influence retrieval before it becomes durable relationship truth.

## Character Memory Relevance

Character continuity should feel more human-like than exact search alone. Partial cues and "this reminds me of that" recall matter.

At the same time, false continuity is harmful. Memories should not become meaningfully associated merely because they share a broad recurring entity.

## Considered Options

1. Create ordinary pairwise associations from weak co-occurrence.
2. Block weak co-occurrence and drop serendipitous recall as a product goal.
3. Preserve serendipitous recall through controlled activation and graph-internal associative structures.

## Decision Outcome

Chosen option: **Preserve serendipitous recall through controlled activation and graph-internal associative structures**.

This keeps the low-information pairwise-link guard intact while leaving a clear path for bounded, explainable recall that can use evidence before it becomes durable relationship truth.

## Consequences

### Positive

- Preserves long-term graph quality.
- Avoids broad-entity clique growth.
- Keeps retrieval bounded and explainable.
- Leaves room for richer human-like recall later.

### Negative / Tradeoffs

- Some weak serendipitous recall may be missing before controlled associative recall is implemented.
- Associative recall will require additional machinery.
- Retrieval may depend more on semantic, temporal, thread, salience, and scope support until associative units exist.

## Validation

- Retrieval guardrail tests should verify that broad low-selectivity co-occurrence does not create ordinary durable pairwise links.
- Controlled associative recall tests should verify that broad entities plus narrowing evidence can support bounded associative recall.
- Durable association promotion should require multiple supporting signals or explicit rationale.

## Revisit When

Revisit during controlled associative recall and clustering, or earlier if diagnostics show the low-information link guard is harming important long-term recall.
