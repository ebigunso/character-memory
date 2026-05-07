# Plan: Retrieval Telemetry For Default Tuning

- status: implemented
- generated: 2026-05-04
- last_updated: 2026-05-04
- work_type: code

## Goal
- Add compact, backend-agnostic retrieval telemetry that makes future default-budget tuning measurable without changing current retrieval defaults, eval configurations, or graph-authoritative hybrid retrieval semantics.

## Definition of Done
- Retrieval output can report configured budgets, vector candidate counts, selected graph root counts, expansion outcomes, bounded failure summaries, omission summaries, and section-limit pressure.
- Telemetry remains suitable for public/debug DTOs and does not expose Qdrant/Oxigraph-specific implementation details.
- Existing retrieval behavior and default limits remain unchanged.
- Serialization and pipeline tests cover the new telemetry shape.
- Required Rust checks and focused retrieval tests pass or have explicit recorded waivers.

## Scope / Non-goals
- Scope:
  - `src/api/types/retrieval.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
  - adjacent retrieval unit tests
  - documentation notes only if needed to explain debug telemetry
- Non-goals:
  - Running the budget sweep.
  - Changing `RetrievalCandidateLimits`, `RetrievalGraphLimits`, or `ContinuitySectionLimits` defaults.
  - Changing LongMemEval or LoCoMo eval configs.
  - Adding vector-only fallback behavior.
  - Exposing backend-specific Qdrant/Oxigraph query internals through public DTOs.

## Context (workspace)
- Related files/areas:
  - `src/api/types/retrieval.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/internal/repositories/graph_authority_store.rs`
  - `docs/decisions/implementation/ADR-I-0006-bounded-graph-expansion.md`
  - `docs/project_philosophy.md`
  - `docs/roadmap/development_roadmap.md`
- Existing patterns or references:
  - `RetrievalContext` already carries candidate, graph, section, lifecycle, trace, and object-type policy.
  - `RetrievalRationale` already reports vector candidate count, graph verified count, stale omissions, and lifecycle omissions.
  - `RetrievalTrace` already provides vector candidates, graph relations, lifecycle decisions, stale omissions, and section assignments.
  - The default-tuning question needs better cost/pressure telemetry before any default changes are defensible.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`
  - `docs/decisions/implementation/ADR-I-0006-bounded-graph-expansion.md`
  - `docs/project_philosophy.md`
  - `docs/roadmap/development_roadmap.md`

## Open Questions (max 3)
- None.

## Resolved Decisions
- Add a nested backend-agnostic telemetry struct under `RetrievalRationale`.
- Expose aggregate telemetry without requiring `include_trace`.
- Gate high-cardinality per-root expansion details behind `include_trace`.
- Keep section-limit pressure distinct from lifecycle/currentness/provenance omissions.
- Do not change retrieval defaults, eval configs, or vector-only behavior in this scope.

## Assumptions
- A1: Telemetry is part of explainability/debuggability and should remain backend-agnostic.
- A2: Existing `include_trace` behavior can gate high-detail telemetry if the final shape includes per-root or per-expansion records.
- A3: Aggregate telemetry should be available without forcing callers to inspect long trace arrays.
- A4: Budget sweep execution and default changes are deferred until better datasets and measurement preparation are available.

## Tasks

### Task_1: Design Telemetry DTO Boundary
- type: design
- owns:
  - `docs/coding-agent/plans/active/retrieval-telemetry-for-default-tuning-plan.md`
  - `src/api/types/retrieval.rs`
- depends_on: []
- description: |
  Decide whether retrieval telemetry belongs as fields on `RetrievalRationale`, as a nested struct, or as trace-only detail. Define aggregate and optional detailed fields before implementation.
