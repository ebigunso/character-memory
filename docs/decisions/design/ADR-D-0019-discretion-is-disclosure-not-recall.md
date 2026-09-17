---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "Without this record, future work on privacy, compliance, or multi-person deployments would likely gate retrieval on sensitivity, so that a memory told in confidence is not surfaced outside its original setting."
  detected_signals: "A rejected alternative likely to be re-proposed; a cross-boundary contract between the memory substrate and application policy; a decider's ruling setting a durable governance default."
  cost_of_violation: "A character that cannot recall what it plainly experienced behaves inconsistently across settings, cannot use a confidence to resolve a situation where it should, and shows the blankness the catalog names as a core continuity failure; persisted eligibility on memories is costly to unwind once applications depend on it."
  cost_of_over_extension: "Reading this record as forbidding any enforced boundary would leave compliance-bound operators without a lawful option; the record permits explicit query-time policy chosen by the application."
depends_on: [ADR-I-0016-use-retrieval-intent-as-query-time-policy.md]
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-D-0019: Discretion is a property of disclosure, not of recall

## Context and Problem Statement

Small-circle and independent-entity deployments hold memories that must not be repeated outside the setting in which they were learned: a confidence from one person, a private frame of a topic also discussed in a group, a regulated fact. The obvious protection is to keep such memories from surfacing at all when the setting differs.

That is not how a person handles a confidence. A person told something in the next room still knows it here. They choose not to say it, and knowing it still shapes what they do. A character whose recall is gated by sensitivity behaves inconsistently between settings, cannot draw on a confidence when doing so would resolve a situation, and shows blankness toward something it plainly experienced.

At the same time, a language model's discretion is not reliable under pressure, and operators bound by regulation need an enforceable boundary somewhere.

## Decision Drivers

- Consistency of behavior across settings depends on the character knowing the same things in each.
- Discretion about what to say is the language model's competence, exercised at response time.
- Operators must be able to enforce a boundary where regulation or policy requires one.
- Retrieval policy is query-time and never persisted on memory objects (ADR-I-0016).

## Decision

Recall is never gated by privacy, sensitivity, or compliance by default. Discretion is disclosure, and disclosure belongs to the language model.

The substrate makes discretion possible. Every memory carries its frame: who was present, who said it, whether it was firsthand or told, and in which setting. Retrieval reports the frame with the memory, so the character can be consistent and careful at once.

Where an application must enforce a boundary, it does so as an explicit query-time policy over the frame, chosen by the application for that retrieval. Such a boundary is never stored on the memory as eligibility, and it is never part of the default retrieval path.

## Character Memory Relevance

Continuity means the same character in every room. A memory the character holds in one setting and lacks in another is two characters. Keeping discretion at disclosure preserves one character with judgment, and keeping the boundary at explicit application policy keeps the substrate honest about what it knows while leaving the accountable party in control of what may be said.

## Implementation Impact

- Observations and derived memories carry attribution alongside the participants and setting episodes already record.
- Retrieval outcomes include the frame of each admitted memory.
- Any partition is a retrieval option over the frame, applied per query, and recorded in the retrieval trace as an applied policy.
- Evaluation scenarios for person-keyed separation and group versus private frames measure that the frame is present and correct on recall and, at the behavioral tier, that the character does not disclose across it. They do not measure that a memory failed to surface.

## Considered Options

1. Gate recall on sensitivity so a memory never surfaces outside its original setting.
2. Recall everything with its frame, discretion at disclosure, enforced boundaries as explicit application-chosen query-time policy.
3. Recall everything with no frame and leave discretion entirely to the model.

## Decision Outcome

Chosen option: **2. Recall everything with its frame, and enforce boundaries only as explicit query-time policy**. It is the only option that keeps one consistent character, gives the model what it needs to be discreet, and still gives operators an enforceable boundary.

### Rejected Alternatives

Gating recall on sensitivity is rejected outright for the default path. It produces the catalog's false-blankness failure and splits the character by setting. Its only legitimate form is the explicit query-time policy this record permits.

Recall without a frame is rejected because discretion without knowing who was present is guesswork. It would be reopened only if the frame proved impossible to record at write time, which the existing participant and setting fields on episodes show is not the case.

## Consequences

- Positive: the character is consistent across settings and can be careful without being blank.
- Positive: operators keep an enforceable, inspectable boundary without changing what the character knows.
- Negative / tradeoffs: disclosure failures by the model are possible in the default mode; deployments that cannot accept that opt into the policy.
- Negative / tradeoffs: the frame must be recorded at write time for every memory, which constrains the generation path.

## Decision Boundary

Invariant: no memory object stores a sensitivity, audience, or compliance eligibility that retrieval honors by default, and no default retrieval path omits a memory because of its setting.

Not covered: the shape of the frame fields, the vocabulary of application-chosen partition policies, and how the behavioral tier judges discretion.

## Validation

- Retrieval tests show a memory learned in one setting is admitted when retrieved for another, with its frame reported.
- Tests show a partition applied as a query option omits across the frame and its trace records the applied policy.
- Schema review rejects any persisted eligibility field tied to audience or sensitivity.

## Revisit When

A deployment class emerges for which no explicit query-time policy can satisfy its obligation and only persisted eligibility would, and the obligation is shown to be one the substrate rather than the application must carry.

## More Information

- ADR-I-0016 establishes retrieval intent as query-time policy, the same principle applied here to boundaries.
- ADR-D-0017 covers erasure obligations, which remain out-of-band.
- The continuity situation catalog's small-circle situations describe the target behavior.
