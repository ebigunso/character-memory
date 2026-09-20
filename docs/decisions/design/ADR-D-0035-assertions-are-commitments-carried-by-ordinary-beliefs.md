---
status: accepted
adr_type: design
date: 2026-09-21
deciders: ["ebigunso"]
consulted: ["GPT-6 Astra"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0034-an-entity-is-a-notion-the-character-holds.md, ADR-D-0028-interpreted-memory-carries-its-evidence.md]
---

# ADR-D-0035: Assertions are the character's commitments, carried by ordinary beliefs

## Context and Problem Statement

A notion's name and everything else understood about it are beliefs, but recall sometimes needs to act on a belief without interpreting its prose. Treating every statement about a notion as an assertion would let someone else's claim or the character's doubt direct recall as if the character had accepted it. Requiring a separate kind of memory for every aspect would instead limit what the character can believe to what a schema anticipated.

## Decision

A belief about a notion is ordinary interpreted memory with that notion among its subjects. It can express anything in text, with the same grounding and supersession as other interpreted memory. Where recall needs a mechanical reading, the belief may carry assertions drawn from a closed predicate vocabulary. Each assertion concerns one of that belief's notion subjects and expresses the character's own commitment, never merely another speaker's claim or a possibility under consideration. Doubt and reported claims remain representable without an assertion.

The distinction between a commitment and a reported or tentative belief is an authoring contract. Validation does not check it; structural checks arrive with attribution. As ADR-D-0028 requires, those checks enforce what the evidence can support, never the truth of a judgment.

New predicates enter with a concrete reading behavior that needs them. The vocabulary does not enumerate everything the character might think about a notion.

## Why

Recall must distinguish what the character takes to be so from what it has merely heard or considered. Keeping assertions inside ordinary beliefs gives a mechanical commitment the same evidence and revisability as its surrounding understanding without limiting that understanding to predefined aspects.

## Rejected Alternatives

- A distinct subtype for every aspect of a notion: rejected outright because unanticipated beliefs would require a schema change to exist.
- An entity-to-entity link for a belief relating notions: rejected outright because a link carries no evidence and cannot be superseded; sameness and other relations the character believes need the evidence and revision of an ordinary belief.
- Prose alone for every reading: rejected because it cannot honor an exact-name cue; reopen if evaluation demonstrates that interpreting text at recall provides that deterministic cue without a structured assertion.

## Decision Boundary

Invariant: beliefs about notions remain ordinary interpreted memories; a mechanical assertion belongs to a closed vocabulary, concerns a subject of its containing belief, and expresses the character's commitment; an unasserted belief can carry doubt, hearsay or an aspect with no mechanical reader.

Not covered: predicate names and payloads, serialization, graph representation, name normalization, cue matching and expansion budgets, or how a model earns a commitment from evidence.

## Validation

- An assertion about a notion outside its memory's subjects is rejected, as is a predicate outside the vocabulary.
- A reported name held as text alone does not cue a notion through the exact-name reader.
- Superseding a naming belief changes which name cues the notion while preserving the old belief as history.

## Revisit When

A required recall behavior cannot distinguish commitment from reported or tentative understanding within ordinary interpreted memory.

## More Information

- This record settles the forms of beliefs about notions, the first half of ADR-D-0034's open item on their forms and how recall follows them. The cost of recall reading through those beliefs remains unmeasured.
- ADR-D-0028 supplies the grounding and attribution rules; the attribution checks enforce the structural support for an assertion without judging its truth.
