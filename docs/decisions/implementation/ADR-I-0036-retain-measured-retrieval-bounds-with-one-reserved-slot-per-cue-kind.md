---
status: proposed
adr_type: implementation
date: 2026-09-22
deciders: ["ebigunso"]
consulted: ["GPT-6"]
informed: []
supersedes: [ADR-I-0022-retain-measured-retrieval-defaults.md]
superseded_by: null
depends_on: [ADR-I-0010-use-continuous-selectivity-and-smooth-fanout.md, ../design/ADR-D-0022-recall-is-activation-by-scene-cues.md]
---

# ADR-I-0036: Retain measured retrieval bounds with one reserved slot per cue kind

## Context and Problem Statement

Retrieval has finite room for memories brought by the topic and by the scene. Increasing a bound can admit more context without recovering anything relevant; reducing a cue's reservation can let another cue occupy all the room. The decision is which measured defaults to retain and where their selectivity rule applies, while preserving recall of both experiences and what the character knows about a named notion.

The defaults retained on the measurement basis are smoothing and fanout shaping at one, the relation/object fanout ranges of zero to twenty for beliefs about a notion, one to five for participant experiences and zero to fifteen for thread memories, candidate limits of 48 vector candidates and twelve graph roots, and one reserved slot each for participant, place and topic. The activity floor of one is provisional: the calibration did not measure activity pressure.

The binding-scale calibration uses one central notion with 48 experiences across salience levels and embedding clusters, with scored selectivity decisions. Changing smoothing, fanout shaping or individual fanout caps does not improve the returned sets; larger root limits recover no additional relevant memory and increase context size. The scene-overlap calibration uses 48 experiences at one workplace with one described companion, identical and reworded descriptions, eight graded on-topic memories, both identifier orders, default caps and deterministic embeddings. With one reserved slot per kind, participant and place pressure, separately and together, leaves six of eight topic memories, equal to the topic-alone result. Sweeping a competing participant or place floor through zero, one, two, three and five leaves six, six, five, five and three topic memories; a stranger's description brings one, one, two, three and five occasions. A zero reservation still recalls one occasion: it does not disable the cue. These synthetic results do not establish a similarity bound for natural-language descriptions.

Familiarity also changes what an entity-root cap means. A notion present in nearly every experience is a weak clue to which occasion matters, but knowing much about someone must not erase that knowledge when the scene names them: a measured case with 200 beliefs about a named notion brought none under inverse-frequency selection.

## Decision

Retain the retrieval defaults as measured, with the activity floor provisional, and reserve one slot for each cue kind the scene gives. A floor reserves room per given kind, not per person or place; unused room remains available to the other memories. Topic-only retrieval keeps its selection.

The participant-episode budget follows the share of distinct experiences a notion takes part in, across the paths by which those experiences are reached. A ubiquitous notion brings its latest eligible occasion, and a more distinctive notion can bring more; repeated observations of one occasion do not make it several experiences. Missing or unhealthy statistics retain the conservative latest-occasion fallback. No identity receives special treatment.

Beliefs about a notion the scene names are chosen outside entity-root selectivity, within the existing hard bounds. Naming a notion is a direct cue to what is known about it, not an inference from its rarity. The retained inverse-frequency rule does not suppress that state.

## Why

The measured participant, place and topic reservations prevent starvation by another cue, while more reserved scene room admits weaker occasions at the conversation's expense; the activity reservation awaits measurement. Participant reach follows the share of experiences the notion takes part in, whereas beliefs about a notion the scene names are chosen outside entity-root selectivity because a named root is not inferred.

## Rejected Alternatives

- Raise the cue floors together: extra participant or place reservations reduce on-topic retention in the measured overlap cases; reopen if a representative calibration shows useful recall gained without that loss.
- Use a zero floor to silence an unfamiliar description: a floor controls reservation, and zero still admits an occasion; assess a description-similarity bound separately when suitable embedding evidence exists.
- Tighten fanout caps or enlarge the root cap without a measured recall benefit: the measured changes either preserve the same output or add context cost; reopen when a binding workload demonstrates relevant memories recovered or irrelevant context avoided.
- Apply inverse-frequency suppression to beliefs about a named notion: rejected outright; the scene's direct cue must not lose its state because the character knows a great deal about it.

## Decision Boundary

Invariant: retrieval defaults require a measured basis that preserves room for each given cue kind; participant reach measures distinct experiences, and beliefs about a named notion remain outside entity-root selectivity. Hard bounds, conservative fallback and entity-neutrality remain constraints on calibration.

Not covered: replacement values justified by renewed calibration, the formulas and counter representation, the ordering and propagation of individual candidate routes, description-similarity or topic-relevance bounds, and selectivity for non-entity roots. Numeric defaults live in configuration and code; this record does not make their measured values universal constants.

## Validation

- Pin the library and evaluation inputs, vary one default at a time, repeat each configuration and compare returned identities, order, context size and the library's cue evidence against the topic-alone and description-removed controls.
- Exercise participant and place pressure separately and together, including reworded descriptions, unfamiliar descriptions, and identifiers ordered against occasion time; report which cue kinds and pressures ran, rather than treating an unchanged inactive floor as proof of protection.
- Verify that a ubiquitous participant reaches its latest eligible occasion across episode and observation paths, and that a named notion with many beliefs still brings its eligible state within the hard caps.
- Preserve the default-value and topic-only retrieval checks. Keep the complete calibration artifacts in the public companion evaluation repository; its tooling is a development aid, not core library functionality.

## Revisit When

A measured similarity bound for descriptions or a relevance bound for the topic changes which memories compete; a change to retrieval routes or pack admission changes which memories compete; a representative corpus replaces the synthetic workload; or a different statistics regime invalidates the scored-decision calibration. Repeat the affected calibration before changing the values. The scene-overlap measurement uses deterministic synthetic embeddings, so production embedding behavior requires separate evidence.

## More Information

- [ADR-I-0010](ADR-I-0010-use-continuous-selectivity-and-smooth-fanout.md) defines continuous selectivity; [ADR-D-0022](../design/ADR-D-0022-recall-is-activation-by-scene-cues.md) requires recall by scene cues and room for each kind.
- The public companion evaluation repository, `CharacterMemoryEvals`, retains the scene reminder calibration at `docs/evidence/calibration/scene-reminders-2026-09-21/corrected-description-reading.md`, with the complete table and raw measurements recoverable at `CharacterMemoryEvals@784c2784260196352d7cc5184e0992260f0b1b0b`; its tooling is a development aid, not core library functionality. The first reading in that directory rested on a probe whose wording carried the same vector as a stored wording, so it is kept and marked superseded rather than removed.
- The [retrieval calibration closeout](../../roadmap/roadmap-phases/v0_1_5_closeout_report.md) points to the binding-scale sweeps and their limitations.