- acceptance:
  - Plan decision log records the chosen DTO shape.
  - Telemetry fields are backend-agnostic and stable enough for callers to reason about.
  - High-cardinality details are gated by `include_trace` or otherwise bounded.
  - The plan explicitly records that no retrieval default values are changed.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Decision log updated with telemetry DTO shape, trace gating, and default-preservation boundary"

### Task_2: Add Retrieval Telemetry Types
- type: impl
- owns:
  - `src/api/types/retrieval.rs`
- depends_on: [Task_1]
- description: |
  Add DTO fields or structs for retrieval budget/cost/pressure telemetry with serde compatibility and clear default behavior.
- acceptance:
  - Telemetry includes configured vector candidate and graph root limits.
  - Telemetry includes returned vector candidate count and selected graph root count.
  - Telemetry includes aggregate graph expansion object/relation counts and bounded failure counts by reason.
  - Telemetry includes section-limit pressure or omitted-by-section-limit counts.
  - Serialization tests cover representative telemetry output.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib api::types::retrieval"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review DTO names for backend-agnostic wording and compatibility with retrieval rationale"

### Task_3: Populate Telemetry In Retrieve Pipeline
- type: impl
- owns:
  - `src/internal/repositories/retrieve_pipeline.rs`
- depends_on: [Task_2]
- description: |
  Populate retrieval telemetry during query embedding, vector search, candidate root selection, graph expansion absorption, lifecycle filtering, stale omission recording, and section assignment.
- acceptance:
  - Telemetry reports candidate/root counts from the actual pipeline execution.
  - Telemetry reports bounded graph failures without requiring callers to parse trace relations.
  - Telemetry reports section-limit pressure consistently with existing stale omission behavior.
  - Existing retrieval ranking and context-pack assembly outputs remain unchanged except for added telemetry fields.
  - Existing trace behavior remains compatible with `include_trace`.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::repositories::retrieve_pipeline --lib"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review pipeline telemetry for no ranking/default behavior changes"

### Task_4: Add Measurement-Oriented Regression Coverage
- type: test
- owns:
  - `src/api/types/retrieval.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
- depends_on: [Task_2, Task_3]
- description: |
  Add tests that prove telemetry reflects budget pressure and graph expansion outcomes in representative retrieval scenarios.
- acceptance:
  - Tests cover fewer vector candidates than the configured limit.
  - Tests cover graph root truncation by `max_graph_roots`.
  - Tests cover bounded expansion failure summaries.
  - Tests cover section-limit omissions.
  - Tests assert default retrieval limits are unchanged.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::repositories::retrieve_pipeline --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib api::types::retrieval"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review tests for measurement usefulness and no budget-sweep execution"

### Task_5: Closeout Validation
- type: review
- owns: []
- depends_on: [Task_2, Task_3, Task_4]
- description: |
  Run required checks and review the telemetry change against explainability, compatibility, and default-preservation requirements.
- acceptance:
  - Required validation evidence is complete or explicitly waived.
  - Reviewer confirms no retrieval defaults, eval configs, or vector-only behavior changed.
  - Reviewer confirms telemetry is adequate input for a later budget sweep plan.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo check"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo test internal::repositories::retrieve_pipeline --lib"
  - kind: review
    required: true
    owner: reviewer
    detail: "Full diff review against this plan and ADR-I-0006"

## Task Waves (explicit parallel dispatch sets)

Interpretation:
- Tasks listed in the same wave are intended to be dispatched in parallel by default when `owns` are disjoint and dependencies are met.
- Waves are executed sequentially.

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (parallel): [Task_4]
- Wave 5 (parallel): [Task_5]

## E2E / Visual Validation Spec

- Not applicable. This plan does not touch UI or browser-facing flows.

## Rollback / Safety
- Revert telemetry DTO and population changes if the public/debug shape is not acceptable.
- Keep existing retrieval defaults and behavior unchanged so rollback does not affect retrieval semantics.

## Progress Log (append-only)

- 2026-05-04 00:00 Wave 1 completed: [Task_1]
  - Summary: Telemetry DTO boundary resolved and open questions moved into resolved decisions.
  - Validation evidence: Orchestrator reviewed the decision log and default-preservation boundary.
  - Notes: Budget sweep execution remains deferred.

- 2026-05-04 00:00 Wave 2 completed: [Task_2]
  - Summary: Added backend-agnostic retrieval telemetry DTOs, graph expansion telemetry DTOs, section-pressure summaries, and trace-gated graph expansion trace DTOs.
  - Validation evidence: `cargo test --lib api::types::retrieval` passed.
  - Notes: New DTOs are additive and re-exported with existing public retrieval types.

- 2026-05-04 00:00 Wave 3 completed: [Task_3]
  - Summary: Populated telemetry in the retrieve pipeline for configured limits, vector candidates, graph root selection, graph expansion summaries, bounded failures, and section pressure.
  - Validation evidence: `cargo test internal::repositories::retrieve_pipeline --lib` passed.
  - Notes: Ranking, filtering, defaults, and eval configs were not changed.

- 2026-05-04 00:00 Wave 4 completed: [Task_4]
  - Summary: Added regression coverage for DTO serialization/defaults, graph root truncation, bounded failures, section-limit pressure, and trace-gated graph expansion detail.
  - Validation evidence: `cargo test --lib api::types::retrieval` passed; `cargo test internal::repositories::retrieve_pipeline --lib` passed.
  - Notes: Tests assert default candidate limits remain unchanged.

- 2026-05-04 00:00 Wave 5 completed: [Task_5]
  - Summary: Completed reviewer loop. First review found missing serde default compatibility for old trace payloads; fixed with `#[serde(default)]` and a regression test. Second review reported no findings.
  - Validation evidence: `cargo test --lib api::types::retrieval` passed; `cargo test internal::repositories::retrieve_pipeline --lib` passed; `cargo fmt --check` passed; `cargo check` passed; `cargo test --no-run` passed.
  - Notes: Reviewer noted a non-blocking future API policy question for public struct literal compatibility.

