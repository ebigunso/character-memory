---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0009-entity-neutral-retrieval-policy.md, ADR-D-0011-scope-continuity-around-arbitrary-entities-and-contexts.md]
---

# ADR-D-0020: Memory is first-person, and the remembering character is an ordinary entity named by the application

## Context and Problem Statement

The philosophy forbids hard-coded roles and rejects third-person archive framing, but nothing says whose memory a store holds. The domain carries entity types for a user and an assistant, and design drafts refer to an assistant-self entity by convention. Independent-entity deployments make the gap concrete: a character with a life of its own must answer what it did yesterday regardless of who asks, keep projects between interactions with anyone, remember what it asked of other agents, and verify what others tell it about its own past. The fork is whether the subject of memory is implicit, a role, or an entity like any other.

## Decision

Memory is first-person. A memory store has one remembering subject, and that subject is an ordinary entity in its own graph. Its actions are episodes it participated in, its promises are its commitments, and its history persists between interactions with anyone.

The identity of the self is supplied by the application, at construction or per scope. No object type, entity type, or retrieval path treats the self as a special role. Entity types that encode application roles are not core schema truth. Open loops and commitments carry an actor and a counterpart, so the character can owe and be owed.

## Why

Character is accumulated through remembered experience, and experience has a subject. Making the subject an ordinary entity gives the character a self to remember from, including what it did, what it promised, and what it was told about itself, while keeping every type in the schema ignorant of what a self is.

## Rejected Alternatives

- An implicit subject, where the store is "the assistant's" memory and the assistant appears in no episode: rejected outright; it cannot represent the character's own actions or hold its commitments as its own, and it turns memory into a chronicle of others.
- A role-typed self marked by an assistant entity type: rejected because it hard-codes an application role into the schema, which ADR-D-0009 forbids; reopen only if a retrieval policy is shown to require knowing the self by type rather than by the identity the application supplied.

## Decision Boundary

Invariant: the self is an entity the application identifies, and no core type or retrieval path distinguishes it by kind.

Not covered: whether the self is declared at construction, per scope, or both; a convenience default for single-character applications; the migration away from the user and assistant entity types.

## Validation

- Tests show the self appearing as a participant in its own episodes and as the actor of its commitments, with no special handling in retrieval.
- Schema review rejects any entity type or field that encodes an application role.
- Evaluation scenarios for the independent-entity situations assert first-person answers regardless of the asking party.

## Revisit When

A deployment requires several remembering subjects to share one store with distinct first-person views, which reopens the one-subject-per-store premise rather than the ordinary-entity decision.

## More Information

- ADR-D-0009 establishes entity-neutral retrieval policy.
- ADR-D-0011 establishes scope continuity around arbitrary entities.
- The continuity situation catalog's independent-entity situations describe the target behavior.
