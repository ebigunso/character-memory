---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-D-0006-supersession-and-suppression--superseded-by-ADR-D-0018.md]
superseded_by: null
depends_on: [ADR-D-0021-append-only-memory-record-with-out-of-band-purge.md]
---

# ADR-D-0018: Recall is complete, and forgetting happens only through suppression or supersession

## Context and Problem Statement

The target is human-comparable memory, and human memory fades: detail goes, gist stays, importance resists. The direct way to build that is decay, a stored importance that falls with time or a reachability that shrinks with age. But a character built on this library acts for people. It is asked favors, given instructions, and dispatched on tasks, usually in its default mode. A person who forgets an instruction answers for it; software answers through its operator, who must be able to rely on complete recall. The fork is whether any fading belongs in the substrate or only in what the character volunteers.

## Decision

Recall is complete. The character can find any memory it holds. No memory becomes less reachable because time passed, and no stored measure of importance, confidence, or stability changes without a write that carries provenance.

Forgetting takes exactly two forms, each intentional, reversible, and inspectable: suppression removes a memory's influence and keeps the record for audit and un-suppression; supersession replaces a memory with a corrected one and keeps the old one as history. Corrections create new records linked to the old ones. Default retrieval excludes suppressed and superseded records unless a caller asks for them.

A memory that is no longer current, such as a finished project or a relationship that ended, is not forgotten. It leaves current views through a change of currency and remains fully eligible for recall. Archival is not a retention state and not a forgetting operation.

Human-shaped fading is a property of ranking and expression: recent and important material is offered in detail, old and minor material as gist, and the whole remains available when it matters. Elapsed time may be a query-time ranking signal. Familiarity and the weight of repeated evidence are derived from provenance at query time, never stored as scores that move, so there is no reinforce operation.

## Why

Every behavior the situation catalog attributes to human forgetting traces to a mechanism that loses nothing: attention at write time, ranking and expression at recall, and the three explicit operations for every case where influence should stop. A memory that faded on its own cannot be found when it is needed and cannot be explained from any trace, which fails both the roles that rely on the character most and the principle that recall is inspectable.

## Rejected Alternatives

- Stored decay of salience or eligibility: rejected outright; it makes the same query return different memories on different days for no reason a trace can show, and it turns a derived policy value into durable graph truth.
- A decaying retrieval mode selectable per query, with complete recall as another mode: rejected because consumer applications run the default mode when interacting with the outside world, including when dispatching tasks; reopen only if a deployment class is identified whose ideal behavior requires loss rather than discretion.
- Overwriting or hard-deleting on correction or forgetting: rejected outright; the record is append-only under ADR-D-0021.
- Archival as a retention state that default retrieval omits: rejected outright; in the default mode it behaves exactly like suppression while promising a retrievability the default path does not deliver, and everything it was meant to express is currency.

## Decision Boundary

Invariant: a memory's retrieval eligibility changes only through suppression or supersession, each a recorded decision with an actor; a change of currency never changes eligibility; no stored importance, confidence, or stability changes without a provenance-carrying write.

Not covered: whether elapsed time is a ranking signal and with what weight; how consolidation summarizes periphery; how expression renders gist versus detail; how currency is represented on threads, derived memories, and relationship state; the lifecycle field names and filter defaults.

## Validation

- Lifecycle tests show a memory's eligibility is unchanged by the passage of time alone and changes only through the two named operations.
- Tests show a thread or derived memory that is no longer current is omitted from current views with that reason and remains admissible to recall on a matching query.
- Retrieval traces name a lifecycle decision for every omitted eligible memory, never an age.
- Correction tests show supersession links from the new record to the old.
- Review rejects any stored score that a maintenance pass updates without provenance.
- Evaluation scenarios for tasks and favors assert recall of instructions across long gaps.

## Revisit When

A deployment class is identified whose ideal behavior requires a memory to become unreachable rather than undisclosed, and discretion at expression demonstrably cannot serve it.

## More Information

- ADR-D-0021 keeps the record append-only and places erasure out of band.
- ADR-D-0019 keeps discretion at disclosure, so that nothing here is read as a reason to hide a memory.
- ADR-I-0008 and ADR-I-0017 establish that derived values are not graph truth and that evidence, not scores, is persisted.
