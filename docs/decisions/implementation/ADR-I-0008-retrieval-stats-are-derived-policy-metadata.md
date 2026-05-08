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

# ADR-I-0008: Treat retrieval stats as derived policy metadata, not graph truth

## Context and Problem Statement

v0.1.2 introduces retrieval statistics to control fanout around high-degree or low-selectivity entities. These stats need to survive app restarts, but they must not become a third source of truth alongside Qdrant and Oxigraph.

The problem: what authority should retrieval statistics have?

## Decision Drivers

- Preserve the existing authority split.
- Avoid using counters as memory truth.
- Keep stats rebuildable from graph authority.
- Support conservative fallback when stats are missing or unhealthy.
- Prevent retrieval correctness from depending on a derived index.

## Decision

Add a `RetrievalStatsStore` for derived counters and selectivity inputs, but treat it as policy metadata only.

The authority split is:

```text
Qdrant:
  vector candidate recall and coarse payload hints

Oxigraph:
  authoritative memory graph, relationships, provenance, lifecycle, currentness, expansion context

RetrievalStatsStore:
  derived counters for selectivity scoring and fanout policy
```

Stats may guide fanout policy. They must not decide:

```text
memory existence
relationship truth
provenance
lifecycle/currentness
suppression/deletion
final context inclusion
```

## Implementation Impact

- Retrieval must still validate final inclusion through Oxigraph.
- Stats can be stale or unhealthy without making memory truth unsafe.
- Stats should be rebuildable from graph authority.
- Missing/unhealthy stats should trigger conservative fanout.
- Diagnostics should report stats health but not repair or override graph state by default.

## Considered Options

1. Store selectivity/counters in Oxigraph and query aggregates during retrieval.
2. Store selectivity/counters in Qdrant payloads.
3. Add a separate derived `RetrievalStatsStore` with no authority over graph truth.

## Decision Outcome

Chosen option: **3. Add a separate derived RetrievalStatsStore with no authority over graph truth**.

This keeps retrieval fast and restart-safe while preserving Oxigraph as the source of graph truth.

## Consequences

### Positive

- Avoids graph-wide aggregate scans during normal retrieval.
- Keeps retrieval fanout policy fast and configurable.
- Preserves the existing Qdrant/Oxigraph authority split.
- Allows stats rebuild and diagnostics without changing memory truth.

### Negative / Tradeoffs

- Adds a third persistence surface.
- Requires consistency checks and health diagnostics.
- Stats update failures must be handled explicitly.

## Validation

- Tests should show Qdrant candidates cannot enter final context without Oxigraph validation even when stats suggest relevance.
- Tests should show missing/unhealthy stats produce conservative fanout.
- Tests should show stats are not used to decide retention/currentness/provenance.
- Rebuild tests should be added when rebuild tooling exists.

## Revisit When

Revisit if future architecture introduces a single transactional store for graph and stats. Until then, graph authority must remain Oxigraph.
