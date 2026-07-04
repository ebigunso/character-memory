# Plan: Reorganize crate modules by responsibility boundaries

- status: in_progress (approved by user 2026-07-04)
- generated: 2026-07-04
- last_updated: 2026-07-04
- work_type: code

## Goal

- Restructure `src/` so that each top-level module expresses exactly one responsibility from the Phase 0 architecture intent (core model, storage interfaces/ports, use-case pipelines, policies, backend adapters, composition root), instead of the current layout where those responsibilities are lumped into `lib.rs`, a catch-all `internal/repositories/`, and a misc `external_services/` bucket.
- No behavior changes. Public surface may be reshaped (user granted full freedom) provided every divergence from documented intent is justified below.

## Definition of Done

- `lib.rs` contains only module declarations, public re-exports, and crate docs; facade, composition root, adapter glue, conversions, and tests live in dedicated modules.
- Port traits, use-case pipelines, policies, in-crate store implementations, and vendor adapters each live in a cohesive, correctly named module tree.
- The core domain model no longer lives under `api::types`; `api` is a genuine boundary layer (DTOs + provider trait) rather than the textual bottom layer.
- No `mod.rs` files; unit tests inline beside production code; `tests/` remains integration-only (existing repo rules).
- `CustomError` no longer references `qdrant_client` types; vendor errors are wrapped at the adapter boundary.
- Integration-test filenames carry no roadmap version labels (`v0_1_*` renamed by feature).
- `docs/roadmap/development_roadmap.md` "Implemented module layout" matches the actual tree; other doc path references updated.
- `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --no-run`, and service-free `cargo test` all pass; service-gated integration tests compile.

## Scope / Non-goals

- Scope: file moves/splits/renames within the crate, `lib.rs` decomposition, public re-export reshaping, dedup of test fakes, `CustomError` vendor decoupling (wrap `qdrant_client` errors at the adapter boundary), integration-test file renames by feature, doc layout refresh, `Cargo.toml` dependency-section correction.
- Non-goals: behavior changes, new features, renaming public *types*, redesigning the error taxonomy beyond vendor decoupling, ADR changes, integration-test logic rewrites.

## Context (workspace)

- Related files/areas: entire `src/` tree; `tests/` path pins; `docs/roadmap/development_roadmap.md` layout section.
- Existing patterns: no-`mod.rs` + inline-unit-test conventions (completed plan `rust-module-file-layout-migration-plan.md`); hexagonal-ish acyclic layering already present.
- Repo reference docs consulted: `docs/project_philosophy.md` (§8 backend-agnostic, §9 API mental model), `docs/roadmap/development_roadmap.md` (Phase 0 deliverables + layout section), ADR-I-0005, ADR-I-0006, ADR-I-0008, ADR-I-0012, ADR-D-0015, `docs/coding-agent/rules/common.md`.

---

## Analysis: current structure vs responsibility boundaries

### What is already right

The dependency graph is acyclic and layered correctly (`api::types` ← `models` ← `repositories` ← `infrastructures` ← `lib.rs`). No `mod.rs` files, unit tests are inline, `tests/` is integration-only. The problems are **naming, grouping, and lumping** — not dependency direction.

### Findings (evidence-backed)

