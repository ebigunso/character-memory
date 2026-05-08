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

# ADR-I-0010: Use continuous selectivity and smooth fanout

## Context and Problem Statement

High-degree entities can cause broad expansion, but hard categories such as `broad` or `hub` risk creating brittle thresholds and persistent labels that go stale as the graph grows.

The problem: how should retrieval policy decide how much to expand through an entity/relation/object combination?

## Decision Drivers

- Avoid persisted entity categories.
- Keep selectivity relation-specific and object-type-specific.
- Make fanout tunable and smooth rather than cliff-based.
- Allow supporting evidence to restore bounded relevance for broad entities.
- Keep diagnostics explainable.

## Decision

Persist counters, not categories. Compute selectivity at retrieval time.

For an entity `e`, relation `r`, object type `o`, and lifecycle scope `s`:

```text
n = count(e, r, o, s)
N = global_count(r, o, s)

raw_selectivity =
  ln((N + α) / (n + α)) / ln(N + α)
```

Clamp the result to:

```text
0.0..1.0
```

Use smoothing:

```text
α = 1.0 initially
```

Use smooth fanout policy rather than category cliffs:

```text
base_budget = relation_policy.max_fanout
specificity_factor = raw_selectivity ^ gamma
support_factor =
  1.0
  + semantic_support
  + thread_support
  + temporal_support
  + salience_support
  + currentness_support
  + correction_support
  + explicit_scope_support

fanout_budget =
  clamp(
    floor(base_budget * specificity_factor * support_factor),
    relation_policy.min_fanout,
    relation_policy.max_fanout
  )
```

Diagnostic labels such as `selective`, `broad`, or `very broad` may be derived for display, but they must not be persisted as entity state or become the core fanout mechanism.

## Implementation Impact

- Add counters to `RetrievalStatsStore`.
- Add selectivity scoring policy with configurable smoothing and gamma.
- Add relation/object-specific fanout budgets.
- Add retrieval rationale fields for selectivity score, supporting signals, chosen fanout, expanded count, included count, and rejected count.
- Ensure stats missing/unhealthy produces conservative fanout.

## Considered Options

1. Persist entity selectivity categories.
2. Use hard-coded fanout limits by entity type.
3. Compute continuous selectivity from counters and use smooth fanout policy.

## Decision Outcome

Chosen option: **3. Compute continuous selectivity from counters and use smooth fanout policy**.

This provides bounded expansion without brittle entity categories or identity-specific exceptions.

## Consequences

### Positive

- Selectivity adapts as the graph grows.
- Entity broadness is relation-specific rather than global.
- Broad entities can still be useful when supporting evidence exists.
- Diagnostics can show numeric inputs and policy decisions.

### Negative / Tradeoffs

- Requires policy tuning.
- Requires stats health management.
- Formula may need revision after observing real retrieval distributions.

## Validation

- Increasing entity count while holding global count constant must not increase selectivity.
- Increasing supporting evidence may increase fanout but never above relation-specific caps.
- Low-selectivity entity evidence alone cannot flood the context pack.
- High-selectivity entity evidence contributes meaningfully to retrieval.
- Diagnostic labels are derived only and are not persisted as entity state.

## Revisit When

Revisit the formula if diagnostics show poor correlation between selectivity score and retrieval quality, or if score changes produce unstable context packs. The next step should be adjusting formula/configuration, not persisting categories.
