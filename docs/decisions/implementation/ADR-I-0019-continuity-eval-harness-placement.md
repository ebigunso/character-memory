---
status: accepted
adr_type: implementation
date: 2026-07-05
deciders: ["ebigunso"]
consulted: ["GPT-5.5"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0019: Place the continuity evaluation harness in the private evals repository

> Update (2026-07-19): the `CharacterMemoryEvals` repository has since been made public. The placement decision and delegation boundary below are unchanged; the privacy rationale and wording constraints in this record describe the situation at decision time and are retained as history. Current wording guidance lives in the repository documentation rules.

## Context and Problem Statement

The v0.1.4 continuity evaluation harness measures whether the v0.1 substrate supports character continuity before scoped continuity features are built on top of it. The phase plan defines the harness as deterministic evaluation tooling that exercises the public library surface, observes retrieval and lifecycle behavior, and reports measurements without changing library behavior or defaults.

ADR-I-0018 organized the library into responsibility-boundary modules and set a revisit trigger for roadmap concepts that have no unambiguous home in that layout. The continuity evaluation harness triggers that question: it is operational evaluation tooling, not a library responsibility such as domain modeling, retrieval policy, use-case execution, adapter implementation, or composition.

The harness repository is private. References to it in this public repository must be explicit enough that public readers understand that the inaccessible repository is intentional, and that the evaluation tooling is a development aid rather than core library functionality.

## Decision Drivers

- The library repository should contain core functionality, public API contracts, implementation ADRs, and roadmap documentation, not development-only evaluation products.
- The harness must consume `character_memory` through the same public API boundary an external user would exercise.
- Evaluation fixtures, runner configuration, generated reports, and local service prerequisites should evolve without expanding the library's public surface or module tree.
- Public documentation must avoid machine-local absolute paths and describe private companion repositories by role, not by a local checkout location.
- The placement must leave a clear delegation boundary between this harness and external long-memory benchmarks.

## Decision

Place the continuity evaluation harness in the private companion repository `CharacterMemoryEvals` as a dataset crate. The evals repository consumes `character_memory` through the public API using a sibling-checkout path dependency.

The harness is a development aid. It is not core library functionality, is not part of the public crate API, and does not change library behavior or defaults. This repository records the decision, roadmap intent, and cross-repository contract; the private evals repository owns fixtures, dataset code, runner configuration, metric implementations, generated reports, and harness-specific documentation.

The delegation boundary is:

- `character_memory` owns library behavior, public API contracts, configuration semantics, telemetry exposed by the library, and implementation decisions.
- `CharacterMemoryEvals` owns continuity evaluation scenarios, deterministic fixture generation, metric calculation, report assembly, and commands for running the harness.
- External benchmark datasets such as LongMemEval and LoCoMo remain comparison or inspiration points. They do not define the Character Memory continuity harness contract, fixture semantics, or acceptance criteria.

The versioning implication is cross-repository: the evals repository may pin or update its path dependency to the sibling checkout during development, while this repository's crate version changes only under the normal release policy. Completing the v0.1.4 milestone includes the user-approved crate version bump to `0.1.4`; later harness improvements do not by themselves require library version bumps unless they expose a library behavior, API, or configuration change.

## Considered Options

1. Private companion repository dataset crate (`CharacterMemoryEvals`) consuming the library through the public API.
2. Add an in-repository `evals/` directory or workspace crate.
3. Place the harness under `examples/`.
4. Add a feature-gated module inside the library crate.

## Decision Outcome

Chosen option: **Option 1**.

Option 2 keeps the code close to the library but makes development-only fixtures, generated data, runner configuration, and report formats part of this repository's normal maintenance surface. It also makes the harness look like shipped library functionality even though it is a private development aid.

Option 3 misuses examples: the harness is not a small public demonstration of API use. It requires deterministic scenario data, local service prerequisites, persistent-store restart checks, report generation, and metric interpretation that belong to evaluation infrastructure.

Option 4 is the wrong boundary. A feature-gated module would place evaluation concerns inside the crate module layout that ADR-I-0018 reserves for core responsibilities, and would invite library behavior or API changes to satisfy harness needs.

## Consequences

### Positive

- The library module layout remains focused on core responsibilities from ADR-I-0018.
- The harness verifies the public API boundary instead of relying on crate internals.
- Development-only fixture and report churn stays out of the public repository.
- Public readers have a clear explanation for why harness implementation references point to a private repository.

### Negative / Tradeoffs

- Cross-repository changes need explicit coordination when telemetry, configuration, or public API shapes change.
- Public contributors cannot inspect or run the private harness from this repository alone.
- Documentation in this repository must be disciplined about naming the private repo without implying that it is public or part of the core crate.

## Validation

- `CharacterMemoryEvals` imports `character_memory` through the public API boundary.
- The continuity harness observes and reports; it does not modify library behavior or defaults.
- This repository contains no machine-local absolute paths when referring to the private evals repository.
- Roadmap and implementation documentation describe `CharacterMemoryEvals` as a private companion repository and the harness as development-aid tooling.

## Revisit When

- The harness needs private crate internals instead of the public API boundary.
- Evaluation results require new library telemetry, configuration, or API contracts.
- The evals repository becomes public, moves into this repository, or changes from development aid to supported product surface.
- External benchmark compatibility becomes an explicit product requirement rather than a comparison input.

## More Information

- Roadmap phase: `docs/design/roadmap-phases/v0_1_4_continuity_evaluation_harness.md`.
- Related ADR: ADR-I-0018, whose revisit trigger covers concepts with no unambiguous home in the responsibility-boundary module layout.