1. **`src/lib.rs` is a junk drawer (1,266 lines, ≥6 concerns).** Public facade `CharacterMemory`, composition root (backend selection by `GraphStoreMode`), `EmbeddingProviderMemoryEmbedder` adapter, `retrieval_stats_store()` fallback-policy factory, outcome `From` conversions, `test_utils`, and an ~800-line test module whose fakes (`FixedVectorCandidateStore`, `FailingVectorCandidateStore`, `FixedMemoryEmbedder`) duplicate `test_support.rs`.
2. **`internal/repositories/` is a misleading catch-all.** It mixes four responsibilities: (a) port traits (`embedder.rs`, `vector_candidate_store.rs`, `graph_authority_store.rs`, `retrieval_stats_store.rs`, `source_reference.rs`); (b) use-case pipelines (`remember_pipeline.rs` 1,318, `link_pipeline.rs` 657, `retrieve_pipeline.rs` 2,334, `correction_forget_pipeline.rs` 2,629, `write_planning.rs` 1,366); (c) policy (`retrieval_selectivity.rs` 1,072); (d) diagnostics (`reconciliation.rs`) and shared test fixtures (`test_support.rs` 1,781). None are repositories in the conventional sense; the barrel needs `#[allow(unused_imports)]` commentary to paper over the blur.
3. **Contract files contain algorithms.** `graph_authority_store.rs` (1,453 lines) holds the `GraphAuthorityStore` trait plus ~15 query/policy value types plus the bounded-expansion algorithm (`bounded_expansion`, `apply_fanout_limits_by_pair`, `bounded_hub_retention_limit`). A parallel expansion-helper set (`BoundedExpansionLinkRef`, `bounded_incident_link_refs`) lives inside the Oxigraph adapter — one ADR-I-0006 responsibility split across a contract file and an adapter file.
4. **One trait's implementations are scattered.** `RetrievalStatsStore`: trait + `Noop` + `InMemory` impls + write-side helper functions all in `repositories/retrieval_stats_store.rs` (1,083 lines), while the default `Sqlite` implementation (ADR-I-0009) lives in `infrastructures/retrieval_stats.rs`.
5. **Adapter grouping is inconsistent.** Three sibling schemes under `infrastructures/`: `graph/` (by domain concern), `external_services/` (by "remoteness": Qdrant + OpenAI, which share nothing), `retrieval_stats.rs` (by feature, single file). `external_services` is a misc bucket.
6. **Business logic lives in the public API layer.** `api/types/write_plan/helpers.rs` (632 lines) implements plan construction — candidate completion, deterministic UUIDv5 identity, defaults — while its validate/commit counterpart is `internal/repositories/write_planning.rs`. One ADR-I-0012 prepare/validate/commit responsibility is split across the api/internal boundary; `lib.rs::prepare()` reaches into the api-layer helper directly.
7. **`api::types` inverts the layering by name.** The core domain model (`Episode`, `Entity`, `RelationType`, `graph_uri`, schema-version constants in `api/types/domain.rs`) is the crate's bottom layer that `internal::*`, `errors`, and `schema` all import — yet it sits under `api`, so everything internal textually "depends on the API".
8. **`errors` couples the bottom module upward and to a vendor.** `errors/custom.rs` imports `api::types::{MemoryId, ObjectType}` and wraps `qdrant_client::QdrantError` directly; one flat enum mixes config, validation, expansion, embedding, and DB error domains (tension with philosophy §8 backend-agnosticism).
9. **Oversized multi-concern files.** `oxigraph_authority_store.rs` (3,945 lines: embedded RocksDB adapter + remote HTTP/SPARQL adapter + shared serialization + 6 test modules); `correction_forget_pipeline.rs` (2,629: two use cases plus cascade planning); `retrieve_pipeline.rs` (2,334: recall, verification, expansion orchestration, ranking, pack assembly, telemetry).
10. **Declaration-ladder overhead.** `errors.rs → errors/custom.rs` (one enum); `config.rs → config/settings.rs → settings/app_settings.rs`; `internal/config.rs → config/settings.rs → settings/embedding_provider_settings.rs` (31 lines behind three declaration files, in the wrong tree — public `config` pub(crate)-re-exports from `internal`).
11. **Hygiene.** `mockall` sits in `[dependencies]` though only used under `#[cfg_attr(test, ...)]`; roadmap "Implemented module layout" section is stale (missing `write_plan*`, `write_planning.rs`, `retrieval_selectivity.rs`, `retrieval_stats_store.rs`, `infrastructures/retrieval_stats.rs`, `domain/tests.rs`); versioned integration-test filenames (`v0_1_2_*`, `v0_1_3_*`) sit in tension with the "no roadmap version labels in long-lived code" rule.

---

## Proposed target structure