- 2026-05-04 00:00 Plan drafted on `feature-2026-05-04-retrieval-telemetry-for-default-tuning`.
  - Summary: Created execution plan for retrieval telemetry needed before future default-budget tuning.
  - Validation evidence: Planning artifact only; implementation validation pending.
  - Notes: Budget sweep execution and default changes remain deferred.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-04 00:00 Decision:
  - Trigger / new insight: Default retrieval-budget tuning needs reliable cost and pressure telemetry before changing core defaults.
  - Plan delta (what changed): Created a dedicated telemetry/preparation scope and deferred the actual budget sweep.
  - Tradeoffs considered: Add measurement support without reducing eval context, changing defaults, or weakening graph-authoritative retrieval.
  - User approval: yes; user requested branches and committed plans for ready work scopes while waiting on the budget sweep.

- 2026-05-04 00:00 Decision:
  - Trigger / new insight: Remaining retrieval telemetry open questions were resolved before implementation.
  - Plan delta (what changed): Add a nested backend-agnostic telemetry struct under `RetrievalRationale`; expose aggregate telemetry without requiring `include_trace`; gate high-cardinality per-root details behind `include_trace`; keep section-limit pressure distinct from lifecycle/currentness/provenance omissions.
  - Tradeoffs considered: Keep ordinary rationale readable while preserving enough aggregate data for future default tuning and enough trace detail for debugging.
  - User approval: yes; user accepted these recommendations.

## Notes
- Risks:
  - Public telemetry can become noisy if per-root details are not bounded.
  - Backend-specific terminology would make the API less portable.
  - Telemetry must not accidentally become a new ranking input in this branch.
- Edge cases:
  - Graph root truncation.
  - Bounded expansion timeout/node/fanout/hub failures.
  - Section overflow after otherwise valid graph verification.
  - Trace disabled.
  - Empty retrieval results.
