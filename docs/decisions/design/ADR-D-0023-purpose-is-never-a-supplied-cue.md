---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0022-recall-is-activation-by-scene-cues.md, ../implementation/ADR-I-0013-deterministic-helpers-do-not-infer-high-level-meaning.md]
---

# ADR-D-0023: Purpose is never a supplied cue; it surfaces from memory and re-cues one bounded hop

## Context and Problem Statement

The scene that drives recall carries who, where, when, and what is in progress, and the natural next field is why. Goal-directed recall is real: what someone is trying to do shapes what comes to mind. The fork is whether purpose is an input the application supplies to retrieval, or something that comes up from memory and then shapes recall from inside.

## Decision

The retrieval input carries no purpose, goal, or intent field. What the character is trying to do surfaces from memory, as an open loop, a commitment, a thread in progress, or a character signal, and once surfaced it re-cues one bounded hop: an open loop pulls the counterpart's objections, a thread pulls its last decision. A dispatched task's purpose reaches recall the same way, because it arrives in the interaction as content and is remembered as an episode with an open loop and its rationale.

## Why

A purpose handed in from outside is a persona assigned for one turn; a purpose that comes up from what the character remembers is accumulated character, and letting memory shape behavior means the why must come from memory. The library also cannot infer purpose from text without the high-level inference ADR-I-0013 keeps out, and most consumers cannot supply it reliably, so a field would be filled well only by the applications that need it least.

## Rejected Alternatives

- A goal or intent field on the retrieval input: rejected outright for the reason in Why.
- Inferring purpose inside the library from the current turn: rejected outright under ADR-I-0013.
- Unbounded spreading activation from surfaced purpose: rejected outright; the re-cue is one hop from surfaced state, which keeps it explainable and inside the roadmap's rule against unbounded spreading activation.

## Decision Boundary

Invariant: no retrieval input carries a purpose, goal, or intent; the re-cue from surfaced state is bounded to one hop and recorded in the trace.

Not covered: which state kinds re-cue, their weights, and how the trace names the re-cue.

## Validation

- Schema and API review reject any purpose, goal, or intent field on the retrieval input.
- A trace shows an item admitted because it serves a surfaced open loop, thread, or commitment, and no item admitted through a second hop.
- A dispatched task scenario shows its purpose reaching recall through the open loop written from the interaction.

## Revisit When

A deployment class is found whose ideal behavior needs a purpose the character cannot have remembered, and the interaction cannot carry it as content.

## More Information

- ADR-D-0022 defines activation by scene cues, which this record keeps purpose out of.
- ADR-I-0013 keeps high-level inference out of deterministic helpers.
- The philosophy's section 7 states the persona-versus-character reason at behavior level.
