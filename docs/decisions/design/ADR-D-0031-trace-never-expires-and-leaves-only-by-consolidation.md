---
status: accepted
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0021-append-only-memory-record-with-out-of-band-purge.md, ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md]
---

# ADR-D-0031: Trace never expires: it leaves the short-term store only by consolidation, and neglect is answered with a warning

## Context and Problem Statement

The short-term store holds the literal trace of recent experience until reflection consolidates it (ADR-D-0026), and the application decides when reflection runs. An application may reflect late, rarely, or never, and the store then grows with raw text it was meant to hold briefly. The ordinary answer is a retention horizon or a size ceiling that drops the oldest entries. The fork is whether the library protects the store's size or the character's experience.

## Decision

Trace never expires. An entry stays in the short-term store until consolidation has consumed it, however long that takes. Release after consolidation is the only way an entry leaves, apart from the out-of-band purge of ADR-D-0021, whose scope includes this store.

Neglect is answered with a warning, never with loss. The library reports how much has accumulated and for how long, the report escalates with volume and age, and past a threshold it is loud: on every write and every recall, to the application and to the developer's log. It is never shown to the character as tiredness, which would perform a limitation the character does not have, and it never reaches an end user.

## Why

Experience that was never reflected on can only become poorer memory if it is dropped, and the loss is permanent and silent. A late reflection still produces good memory from old trace. The cost of the other choice, a store that grows while an application neglects it, is visible, recoverable by reflecting, and made impossible to miss.

## Rejected Alternatives

- A retention horizon that drops unconsolidated trace after a time: rejected because it turns an application's neglect into the character's permanent loss.
- A size ceiling that drops the oldest trace: rejected for the same reason; what a store does when it cannot accept a new write is a different question, and refusing loudly is not dropping.
- A quiet warning, logged once: rejected because a store of raw text that nobody is consolidating must not be easy to overlook.

## Decision Boundary

Invariant: the library never drops unconsolidated trace; an entry leaves the short-term store only by release after consolidation or by the out-of-band purge; overlong retention is reported, escalating with volume and age, loudly past a threshold, to the application and the developer and never to the character or an end user.

Not covered: the warning's thresholds and form; what the store does as it nears the capacity it was given, short of dropping trace; how release is made idempotent.

## Validation

- Trace far older than any warning threshold is still present, still recallable, and consolidates normally when reflection finally runs.
- As trace accumulates and ages the reported level rises, and past the threshold every write and recall outcome carries it.
- No code path removes an unconsolidated entry except release after a committed consolidation and the purge.

## Revisit When

Stores of neglected trace grow in practice to the point where they are raw archives in all but name, which would reopen the boundary with ADR-D-0015.

## More Information

- ADR-D-0027 relies on this: a span the character was present for is always covered, by trace awaiting consolidation or by the durable account written from it.
- ADR-I-0035 leaves the schedule of reflection to the application, which is why neglect is possible at all.
