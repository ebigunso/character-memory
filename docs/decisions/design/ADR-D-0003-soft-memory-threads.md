---
status: proposed
adr_type: design
date: 2026-04-26
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0003: Treat memory threads as soft continuity overlays

## Context and Problem Statement

A memory thread is useful for representing ongoing projects, recurring topics, relationship arcs, unresolved tensions, and repeated themes. However, if threads are modeled as hard chat containers, the system becomes too chat-centric and brittle. Real continuity is often fuzzy and many-to-many.

## Decision Drivers

- Support ongoing continuity without requiring every episode to fit one clean thread.
- Avoid overfitting the model to chat-thread UI structures.
- Allow an episode to contribute to multiple projects, preferences, or relationship arcs.
- Leave room for future non-chat modalities without redesigning thread membership.

## Decision

`MemoryThread` is a soft continuity overlay.

Thread membership is:

```text
optional
many-to-many
confidence-scored
revisable
represented as a MemoryLink or equivalent edge
```

An episode must not be required to have exactly one thread.

## Character Memory Relevance

Character continuity is not always organized by explicit conversation threads. It may emerge from repeated cues, projects, tensions, corrections, and commitments. Soft threads preserve this flexibility while still giving retrieval a strong continuity anchor.

## Implementation Impact

- Do not add a mandatory `thread_id` field to `Episode` as the only thread representation.
- Use links such as `part_of_thread` with optional confidence and rationale.
- Retrieval should function when no thread exists, and should support multiple thread links when present.

## Considered Options

1. Every episode belongs to exactly one thread.
2. A thread is just an external chat/session ID.
3. A thread is an optional, soft continuity structure.

## Decision Outcome

Chosen option: **3. A thread is an optional, soft continuity structure**.

This preserves the value of threads without making the starter schema rigid or chat-only.

## Consequences

### Positive

- Better models long-running projects and relationship arcs.
- Supports ambiguous or evolving thread assignment.
- Avoids forcing artificial structure onto every episode.

### Negative / Tradeoffs

- Retrieval and summarization must handle uncertain and multiple memberships.
- Thread summaries may require periodic cleanup or consolidation.

## Validation

- Tests should cover episodes with zero, one, and multiple thread links.
- Tests should cover thread membership confidence and rationale if implemented.
- Retrieval tests should confirm that active thread matches boost relevance but are not required.

## Revisit When

Revisit if most applications only use explicit chat threads and soft membership proves unnecessary overhead.
