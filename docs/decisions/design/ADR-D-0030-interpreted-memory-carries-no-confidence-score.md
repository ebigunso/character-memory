---
status: accepted
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md, ADR-D-0020-memory-is-first-person.md, ADR-D-0028-interpreted-memory-carries-its-evidence.md]
---

# ADR-D-0030: Interpreted memory carries no confidence score; how firmly it is held is read from how it was formed

## Context and Problem Statement

Interpreted memory has carried a confidence number, and the number means two different things. At write time it is how sure the producer was, which from a language model is a guess about its own output. Assessed later it is how well the memory has held up, which changes with every new piece of evidence, so a stored value is stale as soon as it is written. Beneath the ambiguity is a framing borrowed from agents that cite sources and answer for facts. A character's memory is not a fact. It is a first-person interpretation of what happened (ADR-D-0020): "Bob says he is vegetarian" is true as an account whether or not Bob is. The fork is whether a memory says how much to trust it with a score, or with the account of how it came to be held.

## Decision

No interpreted memory carries a confidence score. How firmly something is held is read, when it is needed, from how the memory was formed and what is linked to it: whether it was said plainly or inferred, heard once or many times, the character's own words or someone's claim about them, contradicted or corrected since (ADR-D-0028). The reader hedges from reasons, which is how people hedge, and nothing is ranked or resolved by a number standing in for them.

## Why

A score answers a question nobody asks of their own memory. "How sure am I?" is answered by remembering how one came to know, and a system that records that has no use for a number that must be either guessed or perpetually recomputed.

## Rejected Alternatives

- A confidence supplied by the model that writes the memory: rejected outright; it is not evidence.
- A confidence the library computes from evidence and stores at write time: rejected because it is an answer to the later question stored at the earlier moment, and is stale at the next piece of evidence; what it would summarize is already recorded and can be read when needed.
- Ranking or resolving conflicts by confidence: rejected because it hides what is uncertain but important, and because contradictions are held, not resolved (ADR-D-0028).

## Decision Boundary

Invariant: no interpreted memory carries a confidence score; how firmly a memory is held is derived at read time from its recorded basis and linked evidence.

Not covered: the confidence recorded on thread membership and other links, which the generation phase's plan examines under the same reasoning; importance, which judges what matters and not what is true, and stays as ADR-D-0018 has it; how a rendered pack words a hedge.

## Validation

- Schema and API review find no confidence field on interpreted memory.
- A memory formed from one overheard remark and one formed from a plain repeated statement are hedged differently at the behavioral tier, from their recorded basis alone.

## Revisit When

A consumer needs to answer for the truth of what the character remembers, as a source-citing agent does, in a way that reading the basis at recall cannot serve, or measurement shows that hedging from the recorded basis is unreliable in practice.

## More Information

- ADR-D-0028 defines the basis a memory records.
- ADR-D-0018 keeps importance as a weight on activation that changes only by a write.
