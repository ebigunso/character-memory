---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0012-separate-memory-candidates-from-committed-memory.md, ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md, ../implementation/ADR-I-0012-use-prepare-validate-commit-write-workflow.md]
---

# ADR-D-0025: Durable memory has one writer, the validated write path, and everything that happens in the moment lands as trace

## Context and Problem Statement

A character must know at once what it just did and what it was just told, and a routine write must cost no language-model call, or applications will not write. The obvious shortcuts all write durable memory early: map a tool result onto the commitment it fulfils, let the character's model note a fact straight into memory, treat a conversation line as a recorded state change. Each needs interpretation to be right, a tool result is unstructured output, a note can be wrong or planted, a line may be jest, and each is a separate way to corrupt memory that lasts. The fork is whether immediacy is bought with early durable writes or with recall over what was just experienced.

## Decision

Durable memory is written only through the validated write path: consolidation by reflection, or a caller that deliberately prepares, validates, and commits a plan. Everything that happens in the moment, a conversation line, a tool result, a note the character's model chose to make, lands as trace in the short-term store and nowhere else.

Immediacy comes from recall. Recent trace is reachable by scene, time, entity, and topic, which are the cues a mechanical write can carry; the state and stored-intention routes read interpreted durable memory and never read trace. The reader comprehends trace as it reads, and a change to how things stand is discovered at recall by bringing recent trace alongside the durable state it may bear on. Reflection later records the change properly. A note made through a memory tool is the character noting something, with the turn that prompted it as its origin; it is a claim to be weighed, never a fact copied forward.

## Why

Interpretation at the moment of experience is either paid for on every write or guessed, and a guess written into lasting memory is the failure this library exists to prevent. One writer means one place where quality is enforced and one mechanism to get right, and the reader at recall is a language model that resolves "open item, newer line saying it was done" without anything having been mapped in advance.

## Rejected Alternatives

- Mapping tool results or events onto remembered state at write time: rejected outright as a library behavior, because the mapping is interpretation; an application that itself keeps the identity of a memory it created may still resolve it through the ordinary link operation as a deliberate act.
- Letting a memory tool write durable memory: rejected outright; it makes lasting memory depend on a model's phrasing mid-response and gives whoever is talking to the character a way to plant facts.
- Requiring a generation call on every write: rejected because applications will write rarely or not at all, and the character then has no memory of today until someone pays; it remains available to an application that prefers it, through reflection over a session's text.
- Treating raw lines as recorded state changes and linking them at write time: rejected outright; deciding that a line is a state change, and of what, is the interpretation being deferred.

## Decision Boundary

Invariant: nothing writes durable memory except a validated plan; in-the-moment input of every kind is trace; a note is a claim with its origin recorded; a change of state before consolidation is served by recall, not by an early durable write.

Not covered: the short-term store's shape and bounds (ADR-D-0026), how recall brings trace alongside durable state, the signal that suggests early reflection, and the memory tool's surface.

## Validation

- No code path from a mechanical write or a memory tool reaches graph authority or the durable vector store.
- A scenario in which a task is completed and the character is asked about it before any reflection shows the open item and the newer trace together, and the durable record unchanged.
- A planted note is recalled as the character's note with its origin and produces no commitment at reflection without support in the record.

## Revisit When

Measurement shows that recall over recent trace cannot carry changes of state reliably enough before consolidation, in a way no improvement to the short-term lookup fixes.

## More Information

- ADR-D-0012 separates candidates from committed memory; ADR-I-0012 defines the validated path.
- ADR-D-0026 defines the short-term store; ADR-D-0028 defines what the validated path enforces on interpreted memory.
