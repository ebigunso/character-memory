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

# ADR-I-0009: Use SQLite as the default retrieval stats store

## Context and Problem Statement

v0.1.2 needs persistent derived counters for entity selectivity and fanout policy. The stats data is structured, relational, and local to the application in the default deployment shape.

The problem: what should the default stats persistence backend be?

## Decision Drivers

- Keep stats persistent across app restarts.
- Avoid requiring a network database for normal single-process or single-container use.
- Support indexed lookups, composite keys, transactions, and simple aggregate diagnostics.
- Keep the implementation replaceable for multi-replica deployments.
- Avoid turning stats into a general analytics subsystem.

## Decision

Use SQLite as the default `RetrievalStatsStore` implementation.

Recommended implementations:

```text
SqliteRetrievalStatsStore:
  default persistent implementation

InMemoryRetrievalStatsStore:
  deterministic tests and fixtures
```

Possible future implementations:

```text
PostgresRetrievalStatsStore
RedbRetrievalStatsStore
```

The stats store should live with the main app process:

```text
native app:
  local app data directory

single app container:
  mounted persistent volume

multiple app replicas:
  future Postgres adapter instead of sharing SQLite over network storage
```

## Implementation Impact

- Add stats configuration such as `RETRIEVAL_STATS_STORE` and `RETRIEVAL_STATS_PATH`.
- Keep SQLite schema internal.
- Keep `InMemoryRetrievalStatsStore` for tests.
- Do not require Postgres or a NoSQL service for v0.1.2.
- Do not use SQLite stats as graph authority.

## Considered Options

1. Store stats in Oxigraph only.
2. Store stats in Qdrant payloads only.
3. Use SQLite as the default derived stats store.
4. Require Postgres for stats from the start.
5. Use a pure-Rust embedded key-value store from the start.

## Decision Outcome

Chosen option: **3. Use SQLite as the default derived stats store**.

SQLite fits the structured counter workload without requiring another service, while still allowing a future Postgres adapter for multi-replica deployments.

## Consequences

### Positive

- Simple local persistence across restarts.
- Good fit for composite-key counters and diagnostics.
- Avoids requiring network services beyond existing backends.
- Easier to inspect during development.

### Negative / Tradeoffs

- Adds SQLite dependency and operational path configuration.
- SQLite is not appropriate for concurrent writes from multiple app replicas over shared network storage.
- Pure-Rust environments may prefer a future `RedbRetrievalStatsStore`.

## Validation

- Tests should show stats survive app restart.
- Tests should show idempotent ledger behavior prevents duplicate increments.
- Tests should show unhealthy stats trigger conservative fanout.
- Integration docs should describe native and single-container deployment assumptions.

## Revisit When

Revisit SQLite as the default if one of these becomes true:

```text
multiple app instances need concurrent shared writes to the same stats store
deployment policy forbids SQLite/C dependencies
stats workload becomes analytics-heavy rather than counter/update-heavy
the application already requires Postgres for other state
```