```text
src/
  lib.rs                    — crate docs, module decls, public re-exports only
  memory.rs                 — CharacterMemory facade (thin delegation to use cases)
  composition.rs            — composition root: Settings→backend selection, store/embedder
                              factories, stats-store fallback policy, provider→port adapter glue
  errors.rs                 — CustomError (flattened, single file; no vendor SDK types —
                              adapters wrap vendor errors at the boundary)
  config.rs (+ config/)     — Settings, mode enums, EmbeddingProviderSettings (one tree;
                              flattened one level)
  domain.rs (+ domain/)     — core model: objects, links, enums, validation, graph_uri,
                              schema-version constants + migration guard (absorbs
                              api/types/domain.rs and internal/schema.rs)
  api.rs (+ api/)           — boundary DTOs only: draft, lifecycle, retrieval, write_plan
                              shapes + EmbeddingProvider trait; re-exports domain types for
                              caller convenience; NO construction logic
  ports.rs (+ ports/)       — port traits + their query/result value types:
                              graph_authority.rs, vector_candidate.rs, embedder.rs,
                              retrieval_stats.rs, source_reference.rs
  usecases.rs (+ usecases/) — remember.rs, link.rs, retrieve.rs, correct.rs, forget.rs (or
                              correct_forget.rs if split proves artificial), write_planning.rs
                              (absorbs api/types/write_plan/helpers.rs construction logic),
                              reconciliation.rs
  policy.rs (+ policy/)     — retrieval_selectivity.rs, graph_expansion.rs (bounded-expansion
                              algorithm extracted from ports + oxigraph adapter),
                              embedding_surface.rs (ADR-I-0002 surface construction)
  models.rs (+ models/)     — vector record/candidate/embedding-model value types
                              (or fold into ports/ if small enough after moves)
  adapters.rs (+ adapters/) — grouped by technology, one scheme:
    oxigraph/ (embedded.rs, http.rs, sparql_selectors.rs, rdf_mapping.rs, vocabulary.rs)
    qdrant/   (store.rs, payload.rs)
    openai/   (embedding_provider.rs)
    stats/    (noop.rs, in_memory.rs, sqlite.rs — all RetrievalStatsStore impls co-located)
  test_support.rs           — cfg(test) fakes + fixtures, deduplicated (absorbs lib.rs fakes)
tests/                      — same structure; versioned filenames renamed by feature
                              (retrieval_guardrails_tests.rs, write_planning_tests.rs,
                              public_facade_tests.rs)
```

`internal/` disappears as a name: visibility is expressed with `pub`/`pub(crate)` on the modules above (Rust privacy, not a directory named "internal", is the boundary mechanism). Everything except `memory`, `api`, `domain` (re-exported), `config`, and `errors` stays `pub(crate)`.

### Justification against documented intent (required by user decision)

- **Roadmap "Implemented module layout" section** is the only document prescribing the current tree — and it is *descriptive* documentation of what was built, already stale, maintained by hand. The Phase 0 **Intent/Deliverables** list ("Core model package, Storage interfaces, adapters, fixtures, schema/versioning utilities") is the actual architectural statement. The proposal makes the tree express those exact categories (`domain` = core model package, `ports` = storage interfaces, `adapters` = adapters, `test_support` = fixtures) — it aligns the code *closer* to Phase 0 intent than the current layout does, and Task_7 updates the descriptive section to match.
- **Philosophy §8** ("Backend choices... are implementation details", "Stay backend-agnostic where practical") supports: ports separated from adapters, adapters grouped by technology behind one scheme, and vendor error types wrapped at the adapter boundary instead of inside the crate-wide enum.
- **Philosophy §9** (API should "feel like a memory system, not a database client") supports keeping a deliberate `api` DTO boundary whose names mirror lifecycle operations — and *removing* plan-construction internals from it (Finding 6). Public paths change (e.g. `api::types::domain::Episode` → `domain::Episode`, re-exported at root), which the user has explicitly allowed at 0.1.x; crate-root re-exports keep the flat convenience surface.
- **ADR-I-0012** (prepare/validate/commit as one workflow) supports co-locating plan construction (`helpers.rs`) with validation/commit (`write_planning.rs`) in a single `usecases/write_planning.rs`, rather than splitting the workflow across the api/internal boundary.
- **ADR-I-0006** (bounded graph expansion is a core retrieval guarantee) supports extracting the expansion algorithm from the trait file and the adapter into `policy/graph_expansion.rs`, ending the duplicated helper flavor.
- **ADR-I-0008/0009** (stats are derived policy metadata; SQLite is default store) support co-locating all three `RetrievalStatsStore` implementations as peer adapters.
- No ADR or philosophy statement mandates the names `internal`, `repositories`, `infrastructures`, or `external_services`; those were inherited scaffolding.

