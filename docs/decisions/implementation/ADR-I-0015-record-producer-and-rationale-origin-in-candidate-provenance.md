---
status: accepted
adr_type: implementation
date: 2026-05-16
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0015: Record producer and rationale origin in candidate provenance

## Context and Problem Statement

v0.1.3 prepares the write path for future generated memory candidates. Once a candidate is committed, it may be difficult to reconstruct whether the candidate and rationale came from the caller, a deterministic helper, a rule processor, a model processor, an import tool, or the system.

The write path needs narrow origin metadata without creating a generic MetaMemory plane.

## Decision

`CandidateProvenance` includes narrow candidate-origin fields.

```rust
enum CandidateProducerKind {
    Caller,
    DeterministicHelper,
    RuleProcessor,
    ModelProcessor,
    ImportTool,
    System,
    Unknown,
}

enum RationaleOrigin {
    ProvidedByCaller,
    ProvidedByProcessor,
    InferredByProcessor,
    Unavailable,
}
```

These fields are write-time provenance. They are not a generic MetaMemory plane and are not added to every committed memory object as generic metadata.

## Consequences

- Generated candidates can share the manual write path while preserving origin clarity.
- Validation can reject inferred rationale represented as caller-provided rationale.
- The committed memory model does not gain generic rationale metadata.

## Validation

- Candidate validation accepts known producer kinds.
- Rationale origin is explicit when rationale text is present.
- Missing rationale can be represented as unavailable.
