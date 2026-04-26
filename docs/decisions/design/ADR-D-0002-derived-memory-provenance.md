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

# ADR-D-0002: Require provenance for behavior-influencing derived memories

## Context and Problem Statement

Derived memories such as preferences, reflections, commitments, open loops, and character signals can influence future assistant behavior. If these records can be created without a source episode or observation, they become arbitrary persona patches rather than accumulated character memory.

## Decision Drivers

- A character should be accumulated through remembered experience, not assigned by ungrounded labels.
- Developers need to inspect where a behavior-influencing memory came from.
- Corrections must be able to find and revise the memories affected by a source episode.
- Future reflection and belief-tracking layers need stable provenance paths.

## Decision

Every `DerivedMemory` with `can_use_for_generation = true` or equivalent behavior-influencing status must trace back to at least one `Episode` or `Observation`.

This applies to at least these derived types:

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

## Character Memory Relevance

This is the starter invariant for the whole project:

```text
Every behavior-influencing DerivedMemory must be traceable back to remembered experience.
```

Without this, the system can still store facts, but it no longer supports accumulated character.

## Implementation Impact

- `DerivedMemory` should include `derived_from_episode_ids` and/or `derived_from_observation_ids`.
- Insertion should fail, warn, or mark the memory unusable for generation if provenance is absent.
- Retrieval formatting should include source pointers when derived memories are used in context.

## Considered Options

1. Allow free-floating derived memories.
2. Require provenance only for factual claims.
3. Require provenance for all behavior-influencing derived memories.

## Decision Outcome

Chosen option: **3. Require provenance for all behavior-influencing derived memories**.

This best protects character continuity and auditability while still allowing non-behavioral notes to exist as implementation artifacts if needed.

## Consequences

### Positive

- Prevents arbitrary character or preference overwrites.
- Makes correction and inspection possible.
- Creates a clean migration path to future claim/evidence/belief modeling.

### Negative / Tradeoffs

- Some generated summaries or reflections may need to be rejected or marked provisional until source IDs are available.
- Implementers must pass provenance through extraction and reflection pipelines.

## Validation

- Unit tests should reject or downgrade behavior-influencing `DerivedMemory` records without source IDs.
- Retrieval tests should verify that source episode/observation IDs are available for returned derived memories.
- Correction tests should be able to find derived memories affected by a corrected episode or observation.

## Revisit When

Revisit if a future explicit policy allows certain manually authored character settings. Those should be modeled separately from accumulated derived memories.
