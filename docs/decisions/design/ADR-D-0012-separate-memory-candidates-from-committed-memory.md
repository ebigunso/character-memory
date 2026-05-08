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

# ADR-D-0012: Separate memory candidates from committed memory

## Context and Problem Statement

Future assisted generation will eventually produce memory candidates from raw conversation or transcript-like input. If generated candidates are treated as committed memory immediately, the system risks false continuity, unsupported preferences, incorrect commitments, and ungrounded character signals.

The library also needs a way for manual caller-provided writes and future generated writes to share the same validation and commit path.

## Decision Drivers

- Generated material should not become behavior-influencing memory before validation.
- Manual and future generated writes should share one safe persistence path.
- Applications should be able to inspect, review, or reject candidates before commit.

## Decision

Character Memory will distinguish memory candidates from committed memory.

A candidate is draft material. It may describe an `Episode`, `Observation`, `Entity`, `MemoryThread`, `DerivedMemory`, `MemoryLink`, vector index record, or stats update, but it is not behavior-influencing memory until it passes validation and is committed.

The common workflow is:

```text
MemoryCandidate
  -> RememberWritePlan
  -> validation
  -> commit
```

## Character Memory Relevance

This protects the principle that character is accumulated from remembered experience rather than assigned by arbitrary generated interpretations.

A generated reflection, preference, commitment, or character signal must not become behavior-influencing memory merely because a processor proposed it.

## Implementation Impact

The write path should expose candidate and plan types that are inspectable before commit. `remember()` can remain a convenience API, but it should use the same candidate, validation, and commit path internally.

## Considered Options

1. Treat generated candidates as committed memory immediately.
2. Keep manual writes and generated writes on separate persistence paths.
3. Separate candidate material from committed memory and require shared validation before commit.

## Decision Outcome

Chosen option: **3. Separate candidate material from committed memory and require shared validation before commit**.

This keeps future assisted generation from bypassing the core memory invariants and preserves a direct manual path through the same machinery.

## Consequences

### Positive

- Future assisted generation can be added without bypassing validation.
- Manual and generated writes can share one safe commit path.
- Applications can inspect, review, or reject candidate writes before persistence.
- Behavior-influencing `DerivedMemory` remains provenance-gated.

### Negative / Tradeoffs

- The write path becomes more complex than direct object insertion.
- Applications may need to understand the difference between candidate, plan, and committed object.
- Some simple use cases may prefer the convenience wrapper `remember()`.

## Validation

- Behavior-influencing `DerivedMemory` candidates without `Episode` or `Observation` provenance must fail validation.
- Invalid candidates must not be committed.
- `commit()` must revalidate a plan before persistence.
- Tests should cover candidate creation, validation failure, and successful commit.

## Revisit When

Revisit during the assisted remember workflow and memory candidate generation phase if candidate states need to expand to support accepted, deferred, review-needed, rejected, and invalid states.

## More Information

- Related roadmap phase: remember intake interfaces and deterministic write planning.
- Future roadmap phase: assisted remember workflow and memory candidate generation.
