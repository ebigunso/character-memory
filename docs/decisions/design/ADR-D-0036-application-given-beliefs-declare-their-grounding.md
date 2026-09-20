---
status: accepted
adr_type: design
date: 2026-09-21
deciders: ["ebigunso"]
consulted: ["GPT-6"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0034-an-entity-is-a-notion-the-character-holds.md, ADR-D-0028-interpreted-memory-carries-its-evidence.md]
---

# ADR-D-0036: Application-given beliefs declare their grounding without inventing experience

## Context and Problem Statement

An application may already have a name to give a notion before the character has experienced anything about it. Requiring source experiences for that belief forces the application to invent an episode; simply permitting a missing source makes a deliberate gift indistinguishable from an ungrounded candidate. The distinction must survive persistence and later correction.

## Decision

The application can declare that it gives a belief about a notion. This declaration is durable grounding in place of source experiences, accepted only when the belief has at least one notion subject and cites no source experiences. A belief with neither sources nor this declaration is rejected. Given grounding does not itself assert a name or any other predicate, and does not make the belief immune to supersession or suppression.

A replacement declares its own application-given grounding; that declaration is never inferred from the belief it supersedes.

## Why

A deliberate application contribution is a real basis for memory, but it is not an experience the character had. Recording that distinction allows an initial understanding without manufacturing evidence or weakening the source floor for other memories.

## Rejected Alternatives

- Invent an episode to stand for the application's contribution: rejected outright because it records an experience that did not occur.
- A free-text source string for an application-given belief: rejected outright because nothing would read the string; a declaration records the grounding distinction.
- Admit source-less beliefs without declared grounding: rejected outright because a missing source could no longer be distinguished from intentional input.
- Infer a replacement's grounding from its predecessor: rejected outright because revising a belief does not establish the replacement's basis.
- Extend given grounding to memories with no notion subject: reopen when a concrete application-authored memory requires that broader source-free admission and its boundary is decided.

## Decision Boundary

Invariant: application-given grounding is explicit and durable, substitutes for source experiences only on a belief about a notion, cannot coexist with source experiences, and is declared anew by a replacement.

Not covered: the marker's field shape, draft constructors, correction-origin provenance, graph encoding, or other forms of caller-declared grounding addressed by generation work.

## Validation

- A notion and an application-given naming belief can be committed before any episode or observation exists.
- Admission rejects a given belief with sources, a given belief with no notion subject, and a source-less belief with no given declaration.
- Correcting a given belief without source experiences requires an explicitly given replacement; supersession preserves the original belief as history.

## Revisit When

A supported kind of application-authored memory needs source-free grounding beyond a belief about a notion, or a consumer needs to distinguish multiple application-given bases.

## More Information

- This record closes the application-given name half of ADR-D-0034's open item on how an identity key or a name given by the application enters as the first belief. The identity-key half remains open under ADR-D-0029 and ADR-I-0020.
- It fixes one form of the declared grounding for each kind of candidate left open by ADR-D-0028: an application-given belief about a notion. Other forms remain with that record's generation-phase work.
