---
status: accepted
adr_type: design
date: 2026-09-20
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0009-entity-neutral-retrieval-policy.md, ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md, ADR-D-0020-memory-is-first-person.md, ADR-D-0021-append-only-memory-record-with-out-of-band-purge.md, ADR-D-0028-interpreted-memory-carries-its-evidence.md, ADR-D-0029-the-scene-is-given-as-perceived-and-the-application-never-resolves-it.md]
---

# ADR-D-0034: An entity is a notion the character holds, and what it is, whether it is, and what it is the same as are the character's own beliefs

## Context and Problem Statement

Everything else in memory is the character's interpretation, held with its evidence and revisable by supersession. The entity alone has behaved as a fact: a stored object saying that a thing exists, is one thing, is a person, and is called Bob. Perception is wrong in every one of those ways. What was taken for two turns out to be one, an alias or another face of the same mechanism. What was taken for one turns out to be two. What was believed to exist turns out to have been invented or imagined. An append-only record (ADR-D-0021) cannot repair a wrong fact, which is why entity handling kept acquiring special rules: never merge, link as possibly the same, plan a phase for entity correction. A bare identity with no name does not escape the problem, because it still asserts that something exists and is one thing. The fork is what an entity is a record of.

## Decision

An entity records a notion: that the character came to have someone or something in mind. That is a fact about the character, true from the moment the notion forms and true forever after, as how an episode felt is (ADR-D-0033). It asserts nothing about the world, not that a thing exists, not that it is one thing, not that it differs from any other.

Everything about the world is a belief the character holds about its notions, held like any other interpreted memory, with evidence, attributed to the character, current by supersession (ADR-D-0028): what the notion is called and what kind of thing it is; that two notions are of the same thing; that one notion was really two; that one place lies within another; and the notion's standing, whether anything answers to it at all. A notion carries no standing when it forms. Taking something at face value is the absence of a disbelief, and a character in a story the character was told is believed fictional from the start.

Understanding changes only by adding beliefs. Nothing is ever merged, split, or deleted. Two notions found to be one thing stay two notions joined by a belief, and recall reads through the belief; if the belief proves wrong, one supersession parts them. A notion found to be a conflation stays, as what the character used to take for one thing, beside the new notions and the belief that relates them. A notion found to be fake stays, because the character really did have it in mind and what it experienced really happened; what changes is what the character now makes of it.

A change of understanding must be earned, and costs one belief to accept. That a friend never existed, or that two people are one, is the character's own conclusion and meets the bar ADR-D-0028 sets for conclusions: one person's say-so is held as their claim beside what the character already understands, with the conflict recorded, and the notion's standing moves to doubt before it moves further. A well-founded understanding takes more to overturn than a slight one, because that is what the evidence warrants, and because a character whose world can be rearranged by one remark can be made to disown its friends. How well founded it is, is judgment about the evidence, and not a count of what rests on the notion: a hundred letters that all came through one hand are one source. But reluctance is never bought with cost. Accepting a new understanding is one belief. What hangs on the notion is not rewritten then: it is read through the current belief at recall, where the reader understands "the letters I took to be Lin's" as readily as it understands a newer line that says a task is done. What the character currently holds to be so about the notion, its state, what it owes, what it is owed, is restated first, in bounded reflection passes that the change itself calls for. Episodes are not restated at all: they record what was experienced and how it was understood then, and stay true as that.

An application's deliberate correction is a decision about memory and not a perception, and takes effect as corrections do.

## Why

A notion cannot be wrong, so it never needs the repair an append-only record cannot give, and everything that can be wrong is already the kind of thing this library knows how to hold, doubt, and revise. Separating the bar for changing one's mind from the cost of having changed it keeps what is sound in how people meet unwelcome news, doubt first and conviction in proportion to the evidence, and declines what is not: clinging to a belief because rearranging a life around its loss is expensive.

## Rejected Alternatives

- An entity as a stored fact with a name, a kind, and an identity, corrected by merge and split operations: rejected because every such operation rewrites the record, a wrong one cannot be undone, and the entity it produces is as much a guess as the one it replaced.
- A bare identity with everything else held as belief about it: rejected because it still asserts that a thing exists and is one thing, which is exactly what turns out wrong.
- No stored entity at all, with people and places emerging at recall from descriptions in memories: rejected because nothing stable would exist for a memory to attach to or for recall by person and place to follow, and every recall would re-derive who is who.
- Rewriting everything that rests on a notion when its standing changes: rejected because it makes acceptance expensive in proportion to how much the character knew, which is the pressure toward denial this record declines to build.
- A bar for changing understanding that the library scales by how much rests on the notion: not adopted; how much evidence an established understanding deserves is judgment, and volume is not independence. Reopen if evaluation shows reflection overturns well-founded understanding on thin evidence.

## Decision Boundary

Invariant: an entity asserts only that the character has the notion; its name, kind, standing, sameness with another, composition, and containment are beliefs with evidence, attributed to the character, and current by supersession; no operation merges, splits, or deletes entities; recall reads notions through the beliefs current about them; a change of standing, sameness, or composition meets the bar for the character's own conclusions, and a perceived claim alone never makes it; accepting such a change writes a belief and rewrites nothing that rests on the notion; episodes are never restated for it.

Not covered: the forms these beliefs take and how recall follows them, whose cost is measured; how reflection orders and bounds the restatement that follows a change; how an identity key or a name given by the application enters as the first belief about a notion; the vocabulary of standing; how a rendered pack words a doubted or disbelieved notion.

## Validation

- Schema and API review find no name, kind, or other claim about the world on the entity itself, and no merge, split, or delete operation on entities.
- Two notions later believed to be one are recalled together by either name; when that belief is superseded they are recalled apart, and no memory was rewritten in either direction.
- Told once, by one person, that a long-known friend never existed, the character holds the claim beside what it knows and its behavior toward the friend shows doubt, not reversal; the same from a stranger with a motive changes nothing durable.
- With corroboration, the standing changes by one belief; the next recall already presents the friend's memories under the new understanding; what was owed to the friend is closed as moot in the first reflection pass; the episodes are unchanged.
- A notion believed fake is still recognized by name, with its standing.

## Revisit When

Measurement shows that reading notions through current beliefs makes recall by person or place too slow or too unreliable, or that lazy restatement leaves the character acting on a superseded understanding often enough to matter.

## More Information

- ADR-D-0029 decides what the application supplies; this record decides what becomes of it.
- The continuity situation catalog's F14 and F24 to F26 describe the target behavior: names, the friend who never existed, two names for one person and the reverse, and being told the world is otherwise.
- ADR-D-0028's grounding rules apply to these beliefs as to any interpreted memory, including that a belief relating two notions is grounded in where at least one of them is mentioned.
- The temporal-validity phase's planned work on entity aliases, roles over time, and correction is, under this record, ordinary supersession of beliefs about a notion.
