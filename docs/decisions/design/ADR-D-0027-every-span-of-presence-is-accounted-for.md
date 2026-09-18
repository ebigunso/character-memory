---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0020-memory-is-first-person.md, ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md]
---

# ADR-D-0027: Every span the character was present for is accounted for in durable memory, and absence is known as absence

## Context and Problem Statement

On an ordinary day the honest result of reflecting is very little, and it is tempting to let an uneventful stretch produce nothing at all. A character that stores nothing for a quiet afternoon cannot tell that afternoon from a week it was switched off. Those are different facts about its own life, "I was there and no one came" and "I was not there", and elapsed-time awareness is dishonest without the difference. The fork is whether "nothing happened" may mean "nothing recorded".

## Decision

The character's timeline has no unexplained holes. Every span in which it was present is accounted for by something durable, an episode or the day's gist, even when all that can be said is that the span was quiet. "Nothing" may describe interpretation and never the record. A span with no trace at all means the character was absent, and recall can say so.

Presence is reported mechanically by scene boundaries, which need no content, and consolidation turns an empty span into a line of the day's gist.

Trace that must be dropped before it was consolidated, because the short-term store reached its horizon or its ceiling, is never dropped silently. The library itself writes a durable account that the character was present for that span and that what happened was not kept, through the validated path, with the span and its scene and no interpretation. A span the application excluded from memory is accounted for the same way: the character was present, and what happened was withheld at the application's request. A present span therefore always has an account, of what happened, of the fact that it was lost, or of the fact that it was withheld, and only a span with no account at all means absence. A character can then say "I was there, and I cannot recall it", which is different from both a quiet afternoon and not having been there.

## Why

Memory is first-person, and a first-person history that cannot distinguish rest from non-existence is not a history of a continuing subject. Accounting for presence costs a line per quiet span and buys an honest answer to "what happened while I was away" in both of its meanings.

## Rejected Alternatives

- Recording nothing for uneventful spans: rejected outright for the reason above.
- A heartbeat record at a fixed interval: rejected because presence is a span with a beginning and an end, which scene boundaries already give; a heartbeat adds volume and no information.
- Refusing new writes when the short-term store is full, to protect unconsolidated trace: rejected because refusing new experience to protect old loses more and blocks the application; the accumulation signal rises well before the bound, and a loss is recorded rather than prevented.
- Recording absence as a durable object: rejected because absence is the lack of any account and is derivable; reopen only if a deployment needs to remember why it was absent, which is an ordinary episode on its return.

## Decision Boundary

Invariant: every span of presence is covered by a durable account, of what happened, of the fact that its trace was lost unconsolidated, or of the fact that it was withheld by exclusion; unconsolidated trace is never dropped without that account; an uncovered span means absence; consolidation never drops a span for being empty.

Not covered: how scene boundaries are reported, how quiet spans are summarized or grouped within the day's gist, and how recall phrases absence.

## Validation

- A scenario with a quiet present span and an absent span of equal length shows a durable account for the first, none for the second, and recall distinguishing them.
- Consolidation over an empty span commits an account and releases the boundary entries.
- When the store drops entries that were never consolidated, a durable account of the lost span exists afterward, and recall distinguishes remembered, lost, and absent.

## Revisit When

A deployment's notion of presence cannot be expressed as spans, for example a character that is always partially attending to many places at once.

## More Information

- ADR-D-0020 makes the character the subject of its own memory.
- The continuity situation catalog's F3 and the elapsed-time situations describe the target behavior.
