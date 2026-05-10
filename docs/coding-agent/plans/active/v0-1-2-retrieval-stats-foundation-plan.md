# Plan: v0.1.2 Retrieval Stats Foundation

- status: approved
- generated: 2026-05-09
- last_updated: 2026-05-10
- work_type: code

## Goal
- Add the derived retrieval statistics foundation required by v0.1.2: a backend-neutral stats contract, deterministic in-memory store, persistent SQLite store, configuration, composition wiring, and write-path updates that keep stats useful without making them memory truth.

## Definition of Done
- `RetrievalStatsStore` exists as an internal contract for edge-ledger and entity/relation/object counters.
- In-memory and SQLite stats stores exist; SQLite counters survive restart/reopen.
- Settings can select stats store mode, path, health fail mode, smoothing, gamma, and relation/object fanout policy defaults.
- Remember/link/lifecycle write paths update stats after graph-authoritative mutation without changing graph or vector authority.
- Stats update failures are visible through health/diagnostic state and cause later retrieval policy to have a conservative fallback path.
- Required Rust validation passes and Reviewer approves the authority boundary.

## Scope / Non-goals
- Scope:
  - Internal stats contract and data model.
  - In-memory and SQLite stats store implementations.
  - App settings and default construction wiring.
  - Stats updates from graph write and lifecycle mutation paths.
  - Unit/restart tests for counter persistence, idempotency, and health states.
- Non-goals:
  - Stats-guided retrieval fanout implementation.
  - Retrieval rationale/trace public DTO expansion.
  - Durable link co-occurrence admission policy.
  - Stats rebuild/backfill tooling.
  - Public admin API or dashboard.
  - Postgres, Redb, or network stats backends.

## Context (workspace)
- Related files/areas:
  - `docs/roadmap/development_roadmap.md`
  - `docs/design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`
  - `docs/decisions/implementation/ADR-I-0008-retrieval-stats-are-derived-policy-metadata.md`
  - `docs/decisions/implementation/ADR-I-0009-use-sqlite-as-default-retrieval-stats-store.md`
  - `src/config/settings/app_settings.rs`
  - `src/internal/config/settings.rs`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/remember_pipeline.rs`
  - `src/internal/repositories/link_pipeline.rs`
  - `src/internal/repositories/correction_forget_pipeline.rs`
  - `src/internal/repositories/test_support.rs`
  - `src/lib.rs`
  - `Cargo.toml`
- Existing patterns or references:
  - Config parsing is centralized in `Settings`.
  - Repository contracts live under `src/internal/repositories/`.
  - Tests prefer deterministic fakes and unit tests near production modules.
  - Stats must remain derived policy metadata; Oxigraph remains graph truth.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`

## Open Questions (max 3)
- None.

## Resolved Decisions
- Use `rusqlite` with bundled SQLite for the default persistent stats implementation.
- Keep stats update failures internal for this phase through stats health and diagnostics; do not change public remember/link/correct/forget outcome shapes unless a later plan establishes a concrete caller workflow.
- Missing or unhealthy stats must fall back conservatively: do not expand broadly from stats alone, use minimum fanout for low-selectivity paths, keep existing static graph caps as hard upper bounds, and require stronger support signals before expanding through broad entities.

## Assumptions
- A1: SQLite is the intended default because ADR-I-0009 is accepted.
- A2: This plan may add `Cargo.toml`/`Cargo.lock` dependencies for `rusqlite` with bundled SQLite and temp-file testing.
- A3: Stats update ordering follows the roadmap: graph mutation, vector maintenance, then stats delta.
- A4: Public `CharacterMemory` facade shape should remain stable unless constructor composition needs a non-breaking internal update.

## Tasks

### Task_1: Define Stats Contract, Config, And Policy Inputs
- type: impl
- owns:
  - `src/internal/repositories.rs`
  - `src/internal/repositories/retrieval_stats_store.rs`
  - `src/config/settings/app_settings.rs`
  - `src/internal/config/settings.rs`
  - `Cargo.toml`
  - `Cargo.lock`
- depends_on: []
- description: |
  Add a backend-neutral `RetrievalStatsStore` contract, stats health/fail-mode types, counter/query DTOs, deterministic edge-key expectations, and settings for store selection, path, smoothing, gamma, and relation/object fanout policy inputs.
- acceptance:
  - Contract can represent entity/relation/object counters, global counters, edge ledger idempotency, lifecycle/currentness state, and health.
  - Settings parse retrieval stats mode/path/fail mode and selectivity parameters with stable defaults.
  - Dependency selection is recorded as `rusqlite` with bundled SQLite unless implementation discovers a blocking validation issue that requires replanning.
  - Conservative fallback defaults are represented for downstream fanout policy: minimum fanout for weak/low-selectivity paths, existing static graph caps as hard upper bounds, and stronger-support requirement for broad expansion.
  - No persisted selectivity category type is introduced.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test settings retrieval_stats --no-fail-fast"
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"

