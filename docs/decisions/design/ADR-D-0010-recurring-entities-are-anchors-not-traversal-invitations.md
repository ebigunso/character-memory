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

# ADR-D-0010: Treat recurring entities as continuity anchors, not traversal invitations

## Context and Problem Statement

Entities are first-class anchors for recall. Over years of memory accumulation, some entities will become high-degree: a person, place, project, character, object, topic, organization, faction, or scene may appear in hundreds or thousands of memories.

The problem: how should retrieval use recurring entities without causing unbounded expansion, context pollution, or weak relevance?

## Decision Drivers

- Preserve the value of entity continuity.
- Prevent high-degree entities from flooding context packs.
- Avoid global penalties that treat broad entities as unimportant.
- Keep retrieval bounded and explainable.
- Support many application domains without identity-specific exceptions.

## Decision

A recurring entity should be treated as a continuity anchor, not as permission to expand through all connected memories.

High degree affects expansion policy, not entity importance.

Low selectivity means:

```text
Do not expand broadly from this entity unless additional retrieval evidence supports it.
```

Supporting evidence may include:

```text
semantic similarity
thread membership
temporal relevance
salience
currentness
correction/supersession relevance
explicit retrieval scope
application-provided scope
```

## Character Memory Relevance

Human-like recall is associative, but association is not the same as indiscriminate traversal. A recurring entity should help the system remember continuity, not cause the assistant or character to drag unrelated memories into context.

This decision protects the product goal: stable continuity without false relevance.

## Implementation Impact

- Retrieval should compute relation-specific selectivity from derived counters.
- Low-selectivity entity matches require additional support before broad expansion.
- Retrieval rationale must distinguish high-selectivity entity matches from low-selectivity entity matches.
- Explicit scope may justify bounded expansion through a broad entity.
- Fanout caps remain relation-specific and must not be bypassed by entity importance.

## Considered Options

1. Treat all entity matches as equally strong retrieval signals.
2. Globally penalize high-degree entities as unimportant.
3. Use high degree to restrict expansion while allowing supporting evidence to restore bounded relevance.

## Decision Outcome

Chosen option: **3. Use high degree to restrict expansion while allowing supporting evidence to restore bounded relevance**.

This keeps broad entities useful without making them context-polluting traversal hubs.

## Consequences

### Positive

- Reduces context pollution around recurring entities.
- Keeps high-degree central entities usable when properly scoped.
- Avoids conflating broadness with low importance.
- Supports long-lived memory graphs.

### Negative / Tradeoffs

- Some relevant memories may be missed if fanout policy is too conservative.
- Selectivity and support weighting require tuning.
- Diagnostics are needed to detect over-restriction.

## Validation

- Tests should show that low-selectivity entity evidence alone cannot flood a context pack.
- Tests should show that broad entities can still contribute when supported by semantic, thread, temporal, salience, currentness, correction, or explicit scope evidence.
- Retrieval rationale should identify when broad-entity expansion was rejected or allowed.
- Fanout budgets must remain capped by relation policy.

## Revisit When

Revisit if real deployments show that selectivity-based fanout control is harming recall for central but broad entities. The first adjustment should be policy configuration or scope-aware weighting, not entity identity special-casing.
