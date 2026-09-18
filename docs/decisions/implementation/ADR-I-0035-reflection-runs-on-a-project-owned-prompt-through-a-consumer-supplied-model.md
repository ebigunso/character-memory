---
status: proposed
adr_type: implementation
date: 2026-09-19
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: []
superseded_by: null
depends_on: [../design/ADR-D-0025-durable-memory-has-one-writer.md, ../design/ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md, ../design/ADR-D-0028-interpreted-memory-carries-its-evidence.md, ADR-I-0012-use-prepare-validate-commit-write-workflow.md, ADR-I-0013-deterministic-helpers-do-not-infer-high-level-meaning.md]
---

# ADR-I-0035: Reflection runs on a default prompt the project owns, through a completion port the consumer implements, and the library hard-codes no model

## Context and Problem Statement

Consolidation needs a language model. A port alone means nobody can reflect without authoring a quality-critical prompt as well as integrating a model, and then trace accumulates indefinitely and the character never forms lasting memory from it, which defeats the design. A bundled model client means the library tracks vendors and versions forever and excludes anyone running a model it does not support. The fork is what the library ships and what the consumer supplies.

## Decision

The library ships a default reflection prompt that the project owns, versions, and evaluates, and the consumer may override it. The consumer supplies the model by implementing a minimal completion port, in the pattern of the embedding provider. The library carries no model client and names no model.

The library asks the model for structured output and validates it through prepare, validate, and commit, including the evidence rules of ADR-D-0028, so a weak or misprompted model yields diagnostics and an unreleased trace, never bad memory. The prompt version is recorded on everything a reflection produces. Privacy exclusions are applied before the prompt is built. Reflection's input is bounded, and a long span is processed in order in bounded pieces. Short-term entries are released only after the plan that consumed them commits, and the release is idempotent.

Scheduling is the application's: the library reports how much a scope has accumulated since its last reflection, and why, and selects a scope's bounded input; it never runs reflection on its own. Reflection is safe to run beside recall and mechanical writes on the same memory.

## Why

Owning the prompt keeps the quality-critical instructions with the project that can evaluate them, while taking the model through a port keeps the library free of a maintenance surface that changes monthly and open to any model a consumer can call. ADR-I-0013 is unchanged: deterministic helpers still infer nothing, and inference lives behind this port.

## Rejected Alternatives

- A port with no default prompt: rejected because every consumer would have to author and evaluate a quality-critical prompt before trace could ever be consolidated; the default removes prompt authoring, while integrating a model stays the consumer's by design.
- A bundled client for named models: rejected because of the maintenance surface and because it excludes unsupported models; reopen only as an optional companion crate outside the library.
- Free-text reflection output parsed heuristically: rejected outright; the write path validates structure.
- A scheduler inside the library, including an opt-in background worker that triggers on scene boundaries and accumulation: rejected because what counts as a finished conversation or a finished action log differs widely between applications, and only the application knows it and what a call costs; any default timing would be an arbitrary assumption that constrains how the library can be used. Reflection needs no idle character, so the application is free to start it in the background whenever it judges an event finished; the library provides the signal, the selection, and safe concurrent execution.

## Decision Boundary

Invariant: the project owns a versioned default prompt that consumers may override; the model arrives only through the consumer's port; output is structured and validated; the prompt version is recorded on outputs; trace is released only after commit; the library schedules nothing.

Not covered: the port's exact signature, the output schema, the prompt text, the signal's thresholds, the piece size, and the guide's recommended schedules.

## Validation

- Reflection runs end to end with a test double implementing the completion port and no network.
- Malformed or rule-breaking model output produces diagnostics, commits nothing, and releases nothing.
- Outputs carry the prompt version; an overridden prompt carries the consumer's identifier.
- A reflection running beside concurrent recall and writes on the same scope leaves a consistent supersession chain.

## Revisit When

Structured output proves unattainable on the models consumers actually run, or the default prompt cannot be evaluated without pinning a model family.

## More Information

- The evaluation harness in the public companion `CharacterMemoryEvals` repository, a development aid and not core library functionality, pins the prompt version and freezes reflection outputs the way it freezes embeddings.
- The guide page on when to write and when to reflect is a deliverable of the generation phase.