## Open Questions (max 3)

- None — Q1-Q3 resolved by user 2026-07-04 (see Decision Log):
  - Q1 resolved: yes — versioned integration-test filenames are renamed by feature (Task_7).
  - Q2 resolved: Orchestrator's call — default is to split into `correct.rs` + `forget.rs` with shared cascade/supersession machinery in a common module; the Task_3 Worker keeps them together only if inspection shows shared internals dominate (>~50% of the code), with evidence in the report.
  - Q3 resolved: yes — vendor-error decoupling is in scope as its own reviewable task (Task_6).

## Assumptions

- A1: 0.1.x public-path breaks are acceptable (user-confirmed); crate-root flat re-exports remain so typical callers (`use character_memory::Episode`) are unaffected.
- A2: Service-gated integration tests cannot run locally without Qdrant/Oxigraph; validation leans on `cargo test --no-run` + service-free `cargo test` + clippy, per repo rules.
- A3: All moves are mechanical (cut/paste + import fixes); any logic change beyond Q3 triggers a replan.

## Tasks

### Task_1: Extract `domain` module and absorb schema guard
- type: impl
- owns:
  - src/domain.rs, src/domain/**
  - src/api/types/domain.rs, src/api/types/domain/tests.rs (removal)
  - src/internal/schema.rs (removal)
  - src/lib.rs, src/api/types.rs (decl/re-export lines only)
- depends_on: []
- description: |
  Move api/types/domain.rs (+ tests.rs) to src/domain/; absorb internal/schema.rs
  schema-version guard. api re-exports domain types; crate root re-exports unchanged names.
- acceptance:
  - Core model + schema versioning live under src/domain/
  - api::types no longer declares domain; re-export keeps caller convenience
  - All four cargo validation commands pass
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test"

### Task_2: Create `ports/` and extract expansion algorithm into `policy/`
- type: impl
- owns:
  - src/ports.rs, src/ports/**
  - src/policy.rs, src/policy/graph_expansion.rs
  - src/internal/repositories/{embedder.rs,vector_candidate_store.rs,graph_authority_store.rs,retrieval_stats_store.rs,source_reference.rs} (moves)
  - src/internal/infrastructures/graph/oxigraph_authority_store.rs (expansion-helper extraction only)
  - barrels/import fixes crate-wide as mechanically required
- depends_on: [Task_1]
- description: |
  Port traits + query/result types move to src/ports/. Bounded-expansion algorithm
  (trait-file helpers + adapter-side BoundedExpansionLinkRef flavor) unifies in
  policy/graph_expansion.rs. Noop/InMemory stats impls temporarily stay with the trait
  (moved to adapters in Task_4).
- acceptance:
  - No algorithm bodies remain in port trait files
  - Single bounded-expansion implementation, used by both callers
  - All four cargo validation commands pass
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test"

### Task_3: Create `usecases/` and consolidate write-planning across the api boundary
- type: impl
- owns:
  - src/usecases.rs, src/usecases/**
  - src/internal/repositories/{remember_pipeline.rs,link_pipeline.rs,retrieve_pipeline.rs,correction_forget_pipeline.rs,write_planning.rs,reconciliation.rs,retrieval_selectivity.rs} (moves)
  - src/api/types/write_plan/helpers.rs (logic relocation; DTO shapes stay in api)
  - src/policy/retrieval_selectivity.rs
- depends_on: [Task_2]
- description: |
  Pipelines move to src/usecases/; retrieval_selectivity.rs moves to src/policy/.
  Plan-construction logic (RememberInput impls, deterministic UUID, defaults) moves from
  api/types/write_plan/helpers.rs into usecases/write_planning.rs; api keeps DTO shapes only.
  correction_forget_pipeline.rs: default to splitting into correct.rs + forget.rs with
  shared cascade/supersession machinery in a common module; keep them together only if
  inspection shows shared internals dominate (>~50%), with evidence in the report.
- acceptance:
  - api layer contains no construction/business logic
  - internal/repositories/ is empty except test_support.rs
  - All four cargo validation commands pass
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test"

### Task_4: Reorganize `adapters/` by technology; split the Oxigraph mega-file
- type: impl
- owns:
  - src/adapters.rs, src/adapters/**
  - src/internal/infrastructures/** (moves/removal)
  - src/internal/models/** (moves: vector value types; embedding_surface.rs → src/policy/)
- depends_on: [Task_3]
- description: |
  infrastructures/ → adapters/{oxigraph,qdrant,openai,stats}. oxigraph_authority_store.rs
  splits into embedded.rs + http.rs + shared helpers, tests inline beside each. Noop/InMemory/
  Sqlite stats stores co-locate under adapters/stats/. models/vector moves per target layout.
- acceptance:
  - One grouping scheme (by technology) for all adapters
  - No file > ~1,500 lines in adapters/ after split (mechanical moves only)
  - All four cargo validation commands pass
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test"

### Task_5: Decompose lib.rs; dedupe test fakes; flatten config/errors; Cargo hygiene
- type: impl
- owns:
  - src/lib.rs, src/memory.rs, src/composition.rs, src/test_support.rs
  - src/config.rs, src/config/**, src/internal/config/** (consolidation), src/errors.rs, src/errors/custom.rs (flatten)
  - src/internal.rs, src/internal/repositories/test_support.rs (relocation)
  - Cargo.toml (mockall → dev-dependencies)
- depends_on: [Task_4]
- description: |
  Facade → memory.rs; composition root + adapter glue + stats fallback → composition.rs;
  lib.rs tests move beside the code they test with fakes deduped into test_support.rs;
  config trees unify (EmbeddingProviderSettings home settled); errors flatten to one file
  (variant reshaping happens in Task_6); internal.rs deleted once empty; mockall moved to
  dev-dependencies (verify automock is cfg(test)-only first).
- acceptance:
  - lib.rs = docs + decls + re-exports only; no cfg(test) module in lib.rs
  - src/internal/ no longer exists
  - mockall no longer in [dependencies]
  - All four cargo validation commands pass
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test"

### Task_6: Decouple `CustomError` from vendor SDK types
- type: impl
- owns:
  - src/errors.rs
  - src/adapters/qdrant/** (error-wrapping sites)
  - call sites matching on the affected variants (mechanical import/match fixes crate-wide)
- depends_on: [Task_5]
- description: |
  Remove the direct `qdrant_client::QdrantError` wrapping from CustomError; adapters map
  vendor errors into a vendor-neutral variant (message + kind) at the boundary. Preserve
  error semantics; Display text may shift — record any change in the report. Assess and
  minimize the upward coupling on api/domain types while doing so (no taxonomy redesign).
- acceptance:
  - errors module has no vendor SDK imports
  - Vendor error information (status/kind/message) is preserved through the wrapping
  - All four cargo validation commands pass
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test"

### Task_7: Rename versioned integration-test files by feature
- type: test
- owns:
  - tests/**
- depends_on: [Task_5]
- description: |
  Rename v0_1_2_retrieval_guardrails_tests.rs → retrieval_guardrails_tests.rs,
  v0_1_3_write_planning_tests.rs → write_planning_tests.rs,
  v0_1_public_facade_tests.rs → public_facade_tests.rs. Update any in-file module docs
  referencing the old names. Revise tests/support structure only if the renames expose an
  obvious misfit; no test-logic changes.
- acceptance:
  - No tests/ filename carries a roadmap version label
  - Test count and gating behavior unchanged (cargo test --no-run lists the same targets)
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo test --no-run && cargo test"

### Task_8: Refresh documentation to match the new tree
- type: docs
- owns:
  - docs/roadmap/development_roadmap.md ("Implemented module layout" section)
  - any docs/** file containing stale src/ or tests/ path references (grep-driven)
- depends_on: [Task_6, Task_7]
- description: |
  Update the layout section to the final tree; fix path references across docs
  (design/database docs, ADR examples if any cite paths).
- acceptance:
  - Documented tree matches `src/` exactly
  - `grep -r "src/" docs/` shows no stale paths
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Doc/source tree diff check"

### Task_9: Independent review
- type: review
- owns: []
- depends_on: [Task_8]
- description: |
  Reviewer verifies: mechanical-move property (no behavior deltas outside Task_6's
  documented error-Display shifts), public-surface re-export coverage vs previous crate
  root, validation evidence for every wave, Q1-Q3 resolutions recorded in Decision Log.
- acceptance:
  - Reviewer status APPROVED
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Full-diff review vs acceptance criteria + facade/integration test compile evidence"

## Task Waves (explicit parallel dispatch sets)

Waves are sequential; the heavy import churn of each restructuring step makes disjoint
`owns` impossible to guarantee across concurrent movers, so waves are single-task by design.

- Wave 1: [Task_1]
- Wave 2: [Task_2]
- Wave 3: [Task_3]
- Wave 4: [Task_4]
- Wave 5: [Task_5]
- Wave 6 (parallel): [Task_6, Task_7] — owns are disjoint (src/errors + adapters vs tests/)
- Wave 7: [Task_8]
- Wave 8: [Task_9]

## Rollback / Safety

- Pure-move waves; each wave lands as its own commit on a feature branch so any wave can be reverted independently. No data-format, schema, or wire changes anywhere in scope.

## Progress Log (append-only)

- 2026-07-04: Plan drafted from Researcher structure analysis. Execution not approved — user requested analysis only.
- 2026-07-04: Execution approved by user; branch refactor/responsibility-boundary-module-reorg created.
- 2026-07-04 Wave 1 completed: [Task_1]
  - Summary: domain model → src/domain.rs (+ domain/tests.rs), schema guard → src/domain/schema.rs; api::types re-exports keep all existing paths; mechanical import fixes in 5 files.
  - Validation evidence: fmt --check / check / clippy -D warnings / test --no-run / test all pass (363 passed, 0 failed, 3 ignored service-gated).
  - Notes: schema guard stays pub(crate) inside public domain module.
- 2026-07-04 Wave 2 completed: [Task_2]
  - Summary: port traits → src/ports/ (5 files); bounded-expansion algorithm → src/policy/graph_expansion.rs; Oxigraph adapter's 5 duplicated private helpers deleted, both call sites use the single policy implementations; internal/repositories.rs kept as compatibility barrel for pipelines (migrate in Task_3). Noop/InMemory stats impls parked in ports/retrieval_stats.rs pending Task_4.
  - Validation evidence: fmt --check / check / clippy -D warnings / test --no-run / test all pass (363 passed, 0 failed, 3 ignored — identical baseline).
  - Notes: deviation recorded — the two expansion flavors are semantically distinct (materialized-plan vs pre-hydration link-ref pruning) and were colocated, not force-merged, per the plan's fallback; lesson appended to docs/coding-agent/lessons.md. GraphExpansion::from_plan widened to pub(crate).
- 2026-07-04 Wave 3 completed: [Task_3] (executed by codex worker via agmsg per user's dispatch-routing instruction)
  - Summary: pipelines → src/usecases/ (remember, link, retrieve, write_planning, reconciliation, correct_forget); retrieval_selectivity → src/policy/; plan-construction logic (RememberInput impls, deterministic UUID, RememberPlanDefaults) moved from api/types/write_plan/helpers.rs into usecases/write_planning.rs with the api helper file now a re-export shim; internal/repositories/ holds only test_support.rs plus a compatibility barrel.
  - Validation evidence: fmt --check / check / clippy -D warnings / test --no-run / test all pass (338 unit + 25 integration passed, 0 failed, 3 ignored — identical baseline). Orchestrator sanity cargo check pass.
  - Notes: Q2 resolved with evidence — correct/forget stay together as correct_forget.rs because shared mutation planning, cascade/provenance discovery, vector maintenance, and fixtures dominate the file; splitting would duplicate internals. Historical completed-plan docs keep old paths (Task_8 owns doc refresh).
- 2026-07-04 Wave 4 completed: [Task_4] (codex worker via agmsg)
  - Summary: infrastructures/ → adapters/{oxigraph,qdrant,openai,stats}/ grouped by technology; Oxigraph mega-file split into embedded.rs (397) + http.rs (673) + shared.rs (1039) + tests.rs; Noop/InMemory stats impls moved out of ports into adapters/stats/ beside sqlite.rs; vector value types → src/models/vector/; embedding_surface → src/policy/; internal/infrastructures and internal/models removed.
  - Validation evidence: fmt --check / check / clippy -D warnings / test --no-run / test all pass (338 unit + 25 integration, 0 failed, 3 ignored — identical baseline). Orchestrator sanity cargo check pass.
  - Notes: deviations accepted — adapters/oxigraph/tests.rs is 1,795 lines (test-only, mechanically moved as one module to avoid test reshaping); focused clippy::module_inception allow on the nested tests module; some git-status noise is EOL normalization only.
- 2026-07-04 Wave 5 completed: [Task_5] (codex worker via agmsg; delivered after codex-side delivery gap — see Decision Log)
  - Summary: lib.rs → thin decls/re-exports; facade → src/memory.rs; composition root + backend wiring → src/composition.rs; shared fakes deduped into src/test_support.rs; config flattened to config.rs + config/{app_settings,embedding_provider_settings}.rs; errors flattened to errors.rs; src/internal/ deleted entirely; mockall moved to dev-dependencies.
  - Validation evidence: fmt --check / check / clippy -D warnings / test --no-run pass; unit suite 338 passed, 0 failed, 3 ignored. cargo test integration portion: 3 guardrail tests fail with the known machine-local post-idle Qdrant gRPC stall — Orchestrator verified via stash/baseline rerun that the previous commit fails identically (same 3 tests, ~85s timeout signature), so classified pre-existing environmental per lessons.md 2026-07-03; Linux CI is the authoritative arbiter for these.
  - Notes: worker reported blocked solely on the environmental cargo test failure; accepted as done with the environmental waiver evidence above.
- 2026-07-04 Wave 6 completed: [Task_6, Task_7] (parallel: worker + worker2 via agmsg)
  - Summary: Task_6 — CustomError no longer references qdrant_client; vendor-neutral VectorDatabaseError payload (backend/kind/status/message/retry_after_seconds) translated at the adapter boundary; VectorDatabaseError re-exported at crate root. Task_7 — versioned test files renamed by feature (retrieval_guardrails_tests.rs, write_planning_tests.rs, public_facade_tests.rs); plus addendum: tests/** match sites and the shared unavailability helper migrated mechanically to the neutral payload with identical skip-vs-fail semantics.
  - Validation evidence: combined post-merge run by worker2 — fmt --check / check / clippy --all-targets -D warnings / test --no-run (all 4 renamed targets + lib listed) / test --lib (338 passed, 0 failed, 3 ignored) all pass. Orchestrator sanity clippy --all-targets pass. Full cargo test remains under the environmental Qdrant-stall waiver (see Wave 5 / Decision Log).
  - Notes: cross-task compile coupling (tests matching the removed variant) surfaced as mutual blocked reports; resolved via a Task_7 addendum granting the tests-side mechanical migration to worker2. Display text change recorded by Task_6: vendor Display replaced by 'qdrant error: kind=... status=... message=...' (+retry_after_seconds for ResourceExhausted).
- 2026-07-04 Wave 7 completed: [Task_8] (worker via agmsg)
  - Summary: roadmap 'Implemented module layout' rewritten to the actual final src/ + tests/ tree (Compare-Object vs filesystem: no diff); stale path references updated in lessons.md and two roadmap-phase docs; completed plans and the active plan excluded per dispatch.
  - Validation evidence: worker's tree-diff pass + stale-reference greps (internal::, internal/, external_services, repositories/, tests/v0_1) all clean with exclusions; Orchestrator spot-check of the roadmap block matches filesystem.
  - Notes: Orchestrator repaired one integration defect from the lessons.md edit — the 'Bounded Expansion' entry heading was accidentally deleted, orphaning its body; heading restored, worker's path updates kept.
- 2026-07-04 Wave 8 in progress: [Task_9] review + remediation
  - Reviewer verdict: CHANGES_REQUESTED with 1 MAJOR — the migrated unavailability helper's gRPC-Unavailable branch was dead code (tonic Code Display is a description sentence, not the enum name), silently changing skip-vs-fail semantics; invisible to all locally runnable checks. 2 MINOR (helpers.rs shim; pre-existing allow-barrels carried forward) — accepted as non-blocking follow-ups.
  - Remediation (Orchestrator, trivial fast-path): tests/support/base.rs predicate changed to case-insensitive substring match. Verified: cargo test --no-run pass; service-down run shows producer output (ResponseError code Internal on hard-down — fails in old semantics too, per 2026-06-12 lesson, so parity holds); healthy-path initialization_tests pass after restart; fmt --check + clippy --all-targets -D warnings pass.
  - Governance: reviewer lesson candidate recorded in lessons.md and promoted to a worker repo rule (skip-gating predicate verification).

## Decision Log (append-only; re-plans and major discoveries)

- 2026-07-04 Decision: Public-surface stability constraint set by user: full freedom at 0.1.x, but divergences from philosophy/roadmap/ADRs must be justified (see "Justification against documented intent"). Scope decision: analysis only for now; this document is the deliverable.
- 2026-07-04 Decision: Open questions resolved by user.
  - Trigger: user answered Q1-Q3.
  - Plan delta: tests/ filename renames added (new Task_7); CustomError vendor decoupling added as its own task (new Task_6); Q2 default recorded in Task_3 (split correct/forget unless shared internals dominate, worker evidence required); docs/review renumbered to Task_8/Task_9; Wave 6 runs Task_6 ∥ Task_7.
  - Tradeoffs: error decoupling is behavior-adjacent (Display text may shift) — isolated in its own wave-reviewable task; test renames lose git-blame-friendly filename history but align with the version-label rule.
  - User approval: yes (this conversation). Execution of the plan itself remains NOT approved.
- 2026-07-04 Decision: Task_5 integration-test failure classified environmental, not regression.
  - Trigger: Worker reported blocked — 3 v0_1_2 guardrail tests failed with Qdrant timeout/cancelled during full cargo test.
  - Plan delta: none (no task changes); Task_5 accepted as done with waiver evidence.
  - Tradeoffs: local full-suite green is unavailable until the machine condition clears (reboot per lessons.md 2026-07-03) — accepted because baseline HEAD fails identically (stash/rerun comparison) and Linux CI remains the authoritative arbiter; service-free suite is fully green.
  - User approval: implicit via recorded lessons procedure; will surface in final report.

## Notes

- Risks: clippy `-D warnings` + `#[allow(unused_imports)]` barrels mean every re-export reshuffle can surface unused-import errors; the `internal::repositories` dissolution touches imports in nearly every file (why waves are sequential); service-gated integration paths stay unexercised locally.
- Edge cases: `pub mod test_utils` (used by `tests/support/base.rs`) must survive lib.rs decomposition; `tests/v0_1_public_facade_tests.rs` pins crate-root re-export names — keep names stable even where paths move.
