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

# ADR-D-0004: Return a ContinuityContextPack instead of a flat retrieval list

## Context and Problem Statement

A flat ranked list of memories is suitable for ordinary RAG, but Character Memory needs to provide situated context for ongoing behavior. The assistant needs to know which memories are episodes, which are preferences, which are open loops, which are active threads, and why they were retrieved.

## Decision Drivers

- Retrieval should support continuity of behavior, not just similarity search.
- Prompt/context integration should distinguish memory categories.
- Developers should be able to inspect retrieval rationale.
- Suppressed, superseded, or deleted memories should be filtered consistently.

## Decision

The primary retrieval output is `ContinuityContextPack`, with typed sections such as:

```text
active_threads
relevant_episodes
relevant_observations
derived_memories
user_preferences
open_loops
commitments
character_signals
relationship_notes
rationale
```

The system may internally use vector top-k and graph expansion, but the public result should not be only a flat top-k list.

## Character Memory Relevance

The purpose of retrieval is to help the assistant behave as a continuing participant. A continuity context pack makes ongoing threads, commitments, preferences, and character signals first-class instead of burying them among generic search results.

## Implementation Impact

- Retrieval APIs should return typed groups or a structure that can be rendered into typed groups.
- Each included item should carry enough metadata for provenance and filtering.
- Ranking may still exist internally, but final assembly should be category-aware.

## Considered Options

1. Return vector top-k only.
2. Return a generic retrieval bundle with graph context.
3. Return a structured `ContinuityContextPack`.

## Decision Outcome

Chosen option: **3. Return a structured `ContinuityContextPack`**.

This keeps retrieval aligned with the product goal instead of exposing backend retrieval mechanics as the main abstraction.

## Consequences

### Positive

- Makes prompt integration clearer and safer.
- Supports open-loop and commitment retrieval directly.
- Improves debuggability through retrieval rationale.

### Negative / Tradeoffs

- More API design work than returning a single list.
- Some callers may still want raw ranked results, which may need to be exposed as optional diagnostics.

## Validation

- Retrieval tests should assert typed sections, not only item counts.
- Tests should cover active thread retrieval, preference retrieval, commitment retrieval, and open-loop retrieval.
- Suppressed and superseded memories should be excluded by default.

## Revisit When

Revisit if application integrations consistently prefer raw retrieval primitives over packaged continuity context.
