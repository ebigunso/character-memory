---
status: accepted
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0015-keep-raw-source-storage-outside-core.md, ADR-D-0022-recall-is-activation-by-scene-cues.md, ADR-D-0023-purpose-is-never-a-supplied-cue.md, ADR-D-0025-experience-enters-durable-memory-by-one-path.md]
---

# ADR-D-0026: A short-term store outside core memory holds recent trace until consolidation, and recall reaches it by scene, time, entity, and topic

## Context and Problem Statement

A person remembers this morning this afternoon, by what it was about, before any sleep has reorganized it. What they hold of the day is literal, bound to its circumstances, and directly reachable from a partial cue. A character needs the same, from writes that cost nothing. ADR-D-0015 keeps raw source material out of core memory and says that if raw storage is ever needed it requires its own record and starts outside core. Leaving retention of recent raw text to applications means most will not do it, and the character then has no today. The fork is where the literal trace of recent experience lives.

## Decision

Recent experience is held in a short-term store beside core memory, not inside it. A mechanical write puts there the scene, the raw snippet of what happened, its time, and the application's source pointer if one was given, with no language-model call and no judgment. Indexing is mechanical too, and in its default configuration a write calls no model of any kind; an index that needs an embedding is the consumer's choice and interprets nothing. Scene boundaries are written the same way, with no content required.

Recall reaches the store. Because trace is indexed when it is written, this morning is reachable by what it was about as well as by scene, time, and entity; recall reads it beside durable memory and marks what is recent and unconsolidated. The routes that read interpreted memory, how things stand and what was intended, never read trace; what they surface reaches trace only through the one bounded re-cue hop of ADR-D-0023.

Consolidation drains it. Reflection reads trace, writes durable memory through the validated path (ADR-D-0025), and releases what it consumed.

It is never core memory: nothing in it enters graph authority or the durable vector store, it is not a memory substrate, and it is reachable only through recall and consolidation, never as a log to be searched or exported.

## Why

Two stores with different jobs is how fast and slow memory are usually described in people: a quick, literal, unconsolidated trace and a slow, interpreted, durable record. Keeping the literal one outside core memory and draining it on consolidation gives the character its day without turning the durable record into a transcript archive, which is the thing ADR-D-0015 protects.

## Rejected Alternatives

- Raw bodies on unconsolidated episodes inside graph authority: rejected because it reverses ADR-D-0015 where a separate draining store does not need to.
- Leaving recent raw retention to the application: rejected because a discipline left to consumers will mostly not happen, and then the character has no memory of today.
- Recent trace reachable by time and scene only, never by topic: rejected because a busy day does not fit in a recency floor and a person reaches this morning by what it was about; reopen only if measurement shows the floor suffices.
- A second graph for the short-term store: rejected because its access is by scene, time, and content, which a keyed, time-ordered, indexed table serves; reopen if a recall route needs traversal within recent trace.

## Decision Boundary

Invariant: recent trace lives outside core memory; it is written mechanically, with no language-model call; it is indexed for recall and reached only through recall and consolidation; it never enters graph authority or the durable vector store; consolidation drains it.

Not covered: how long trace is held, which ADR-D-0031 decides; what happens to a span the application asks not to be remembered, which ADR-D-0032 decides; which index method serves recall best, which is decided by measurement, provided the default write calls no model; the entry bounds; the storage engine, and with it who protects the store at rest and keeps one character's trace from another's, the library or the host application.

## Validation

- A mechanical write makes no language-model call, no model call of any kind in the default configuration, and no write to graph authority or the durable vector store.
- A scenario asks by topic about something from earlier the same day before any reflection, and recall returns it marked as recent and unconsolidated.
- After consolidation commits, the consumed entries are gone, and the durable episode carries every source pointer its consolidated entries supplied; an entry written without one consolidates just the same.
- No public operation lists, searches, or exports the store's contents outside recall and consolidation.

## Revisit When

Deployments show the need to reprocess old scenes from source often enough that a draining store is the wrong shape, or measurement shows recall over recent trace is not needed by topic.

## More Information

- ADR-D-0015 stays in force: core memory stores no raw source material, and this store is the separate, outside-core arrangement that record anticipates.
- ADR-D-0025 makes this store the only destination of in-the-moment input.
- The fast-store lookup measurement, including paraphrase, unsegmented scripts such as Japanese, and a changed fact stated in different words, is planned in the public companion evaluation repository `CharacterMemoryEvals`, a development aid and not core library functionality.