### Task_2: Implement In-Memory And SQLite Stats Stores
- type: impl
- owns:
  - `src/internal/infrastructures/retrieval_stats/**`
  - `src/internal/repositories/retrieval_stats_store.rs`
  - `src/internal/repositories/test_support.rs`
  - `Cargo.toml`
  - `Cargo.lock`
- depends_on: [Task_1]
- description: |
  Implement deterministic in-memory and SQLite-backed stats stores with internal tables for edge ledger, entity relation counts, global relation counts, and stats metadata/health.
- acceptance:
  - In-memory store is deterministic and suitable for unit tests.
  - SQLite store creates or migrates its internal schema on open.
  - Duplicate edge updates are idempotent.
  - Restart/reopen preserves counters and health metadata.
  - Corrupt or unavailable stats state is representable without giving stats authority over memory truth.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test retrieval_stats --no-fail-fast"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"

### Task_3: Wire Stats Into Construction And Write Paths
- type: impl
- owns:
  - `src/lib.rs`
  - `src/internal/repositories/remember_pipeline.rs`
  - `src/internal/repositories/link_pipeline.rs`
  - `src/internal/repositories/correction_forget_pipeline.rs`
  - `src/internal/repositories/test_support.rs`
- depends_on: [Task_1, Task_2]
- description: |
  Add stats store composition and update stats after graph-authoritative writes and vector maintenance for remember, link, correction, forget, suppression, supersession, and currentness changes.
- acceptance:
  - Public construction wires a default SQLite stats store when configured for persistent stats.
  - Tests can inject in-memory/fake stats stores.
  - Stats updates occur after graph mutation and vector maintenance.
  - Duplicate retry/write paths do not inflate counters.
  - Lifecycle/currentness/supersession changes update active/current counters while Oxigraph remains final authority.
  - Stats update failures are recorded internally as health/diagnostic state and do not alter public remember/link/correct/forget outcome shapes in this phase.
  - Stats failures do not decide existence, lifecycle, currentness, provenance, or final inclusion.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test remember_pipeline link_pipeline correction_forget_pipeline retrieval_stats --no-fail-fast"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review write ordering and confirm stats remain derived policy metadata only"

### Task_4: Foundation Closeout Review
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Validate the stats foundation against v0.1.2 docs and authority invariants before retrieval fanout work begins.
- acceptance:
  - All required worker validation evidence is present.
  - Reviewer approves contract boundaries, config defaults, persistence behavior, and write-path ordering.
  - No roadmap-version labels are introduced in durable production comments, identifiers, or user-facing errors.
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
  - kind: review
    required: true
    owner: reviewer
    detail: "Final review of stats foundation implementation"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. No UI/user-flow changes.

## Rollback / Safety
- Stats are derived. If the new stats store is unhealthy or absent, retrieval plans must fall back conservatively rather than trusting stats.
- SQLite schema and dependency changes can be reverted with this feature plan's touched files if the dependency decision changes before implementation.

## Progress Log (append-only)

- 2026-05-09 Draft created:
  - Summary: Split v0.1.2 into feature plans and drafted the stats foundation plan.
  - Validation evidence: Researcher mapped v0.1.2 source docs and affected code areas; no implementation dispatched.
  - Notes: Awaiting user approval and open-question decisions.
- 2026-05-10 Open questions resolved:
  - Summary: Recorded approved recommendations for SQLite dependency choice, stats failure surfacing, and conservative fallback defaults.
  - Validation evidence: Plan-only update; `git diff --check -- docs/coding-agent/plans/active` pending.
  - Notes: No implementation dispatched.
- 2026-05-10 Plan approved for execution:
  - Summary: User requested each plan be committed on its own implementation branch and readied for execution.
  - Validation evidence: Plan status updated to approved; implementation remains pending.
  - Notes: No Worker tasks dispatched yet.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-09 Decision:
  - Trigger / new insight: v0.1.2 spans stats persistence, retrieval policy, diagnostics, link admission, and entity-neutral fixtures.
  - Plan delta (what changed): Split the milestone into a stats foundation plan, retrieval fanout/rationale plan, and link guard/entity-neutral hardening plan.
  - Tradeoffs considered: A single plan would preserve one milestone artifact but make validation and review too broad.
  - User approval: no
- 2026-05-10 Decision:
  - Trigger / new insight: User approved the recommended answers to all stats foundation open questions.
  - Plan delta (what changed): Added resolved decisions for `rusqlite` with bundled SQLite, internal-only stats update failure surfacing, and conservative missing/unhealthy stats fallback defaults.
  - Tradeoffs considered: `rusqlite` keeps the embedded counter workload simple; public outcome changes are deferred until a concrete caller workflow requires them; conservative fallback protects retrieval from treating stale derived stats as authority.
  - User approval: yes

## Notes
- Risks:
  - SQLite dependency choice may affect Windows/native validation.
  - Stats update failure semantics can leak into public API design if not bounded.
  - Composition wiring may touch many tests because current construction has graph/vector/embedder only.
- Edge cases:
  - Duplicate retries must not increment counters twice.
  - Lifecycle flips must update active/current counts without changing total history counters incorrectly.
  - Missing/unhealthy stats must never make retrieval include otherwise invalid graph objects.
