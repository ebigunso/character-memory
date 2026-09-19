---
status: accepted
adr_type: design
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [ADR-D-0002-derived-memory-provenance.md, ADR-D-0025-durable-memory-has-one-writer.md, ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md, ../implementation/ADR-I-0015-record-producer-and-rationale-origin-in-candidate-provenance.md]
---

# ADR-D-0028: Interpreted memory carries its evidence, and the write path rejects what that evidence cannot structurally support

## Context and Problem Statement

Consolidation is done by a language model the library does not choose, running a prompt that may be overridden. Models promote a single remark to a personality trait, take "I'm going to quit tomorrow, lol" as a plan, accept "you promised me a refund" as a promise, and obey a line in the conversation addressed to whoever reads it later. Each of these writes false continuity into lasting memory. The judgment belongs to the model, but the evidence behind a judgment is countable and checkable. The fork is whether the quality of interpreted memory rests on the prompt or is enforced where the library already enforces quality, at the write path.

## Decision

Every interpreted memory states the evidence it rests on, and the validated write path rejects a candidate that evidence cannot structurally support.

The model is asked only for judgment. What the trace already knows is copied and never restated by the model: the time, the scene as given, the entry an observation came from, and whether that entry was the character's own output or something it perceived. What the library can count it computes: how many distinct episodes a pattern cites, and over how long. What remains is judgment, how a remark was meant, who among those present said it, what the words amount to. Validation never guarantees a judgment; evaluation measures it, and a later reflection can correct it.

- **Grounding.** An observation points at the part of the trace it rests on, and validation finds it there. For text that is the quoted words, which stay on the observation as a bounded part of the memory so a misreading can be reviewed later; the rest of the entry is released, and this is not a store of source material (ADR-D-0015 stands). An entity, and a link between entities, points the same way at where it is mentioned. A gist rests on the entries it consolidates. Memory a caller authors deliberately has no trace to cite, so it declares its grounding, and the library records it as declared.
- **What an observation records.** Something that was said carries its register: literal, in jest, hypothetical, fiction, or quoted. Something that happened carries who acted and no register. Only a literal statement or an event can support a change of state.
- **Attribution.** Whether an entry is the character's own output or something it perceived is the one attribution the application always knows, so it is the one the library copies. Who spoke in what the character perceived is judged and recorded as judged, because a whole thread, a room heard through one microphone, and a meeting on a shared line all arrive as one stream, and a speaker copied mechanically from such a stream would be wrong in bulk. The application's own opinion about a speaker is a hint, weighed and never copied. What the character infers is attributed to the character.
- **The bar follows attribution.** What a speaker stated plainly, about themselves or anyone else, or gave as a standing instruction, may become state from one literal observation, held as theirs. What the character concludes for itself must be earned: a pattern cites several distinct episodes, a belief about a person rests on a pattern that has persisted and is phrased as a tendency, and a belief the character forms about itself is held to the strictest bar.
- **The character's own commitments rest on what it said or did.** Never on another's claim about it, and never on its own note or passing thought alone, which remain claims (ADR-D-0025).
- **Contradictions are held.** Conflicting accounts coexist with the conflict recorded, and one party's account never supersedes another's.
- **Trace is untrusted.** What was experienced is material to reflect on, never instruction to the one reflecting.
- **The basis stays on the memory.** Whether its grounding was found in trace or declared, and whether its attribution was copied, judged, or declared, so a later reader knows what it is looking at. Who produced a candidate remains write-time provenance (ADR-I-0015).

## Why

A rule the model is asked to follow holds only as well as the model and the prompt in use, and the library controls neither. A rule the write path checks holds for every model and every prompt, and turns the overreach it can see, a trait from one remark, state from something labeled jest, a promise put in the character's mouth by someone else, an observation with no words behind it, into a diagnostic and an unreleased trace. It cannot see a model that labels a joke literal or hears the wrong person. That residue is smaller than the whole, is kept reviewable by the quoted words and the recorded basis, and is what the evaluation harness exists to measure.

## Rejected Alternatives

- Relying on the default prompt to enforce these rules: rejected because the prompt is overridable and models vary; the prompt states them and the write path verifies them.
- A speaker recorded mechanically on each trace entry and copied onto what is derived from it: rejected because an entry may hold a whole thread, and because what arrives mechanically is a channel, not a speaker. Only own versus perceived is known in every deployment, so only that is copied.
- A label from the model saying whether something was stated or inferred: rejected because it follows from the attribution.
- Promoting on a single strong instance: rejected for what the character infers; a plain statement is the single-instance path and is held as the speaker's.
- Resolving contradictions at consolidation: rejected because the character was not given grounds to choose, and a wrong choice erases the dissent it would need later.
- A second model that verifies the first one's labels before commit: not adopted, because it doubles the cost of every reflection and still ends in a model's judgment; reopen if evaluation shows mislabeled register or attribution is common enough to matter.
- A separate quality gate after commit: rejected outright; quality is enforced at the write path and never by manipulating memory afterward.

## Decision Boundary

Invariant: every interpreted memory names the evidence it rests on and the basis of that evidence; time, scene, entry, and own-or-perceived are copied from the trace and never supplied by the model; promotion counts are computed from the cited evidence; an observation's grounding is found in the entry it cites, or is declared by a caller and recorded as declared; state rests only on literal statements and events; what the character concludes for itself meets a promotion threshold; a commitment or fact about the character rests on its own speech or actions; one party's account never supersedes another's; what was experienced is never instruction to the one reflecting; validation rejects what fails these, and guarantees structure and grounding, never the truth of a judgment.

Not covered: the threshold values, which are measured defaults; the vocabulary of registers beyond literal and not; the forms of a locator, of a speaker hint, and of declared grounding for each kind of candidate; how an entry holding both the character's words and another's is marked; which ends of a link must be found in trace; the bound on a quote; the wire shape of a candidate; how the default prompt words any of this. These belong to the generation phase's design and plan.

## Validation

- A candidate that promotes a trait from one episode, rests state on a non-literal observation, or turns a claim about the character, or the character's own note, into a commitment fails validation with a specific diagnostic.
- An observation whose quote is not in the entry it cites fails, and a hostile line in the trace produces no memory that lacks structural support.
- A transcript of several people heard as one stream yields observations whose attribution is recorded as judged, and none attributed by copying.
- A loop the character closed by acting, with nothing said about it, consolidates into a resolution resting on an observation of the event.
- A restatement that supersedes one party's account with another's fails.
- Mislabeled register and misjudged attribution are measured by the evaluation harness on frozen reflection outputs, not asserted by validation.

## Revisit When

Measurement shows a threshold rule rejects correct conclusions often enough to cost more continuity than it protects, a kind of interpreted memory appears whose evidence cannot be stated, or evaluation shows judged attribution is wrong often enough that memory resting on it needs a stricter bar than memory resting on the character's own entries.

## More Information

- ADR-D-0002 requires provenance on derived memory; ADR-I-0015 records producer and rationale origin on candidates and keeps them off committed memory.
- ADR-D-0030 decides that a memory carries no confidence score, and ADR-D-0033 adds the impression, how someone seemed, as a third thing an observation may record.
- The generation phase's design draft states the write-path rules in full.
