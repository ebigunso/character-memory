---
status: accepted
adr_type: implementation
date: 2026-05-08
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0011: Guard against durable links from low-information co-occurrence

## Context and Problem Statement

As memory accumulates, broad entities can connect many otherwise unrelated episodes and derived memories. If the system creates durable pairwise links merely because records share a broad entity, it can produce O(N²) edge growth and weak associative recall.

The problem: when should co-occurrence become a durable link?

## Decision Drivers

- Avoid pairwise clique growth around broad entities.
- Preserve the value of meaningful associations.
- Keep associations inspectable and evidence-backed.
- Prevent low-information co-occurrence from polluting retrieval.
- Prepare for v0.5 advanced association/clustering.

## Decision

Do not create durable pairwise links solely because two memories share a low-selectivity entity or broad relation.

Do not create pairwise durable links only because:

```text
two episodes mention the same frequent person
two episodes occur in the same common place
two memories involve the same recurring project
two derived memories share a broad topic
two observations share a low-selectivity participant
```

Durable association should require stronger evidence:

```text
semantic similarity
explicit application-created link
same active thread
causal relationship
temporal relationship
correction/supersession
shared selective entity
repeated pattern
high salience
reflection-derived rationale
```

## Implementation Impact

- Add a low-information co-occurrence guard before durable link creation.
- Use selectivity scores and supporting evidence in future association admission.
- Log or report rejected low-information link candidates in diagnostics.
- Keep raw graph relationships and provenance separate from associative links.

## Considered Options

1. Create durable links for all shared entities.
2. Disable co-occurrence-based links entirely.
3. Allow durable links only when co-occurrence has additional evidence or a selective relation.

## Decision Outcome

Chosen option: **3. Allow durable links only when co-occurrence has additional evidence or a selective relation**.

This preserves useful associations while avoiding unbounded edge growth around broad entities.

## Consequences

### Positive

- Prevents O(N²) pairwise edge growth around recurring entities.
- Keeps associative recall higher signal.
- Makes v0.5 association work safer.
- Supports diagnostics for rejected low-information edges.

### Negative / Tradeoffs

- Some weak but potentially useful associations may not be stored durably.
- Association admission requires more evidence than simple co-occurrence.
- Retrieval may rely on bounded expansion instead of precomputed links for some cases.

## Validation

- Tests should show durable pairwise links are not created solely from shared low-selectivity entity co-occurrence.
- Tests should show durable links can be created when there is stronger evidence such as semantic similarity, same active thread, correction relation, temporal relation, or explicit application intent.
- Diagnostics should count rejected low-information co-occurrence candidates.

## Revisit When

Revisit during v0.5 association/clustering design. At that point, association admission should use selectivity scores and evidence strength, not raw co-occurrence.
