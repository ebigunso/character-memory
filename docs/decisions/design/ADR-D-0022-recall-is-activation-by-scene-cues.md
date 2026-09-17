---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0004-continuity-context-pack.md, ADR-D-0019-discretion-is-disclosure-not-recall.md, ../implementation/ADR-I-0016-use-retrieval-intent-as-query-time-policy.md]
---

# ADR-D-0022: Recall is activation by the cues of the present scene, with a candidate route and an admission floor per cue kind

## Context and Problem Statement

Retrieval seeds from one route, the content of the current query, and expands through the graph from there. Most of what a person carries into a moment has no topic yet: who is here, what was last said with them, what is owed, what is in progress, what fell due, what happened this morning. A walk through an ordinary day across companions, workers, characters, and independent entities found that recall is driven by several kinds of cue, the people present, the place, the time and date, the activity in progress, a stored intention's trigger, and the topic, and that a single-route pipeline leaves most of the day unreachable except by faking a query. The fork is whether recall stays one route with a ranking signal for time, or becomes activation by whatever cues the scene supplies.

## Decision

Recall is activation by the cues of the present scene: who is present, where, when, what is in progress, and the topic of the current turn. Each cue kind has its own way of finding candidates, since nothing scores every memory, and its own admission floor in the pack, so that no cue kind can starve another. A scene with no topic is an ordinary retrieval whose content route is empty. Importance and recency weight activation; currency selects the version of any state that surfaces and never selects or removes items.

## Why

A character that arrives at a moment carrying only what the topic retrieves is a new instance with notes, which is the failure the library exists to remove. A ranking signal cannot admit what was never a candidate, so each cue kind must be able to find its own candidates and be guaranteed room for them.

## Rejected Alternatives

- One content route with time as a ranking signal: rejected outright; every situation without a topic stays unreachable.
- A separate current-state call beside retrieval: rejected because it is the same activation with an empty content route, and two surfaces would drift; reopen only if measurement shows the two need budgets that one retrieval cannot express.
- A retrieval-mode enumeration (project, relationship, follow-up, and so on): rejected outright; the scene's cues are the modes, and an enumeration would hard-code application situations into the library.

## Decision Boundary

Invariant: retrieval takes the present scene and activates candidates through a route per cue kind with a floor per route; currency never selects items.

Not covered: the cue kinds' candidate mechanisms, the floors and weights, which are measured defaults, the scene's field names, and the trace's shape for activation.

## Validation

- With a scene and no topic, retrieval returns the people present's current state and last interaction, active loops and commitments in both directions, the activity's thread, items due, and date matches.
- Under a loud topic, the state and time routes still admit their floor.
- A stored intention surfaces on its trigger without any topical cue.
- The catalog's section D situations at the retrieval tier.

## Revisit When

Measurement shows a cue kind that never admits anything useful at its floor, which retires that route rather than the rule, or a deployment class whose recall needs a cue kind the scene cannot carry.

## More Information

- ADR-D-0004 defines the pack the activated candidates fill.
- ADR-D-0019 defines the scene as the carrier of discretion.
- ADR-D-0023 keeps purpose out of the cues.
- ADR-I-0016 keeps retrieval intent a query-time policy, which this record does not change.
- The continuity situation catalog, section D, is the behavioral standard this record serves.
