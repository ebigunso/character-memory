---
status: accepted
adr_type: implementation
date: 2026-07-04
deciders: ["ebigunso"]
consulted: ["Claude Fable 5"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0018: Organize the crate into responsibility-boundary modules with enforced dependency direction

## Context and Problem Statement

As the crate accumulated its storage contracts, pipelines, policies, and backend adapters, the module tree stopped expressing where a given responsibility lived. A single `internal/repositories/` directory held port traits, use-case pipelines, retrieval policy, and test fixtures; the crate root bundled the public facade with the composition root, adapter glue, and tests; the core domain model sat under `api::types`, so every internal module textually depended on "the API"; and plan-construction business logic lived in the public DTO layer. Contributors could not tell from the tree where new code belonged, and upcoming roadmap phases (scoped continuity and reflection, factual rigor, retrieval observability and governance, controlled associative recall) each add new domain families, use cases, and policies that need unambiguous homes.

A structural rule stated only by file placement is also not self-preserving: a later audit of the reorganized tree found inverted dependency edges (the error module importing domain types through the API layer; the DTO layer re-exporting use-case types) that file-placement review had not caught, because re-exports make inverted edges invisible to placement checks.

## Decision Drivers

- The tree should express the architecture: one top-level module per responsibility, named for that responsibility.
- Backend choices are implementation details (project philosophy: stay backend-agnostic where practical); contracts and their implementations must not share a home.
- Future roadmap phases must have one obvious landing spot per concept without restructuring.
- Layering must be checkable: a stated import-direction contract, not just a directory shape.
- Rust visibility (`pub`/`pub(crate)`) is the language's boundary mechanism; a directory named `internal` duplicates it weakly.

## Decision

Organize `src/` into one module per responsibility, and enforce the dependency direction between them:

```text
domain       core memory model: objects, links, enums, validation, graph IRIs,
             schema-version constants and guard
api          public boundary DTOs (drafts, lifecycle, retrieval, write-plan shapes)
             and the embedding provider trait; conversion between DTOs and domain
             types is boundary work and belongs here; construction and planning
             logic does not
ports        store contracts: graph authority, vector candidates, embedder,
             retrieval stats, source references — traits plus their query/result
             value types, no algorithm bodies
models       shared storage value types used by multiple ports and adapters
policy       retrieval and write policy: selectivity, bounded graph expansion,
             embedding surface construction
usecases     the lifecycle pipelines: remember, link, retrieve, correct/forget,
             write planning, reconciliation
adapters     backend implementations grouped by technology (oxigraph, qdrant,
             openai, stats), implementing ports
memory       the public facade (thin delegation to use cases)
composition  the composition root: settings-driven backend selection, factories,
             provider-to-port glue
errors       the crate error surface
config       settings loading
```

Visibility is expressed with `pub`/`pub(crate)` on these modules directly; there is no `internal/` directory.

Dependency direction is part of the contract:

- `domain` is the bottom layer; it imports no other crate layer.
- `errors` imports only `domain` (never domain types via `api`).
- `ports`, `policy`, and `models` never import `usecases`, and import `api` only for one named exception: the public retrieval trace/telemetry vocabulary (`api::types::retrieval` trace types) that is part of their result contracts. Domain types always come from `crate::domain`. Any new exception must be added to this ADR explicitly, not adopted silently.
- `api` never imports `usecases` or `adapters`; business logic (for example deterministic write-plan construction) lives in `usecases`, with public access provided by crate-root re-exports rather than by placing the logic in the DTO layer (consistent with the prepare/validate/commit workflow and deterministic-helper decisions).
- `adapters` import inward (ports, policy, models, domain, errors) and never each other.
- Only `composition` imports `adapters`; use cases receive implementations through port traits.
- New code imports domain types from `crate::domain` directly; `api::types` re-exports exist for caller convenience, not as the internal path.

Structural reorganizations against this layout must include an import-direction audit (checking the rules above) as review evidence, not only a file-placement check.

## Implementation Impact

- Every roadmap concept has a predetermined home: new domain families extend `domain` and gain an `api` DTO family; new lifecycle operations become `usecases` modules; ranking/bounding/scoring logic goes to `policy`; new backends are `adapters` subtrees behind existing or new `ports`.
- The public surface is the crate root plus `api`; moving internals does not move public paths as long as re-exports are maintained.
- The repository module-layout section of the development roadmap mirrors this tree and is updated when the tree changes.

## Considered Options

1. Responsibility-boundary modules with `pub(crate)` visibility and an explicit import-direction contract.
2. Keep the historical layout (`internal/repositories`, `internal/infrastructures`, facade-plus-everything crate root) and rely on documentation.
3. Split into a multi-crate workspace (one crate per layer) so the compiler enforces direction.

## Decision Outcome

Chosen option: **Option 1**.

Option 2 had already failed: the catch-all names required annotated re-export barrels to explain themselves, and misplacement (business logic in the DTO layer, algorithms in contract files, one trait's implementations split across two trees) accumulated precisely because the tree did not state responsibilities. Option 3 buys compiler-enforced direction at the cost of workspace overhead, version coordination, and public-surface churn that a pre-1.0 single-consumer library does not yet justify; it remains the natural escalation if direction violations recur despite audits.

## Consequences

### Positive

- Location communicates responsibility; misplaced code is visible in review as a wrong-directory diff.
- Contracts, policies, and implementations evolve independently; a new backend touches only `adapters` and `composition`.
- The import-direction contract makes layering violations findable mechanically (grep-level audit), including those hidden behind re-exports.

### Negative / Tradeoffs

- Direction rules are convention plus audit, not compiler-enforced; drift is possible between audits.
- Re-export indexes at the crate root and in `api::types` grow with each domain family and need curation.
- The named trace/telemetry exception means the `ports`/`policy`/`models` direction rule is "never, except the listed vocabulary" rather than a mechanically pure "never"; keeping the exception list in this ADR current requires discipline.

## Validation

- Review evidence for structural changes includes the import-direction audit (`use crate::` analysis) against the rules above.
- `cargo clippy --all-targets -- -D warnings` keeps re-export barrels honest (unused imports fail the build).
- The roadmap's implemented-module-layout section is diffed against the actual tree when structure changes.

## Revisit When

- A roadmap phase produces a concept with no unambiguous home in this layout, or forces repeated exceptions to the direction rules.
- Direction violations keep appearing despite audits — reconsider the multi-crate workspace option.
- The crate approaches a stability commitment (1.0) and the public-surface curation strategy needs to be formalized.

## More Information

- Project philosophy: backend-agnostic implementation details, memory-system-first API shape.
- Related ADRs: prepare/validate/commit write workflow (ADR-I-0012), deterministic helpers do not infer meaning (ADR-I-0013), bounded graph expansion (ADR-I-0006).
- Execution record: `docs/coding-agent/plans/completed/responsibility-boundary-module-reorg-plan.md` (analysis, wave log, post-completion architecture audit, and deferred-debt register).
