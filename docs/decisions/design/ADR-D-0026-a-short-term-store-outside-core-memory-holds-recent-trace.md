---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0015-keep-raw-source-storage-outside-core.md, ADR-D-0021-append-only-memory-record-with-out-of-band-purge.md, ADR-D-0022-recall-is-activation-by-scene-cues.md, ADR-D-0025-durable-memory-has-one-writer.md]
---

# ADR-D-0026: A short-term store outside core memory holds recent trace until consolidation, and recall reaches it by every route

## Context and Problem Statement

A person remembers this morning this afternoon, by what it was about, before any sleep has reorganized it. What they hold of the day is literal, bound to its circumstances, and directly reachable from a partial cue. A character needs the same, from writes that cost nothing. ADR-D-0015 keeps raw source material out of core memory and says that if raw storage is ever needed it requires its own record and starts outside core. Leaving retention of recent raw text to applications means most will not do it, and the character then has no today. The fork is where the literal trace of recent experience lives.

## Decision

Recent experience is held in a short-term store beside core memory, not inside it. A mechanical write puts there the scene, the raw snippet of what happened, its time, and the application's source pointer if one was given, with no model call and no judgment. Scene boundaries are written the same way, with no content required.

Trace never expires. An entry stays until consolidation has consumed it, however long that takes, because dropping experience that was never reflected on can only produce poorer memory than was possible. What grows with neglect is the warning: the accumulation signal escalates with volume and age, and past a threshold the library reports loudly, on every write and every recall, that trace has been held longer than it should be. Each entry is bounded in size, and oversize input is refused or visibly truncated. It is indexed when written, so recent trace is reachable by topic as well as by scene, time, and entity; recall reads it through the same candidate routes as durable memory and marks what belongs to the current conversation. Consolidation reads it, writes durable memory through the validated path, and releases what it consumed. Release after consolidation is the only way an entry leaves the store, apart from the out-of-band purge. The purge path of ADR-D-0021 covers it.

It is never core memory: nothing in it enters graph authority or the durable vector store, it is not a memory substrate, and it is reachable only through recall and consolidation, never as a log to be searched or exported.

## Why

Two stores with different jobs is how fast and slow memory are usually described in people: a quick, literal, short-lived trace and a slow, interpreted, durable record. Keeping the literal one outside core memory and draining it on consolidation gives the character its day without turning the durable record into a transcript archive, which is the thing ADR-D-0015 protects.

## Rejected Alternatives

- Raw bodies on unconsolidated episodes inside graph authority, kept indefinitely: rejected because it reverses ADR-D-0015 where a separate draining store does not need to.
- Leaving recent raw retention to the application: rejected because a discipline left to consumers will mostly not happen, and then the character has no memory of today.
- Recent trace reachable by time and scene only, never by topic: rejected because a busy day does not fit in a recency floor and a person reaches this morning by what it was about; reopen only if measurement shows the floor suffices.
- A retention horizon or size ceiling that drops unconsolidated trace: rejected because it turns an application's neglect into the character's permanent loss; neglect is answered with a loud, escalating warning instead. The cost is that an application that never reflects accumulates raw text in this store, which the warning exists to make impossible to miss.
- A second graph for the short-term store: rejected because its access is by scene, time, and content, which a keyed, time-ordered, indexed table serves; reopen if a recall route needs traversal within recent trace.

## Decision Boundary

Invariant: recent trace lives outside core memory and drains only by consolidation; unconsolidated trace is never dropped by the library, and overlong retention is reported loudly; it is written mechanically; it is indexed for recall and reached only through recall and consolidation; it never enters graph authority or the durable vector store; purge covers it.

Not covered: the index method, lexical, vector, or both, which is decided by measurement; the entry bounds; the warning's thresholds and form; the storage engine; how the release is made idempotent; whether an application may choose to keep consolidated trace longer for reprocessing.

## Validation

- A mechanical write makes no model call and no write to graph authority or the durable vector store.
- A scenario asks by topic about something from earlier the same day before any reflection, and recall returns it marked as recent and unconsolidated.
- After consolidation commits, the consumed entries are gone, and where the application supplied a source pointer the durable episode carries it; an entry written without one consolidates just the same.
- No public operation lists, searches, or exports the store's contents outside recall and consolidation.

## Revisit When

Deployments show the need to reprocess old scenes from source often enough that a draining store is the wrong shape, measurement shows recall over recent trace is not needed by topic, or stores of neglected trace grow in practice to the point where they are raw archives in all but name, which would reopen the boundary with ADR-D-0015.

## More Information

- ADR-D-0015 stays in force: core memory stores no raw source material, and this store is the separate, outside-core arrangement that record anticipates.
- ADR-D-0025 makes this store the only destination of in-the-moment input.
- The fast-store lookup measurement, including paraphrase, unsegmented scripts such as Japanese, and a changed fact stated in different words, is planned in the public companion `CharacterMemoryEvals` repository, a development aid and not core library functionality.
