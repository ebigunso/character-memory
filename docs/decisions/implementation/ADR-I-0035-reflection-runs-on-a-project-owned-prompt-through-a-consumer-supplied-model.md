---
status: accepted
adr_type: implementation
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [../design/ADR-D-0025-experience-enters-durable-memory-by-one-path.md, ../design/ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md, ../design/ADR-D-0027-every-span-of-presence-is-accounted-for.md, ../design/ADR-D-0028-interpreted-memory-carries-its-evidence.md, ../design/ADR-D-0029-the-scene-is-given-as-perceived-and-the-application-never-resolves-it.md, ADR-I-0012-use-prepare-validate-commit-write-workflow.md, ADR-I-0013-deterministic-helpers-do-not-infer-high-level-meaning.md]
---

# ADR-I-0035: Reflection runs on a default prompt the project owns, through a completion port the consumer implements, and the library hard-codes no model

## Context and Problem Statement

Consolidation needs a language model. A port alone means nobody can reflect without authoring a quality-critical prompt as well as integrating a model, and then trace accumulates indefinitely and the character never forms lasting memory from it, which defeats the design. A bundled model client means the library tracks vendors and versions forever and excludes anyone running a model it does not support. The fork is what the library ships and what the consumer supplies.

## Decision

The library ships a default reflection prompt that the project owns, versions, and evaluates, and the consumer may override it. The consumer supplies the model by implementing a minimal completion port, in the pattern of the embedding provider. The library carries no model client and names no model.

The library asks the model for structured output and validates it through prepare, validate, and commit, including the evidence rules of ADR-D-0028, so output that is malformed, ungrounded, or structurally unsupported yields diagnostics and an unreleased trace. This is structural validation, not semantic safety: a schema-valid misjudgment can still commit, which is why evaluation measures reflection's outputs and a later reflection can supersede them. Which prompt ran is reported on the reflection's outcome and in the log, as the project's prompt version or a digest of an overriding prompt's text, and is not stored on the memories a reflection produces: supersession never reads it, consumed trace is gone so nothing can be re-reflected by version, the evaluation harness pins the prompt in its own manifest, and a prompt identifier without the model that ran it, which the library never knows, identifies little. Reflection's input is bounded, and a long span is processed in order in bounded pieces.

Scheduling is the application's. The library reports what has accumulated since the last reflection and why, selects a reflection's bounded input, and runs a reflection when it is called; it never starts one on its own.

Reflection may run in the background beside everything else, and the library makes that safe. An entry is consolidated by at most one reflection, and a reflection that ends without committing, for any reason, leaves its trace intact and free to be selected again. Consolidation is recoverable: after a crash it is either finished exactly as it was planned or not begun, never partial, never duplicated, and never composed afresh over the same trace. Every operation that changes durable memory takes its turn one at a time, and recall never observes one in progress; the model call happens outside any turn, so nothing waits on a model.

## Why

Owning the prompt keeps the quality-critical instructions with the project that can evaluate them, while taking the model through a port keeps the library free of a maintenance surface that changes monthly and open to any model a consumer can call. ADR-I-0013 is unchanged: deterministic helpers still infer nothing, and inference lives behind this port.

## Rejected Alternatives

- A port with no default prompt: rejected because every consumer would have to author and evaluate a quality-critical prompt before trace could ever be consolidated; the default removes prompt authoring, while integrating a model stays the consumer's by design.
- A bundled client for named models: rejected because of the maintenance surface and because it excludes unsupported models; reopen only as an optional companion crate outside the library.
- Free-text reflection output parsed heuristically: rejected outright; the write path validates structure.
- The prompt version recorded on every memory a reflection produces: rejected because nothing reads it, as above, it costs a field on every interpreted memory, and it obliges every consumer who overrides the prompt to keep a versioning scheme; an application that wants the history keeps the outcome it was handed. Reopen as one record per reflection run if a need to find a run's outputs appears. The catalog's F19, that the record shows which reflection concluded what, is met without it: the earlier conclusion stays in the record, superseded, beside the one that replaced it, each with its time and the evidence it rested on, and a reflection is told from another by when it concluded.
- A scheduler inside the library, including an opt-in background worker that triggers on scene boundaries and accumulation: rejected because what counts as a finished conversation or a finished action log differs widely between applications, and only the application knows it and what a call costs; any default timing would be an arbitrary assumption that constrains how the library can be used. Reflection needs no idle character, so the application is free to start it in the background whenever it judges an event finished; the library provides the signal, the selection, and safe concurrent execution.

## Decision Boundary

Invariant: the project owns a versioned default prompt that consumers may override; the model arrives only through the consumer's port; output is structured and validated; which prompt ran is reported on the outcome and never stored per memory; the library schedules nothing; an entry is consolidated by at most one reflection, and an uncommitted reflection leaves its trace intact; after a crash a consolidation is finished as planned or not begun; operations that change durable memory are serialized, and recall never observes one in progress.

Not covered: the protocol that achieves the concurrency and recovery guarantees, which the generation phase's design draft describes and its plan settles; the port's exact signature, the output schema, the prompt text, the signal's thresholds and how accumulation is counted, the piece size, and the guide's recommended schedules.

## Validation

- Reflection runs end to end with a test double implementing the completion port and no network.
- Malformed or rule-breaking model output produces diagnostics, commits nothing, releases nothing, and leaves the trace selectable again; so do a port error, a timeout, and a cancelled reflection.
- A reflection's outcome names the prompt that ran, two different overriding prompts are told apart, and no stored memory carries either.
- Two reflections started together over overlapping trace never consolidate the same entry twice.
- A process killed at any point in a consolidation reopens to a state in which that consolidation is complete or absent, with no partial memory, no duplicate, and no trace lost.
- Recall running beside a commit returns the state before it or the state after it, never part of it, and never a consolidated memory beside the trace it came from.
- A correction and a reflection that touch the same memory leave its supersession chain with one head, in either order.

## Revisit When

Structured output proves unattainable on the models consumers actually run, or the default prompt cannot be evaluated without pinning a model family.

## More Information

- The evaluation harness in the public companion evaluation repository `CharacterMemoryEvals`, a development aid and not core library functionality, pins the prompt version in its own run manifest and freezes reflection outputs the way it freezes embeddings.
- The guide page on when to write and when to reflect is a deliverable of the generation phase.
