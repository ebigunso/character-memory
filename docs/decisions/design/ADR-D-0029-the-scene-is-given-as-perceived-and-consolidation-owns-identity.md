---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0019-discretion-is-disclosure-not-recall.md, ADR-D-0022-recall-is-activation-by-scene-cues.md, ADR-D-0024-continuity-is-scoped-and-the-scope-is-derived-from-the-scene.md, ADR-D-0025-durable-memory-has-one-writer.md, ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md, ADR-D-0028-interpreted-memory-carries-its-evidence.md, ../implementation/ADR-I-0020-restart-identity-via-caller-supplied-ids-not-a-lookup-surface.md]
---

# ADR-D-0029: The scene is given as it is perceived, in descriptions with keys where they exist, and consistency of identity belongs to consolidation

## Context and Problem Statement

Every memory carries its scene and recall takes the present scene as its cue (ADR-D-0019, ADR-D-0022), so the application has to say who is here, where, and what is going on, on every write and every recall. The plainest deployment has nothing to say it with: a character present in a room hears everything through one microphone, as a person hears through their ears, and holds no account identifier for anyone and no name for where it stands. Places in particular have no canonical name and no fixed grain: "my house" is a legitimate place, "the kitchen of my house" is sometimes the one that matters, and the division is fuzzy and changes. If the scene must be identifiers, this deployment can supply almost none and the little it supplies starves reflection of what it needs to judge the situation well. If the scene is free text that names things, slight drifts in wording mint duplicates, and the application is left to work out which stored entity to reuse. The fork is what the application is asked to supply and who keeps identity consistent.

## Decision

The application gives the scene as the character would perceive it, and never resolves, normalizes, or looks anything up. Only the time is required. Who is present, where, what is going on, and anything else about the situation are each given as whatever the application has: a description in words, a label from its own perception such as a recognized voice, a key it already owns, any combination, or nothing. A key is a strengthening where one exists and is never assumed. The description may come from the application's perception, its configuration, or the character's own model noting its situation, and a change of situation is written as it happens, so a description is as fine and as changeable as the moment is.

A description is never an identity. On trace it is stored as given, indexed with the entry's text, and handed to reflection as evidence about the situation, which may be wrong as any perception may. Nothing is created in durable memory at the mechanical write or at recall (ADR-D-0025). Because a description can name the very thing that was to be withheld, it is content where exclusion is concerned and goes when the span it describes is excluded.

Consistency of identity belongs to consolidation. Reflection reads the people, places, and things already known before it writes, and resolves each reference through graph authority to an existing entity, a proposed new one, or, where it is unclear, a separate entity with a possible-same link. Mechanical matching is exact and never fuzzy; whether "Bobby" is "Bob" is judgment, and in doubt the answer is separate and linked, because a duplicate is recoverable and a wrong merge is not. Recall follows a possible-same link, so what is held under a duplicate is still reached. Places are held the same way and may contain one another, the kitchen within the house: a memory attaches to the most specific place its evidence supports, and recall at the wider place reaches the narrower. The scene on a durable memory therefore has two layers, what was given and what consolidation resolved, each part recording which it is, and scope (ADR-D-0024) is derived from the resolved layer. Nothing in this is circular: reflection takes its trace mechanically, by the stretch it was written in, before anything is resolved.

Keys keep the meaning earlier records gave them. A key for a person, place, or thing is the caller-supplied identity of ADR-I-0020, so the same key reaches the same entity with no lookup surface. A key for a context that is no entity, a conversation, a channel, a game zone, is the scope value ADR-D-0024 already takes and never becomes an entity.

The scene given at recall takes the same references. A key cues what it identifies; an exact name cues every entity that bears it; a description is a content cue over the entities and scenes memory holds. An ambiguous reference activates each thing it could mean, an unknown one activates nothing, and the retrieval trace reports which happened. Before reflection has run, a reference reaches the day's trace through the words the entries and their scenes carry.

## Why

What is known about a situation mostly exists only in the moment, and reflection later sees only what was written down, so asking for less than the application perceives makes every later judgment poorer. Asking for more than it perceives, a stable identifier for a person heard across a room, a canonical name for a corner of a house, makes the application guess, and a guess recorded as identity is the wording-drift duplicate or the wrong merge. Putting consistency where a model already reads memory before writing it is the one place it can be done with context, once, by the single writer of durable memory.

## Rejected Alternatives

- Scene fields as application-owned identifiers, with identity derived from them: rejected as the default because the plainest deployment, a character in a room with one microphone, has none; kept as an optional strengthening where a platform supplies them.
- Scene fields restricted to what the application obtained without reading content, names and configured labels only: rejected because it drops most of what is known about the situation, and places and activities often have no label to give.
- The application resolves references itself through a lookup surface: rejected because ADR-I-0020 declines that surface, and because resolution needs the context only consolidation reads.
- Fuzzy mechanical matching of names and places at write time: rejected because it merges on a guess, and a wrong merge cannot be undone under an append-only record.
- A fixed hierarchy or vocabulary of places: rejected because the grain a moment needs is not known in advance and the division changes.

## Decision Boundary

Invariant: only the time is required of a scene; every other part is given as perceived, as a description, a perception label, a key, any combination, or nothing; a description is never an identity, the application never resolves one, and it is content where exclusion is concerned; nothing durable is created at the mechanical write or at recall; references are resolved at consolidation through graph authority to existing, new, or separate-with-possible-same, matching mechanically only on exact forms and never merging on a guess; recall follows possible-same and containment links; a durable memory's scene records what was given and what was resolved, each marked as such; an identity key is a caller-supplied identity under ADR-I-0020 and a setting key is a scope value under ADR-D-0024.

Not covered: the field shapes of a scene and of a reference; how a description cue is matched, which is measured; the normalization used for exact matching; the form of the possible-same and containment links and how their evidence is checked (ADR-D-0028); how reflection gathers stretches into a pass; how entities asserted to be the same are merged, and how a place's subdivision changes over time, which belong to the entity evolution work.

## Validation

- A deployment that supplies only the time, the entries, and free descriptions of the situation writes trace, recalls the same day by those descriptions, and consolidates into entities without the application supplying or storing any identifier.
- The same person and the same place described in drifting words across many sessions consolidate into one entity each, or into separate entities joined by a possible-same link, and recall by any of the wordings reaches the memories held under all of them.
- A memory formed in a narrower place is reached by recall at the place that contains it.
- An ambiguous reference at recall activates each candidate and the retrieval trace says it was ambiguous; an unknown one activates none and the trace says so.
- After a span is excluded, a search for a word that appeared only in its scene description finds nothing.
- Where an identity key is given, repeated writes under changed display names reach one entity, and no duplicate is created.

## Revisit When

Measurement shows description cues reach the right people and places too rarely for recall to arrive carrying what the moment calls for, or duplicates accumulate faster than possible-same links keep them reachable.

## More Information

- The v0.2 draft's scene section and the v0.3 draft's mechanical write state the field shapes and the guide's advice to consumers: write what the character would perceive about its situation now, in whatever detail is available, and add keys where they exist.
- ADR-D-0028 covers who said what within a scene: the character's own output is known, and everything perceived is attributed by judgment.
