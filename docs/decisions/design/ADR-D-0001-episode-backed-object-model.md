---
status: accepted
adr_type: design
date: 2026-04-26
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0001: Use an episode-backed object model

## Context and Problem Statement

Character Memory is meant to preserve continuity of experience, not merely store searchable text. A flat memory record model makes implementation simple, but it blurs the distinction between what happened, what was observed, what was inferred, and how the memory should affect future behavior.

## Decision Drivers

- Preserve episodic continuity as the substrate of character memory.
- Avoid collapsing raw interaction history into isolated facts or summaries.
- Keep the starter model small enough for v0.1 while leaving room for later evidence/belief modeling.
- Support retrieval by time, entity, thread, and derived meaning.

## Decision

The v0.1 domain model will be built around these core objects:

```text
Episode
Observation
Entity
MemoryThread
DerivedMemory
MemoryLink
```

Flat memory records are not the canonical domain model for the implementation.

## Character Memory Relevance

This decision protects the project from drifting into generic RAG. Character continuity depends on remembering events and their later interpretations. The system must be able to say not only “this text is relevant,” but also “this happened in this episode, involved these entities, contributed to this thread, and produced this derived memory.”

## Implementation Impact

- Storage, graph mapping, and vector payloads should use object type fields such as `episode`, `observation`, `derived_memory`, `thread`, and `entity`.
- Storage, retrieval, and public APIs should not require callers to model memory as only `episodic | semantic`.
- `DerivedMemory` represents behavior-influencing interpretations while richer claim/belief objects remain future work.

## Considered Options

1. Use a single flat memory record model with `memory_type = episodic | semantic`.
2. Implement the full future ontology immediately: `Assertion`, `Claim`, `EvidenceLink`, `BeliefAssessment`, etc.
3. Use the v0.1 episode-backed object model.

## Decision Outcome

Chosen option: **3. Use the v0.1 episode-backed object model**.

It preserves the core Character Memory philosophy without forcing the full future belief ontology into the starter implementation.

## Consequences

### Positive

- Makes episodes the primary substrate instead of an implementation detail.
- Keeps derived memories traceable to lived interaction context.
- Allows memory threads, corrections, commitments, and reflections to be represented naturally.

### Negative / Tradeoffs

- More complex than a single generic memory table.
- Requires more explicit storage and retrieval contracts than a flat record model.

## Validation

- Domain model tests should cover all six core object types.
- Retrieval fixtures should include at least one episode, one observation, one thread, one entity, one derived memory, and links among them.
- No public v0.1 API should require callers to model memory as only `episodic | semantic`.

## Revisit When

Revisit if the object model becomes too heavy for basic applications or if a later version introduces first-class belief objects that need to replace part of `DerivedMemory`.
