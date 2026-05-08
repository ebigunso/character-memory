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

# ADR-D-0009: Keep core retrieval policy entity-neutral

## Context and Problem Statement

Character Memory is intended for personal assistants, companions, games, simulations, research systems, and developer tools. If the core library hard-codes special handling for `user`, `assistant`, `player`, `protagonist`, `NPC`, or any other application role, the memory model becomes narrower than the philosophy allows.

The problem: how should the core library treat entities that are important in different application domains?

## Decision Drivers

- Preserve use-case agnosticism in the core memory model.
- Keep entities first-class continuity anchors without assuming which roles matter.
- Avoid personal-assistant-specific assumptions leaking into games, simulations, companions, or research systems.
- Let accumulated graph structure influence retrieval behavior.
- Allow applications to provide scopes or policy hints above the core library.

## Decision

The core library must not hard-code retrieval behavior based on entity names, canonical keys, or application roles.

Retrieval behavior may depend on:

```text
observed graph structure
relation kind
object type
lifecycle/currentness
time
salience
retrieval scope
supporting evidence
application-provided scope
```

but not on assumptions like:

```text
this is the user, therefore always important
this is the assistant, therefore always broad
this is an NPC, therefore lower priority
this is a protagonist, therefore expand more
```

## Character Memory Relevance

Character Memory should support persistent AI characters and assistants across many product shapes. Entity-neutral retrieval protects the project from becoming only a user-profile assistant memory library.

A character can accumulate memory around any recurring entity: person, place, project, object, faction, scene, organization, or concept. The core library should preserve that flexibility.

## Implementation Impact

- Retrieval tests must include high-degree entities across multiple entity types, not only personal-assistant examples.
- Retrieval rules must not check hard-coded entity names, canonical keys, or app roles.
- Applications may provide `ContinuityScope` or custom policy inputs, but those are inputs, not baked-in core assumptions.
- Diagnostics should report entity type and relation behavior without using identity-specific exceptions.

## Considered Options

1. Special-case common assistant roles such as user and assistant.
2. Allow configurable application role special-casing inside the core library.
3. Keep the core library entity-neutral and let applications provide scope/policy hints.

## Decision Outcome

Chosen option: **3. Keep the core library entity-neutral and let applications provide scope/policy hints**.

This best preserves Character Memory's intended breadth while still allowing applications to adapt behavior through explicit scope and configuration.

## Consequences

### Positive

- Keeps the library usable beyond personal assistants.
- Avoids hard-coded assumptions that would be difficult to remove later.
- Encourages retrieval policy to use graph evidence rather than role identity.
- Makes tests more robust by requiring heterogeneous fixtures.

### Negative / Tradeoffs

- Applications may need to pass explicit scope when domain roles matter.
- Some personal-assistant defaults may require adapter-level convenience rather than core shortcuts.
- Retrieval policy must be explained through relation/scoping behavior rather than simple role rules.

## Validation

- Add tests proving no retrieval rule depends on entity name, canonical key, or application role.
- Add synthetic fixtures for high-degree person, place, project, topic, object, and arbitrary custom entity.
- Review retrieval code paths for identity-specific branches.
- Ensure `ContinuityScope` can carry application-provided scope without changing core entity semantics.

## Revisit When

Revisit only if the project intentionally creates an application-specific layer above the core library. The core library should remain entity-neutral.

## More Information

Related roadmap section: v0.1.2 continuous entity selectivity and retrieval guardrails.
