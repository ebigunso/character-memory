---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0020-memory-is-first-person.md, ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md, ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md]
---

# ADR-D-0027: Every span the character was present for is accounted for in durable memory, and absence is known as absence

## Context and Problem Statement

On an ordinary day the honest result of reflecting is very little, and it is tempting to let an uneventful stretch produce nothing at all. A character that stores nothing for a quiet afternoon cannot tell that afternoon from a week it was switched off. Those are different facts about its own life, "I was there and no one came" and "I was not there", and elapsed-time awareness is dishonest without the difference. The fork is whether "nothing happened" may mean "nothing recorded".

## Decision

The character's timeline has no unexplained holes. Every span in which it was present is accounted for by something durable, an episode or the day's gist, even when all that can be said is that the span was quiet. "Nothing" may describe interpretation and never the record. A span with no trace at all means the character was absent, and recall can say so.

Presence is reported mechanically by scene boundaries, which need no content, and consolidation turns an empty span into a line of the day's gist.

Unconsolidated trace is never dropped (ADR-D-0026), so a span the character was present for is always covered: by trace still awaiting consolidation, or by the durable account consolidation wrote from it. A span the application excluded from memory is accounted for as well: the character was present, and what happened was withheld at the application's request. Only a span with neither trace nor account means absence.

## Why

Memory is first-person, and a first-person history that cannot distinguish rest from non-existence is not a history of a continuing subject. Accounting for presence costs a line per quiet span and buys an honest answer to "what happened while I was away" in both of its meanings.

## Rejected Alternatives

- Recording nothing for uneventful spans: rejected outright for the reason above.
- A heartbeat record at a fixed interval: rejected because presence is a span with a beginning and an end, which scene boundaries already give; a heartbeat adds volume and no information.
- Recording absence as a durable object: rejected because absence is the lack of any account and is derivable; reopen only if a deployment needs to remember why it was absent, which is an ordinary episode on its return.

## Decision Boundary

Invariant: every span of presence is covered, by trace awaiting consolidation or by a durable account of what happened or of the fact that it was withheld by exclusion; a span with neither means absence; consolidation never drops a span for being empty.

Not covered: how scene boundaries are reported, how quiet spans are summarized or grouped within the day's gist, and how recall phrases absence.

## Validation

- A scenario with a quiet present span and an absent span of equal length shows a durable account for the first, none for the second, and recall distinguishing them.
- Consolidation over an empty span commits an account and releases the boundary entries.
- A present span that has not yet been consolidated is recognized as present from its trace, and recall distinguishes remembered, withheld, and absent.

## Revisit When

A deployment's notion of presence cannot be expressed as spans, for example a character that is always partially attending to many places at once.

## More Information

- ADR-D-0020 makes the character the subject of its own memory.
- The continuity situation catalog's F3 and the elapsed-time situations describe the target behavior.
