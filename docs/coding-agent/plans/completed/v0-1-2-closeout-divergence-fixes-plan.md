# Plan: v0.1.2 Closeout Divergence Fixes

- status: done
- generated: 2026-06-12
- last_updated: 2026-06-12
- work_type: mixed

## Goal
- Close the known divergences between the v0.1.2 design documents and the implemented code before starting v0.1.3: externally configurable fanout budgets, documented selectivity scope boundary, facade-level integration coverage for v0.1.2 behavior, and design-doc consistency notes.

## Definition of Done
- Per relation/object fanout budgets are configurable through settings, with current hard-coded values preserved as defaults.
- The entity-root-only selectivity scope is explicitly documented as the intended v0.1.2 boundary.
- Facade-level integration tests cover stats persistence/reopen, restart-safe retrieval exclusion behavior, and selectivity-influenced retrieval.
- `associated_with` admission constraints and the retrieval-trace boundary are documented consistently across the phase design docs.
- Full validation passes: `cargo fmt --check`, `cargo check`, `cargo test`.
- Reviewer approves the closeout against v0.1.2 acceptance criteria.

## Scope / Non-goals
- Scope:
  - Configuration surface for existing fanout specs (`About -> DerivedMemory`, `Involves -> Episode`, `PartOfThread -> DerivedMemory`) plus a documented default for unlisted pairs.
  - Documentation alignment in `docs/design/roadmap-phases/` and `docs/design/database/` where affected.
  - New integration tests under `tests/`.
- Non-goals:
  - Widening selectivity beyond entity-root candidates (documented boundary instead; revisit in v0.1.5 with eval data).
  - New retrieval signals, fanout formula changes, or selectivity math changes.
  - Public reconciliation/diagnostics facade.
  - Any v0.1.3 write-plan work.
  - Destructive deletion (permanently out of scope by project direction).

## Context (workspace)
- Related files/areas:
  - `src/internal/repositories/retrieval_selectivity.rs` (hard-coded `fanout_specs()`)
  - `src/config/settings/app_settings.rs` (existing stats/selectivity config patterns)
  - `src/internal/repositories/retrieve_pipeline.rs` (selectivity integration)
  - `tests/initialization_tests.rs`, `tests/v0_1_public_facade_tests.rs` (live-service-gated test pattern)
  - `docs/design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md`
  - `docs/design/roadmap-phases/v0_4_retrieval_observability_governance.md`
  - `docs/roadmap/development_roadmap.md` (conceptual `[retrieval.fanout.*]` config shape)
