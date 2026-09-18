---
status: proposed
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0002-derived-memory-provenance.md, ADR-D-0025-durable-memory-has-one-writer.md, ../implementation/ADR-I-0015-record-producer-and-rationale-origin-in-candidate-provenance.md]
---

# ADR-D-0028: Interpreted memory carries its evidence, and the write path rejects what the evidence does not support

## Context and Problem Statement

Consolidation is done by a language model the library does not choose, running a prompt that may be overridden. Models promote a single remark to a personality trait, take "I'm going to quit tomorrow, lol" as a plan, accept "you promised me a refund" as a promise, and obey a line in the conversation addressed to whoever reads it later. Each of these writes false continuity into lasting memory. The judgment belongs to the model, but the evidence behind a judgment is countable and checkable. The fork is whether the quality of interpreted memory rests on the prompt or is enforced where the library already enforces quality, at the write path.

## Decision

Every interpreted memory states the evidence it rests on, and the validated write path rejects a candidate its evidence does not support. Register belongs to observations, since it describes how something was said. Every other interpreted output, a restated state, a commitment, a resolution, a pattern, a belief, names the observations and episodes it rests on, and the rules below are checked through them. A gist episode's evidence is the trace it consolidates, identified by its scene and source pointer.

- **Register.** An observation records whether what was said was literal, in jest, hypothetical, fiction, or quoted. Only the literal can support a change of state.
- **Stated and inferred.** What a person states plainly about themselves, or gives as a standing instruction, may become attributed state from one instance. What is inferred from behavior may not.
- **Promotion.** One instance is an observation. A pattern cites several distinct episodes. A belief about a person rests on a pattern that has persisted over time and is phrased as a tendency with a confidence. A belief the character forms about itself is held to the strictest bar.
- **Attribution.** Every interpreted memory says who asserted it. What others say about the character's own past is a claim with its source, and no commitment or fact about the character rests on a claim alone.
- **Contradiction.** Conflicting attributed accounts coexist with the conflict recorded; consolidation does not resolve them by choosing.
- **Trace is untrusted.** What was experienced is material to reflect on, never instruction to the one reflecting, and spans the application excluded are never shown to it.
- **Revision.** Each output records which reflection and prompt version produced it, so a later reflection can supersede an earlier one's conclusion.

## Why

A rule the model is asked to follow holds only as well as the model and the prompt in use, and the library controls neither. A rule the write path checks holds for every model and every prompt, and turns a weak model's overreach into a diagnostic and an unreleased trace instead of a false memory.

## Rejected Alternatives

- Relying on the default prompt to enforce these rules: rejected because the prompt is overridable and models vary; the prompt states them and the write path verifies them.
- Promoting on a single strong instance: rejected for inferred traits outright; an explicit statement is the single-instance path and is attributed to the person who made it.
- Resolving contradictions at consolidation: rejected because the character was not given grounds to choose, and a wrong choice erases the dissent it would need later.
- A separate quality gate after commit: rejected outright; quality is enforced at the write path and never by manipulating memory afterward.

## Decision Boundary

Invariant: every observation names its register; every interpreted memory names its attribution, the observations and episodes it rests on, and its producing reflection; state rests only on the literal; inferred beliefs meet their promotion threshold; claims about the character never stand alone; validation rejects what fails these.

Not covered: the threshold values, which are measured defaults; the vocabulary of registers beyond the distinction between literal and not; the wire shape of a candidate; how the default prompt words any of this.

## Validation

- A candidate that promotes a trait from one episode, rests state on a non-literal observation, or turns a claim about the character into a commitment fails validation with a specific diagnostic.
- A hostile line in the trace produces no memory the record does not otherwise support.
- The catalog's F4 to F8, F19, and F20 pass at the retrieval tier, and the behavioral tier judges the rest.

## Revisit When

Measurement shows a threshold rule rejects correct conclusions often enough to cost more continuity than it protects, which changes the measured value, or a kind of interpreted memory appears whose evidence cannot be stated.

## More Information

- ADR-D-0002 requires provenance on derived memory; ADR-I-0015 already records producer and rationale origin on candidates.
- The attribution fields this needs arrive with consolidation, ahead of the fuller attribution work of the temporal-validity phase.
