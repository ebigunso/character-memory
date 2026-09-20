---
status: proposed
adr_type: design
date: 2026-09-21
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-D-0019-discretion-is-disclosure-not-recall--superseded-by-ADR-D-0038.md]
superseded_by: null
depends_on: [../implementation/ADR-I-0016-use-retrieval-intent-as-query-time-policy.md, ADR-D-0029-the-scene-is-given-as-perceived-and-the-application-never-resolves-it.md]
---

# ADR-D-0038: Discretion is a property of disclosure, and no recall is gated by the scene, by default or by option

## Context and Problem Statement

A character that holds a confidence must not repeat it to the wrong person, and the obvious mechanical answer is to keep the memory from surfacing outside the setting it came from. The record this one replaces rejected that as a default and still permitted it as an option: an application that had to enforce a boundary could pass a policy at query time that omitted memories across the scene.

Judged by what the character then does, the option is the same mistake as the default. Someone in a room with Alice and Bob who remembers what Alice told them in confidence can steer away from it, spare her the question, and deflect when Bob asks. Someone from whom that memory was withheld is not discreet, only ignorant, and can walk straight into what they should have protected. Discretion needs the knowledge it is discreet about. A rule that admits only what everyone present was party to also removes everything the character lived through alone the moment anyone else is there.

The mechanical answer also assumes the present is known. A character perceives its surroundings through whatever its platform offers: one microphone in a room, a channel with lurkers, a name with no identity behind it. Who is present is given as perceived and is often partial (ADR-D-0029). Any verdict computed from it, that a memory is safe to voice because everyone here was there, would be confident exactly where the character is blind.

## Decision

Recall is never gated by privacy, sensitivity, audience, or compliance, neither by default nor by an option on a retrieval. Discretion is disclosure, and disclosure belongs to the language model.

The substrate makes discretion possible by reporting, not by deciding. Every memory carries its scene as recorded, who was present and in which setting, and retrieval reports that scene with each admitted memory, together with the present scene as it was given and whether it was partial. The library computes no verdict about who may hear a memory. Judging that, under the same uncertainty about the room that a person has, is the model's.

An application with an obligation that discretion cannot carry meets it outside recall: it filters what it passes on using the scene the library reports, or it keeps what must never mix in separate memory stores.

## Why

Continuity means the same character in every room, and a memory held in one setting and lacking in another is two characters. Keeping discretion at disclosure preserves one character with judgment. A filter on recall changes what the character knows, which is the one thing enforcement must not touch, and an option that does it invites being switched on casually and quietly removes what matters, the same reason memory has no exclusion (ADR-D-0032).

## Rejected Alternatives

- Gating recall on sensitivity or setting by default: rejected outright; it produces false blankness toward something the character plainly experienced and splits the character by setting.
- The same gate as an explicit query-time policy, as the replaced record permitted: rejected because it makes the character ignorant rather than discreet, removes solitary experience whenever someone is present, and rests on a knowledge of who is present that the character often lacks; reopen if an obligation appears that neither filtering outside recall nor separate stores can meet.
- A computed verdict beside each memory, shared with everyone present or not: rejected because it is only as good as a perception of the room that is routinely partial, and a false assurance is worse than none; the recorded scene and the given scene are reported and the judgment is left where the uncertainty can be weighed.
- Recall everything with no scene, leaving discretion entirely to the model: rejected because discretion without knowing who was there is guesswork.

## Decision Boundary

Invariant: no memory stores a sensitivity, audience, or compliance eligibility; no retrieval path or option omits a memory because of its scene; retrieval reports each admitted memory's scene as recorded and the present scene as given; the library computes no verdict about disclosure.

Not covered: the shape and names of the scene fields; how a rendered pack presents a memory's scene to the model; how the behavioral evaluation tier judges discretion; store isolation between deployments.

## Validation

- Retrieval tests show a memory learned in one setting is admitted when retrieved for another, with its scene reported, and that the present scene is reported as given, including when it is partial.
- API review finds no retrieval option that omits by participants, setting, or any other part of the scene.
- Schema review rejects any persisted eligibility field tied to audience or sensitivity.
- Evaluation scenarios for person-keyed separation and group versus private scenes measure that the scene is present and correct on recall and, at the behavioral tier, that the character does not disclose across it; they never measure that a memory failed to surface.

## Revisit When

A deployment class emerges whose obligation neither filtering outside recall nor separate stores can satisfy, and the obligation is shown to be one the substrate rather than the application must carry.

## More Information

- Replaces ADR-D-0019 in full. Its first two decisions stand unchanged; the query-time boundary policy it permitted is withdrawn, and the rule against computed disclosure verdicts is added.
- ADR-D-0029 has the scene given as perceived, with only the time required, which is why nothing here may depend on the scene being complete.
- ADR-D-0032 rejected an exclusion operation for the same reason an optional recall gate is rejected here.
- ADR-D-0021 covers erasure obligations, which remain out of band.
- The continuity situation catalog's small-circle situations describe the target behavior.
