---
status: accepted
adr_type: design
date: 2026-09-20
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md, ../superseded/ADR-D-0019-discretion-is-disclosure-not-recall--superseded-by-ADR-D-0038.md, ADR-D-0021-append-only-memory-record-with-out-of-band-purge.md, ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md]
---

# ADR-D-0032: Memory has no exclusion: nothing the character was present for is withheld from it at the library's door

## Context and Problem Statement

Once everything the character experiences is written as trace (ADR-D-0026), the question arises of what to do with what should not be remembered: someone says "off the record", an application's policy covers a stretch, a card number is read aloud. An exclusion operation was designed for it: the application marks a span, the store keeps a marker and none of the content, and the span is accounted for as withheld. It reached far, into presence accounting, into what a scene description may carry, into how a reflection in flight must yield. The fork is whether the library offers a way to keep experience out of memory.

## Decision

The library offers no exclusion. Every case it was meant for already has a better answer.

What someone asks not to be repeated is remembered, together with the asking. A person told something in confidence still knows it, and knowing that it was told in confidence is what shapes how they act when it comes to mind (ADR-D-0019). A character that erased the moment would lose both the confidence and the reason for its discretion.

What the application must never hold, a card number, a credential, a category its policy forbids, the application removes before it writes. The entry is still written, with the secret gone or with a plain note that a private exchange took place, so the text is never stored and the character's timeline has no hole. This is the application's own handling of its input and needs nothing from the library.

What has been written and must go is the out-of-band purge (ADR-D-0021), whose scope includes the short-term store. What should stop influencing the character without going is suppression (ADR-D-0018).

## Why

A cheap way to drop experience, open to an application or to the model running on it, invites dropping whatever is awkward, and awkward moments are often the ones a character is made of. The three legitimate needs differ in kind, discretion, input hygiene, and removal, and an operation that serves all three serves each badly: it erased what discretion needs kept, duplicated what the application can do before the write, and offered a casual path to what ADR-D-0021 deliberately makes deliberate.

## Rejected Alternatives

- An exclusion applied at the mechanical write, leaving a marker and none of the content: rejected for the reasons above. It was drafted and withdrawn before it was accepted, after it had required a third state in presence accounting, a rule that scene descriptions count as content, and a protocol for an exclusion racing a reflection, none of which anything else needed.
- A retroactive exclusion of unconsolidated trace, lighter than a purge: rejected because a light way to remove experience is the risk itself; the purge already reaches the short-term store.
- Letting the character's model exclude what it judges should not be kept: rejected outright; it gives whoever is talking to the character a way to make it forget.

## Decision Boundary

Invariant: no memory operation keeps what the character was present for out of memory; what must not be held is removed by the application before the write, and the entry is still written; what was asked not to be repeated is remembered with the request; removal is the out-of-band purge, and stopping influence is suppression.

Not covered: how an application recognizes and redacts what it must not hold; how the guide words the advice to redact and never to skip the write; the purge tool's design.

## Validation

- API review finds no operation that marks a span as not to be remembered or removes trace outside consolidation and the purge.
- Told something "off the record", the character later shows that it knows it and that it was asked not to repeat it, and does not repeat it unprompted (the behavioral tier).
- An entry written with a secret redacted consolidates normally, and the span it belongs to is covered.

## Revisit When

A deployment is found whose obligations cannot be met by redaction before the write and purge afterward, for example a regulator requiring that the memory system itself enforce a withholding the application cannot be trusted to make.

## More Information

- ADR-D-0019 and the v0.2 design of requested forgetting as a change in how a memory is treated cover the conversational cases.
- ADR-D-0027 accounts for presence with two states, covered and absent; no third, withheld, state exists.
