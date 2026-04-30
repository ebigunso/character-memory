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

# ADR-D-0005: Use DerivedMemory subtypes in v0.1 before a normalized belief ontology

## Context and Problem Statement

The long-term roadmap includes richer factual rigor: assertions, claims, evidence links, belief assessments, source assessments, temporal validity, and volatility. That model is valuable, but implementing it immediately would make v0.1 larger and more truth-maintenance-oriented than necessary for the starter Character Memory use case.

## Decision Drivers

- Keep the first implementation small enough to build and test.
- Prioritize episode-backed continuity over full epistemic modeling.
- Avoid premature public API commitments around a complex belief ontology.
- Preserve migration paths for future factual rigor.

## Decision

v0.1 uses `DerivedMemory` with a `derived_type` field for records such as:

```text
reflection
user_preference
assistant_preference
commitment
open_loop
character_signal
relationship_note
project_note
claim
correction
```

First-class `Assertion`, `Claim`, `EvidenceLink`, `BeliefAssessment`, and `SourceAssessment` objects are deferred to later roadmap stages.

## Character Memory Relevance

This protects YAGNI. The starter version should make the assistant feel continuous through episodes, threads, reflections, commitments, and preferences. Full belief tracking is useful, but it should not define the first implementation.

## Implementation Impact

- `DerivedMemory(derived_type = "claim")` may represent simple factual claims in v0.1.
- Provenance, supersession, confidence, salience, and retention should already be included so migration is possible.
- Future belief objects should be able to derive from or replace selected `DerivedMemory` records.

## Considered Options

1. Implement the full belief ontology immediately.
2. Never implement the belief ontology.
3. Use `DerivedMemory` subtypes now and split specialized objects later.

## Decision Outcome

Chosen option: **3. Use `DerivedMemory` subtypes now and split specialized objects later**.

This gives the starter implementation enough expressive power without overbuilding.

## Consequences

### Positive

- Faster v0.1 implementation.
- Smaller public API surface.
- Easier to adapt as the project learns from use.

### Negative / Tradeoffs

- Factual claims in v0.1 are less rigorously modeled than they will be later.
- Some future migrations may need to split `DerivedMemory` records into more specific objects.

## Validation

- v0.1 tests should cover at least reflection, user preference, open loop, commitment, character signal, claim, and correction derived types.
- The schema should include enough provenance and supersession metadata to migrate selected records later.

## Revisit When

Revisit when factual correction, source credibility, temporal validity, or contradiction handling become common enough to justify first-class belief objects.