- Existing patterns or references:
  - Stats config already follows `[retrieval.stats]` / `[retrieval.selectivity]` conceptual shapes with env/config wiring in `app_settings.rs`.
  - Integration tests skip when Qdrant is unavailable; follow the same gating for new tests.
  - Conservative fallback (fanout <= 1) on missing/unhealthy stats must remain unchanged.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`

## Open Questions (max 3)
- Q1: Should `Cargo.toml` crate version be bumped (currently `0.1.0` while capability is roadmap v0.1.2), or is crate version intentionally decoupled from roadmap phases?

## Resolved Decisions
- Selectivity remains entity-root-only in v0.1.2; the fix is to document this boundary explicitly rather than widen behavior. Widening is revisited in v0.1.5 with eval-harness data.
- Fanout configuration uses the roadmap's conceptual TOML shape (`[retrieval.fanout.<relation>.<object_type>]` with `min`/`max`), mapped onto the existing settings infrastructure. Unknown/unlisted relation-object pairs keep current built-in behavior.
- `associated_with` remains in the v0.1 relation vocabulary but is documented as admissible only with stronger evidence or explicit application intent until v0.5 associative structures exist (consistent with ADR-I-0011 and the implemented link guard).
- Retrieval-trace boundary documented as: light per-retrieval rationale/telemetry now (v0.1/v0.1.2), internal/admin reconciliation diagnostics (v0.1.1), durable first-class `RetrievalTrace` objects deferred to v0.4.

## Assumptions
- A1: Current `fanout_specs()` values (About->DerivedMemory max 20, Involves->Episode max 5, PartOfThread->DerivedMemory max 15) are the correct defaults to preserve.
- A2: Integration tests may use temp-file SQLite stats stores and in-memory or persistent graph mode; restart-safety is simulated by dropping and reconstructing the facade against the same persistent stores.
- A3: No schema or persisted-format changes are needed; this is config surface, tests, and docs only.

## Tasks

### Task_1: Externalize fanout budget configuration
- type: impl
- owns:
  - `src/config/settings/app_settings.rs`
  - `src/internal/repositories/retrieval_selectivity.rs`
  - `src/lib.rs` (only wiring of settings into selectivity, if required)
- depends_on: []
- description: |
  Replace hard-coded `fanout_specs()` with settings-driven per relation/object fanout budgets
  following the roadmap's conceptual `[retrieval.fanout.<relation>.<object_type>]` `min`/`max` shape.
  Current values become defaults. Unlisted pairs keep current default behavior. Conservative
  fallback on missing/unhealthy stats is unchanged. Add unit tests for config parsing, default
  preservation, and override behavior.
- acceptance:
  - Fanout budgets for the three existing relation/object pairs are configurable via settings/env.
  - Defaults match the previously hard-coded values exactly; behavior is unchanged without config.
  - Invalid config (min > max, negative values) is rejected or clamped with a clear error/diagnostic.
  - Conservative fallback behavior on unhealthy stats is unchanged and still tested.
  - No entity identity, name, or application role appears in any fanout policy input.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review: defaults preserved, no behavior change without config, entity-neutrality intact"

### Task_2: Align phase design docs with implemented boundaries
- type: docs
- owns:
  - `docs/design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md`
  - `docs/design/roadmap-phases/v0_4_retrieval_observability_governance.md`
- depends_on: []
- description: |
  Three documentation alignments per the Resolved Decisions:
  1. Document entity-root-only selectivity application as the intended v0.1.2 scope, with widening
     deferred to v0.1.5 eval-driven closeout.
  2. Add an `associated_with` admission note to the v0.1 starter doc: allowed only with stronger
     evidence or explicit application intent until v0.5.
  3. State the retrieval-trace boundary consistently: light rationale/telemetry now, admin-facing
     reconciliation diagnostics in v0.1.1, durable full traces in v0.4.
- acceptance:
  - v0.1.2 doc states the entity-root selectivity boundary and its revisit point.
  - v0.1 starter doc carries the `associated_with` admission constraint with an ADR-I-0011 reference.
  - Trace boundary wording is consistent across the v0.1, v0.1.1, and v0.4 docs.
  - No design content beyond these boundary clarifications is changed.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Doc review against Resolved Decisions and ADR-I-0011/ADR-D-0013 consistency"

### Task_3: Facade-level integration tests for v0.1.2 behavior
- type: test
- owns:
  - `tests/v0_1_2_retrieval_guardrails_tests.rs`
  - `tests/test_utils.rs` (additive helpers only)
- depends_on: [Task_1]
- description: |
  Add integration tests at the `CharacterMemory` facade level, following the existing
  live-service gating pattern (skip when Qdrant unavailable):
  1. Stats persistence: write memories, drop facade, reconstruct against the same SQLite stats
     file, verify counters survive and selectivity telemetry reflects accumulated counts.
  2. Restart-safe retrieval: suppressed/superseded/non-current records remain excluded after
     facade reconstruction against persistent stores.
  3. Selectivity behavior: a high-degree entity fixture yields bounded fanout with telemetry,
     and configured fanout overrides from Task_1 are observable at the facade level.
- acceptance:
  - All three scenarios run as integration tests with deterministic fixtures.
  - Tests use heterogeneous entity fixtures (no role/name special-casing).
  - Tests skip cleanly when required services are unavailable.
  - Tests pass against the Task_1 configurable fanout implementation.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review test scenarios against v0.1.1/v0.1.2 acceptance criteria coverage gaps"

### Task_4: Closeout review
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Independent review of the full diff against v0.1.2 acceptance criteria, entity-neutrality
  requirements, and the Resolved Decisions in this plan. Confirm no behavior drift beyond
  the configured surface and no v0.1.3 scope creep.
- acceptance:
  - Reviewer status is APPROVED
  - All required validation evidence present in Worker reports
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Full closeout review: acceptance criteria, entity-neutrality, scope discipline"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_2]
- Wave 2 (parallel): [Task_3]
- Wave 3 (parallel): [Task_4]

## Rollback / Safety
- All changes are additive (config with preserved defaults, new tests, doc clarifications); revert via git on the working branch.
- No schema, persisted-format, or public API shape changes.

## Progress Log (append-only)
- 2026-06-12 Wave 1 complete.
  - Task_1 done: fanout budgets externalized to `[retrieval.fanout.<relation>.<object_type>]` settings (about_entity.derived_memory, participant_entity.episode, part_of_thread.derived_memory); defaults preserved exactly (0/20, 0/5, 0/15); invalid config rejected with `ConfigParseError` (min > max rejected; negatives rejected by usize parsing); no entity identity/role in policy inputs. Validation: `cargo fmt --check && cargo check && cargo test` passed (302 passed, 0 failed, 2 ignored) with local Qdrant via docker-compose.
  - Task_2 done: v0.1.2 doc gained §7.1 entity-root-only application boundary with v0.1.5 revisit link; v0.1 starter doc gained `associated_with` admission note referencing ADR-I-0011 and light rationale/telemetry wording; v0.1.1 and v0.4 docs aligned on trace boundary (light telemetry now, admin reconciliation diagnostics in v0.1.1, durable RetrievalTrace in v0.4). Manual consistency/link review passed.
  - Orchestrator note: Cargo.toml version corrected 0.1.0 -> 0.1.2 (covered by Task_1 validation run). v0.1.4/v0.1.5 roadmap-phase design docs authored before wave start per user direction.
- 2026-06-12 Wave 2 complete.
  - Task_3 done: tests/v0_1_2_retrieval_guardrails_tests.rs created (3 tests, all executed against live Qdrant: stats persistence/reopen via temp SQLite + persistent graph tempdir; restart-safe supersession/suppression exclusion; high-degree heterogeneous entity fixture with selectivity telemetry and configured About->DerivedMemory max=2 override constraining fanout vs default). tests/test_utils.rs gained additive persistent-store setup helper. Validation: `cargo fmt --check && cargo check && cargo test` passed (299 unit + 1 init + 3 new + 2 facade, 0 failed, 2 ignored).
  - Lesson candidates from Task_3 staged for closeout: config crate set_override needs i64 (not usize); multi-root retrieval can mask entity-root fanout assertions.
- 2026-06-12 Wave 3 complete. Task_4 Reviewer verdict: APPROVED. Independent validation: `cargo fmt --check && cargo check && cargo test` exit 0 (299 unit + 1 init + 3 guardrails + 2 facade passed, 0 failed, 2 ignored; new tests executed, not skipped). One MINOR non-blocking note: dead_code warnings from integration-test helper modules in tests/test_utils.rs — optional future cleanup. Plan closed and moved to completed.

## Decision Log (append-only)
- 2026-06-12: Plan drafted. Destructive deletion confirmed permanently out of scope by project owner: memory is permanent; deletion alters perceived history and disrupts character continuity. Selectivity-scope widening deferred to v0.1.5 (eval-driven closeout) rather than fixed here.
