---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0015-keep-raw-source-storage-outside-core.md, ADR-D-0021-append-only-memory-record-with-out-of-band-purge.md, ADR-D-0022-recall-is-activation-by-scene-cues.md, ADR-D-0023-purpose-is-never-a-supplied-cue.md, ADR-D-0025-durable-memory-has-one-writer.md]
---

# ADR-D-0026: A short-term store outside core memory holds recent trace until consolidation, and recall reaches it by scene, time, entity, and topic

## Context and Problem Statement

A person remembers this morning this afternoon, by what it was about, before any sleep has reorganized it. What they hold of the day is literal, bound to its circumstances, and directly reachable from a partial cue. A character needs the same, from writes that cost nothing. ADR-D-0015 keeps raw source material out of core memory and says that if raw storage is ever needed it requires its own record and starts outside core. Leaving retention of recent raw text to applications means most will not do it, and the character then has no today. The fork is where the literal trace of recent experience lives.

## Decision

Recent experience is held in a short-term store beside core memory, not inside it. A mechanical write puts there the scene, the raw snippet of what happened, its time, and the application's source pointer if one was given, with no language-model call and no judgment. Indexing is mechanical too, and in its default configuration a write calls no model of any kind; an index that needs an embedding is the consumer's choice and interprets nothing. Scene boundaries are written the same way, with no content required.

Exclusion is applied at the mechanical write. For a span the application marks as not to be remembered, the store keeps a marker of the span, that it happened and when, and none of its content or of how the scene was described (ADR-D-0029), so text excluded as it is written is never stored, and so is never indexed, recalled, or shown to a model. If the application excludes a span whose trace is still unconsolidated, the library replaces that entry's content with the same marker and removes it from the index; how that meets a reflection already under way is ADR-I-0035's. Once something has been consolidated, stopping its influence is suppression and removing it is the out-of-band purge.

Trace never expires. An entry stays until consolidation has consumed it, however long that takes, because dropping experience that was never reflected on can only produce poorer memory than was possible. What grows with neglect is the warning: the accumulation signal escalates with volume and age, and past a threshold the library reports loudly, on every write and every recall, that trace has been held longer than it should be. Each entry is bounded in size, and oversize input is refused or visibly truncated. It is indexed when written, so recent trace is reachable by topic as well as by scene, time, and entity; recall reads it through the content, entity, and time routes beside durable memory, and marks what belongs to the current conversation. It never feeds the state route or the stored-intention route, which read interpreted durable memory; surfaced state reaches trace only through the one bounded re-cue hop of ADR-D-0023. Consolidation reads it, writes durable memory through the validated path, and releases what it consumed. Release after consolidation is the only way an entry leaves the store, apart from the out-of-band purge of ADR-D-0021, whose scope includes this store.

It is never core memory: nothing in it enters graph authority or the durable vector store, it is not a memory substrate, and it is reachable only through recall and consolidation, never as a log to be searched or exported.

## Why

Two stores with different jobs is how fast and slow memory are usually described in people: a quick, literal, unconsolidated trace and a slow, interpreted, durable record. Keeping the literal one outside core memory and draining it on consolidation gives the character its day without turning the durable record into a transcript archive, which is the thing ADR-D-0015 protects.

## Rejected Alternatives

- Raw bodies on unconsolidated episodes inside graph authority, kept indefinitely: rejected because it reverses ADR-D-0015 where a separate draining store does not need to.
- Leaving recent raw retention to the application: rejected because a discipline left to consumers will mostly not happen, and then the character has no memory of today.
- Recent trace reachable by time and scene only, never by topic: rejected because a busy day does not fit in a recency floor and a person reaches this morning by what it was about; reopen only if measurement shows the floor suffices.
- A retention horizon or size ceiling that drops unconsolidated trace: rejected because it turns an application's neglect into the character's permanent loss; neglect is answered with a loud, escalating warning instead. The cost is that an application that never reflects accumulates raw text in this store, which the warning exists to make impossible to miss.
- A second graph for the short-term store: rejected because its access is by scene, time, and content, which a keyed, time-ordered, indexed table serves; reopen if a recall route needs traversal within recent trace.

## Decision Boundary

Invariant: recent trace lives outside core memory and drains only by consolidation; unconsolidated trace is never dropped by the library, and overlong retention is reported loudly; it is written mechanically; it is indexed for recall and reached only through recall and consolidation; it never enters graph authority or the durable vector store; it lies within the scope of the out-of-band purge.

Not covered: which index method serves recall best, which is decided by measurement, provided the default write calls no model; the entry bounds; the warning's thresholds and form; the storage engine, and with it who protects the store at rest, keeps one character's trace from another's, and deletes securely, the library or the host application; what the store does as it nears the capacity it was given, short of dropping trace; how the release is made idempotent.

## Validation

- A span excluded as it is written leaves a marker and no content: its text is absent from the store, the index, recall, and every prompt, and a retroactive exclusion of unconsolidated trace leaves the same stored state.
- A mechanical write makes no language-model call, no model call of any kind in the default configuration, and no write to graph authority or the durable vector store.
- A scenario asks by topic about something from earlier the same day before any reflection, and recall returns it marked as recent and unconsolidated.
- After consolidation commits, the consumed entries are gone, and the durable episode carries every source pointer its consolidated entries supplied; an entry written without one consolidates just the same.
- No public operation lists, searches, or exports the store's contents outside recall and consolidation.

## Revisit When

Deployments show the need to reprocess old scenes from source often enough that a draining store is the wrong shape, measurement shows recall over recent trace is not needed by topic, or stores of neglected trace grow in practice to the point where they are raw archives in all but name, which would reopen the boundary with ADR-D-0015.

## More Information

- ADR-D-0015 stays in force: core memory stores no raw source material, and this store is the separate, outside-core arrangement that record anticipates.
- ADR-D-0025 makes this store the only destination of in-the-moment input.
- ADR-D-0023 defines the one bounded re-cue hop through which surfaced state reaches trace.
- The fast-store lookup measurement, including paraphrase, unsegmented scripts such as Japanese, and a changed fact stated in different words, is planned in the public companion evaluation repository `CharacterMemoryEvals`, a development aid and not core library functionality.
