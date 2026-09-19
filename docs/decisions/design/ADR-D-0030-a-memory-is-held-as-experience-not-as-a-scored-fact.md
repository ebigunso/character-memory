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

# ADR-D-0030: A memory is held as experience: it keeps how it felt and how it came to be held, and carries no score of how true it is

## Context and Problem Statement

Interpreted memory has carried a confidence number, and the number means two different things. At write time it is how sure the producer was, which from a language model is a guess about its own output. Assessed later it is how well the memory has held up, which changes with every new piece of evidence, so a stored value is stale as soon as it is written. Beneath the ambiguity is a framing borrowed from agents that cite sources and answer for facts. A character's memory is not a fact. It is a first-person interpretation of what happened (ADR-D-0020): "Bob says he is vegetarian" is true as an account whether or not Bob is. What shapes how a person acts on a memory is how they came to hold it, and how it felt. The fork is whether a memory describes itself with scores, or with its basis and its feeling.

## Decision

No interpreted memory carries a confidence score. How firmly something is held is read, when it is needed, from how the memory was formed and what is linked to it: whether it was said plainly or inferred, heard once or many times, the character's own words or someone's claim about them, contradicted or corrected since (ADR-D-0028). The reader hedges from reasons, which is how people hedge, and nothing is ranked or resolved by a number standing in for them.

An episode keeps how it felt. The gist of an episode is the character's own account and says, in words, how the episode landed for it. That is a fact about the experience, fixed when the memory is formed, and it is never revised: the character was hurt that evening whatever it later comes to think. A later change of heart is a new memory that stands beside the first, "I was hurt then; I see it differently now", and never overwrites the feeling it reconsiders. What may be superseded is what the character makes of things. What it felt at the time may not.

How others seemed is kept the same way, as an impression: an observation that is neither something said nor something that happened, attributed to the character as its own perception and grounded like any observation in what gave rise to it. One impression stands as an observation of that moment. A conclusion about the person drawn from impressions is the character's own and must be earned by a pattern, as ADR-D-0028 requires.

Feeling is carried in words. No memory carries a numeric measure of emotion, for the reason it carries no confidence. Feeling informs the weight consolidation gives an episode, and at recall it works as any content does; it has no recall route of its own.

## Why

A score answers a question nobody asks of their own memory. "How sure am I?" is answered by remembering how one came to know, and a system that records that has no use for a number that must be either guessed or perpetually recomputed. How something felt is the opposite case: it cannot be recovered later from the facts, it never becomes false, and it is what lets a character recall how a topic landed last time and let a hard conversation color the next day, which is most of what makes continuity feel like someone's.

## Rejected Alternatives

- A confidence supplied by the model that writes the memory: rejected outright; it is not evidence.
- A confidence the library computes from evidence and stores at write time: rejected because it is an answer to the later question stored at the earlier moment, and is stale at the next piece of evidence; what it would summarize is already recorded and can be read when needed.
- A numeric valence or intensity on episodes: rejected for the same reasons as confidence, and because a number flattens what the words carry; reopen only if measurement shows recall needs a signal that importance and content do not give.
- Revising an episode's feeling when the character reappraises it: rejected because it erases the past the reappraisal is about, and a character that cannot say "I felt differently then" has no arc.
- A separate store or recall route for emotion: rejected because feeling belongs to the episode it was felt in and already reaches recall as content and as weight.

## Decision Boundary

Invariant: no interpreted memory carries a confidence score or a numeric measure of emotion; how firmly a memory is held is derived at read time from its recorded basis and linked evidence; an episode's gist says how it felt to the character; that feeling is never superseded, and a reappraisal is a new memory beside it; an impression is an observation attributed to the character as its own perception, grounded like any other, and supports a conclusion about a person only through a pattern.

Not covered: the confidence recorded on thread membership and other links, which the generation phase's plan examines under the same reasoning; importance, which judges what matters and not what is true, and stays as ADR-D-0018 has it; how a rendered pack words a hedge or a feeling; the vocabulary of feelings.

## Validation

- Schema and API review find no confidence field and no numeric emotion field on interpreted memory.
- A memory formed from one overheard remark and one formed from a plain repeated statement are hedged differently at the behavioral tier, from their recorded basis alone.
- A topic that landed badly once is approached with that in mind the next time it comes up, and a hard conversation colors the following day without being mentioned unless invited (the catalog's A3 and D10).
- After the character reconsiders an old hurt, recall returns both how it felt then and how it sees it now, and the earlier feeling is not superseded.
- A single impression that someone seemed tired produces no state about that person; repeated impressions across episodes can.

## Revisit When

A consumer needs to answer for the truth of what the character remembers, as a source-citing agent does, in a way that reading the basis at recall cannot serve, or measurement shows that hedging from the recorded basis is unreliable in practice.

## More Information

- ADR-D-0028 defines the basis a memory records and the bar a conclusion must meet.
- ADR-D-0018 keeps importance as a weight on activation that changes only by a write.
- The philosophy already names emotional weight as part of salience and recent emotional tone as a cue; this record gives them a place in what is stored.
