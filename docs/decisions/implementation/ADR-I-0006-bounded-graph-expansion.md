---
status: proposed
adr_type: implementation
date: 2026-04-26
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0006: Bound graph expansion and expose retrieval rationale

## Context and Problem Statement

Hybrid retrieval expands from vector candidates into graph context. Without bounds, expansion around hub entities or active threads can become slow, noisy, or unpredictable. Without rationale, developers cannot inspect why memories influenced generation.

## Decision Drivers

- Keep retrieval latency and output size predictable.
- Prevent hub entities from overwhelming context packs.
- Make memory injection debuggable.
- Preserve the old roadmap's concern for graph expansion controls while adapting output to continuity context.

## Decision

Graph expansion must be policy-bounded.

Expansion policies should support controls such as:

```text
max_depth
max_fanout
max_items_per_section
timeout
hub_entity_limit
allowed_relation_types
retention filters
current-only filters
```

Retrieval should also expose rationale or score components for included items where practical.

## Implementation Impact

- Retrieval APIs should accept a policy or use sane defaults.
- `ContinuityContextPack.rationale` should explain why major items were selected.
- Expansion should fail boundedly, not produce unbounded traversal.

## Considered Options

1. Expand all graph neighbors around candidates.
2. Avoid graph expansion and return vector results only.
3. Use bounded graph expansion with retrieval rationale.

## Decision Outcome

Chosen option: **3. Use bounded graph expansion with retrieval rationale**.

This keeps retrieval useful, predictable, and inspectable.

## Consequences

### Positive

- Prevents runaway graph queries.
- Improves developer trust in retrieved context.
- Supports tests for deterministic retrieval behavior.

### Negative / Tradeoffs

- Some relevant memories may be missed if bounds are too strict.
- Requires tuning retrieval policies by use case.

## Validation

- Stress tests should include hub entities and active high-fanout threads.
- Retrieval should respect max depth, fanout, and section limits.
- Context packs should include rationale for selected categories or records.

## Revisit When

Revisit when association graph retrieval or query-time evidence subgraphs are introduced, as expansion policy may need to become more sophisticated.
