---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [../implementation/ADR-I-0016-use-retrieval-intent-as-query-time-policy.md]
---

# ADR-D-0019: Discretion is a property of disclosure, and recall is never gated by sensitivity by default

## Context and Problem Statement

Small-circle and independent-entity deployments hold memories that must not be repeated outside the setting where they were learned: a confidence from one person, a private scene of a topic also discussed in a group, a regulated fact. The obvious protection is to keep such memories from surfacing when the setting differs. A person does not work that way: told something in the next room, they still know it here, choose not to say it, and are still shaped by knowing. A character whose recall is gated by sensitivity is a different character in each room. At the same time, a language model's discretion is not reliable under pressure, and operators bound by regulation need an enforceable boundary somewhere. The fork is where discretion lives and where enforcement lives.

## Decision

Recall is never gated by privacy, sensitivity, or compliance by default. Discretion is disclosure, and disclosure belongs to the language model.

The substrate makes discretion possible. Every memory carries its scene: who was present, who said it, whether the character was there when it happened, and in which setting. Retrieval reports the scene with each admitted memory.

Where an application must enforce a boundary, it does so as an explicit query-time policy over the scene, chosen by the application for that retrieval and recorded in the retrieval trace as an applied policy. Such a boundary is never stored on a memory as eligibility and is never part of the default retrieval path.

## Why

Continuity means the same character in every room, and a memory held in one setting and lacking in another is two characters. Keeping discretion at disclosure preserves one character with judgment, and keeping enforcement at explicit application policy leaves the accountable party in control of what may be said without changing what the character knows.

## Rejected Alternatives

- Gating recall on sensitivity so a memory never surfaces outside its original setting: rejected outright for the default path; it produces false blankness toward something the character plainly experienced and splits the character by setting. Its only legitimate form is the explicit query-time policy this record permits.
- Recall everything with no scene, leaving discretion entirely to the model: rejected because discretion without knowing who was present is guesswork; reopen only if the scene proves impossible to record at write time, which the participant and setting fields already on episodes show is not the case.

## Decision Boundary

Invariant: no memory object stores a sensitivity, audience, or compliance eligibility that retrieval honors by default, and no default retrieval path omits a memory because of its setting.

Not covered: the shape and names of the scene fields; the vocabulary of application-chosen partition policies; how the behavioral evaluation tier judges discretion.

## Validation

- Retrieval tests show a memory learned in one setting is admitted when retrieved for another, with its scene reported.
- Tests show a partition applied as a query option omits across the scene and its trace records the applied policy.
- Schema review rejects any persisted eligibility field tied to audience or sensitivity.
- Evaluation scenarios for person-keyed separation and group versus private frames measure that the scene is present and correct on recall and, at the behavioral tier, that the character does not disclose across it; they do not measure that a memory failed to surface.

## Revisit When

A deployment class emerges whose obligation no explicit query-time policy can satisfy and only persisted eligibility would, and the obligation is shown to be one the substrate rather than the application must carry.

## More Information

- ADR-I-0016 establishes retrieval intent as query-time policy, the same principle applied here to boundaries.
- ADR-D-0021 covers erasure obligations, which remain out of band.
- The continuity situation catalog's small-circle situations describe the target behavior.
