---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "Without this record, future work would likely model the remembering character as a special role, an assistant entity type or an implicit subject outside the graph, and treat memory as a chronicle of what happened to others."
  detected_signals: "A cross-boundary contract on entity identity between the application and the library; a rejected alternative likely to be re-proposed; a decider's ruling setting a durable governance default."
  cost_of_violation: "Role-typed entities and an implicit subject bake user-and-assistant assumptions into the schema, which the philosophy forbids and which is costly to migrate once stores exist; a character with no self in its graph cannot answer what it did, verify claims about its own past, or hold commitments it made."
  cost_of_over_extension: "Reading this record as requiring every store to hold exactly one character forever would block a simulation that migrates or merges characters; the record fixes the subject per store, not the lifecycle of stores."
depends_on: [ADR-D-0009-entity-neutral-retrieval-policy.md, ADR-D-0011-scope-continuity-around-arbitrary-entities-and-contexts.md]
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-D-0020: Memory is first-person, and the remembering character is an ordinary entity

## Context and Problem Statement

The philosophy forbids hard-coded roles in the core library and rejects third-person archive framing, but it does not say whose memory a store holds. The domain carries entity types for a user and an assistant, and design drafts refer to an assistant-self entity by convention. Nothing establishes that the character is an entity in its own graph, that its own actions are episodes, or that its commitments are its own.

Independent-entity deployments make the gap concrete. A character with a life of its own must answer what it did yesterday regardless of who asks, keep projects between interactions with anyone, dispatch work to other agents and remember what it asked of whom, and verify what others tell it about its own past. None of that is possible if the subject of memory is implicit or is a role.

## Decision Drivers

- Entity-neutral schema: no core type encodes an application role.
- Autobiographical continuity requires the character to be a participant in its own episodes.
- Verifying claims about itself requires a first-person record.
- Commitments and open loops need a party they belong to.

## Decision

Memory is first-person. A memory store has one remembering subject, and that subject is an ordinary entity in its own graph. Its actions are episodes it participated in, its promises are its commitments, and its history persists between interactions with anyone.

The identity of the self is supplied by the application, at construction or per scope. No object type, entity type, or retrieval path treats the self as a special role. Entity types that encode application roles are not core schema truth.

## Character Memory Relevance

A character is accumulated through remembered experience, and experience has a subject. Making that subject an ordinary entity keeps the schema neutral while giving the character a self to remember from: what it did, what it promised, what it was told about itself. Other people are entities in the character's life, not owners of its memory.

## Implementation Impact

- The application names the self entity when it constructs a memory or opens a scope; the library records episodes it participated in like any other participant.
- The user and assistant entity types are deletion candidates in favor of a person type plus the application-declared self.
- Open loops and commitments carry an actor and a counterpart, so the character can owe and be owed.
- Rendering may present the character's own episodes in the first person.

## Considered Options

1. An implicit subject: the store is "the assistant's" memory and the assistant appears in no episode.
2. A role-typed self: an assistant entity type marks the subject.
3. An ordinary entity named by the application as the self.

## Decision Outcome

Chosen option: **3. An ordinary entity named by the application**. It is the only option that satisfies both entity-neutrality and autobiographical continuity: the self is fully represented without any type knowing what a self is.

### Rejected Alternatives

An implicit subject is rejected outright. It cannot represent the character's own actions, cannot hold its commitments as its own, and turns memory into a chronicle of others.

A role-typed self is rejected because it hard-codes an application role into the schema, which ADR-D-0009 and the philosophy forbid. It would be reopened only if a retrieval policy were shown to require knowing the self by type rather than by the identity the application supplied, and no such policy has been identified.

## Consequences

- Positive: autobiographical situations, delegation to other agents, and self-verification all become ordinary graph queries.
- Positive: the schema loses its last role-typed assumptions.
- Negative / tradeoffs: applications must declare the self, which simple single-assistant products may find an extra step; a convenience default may name it for them.
- Negative / tradeoffs: existing entity types for user and assistant are removed, which touches drafts and fixtures.

## Decision Boundary

Invariant: the self is an entity the application identifies, and no core type or retrieval path distinguishes it by kind.

Not covered: whether the self is declared at construction, per scope, or both; the convenience default for single-character applications; the migration of the removed entity types.

## Validation

- Tests show the self appearing as a participant in its own episodes and as the actor of its commitments, with no special handling in retrieval.
- Schema review rejects any entity type or field that encodes an application role.
- Evaluation scenarios for the independent-entity situations assert first-person answers regardless of the asking party.

## Revisit When

A deployment requires several remembering subjects to share one store with distinct first-person views, which would reopen the one-subject-per-store premise rather than the ordinary-entity decision.

## More Information

- ADR-D-0009 establishes entity-neutral retrieval policy.
- ADR-D-0011 establishes scope continuity around arbitrary entities.
- The continuity situation catalog's independent-entity situations describe the target behavior.
