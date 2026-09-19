---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md, ADR-D-0020-memory-is-first-person.md, ADR-D-0028-interpreted-memory-carries-its-evidence.md]
---

# ADR-D-0033: A memory keeps how it felt: feeling is part of the episode, carried in words, and never revised

## Context and Problem Statement

The philosophy names emotional weight as part of why a memory matters and recent emotional tone as something that brings memories back, and the situation catalog asks that a character recall how a topic landed last time and that a hard conversation color the next day. Nothing in what is stored has held any of this: an episode records what happened, and a derived memory records what it means. How something felt cannot be recovered later from the facts, so if it is not kept when the memory is formed it is gone. The fork is whether feeling is part of memory, and if so in what form and with what permanence.

## Decision

An episode keeps how it felt. The gist of an episode is the character's own account (ADR-D-0020) and says, in words, how the episode landed for it. That is a fact about the experience, fixed when the memory is formed, and it is never revised: the character was hurt that evening whatever it later comes to think. A later change of heart is a new memory that stands beside the first, "I was hurt then; I see it differently now", and never overwrites the feeling it reconsiders. What the character makes of things may be superseded. What it felt at the time may not.

How others seemed is kept the same way, as an impression: an observation that is neither something said nor something that happened, attributed to the character as its own perception and grounded like any observation in what gave rise to it. One impression stands as an observation of that moment. A conclusion about the person drawn from impressions is the character's own and must be earned by a pattern, as ADR-D-0028 requires.

Feeling is carried in words and never as a number. It informs the weight consolidation gives an episode, and at recall it works as any content does; it has no recall route of its own.

## Why

How something felt never becomes false, and it is what lets a character approach a sore subject with care and carry yesterday into today, which is most of what makes continuity feel like someone's. Keeping it fixed while letting the character's understanding move is what gives the character an arc: it can say how it felt then and how it sees things now, and both are true.

## Rejected Alternatives

- Leaving feeling to be inferred at recall from what happened: rejected because it cannot be; the same events land differently, and only the moment knew how.
- A numeric valence or intensity on episodes: rejected because a number supplied by a model is not evidence and flattens what the words carry; reopen only if measurement shows recall needs a signal that importance and content do not give.
- Revising an episode's feeling when the character reappraises it: rejected because it erases the past the reappraisal is about, and a character that cannot say "I felt differently then" has no arc.
- A separate store or recall route for emotion: rejected because feeling belongs to the episode it was felt in and already reaches recall as content and as weight.

## Decision Boundary

Invariant: an episode's gist says how it felt to the character; that feeling is never superseded, and a reappraisal is a new memory beside it; an impression is an observation attributed to the character as its own perception, grounded like any other, and supports a conclusion about a person only through a pattern; no memory carries a numeric measure of emotion, and feeling has no recall route of its own.

Not covered: the vocabulary of feelings; how much of a gist is given to feeling; how a rendered pack words it; how feeling enters the judgment of an episode's weight.

## Validation

- A topic that landed badly once is approached with that in mind the next time it comes up, and a hard conversation colors the following day without being mentioned unless invited (the catalog's A3 and D10).
- After the character reconsiders an old hurt, recall returns both how it felt then and how it sees it now, and the earlier feeling is not superseded (the catalog's F23).
- A single impression that someone seemed tired produces no state about that person; repeated impressions across episodes can.
- Schema and API review find no numeric emotion field.

## Revisit When

Measurement shows that feeling carried in the gist's words does not reach recall when the moment calls for it.

## More Information

- ADR-D-0028 defines how an observation is grounded and the bar a conclusion must meet; the impression is a third thing an observation may record, beside something said and something that happened.
- The philosophy's principle that a memory keeps how it felt states the behavior this record gives a place in what is stored.
