---
status: accepted
adr_type: design
date: 2026-10-16
deciders: ["ebigunso"]
consulted: ["GPT-6 Astra"]
informed: []
supersedes: [../superseded/ADR-D-0005-derived-memory-subtypes-before-belief-ontology--superseded-by-ADR-D-0040.md]
superseded_by: null
depends_on: [ADR-D-0030-interpreted-memory-carries-no-confidence-score.md, ADR-D-0035-assertions-are-commitments-carried-by-ordinary-beliefs.md]
---

# ADR-D-0040: Keep beliefs as remembered experience without a fact-acceptance ontology

Accepted by ebigunso on 2026-10-16, through Counsel: “I accept ADR-D-0040 by name, with the retirement of ADR-D-0005.”

## Context and Problem Statement

ADR-D-0005 defers a normalized belief ontology to later roadmap stages and calls for revisiting that choice when factual correction, source credibility, temporal validity or contradiction handling warrant it. Those needs can concern either what a character has come to believe through experience or which propositions a system should accept as facts. The fork is whether developing the character's memory also commits the library to deciding factual acceptance.

## Decision

Beliefs remain ordinary interpreted memories, grounded in the character's experience. Claims, corrections and what the character believes about a source keep their recorded basis and revision paths; they do not become judgments by the library about what is objectively true.

The first-class fact-acceptance objects deferred by ADR-D-0005—`Assertion`, `Claim`, `EvidenceLink`, `BeliefAssessment` and `SourceAssessment`, with confidence and stance for deciding factual acceptance—leave the roadmap rather than remain conditional future work. This is the outcome of that record's revisit, replacing its expectation that richer factual rigor will require a separate belief ontology.

ADR-D-0035's assertions stay: a mechanical assertion is the character's commitment carried by an ordinary belief, not a separate fact-acceptance object. A claim held in interpreted memory and the provenance that grounds it stay too. Source reliability is what the character believes about a source, held as remembered experience.

## Why

This serves the product philosophy's statement, “A belief is built from experience, and the record says which”: the character can remember what it witnessed, was told, said or thought without the library deciding which account is true. A fact-acceptance ontology answers that different question and would turn the character's subjective memory into a truth-maintenance system.

## Rejected Alternatives

- Keep the fact-acceptance ontology deferred or conditional on more correction, attribution or source-reliability work: those needs do not make factual acceptance the library's responsibility; reopen only if its product purpose expands to answering for the truth of remembered claims.
- Remove ordinary belief assertions along with the ontology: rejected outright because a character's mechanically readable commitment and a library verdict on factual acceptance are different things; recall still needs the former without the latter.

## Decision Boundary

Invariant: beliefs remain grounded, revisable interpretations of the character's experience; the library has no first-class ontology that adjudicates factual acceptance. Ordinary claims, provenance and ADR-D-0035's belief-carried assertions are within that boundary. Derived-memory kinds stay distinguished by the `derived_type` field rather than first-class objects for each kind. How firmly a belief is held is read from its recorded basis, as ADR-D-0030 requires.

Not covered: the representation and scheduling of temporal validity, attribution, aliases or roles over time, who else knows, or the character's beliefs about sources; predicate vocabulary, storage and rendering; or fact-checking an application performs outside the library.

## Validation

- Roadmap and phase review find no deferred or conditional fact-acceptance ontology, while temporal understanding, attribution and source reliability remain expressible as the character's experience and beliefs.
- Model and API review distinguish ADR-D-0035's assertions and interpreted claims from the excluded first-class objects, and preserve their grounding and correction paths without a confidence-based factual acceptance layer.

## Revisit When

The library's product purpose expands from preserving a character's experience to accepting responsibility for the truth of remembered claims, and reading their recorded basis cannot serve that responsibility.

## More Information

- [ADR-D-0005](../superseded/ADR-D-0005-derived-memory-subtypes-before-belief-ontology--superseded-by-ADR-D-0040.md) is the retired deferral this record replaces.
- [ADR-D-0001](ADR-D-0001-episode-backed-object-model.md)'s expectation of richer claim or belief objects as future work is foreclosed by this record; its six-object decision stands.
- [ADR-D-0016](ADR-D-0016-do-not-add-generic-metamemory-plane.md)'s Decision line “claim support lives in EvidenceLink” is affected: under this decision, claim support is the recorded basis and provenance an interpreted memory already carries ([ADR-D-0028](ADR-D-0028-interpreted-memory-carries-its-evidence.md)); that line awaits the owner's retirement.
- [ADR-D-0030](ADR-D-0030-interpreted-memory-carries-no-confidence-score.md) keeps the basis of a belief separate from a confidence score.
- [ADR-D-0035](ADR-D-0035-assertions-are-commitments-carried-by-ordinary-beliefs.md) defines the assertions retained by this decision.
- [Product philosophy](../../project_philosophy.md) states the subjective memory boundary; [the v0.2 completion brief](../../coding-agent/briefs/active/v0-2-completion-brief.md) directs the roadmap's change.
