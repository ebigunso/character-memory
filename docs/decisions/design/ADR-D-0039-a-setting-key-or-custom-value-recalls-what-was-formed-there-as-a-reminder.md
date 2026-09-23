---
status: proposed
adr_type: design
date: 2026-09-23
deciders: ["ebigunso"]
consulted: ["GPT-6"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0022-recall-is-activation-by-scene-cues.md, ADR-D-0024-continuity-is-scoped-and-the-scope-is-derived-from-the-scene.md]
---

# ADR-D-0039: A setting key or custom value recalls what was formed there as a reminder

## Context and Problem Statement

A scene can identify a familiar place or an application-defined context exactly. Memories formed there can come to mind again, but sharing that context does not make them about the people present or the work in progress. The fork is whether certainty about the context grants those memories the force of knowing a subject, or reminds the character of what was formed there.

## Decision

A setting key or custom value identifies a context of formation and recalls memories formed there as reminders. A memory resting on experiences from several contexts can be reminded of in any of them. The context match alone does not give a memory the force of a known subject or open connected history. Another cue can independently reach the same memory on its own terms.

## Why

Knowing which room the character is in does not make everything learned in that room pertinent to the encounter. Treating the context as knowledge about a subject lets a habitual place crowd out what the character holds about the people it is meeting; treating it as a reminder preserves what the place can bring without making that claim.

## Rejected Alternatives

- Give a setting key or custom value the force of a known subject because the key matches exactly: rejected outright; certainty about where a memory was formed does not establish what it is about.
- Omit recall by setting keys and custom values: rejected because a familiar context can remind the character of something that has no topical cue; reopen if representative use shows that this road contributes no useful reminders.

## Decision Boundary

Invariant: a setting key or custom value cues the context in which a memory was formed, with reminder reach; it does not establish aboutness or independently open connected history. A memory's other recall roads retain their own reach.

Not covered: key representation, candidate ordering, scores, reservation sizes, how reminders are worded for a model, or the derivation of subject and activity scopes.

## Validation

- A memory formed in several contexts can be recalled by each; a context match alone does not open unrelated history.
- Adding a familiar setting does not remove what is held about the people present by giving place memories the force of known subjects.
- A memory also reached through its subject, activity or topic keeps that road's reach.

## Revisit When

A consumer needs an application-defined context to identify an active subject rather than a context of formation, and cannot express that distinction through a participant or activity cue.

## More Information

- [ADR-D-0022](ADR-D-0022-recall-is-activation-by-scene-cues.md) provides recall by scene cues; [ADR-D-0024](ADR-D-0024-continuity-is-scoped-and-the-scope-is-derived-from-the-scene.md) provides scene-derived scopes. This record depends on both and replaces neither.
- Load-bearing decisions log, ruling 26 (2026-09-21: presence is not aboutness) and ruling 59 (2026-09-22: a place brings the context of formation), carried into the [consolidation plan](../../coding-agent/plans/active/v0-2-consolidation-plan.md).
