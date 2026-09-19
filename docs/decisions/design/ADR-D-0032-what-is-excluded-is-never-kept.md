---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0019-discretion-is-disclosure-not-recall.md, ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md, ADR-D-0021-append-only-memory-record-with-out-of-band-purge.md, ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md, ADR-D-0027-every-span-of-presence-is-accounted-for.md]
---

# ADR-D-0032: What the application asks not to be remembered is never kept: exclusion is applied at the write and leaves only a marker

## Context and Problem Statement

Some of what a character is present for must not be remembered: someone says "off the record", or the application's own policy withholds a stretch. Recall is never gated by sensitivity (ADR-D-0019), so what is stored will be recalled, and the durable record is append-only (ADR-D-0021), so what is consolidated cannot simply be deleted. If the application just skips the write, the character's timeline gains a hole it cannot tell from absence (ADR-D-0027). The fork is where in the life of a memory "do not remember this" takes effect.

## Decision

Exclusion takes effect at the mechanical write. For a span the application marks as not to be remembered, the short-term store keeps a marker, that the span happened and when, and none of its content or of how its scene was described, since a description can name the very thing that was to be withheld. Text excluded as it is written is never stored, and so is never indexed, recalled, consolidated, or shown to a model. The marker keeps the timeline honest: the character was present, and what happened was withheld at the application's request.

An application may also exclude a span after writing it, while its trace is still unconsolidated. The store then holds exactly what it would have held had the span been excluded at the write. Once something has been consolidated, exclusion no longer applies to it: stopping its influence is suppression (ADR-D-0018), and removing it is the out-of-band purge (ADR-D-0021).

## Why

The only text that cannot leak, be recalled at the wrong moment, or colour a later judgment is text that was never kept. Applying exclusion at the one door all experience enters by makes it a property of the store and not a filter every later reader must remember to apply, and the marker costs nothing that was asked to be withheld.

## Rejected Alternatives

- Storing the span and filtering it at recall: rejected because the text is still held, still reaches reflection, and is one missed filter from disclosure.
- Leaving exclusion to the application, which simply does not write the span: rejected because the character then cannot tell a withheld hour from an hour it did not exist.
- Deleting from durable memory on request: rejected as a memory operation by ADR-D-0021; that is the purge, which makes no pretense of preserving continuity.

## Decision Boundary

Invariant: a span excluded at the write leaves a marker of that it happened and when, and nothing of its content or its scene's description; such text is never stored, indexed, recalled, consolidated, or sent to a model; a retroactive exclusion of unconsolidated trace leaves the same stored state; after consolidation, suppression and purge are the only recourse.

Not covered: how an application marks a span; what the marker carries beyond the time, such as keys the application chooses to give; how a retroactive exclusion meets a reflection already under way, which ADR-I-0035 decides; how the durable account of a withheld span is worded.

## Validation

- After a span is excluded at the write, its text and any word that appeared only in its scene description are absent from the store, the index, recall, and every prompt.
- A retroactive exclusion of unconsolidated trace leaves the same stored state as exclusion at the write.
- Recall distinguishes a span that was remembered, one that was withheld, and one the character was absent for.

## Revisit When

A deployment needs a withheld span to stay recoverable under some authority, which would be a different feature, a sealed store, and not a weaker exclusion.

## More Information

- ADR-D-0027 accounts for a withheld span as presence.
- ADR-D-0029 is why a scene's description counts as content here.
