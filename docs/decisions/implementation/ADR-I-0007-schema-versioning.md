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

# ADR-I-0007: Version persisted schemas from the first implementation

## Context and Problem Statement

Character Memory is expected to evolve. The starter schema intentionally leaves room for future belief tracking, richer retention, association graphs, and multimodal support. Without schema versioning, future changes will be risky and hard to migrate.

## Decision Drivers

- Support planned schema evolution without silent breakage.
- Make Qdrant payload and RDF graph migrations explicit.
- Help tests detect old fixture incompatibility.
- Keep future changes compatible with persisted memories where possible.

## Decision

Persist schema version metadata from v0.1 onward.

Examples:

```text
schema_version = "cmem_v0.1"
graph_schema_version = "cmem_graph_v0.1"
qdrant_payload_version = "cmem_qdrant_v0.1"
```

The exact field names may vary, but persisted records should expose enough version metadata to support migration and validation.

## Implementation Impact

- Domain records, Qdrant payloads, and graph mappings should include or be associated with schema versions.
- Fixtures should include schema versions.
- Migration tests should be introduced when versions change.

## Considered Options

1. Add schema versioning only when a migration is needed.
2. Version only the Rust domain model.
3. Version persisted storage schemas from v0.1.

## Decision Outcome

Chosen option: **3. Version persisted storage schemas from v0.1**.

This is low-cost early and high-value later.

## Consequences

### Positive

- Makes future migrations explicit.
- Reduces risk of silent incompatibility.
- Helps storage and graph fixtures remain meaningful.

### Negative / Tradeoffs

- Slight extra metadata in payloads and graph records.
- Requires decisions about migration policy earlier than strictly necessary.

## Validation

- Fixture validation should assert schema version fields.
- Integration tests should fail clearly if unsupported schema versions are loaded.
- Migration notes should be required when schema versions change.

## Revisit When

Revisit if version metadata proves too granular; versions may be consolidated if separate graph/vector/domain versions are unnecessary.
