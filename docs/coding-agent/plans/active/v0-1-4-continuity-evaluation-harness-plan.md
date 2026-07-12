# Plan: v0.1.4 Continuity Evaluation Harness (hosted in CharacterMemoryEvals)

- status: in_progress
- generated: 2026-07-05
- last_updated: 2026-07-12
- work_type: code

Repo references: [CM] = this repository (character-memory, public). [CME] = the private companion repository `CharacterMemoryEvals`, checked out as a sibling directory of this repo. CME hosts all evaluation harnesses (LongMemEval-S, LoCoMo, and the new continuity harness) and consumes this crate via a path dependency. Evaluation tooling is a development aid, not core library functionality; CME is not publicly accessible.

## Goal

- Build the deterministic continuity evaluation harness described in `docs/design/roadmap-phases/v0_1_4_continuity_evaluation_harness.md`, hosted in [CME]: seeded synthetic long-horizon fixtures, a fixture-scripted driver loop exercising the full public facade, the seven continuity metrics, restart measurement of persistent stores, and diffable JSONL/summary reports — reusing [CME]'s runner/report/metric-registry infrastructure and changing no `character_memory` behavior or defaults.
- Precondition owned by this plan: [CME] was assembled quickly and needs an architecture revision to be future-proof before new features land there. That revision runs first, in [CME], under its own plan (Task_4 gates all [CME] feature work here).

## Definition of Done

- [CME] architecture revision plan completed and its repo main is green before any continuity feature task starts in [CME].
- `cargo run -p cmem-eval-runner --features real-character-memory -- run continuity ...` (path adjusted to the post-revision layout) executes end-to-end against local Qdrant + persistent Oxigraph + sqlite stats with the deterministic embedding provider and zero network provider calls.
- All v0.1.4 acceptance criteria from the phase doc (lines 304–317) demonstrably met: deterministic/reproducible runs, no external LLM/embedding calls at eval time, heterogeneous hub entities across ≥3 entity kinds, entity-neutral fixtures/metrics, full facade coverage (remember/retrieve/correct/forget/link and prepare/validate/commit), restart measurement for persistent graph and stats stores, reports with metric values + per-query rationale samples, machine-readable diffable reports, selectivity/fanout measurements usable for v0.1.5 tuning, no library behavior change.
- Reviewer has independently run the harness twice and verified reports identical outside the designated metadata block.
- [CM] validation passes: `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --no-run`.
- [CME] validation passes (its own rules): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, plus a service-free mock smoke run.
- `character_memory` crate version bumped to 0.1.4 (user-approved).
- No committed artifact in [CM] contains machine-local absolute paths; all [CM] docs mentioning [CME] follow the private-repo wording rule (`docs/coding-agent/rules/common.md`, Repo Documentation Wording).

## Scope / Non-goals

- Scope:
  - [CME] pre-phase: architecture review + revision, planned and executed in [CME] under its own plan (this plan only gates on it; see Task_4).
  - [CME] features (post-revision): new continuity dataset crate (working name `cmem-eval-continuity`, subject to the revised architecture); `MemoryAdapter` trait extension for correct/forget/link and prepare/validate/commit (mock + live); controllable-similarity deterministic embedding provider; seeded fixture generator + checked-in fixtures with relevance labels; continuity driver loop + `run continuity` CLI; seven continuity metrics with `metric_support`/registry semantics; restart measurement; configs + README.
  - [CM]: ADR recording harness placement (ADR-I-0018 revisit trigger); time-injectability + telemetry gap + restart-identity audit; additive-only telemetry/time-seam changes if the audit finds gaps (user-approved); version bump to 0.1.4; roadmap bookkeeping with private-repo-aware wording.
- Delegation boundary (established suites): LongMemEval/LoCoMo scoring stays untouched. They default to OpenAI embeddings and external LLM enrichment/QA passes, so they cannot serve as the continuity harness; none of the seven continuity metrics, staged writes, correct/forget/link, or restart behavior are measurable through them. Delegated instead: runner/CLI scaffolding, JSONL/summary report writers, metric registry + `metric_support` (null-never-false-zero) semantics, recall/mrr/ndcg@k machinery, adapter/namespace/config plumbing.
- Non-goals (phase doc + [CME] conventions): no learned retrieval policy, no model-graded scoring or LLM judges, no live LLM calls, no CI-blocking quality gates, no public benchmark, no new memory object types, no new public memory facade methods, no retrieval behavior/default changes (v0.1.5), no dashboards, no fixture DSL beyond scenario needs, no metric plugin system, no soak/perf infra, no changes to LongMemEval/LoCoMo scoring, no docker-compose provisioning in [CME], no making [CME] public.

## Context (workspace)

[CM]:
- Phase design: `docs/design/roadmap-phases/v0_1_4_continuity_evaluation_harness.md`; roadmap §10 (`docs/roadmap/development_roadmap.md` lines 1087–1150); no-new-facade constraint (lines 1614–1618). Philosophy: `docs/project_philosophy.md` §12 success criteria, entity-neutrality (§2.4, §8).
- Facade complete for the loop: `src/memory.rs` (prepare/validate_plan/commit/ remember/link/retrieve/correct/forget). Telemetry surface: `src/api/types/retrieval.rs` (RetrievalRationale/Telemetry/Trace, Selectivity*, GraphExpansion*, LifecycleFilterDecision, SectionAssignment; serde round-trippable).
- ADR-I-0018 layout: harness placement outside the crate resolves the "no unambiguous home" revisit trigger; record via new ADR.
- Restart knobs: `GRAPH_STORE_MODE` (service/persistent/in_memory), `RETRIEVAL_STATS_STORE_MODE` (sqlite/in_memory) in `src/config/app_settings.rs`.

[CME] (as surveyed 2026-07-05; layout may change in the Task_4 revision):
- Rust workspace, edition 2024: `cmem-eval-core` (MemoryAdapter trait + mock, metrics, results/report writers, config), `cmem-eval-longmemeval`, `cmem-eval-locomo`, `cmem-eval-runner` (bin `cmem-eval`, clap subcommands, live `CharacterMemoryAdapter` behind feature `real-character-memory`).
- Consumes `character_memory` as a sibling-directory path dependency (workspace `Cargo.toml` line 26).
- Live adapter (`crates/cmem-eval-runner/src/real_adapter.rs`): per-namespace `CharacterMemory` instances; deterministic hash-bucket `DeterministicEmbeddingProvider` (lines 1332–1358, not similarity-controllable); Oxigraph defaults to in-memory (line 122); only `remember` + `retrieve` exercised today; external_id↔MemoryId maps are adapter-local (lost across restart).
- Reusable: `run synthetic` pattern (`commands.rs:63–68,152`), JSONL/summary writers + `summarize` command, metric registry with `metric_support` semantics, integrity metrics (`suppressed_memory_leakage_rate`, `superseded_current_leakage_rate` in `results.rs:97–118`).
- Repo rules: own `docs/coding-agent/rules/`; validation commands `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, mock synthetic smoke run. No CI, no toolchain pin.
- Git state: cleaned up, on `main` (user-confirmed 2026-07-05).
- Repo reference docs consulted: `docs/coding-agent/rules/common.md` and `docs/coding-agent/rules/orchestrator.md` in both repos.

## Open Questions (max 3)

- None currently. (Q1 branch sequencing: resolved — [CME] is clean on main. Q2 restart identity: resolved — public-API re-association approach approved; facade-level proposal only if the Task_2 audit proves it infeasible.)

## Assumptions

- A1 (user-approved): harness home is [CME]; the continuity harness lands as a new dataset crate under whatever layout the Task_4 architecture revision produces.
- A2 (user-approved): additive-only telemetry fields on existing public types are acceptable; "no new facade APIs" means no new methods on `CharacterMemory`.
- A3 (user-approved): crate version bumps to 0.1.4 at milestone completion.
- A4: local Qdrant (and persistent Oxigraph/sqlite paths) are acceptable eval-run dependencies; "no external calls" covers LLM/embedding providers only. Continuity config validation hard-rejects non-deterministic embedding providers.
- A5: fixtures are generated by a seeded generator and checked in (diff-stable); the generator is rerun only when scenarios change.
- A6: `MemoryAdapter` trait expansion (not facade-direct driving) is the house-consistent choice — the trait is [CME]'s documented API boundary. The Task_4 revision may refine this; if it removes the trait, Task_5/Task_6 re-scope (replan trigger).
- A7: code identifiers use stable domain names (`continuity`, `cmem-eval-continuity`), never version labels (repo naming rule).
- A8 (user-directed): [CM] committed artifacts contain no machine-local absolute paths and word [CME] mentions for public readers (private repo, development aid, not core functionality).

## Tasks

### Task_1: [CM] ADR: continuity harness placement in the private evals repository

- type: design
- owns:
  - docs/decisions/implementation/ADR-I-0019-continuity-eval-harness-placement.md
  - docs/decisions/ (index update only, if an index exists)
- depends_on: []
- description: |
  Write ADR-I-0019 resolving the ADR-I-0018 "no unambiguous home" revisit trigger: the continuity evaluation harness lives in the private companion repository CharacterMemoryEvals as a dataset crate, consuming character_memory strictly through the public API via a sibling-checkout path dependency. Record alternatives considered (in-repo evals/ workspace crate, examples/, feature-gated module) and why rejected; record the delegation boundary vs LongMemEval/LoCoMo; note the cross-repo versioning implication and the crate-version bump policy. Wording constraints (A8): no machine-local absolute paths; state plainly that the evals repository is private and evaluation tooling is a development aid rather than core library functionality, so public readers are not confused by an inaccessible reference.
- acceptance:
  - ADR-I-0019 exists with decision, alternatives, delegation boundary, and cross-repo versioning notes; entity-neutral; no version labels in identifiers.
  - ADR satisfies A8: no local paths; private-repo wording present.
  - ADR cross-references the phase doc and ADR-I-0018 revisit trigger.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CM] cargo fmt --check && cargo check (docs-only change; confirms no accidental code touch)"
  - kind: review
    required: true
    owner: orchestrator
    detail: "ADR matches plan decisions, phase-doc constraints, and the A8 wording rule"

### Task_2: [CM] Time-injectability, telemetry-gap, and restart-identity audit

- type: research
- owns:
  - docs/coding-agent/plans/active/v0-1-4-continuity-evaluation-harness-plan.md (findings appendix only)
- depends_on: []
- description: |
  Three audits, findings appended to this plan: (1) Time: enumerate every wall-clock read (Utc::now or equivalent) in the retrieval/scoring path; determine whether months-scale-gap fixtures work with caller-provided timestamps end-to-end or a reference-time seam is needed. (2) Metrics: for each phase-doc §7.1–7.7 metric, verify computability from the existing public telemetry (`src/api/types/retrieval.rs`) plus fixture labels; enumerate missing fields (per-relation fanout budget-vs-utilization, conservative-fallback activation events, rationale category taxonomy coverage). (3) Restart identity (approved direction): verify an eval harness can re-associate fixture external ids with MemoryIds after dropping and reconstructing CharacterMemory over the same persistent stores using only the public API; if infeasible, characterize the blocking gap precisely (replan trigger with facade-level proposal). Output: findings table (metric -> inputs -> available? -> gap) and a concrete additive-only change list for Task_3 (possibly empty). Findings must not contain machine-local absolute paths (A8).
- acceptance:
  - Every §7 metric has a computability verdict with type/field references.
  - Wall-clock usage in the retrieval path enumerated with file:line references.
  - Restart-identity approach documented as public-API-feasible, or the blocking gap precisely characterized.
  - Task_3 change list is concrete or explicitly empty.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Findings reviewed; Task_3 scope confirmed or Task_3 closed as not-needed"

### Task_3: [CM] Additive telemetry/time-injection changes (conditional)

- type: impl
- owns:
  - src/api/types/retrieval.rs
  - src/usecases/retrieve.rs
  - src/policy/**
  - tests/**
- depends_on: [Task_2]
- description: |
  Implement exactly the additive-only changes approved from Task_2 findings: new optional telemetry fields and/or an internal reference-time seam. Hard constraints: no retrieval behavior change, no default changes, no new facade methods, backward-compatible. If Task_2 finds no gaps, close with zero diff.
- acceptance:
  - All approved gap items implemented; existing tests pass unchanged (except mechanical additions).
  - New fields covered by serde round-trip tests where applicable.
  - No new public facade methods on `CharacterMemory`.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CM] cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run"
  - kind: test
    required: true
    owner: worker
    detail: "[CM] cargo test (unit scope; integration vs local Qdrant if available)"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review: additive-only, no behavior/default change; ADR-I-0018 dependency-direction audit"

### Task_4: [CME] Architecture review and revision (gating pre-phase; own plan in CME)

- type: design
- owns:
  - (CME repo) docs/coding-agent/plans/** (new architecture-revision plan)
- depends_on: []
- description: |
  The evals repository was assembled quickly; before new features land there, run an architecture review and revision INSIDE that repository under its own execution plan, per that repo's own rules and plan lifecycle. This task, in this plan, tracks only the gate. Steps: (1) dispatch research into architectural debt (crate boundaries, MemoryAdapter contract shape, config/report format evolution, feature-gating strategy, determinism discipline, toolchain pinning, test coverage); (2) draft the revision plan in the CME repo's plans/active/; (3) obtain user approval for that plan; (4) execute it to completion there. Continuity feature design inputs (Tasks 5–12 here) are explicit stakeholders of the review: the revised architecture must have an intended home for a continuity dataset crate, an extensible adapter contract, and a stable report/metric registry story.
- acceptance:
  - A CME architecture-revision plan exists in that repo, user-approved, executed to completion (its own Definition of Done met), CME main green.
  - The revised architecture documents where a continuity dataset crate lands and how the adapter contract extends (or replaces) MemoryAdapter.
  - This plan's Decision Log records any re-scoping of Tasks 5–12 the revision causes (replan trigger if A6 or task owns change materially).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace (green on CME main at gate close)"
  - kind: review
    required: true
    owner: orchestrator
    detail: "CME revision plan completed per its own lifecycle; continuity-stakeholder requirements satisfied; re-scoping needs recorded here"

### Task_5: [CME] Extend adapter contract: correct/forget/link + staged writes

- type: impl
- owns:
  - (CME repo) adapter-contract module per post-revision layout (pre-revision: crates/cmem-eval-core/src/memory_adapter.rs)
- depends_on: [Task_4]
- description: |
  Extend the adapter contract (and its mock implementation) with methods mirroring the character_memory facade: correct, forget, link, and the staged write path prepare/validate_plan/commit. Mock implementations must be deterministic and sufficient for service-free smoke runs. Follow the contract shape the Task_4 revision lands (A6).
- acceptance:
  - Contract methods mirror `character_memory` facade semantics (append-only correction: supersession/suppression, no deletion).
  - Mock implements all new methods deterministically with unit tests.
  - Existing LongMemEval/LoCoMo code compiles unchanged (default-method or explicit-impl strategy chosen deliberately).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace"

### Task_6: [CME] Live adapter: new contract methods + deterministic-provider enforcement + persistent-store config

- type: impl
- owns:
  - (CME repo) live adapter + config modules per post-revision layout (pre-revision: crates/cmem-eval-runner/src/real_adapter.rs, crates/cmem-eval-core/src/config.rs)
- depends_on: [Task_5]
- description: |
  Implement the new contract methods on the live CharacterMemory adapter (correct/forget/link/prepare/validate_plan/commit) with external-id round-trip mapping; correct candidate provenance on plan-path writes (ADR-I-0015). Add continuity dataset-kind config validation that hard-rejects non-deterministic embedding providers. Add persistent-store configuration (Oxigraph persistent path, sqlite stats path) and an adapter reconstruct path for restart scenarios (drop + rebuild CharacterMemory over the same stores, re-associating ids via the public API per the Task_2 finding).
- acceptance:
  - All new facade operations round-trip external ids against live stores.
  - Continuity config with a non-deterministic provider fails validation loudly.
  - Adapter reconstructs against existing persistent stores and re-associates ids using only the public API.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace"
  - kind: test
    required: true
    owner: worker
    detail: "[CME] Live smoke vs local Qdrant: staged write + correct + forget + link round-trip in a scratch namespace"

### Task_7: [CME] Controllable-similarity deterministic embedding provider

- type: impl
- owns:
  - (CME repo) new deterministic-embedding module per post-revision layout
- depends_on: [Task_4]
- description: |
  New deterministic `EmbeddingProvider` whose similarity structure is fixture-controllable: fixtures declare concept/cluster assignments so scenario authors can pin near/far relationships (fixture-declared cluster vectors + deterministic seeded noise). Seeded, no I/O, stable across runs/platforms (sorted iteration, no platform-dependent float paths). Small configurable vector_size for fixture compactness. Entity-neutral. The existing hash-bucket provider stays for LongMemEval/LoCoMo compatibility.
- acceptance:
  - Identical inputs + seed => byte-identical embeddings across two process runs.
  - Cosine ordering between fixture-declared concept pairs is controllable and verified by property tests.
  - No role/name special-casing.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace"

### Task_8: [CME] Continuity fixture schema, seeded generator, scenario library

- type: impl
- owns:
  - (CME repo) continuity dataset crate (fixture + generator modules) + checked-in fixture data + workspace-member addition
- depends_on: [Task_4, Task_7]
- description: |
  Scaffold the continuity dataset crate in the post-revision layout. Fixture schema (serde, versioned): scripted interaction events with caller-provided timestamps (months-scale gaps), heterogeneous role-free entity declarations (≥3 entity kinds with high-degree hubs), embedding concept assignments (Task_7), per-query expected-relevance labels, stable fixture IDs, stable namespace/collection names. Seeded generator + scenario library covering phase-doc patterns: long-gap recall, recurring hub entity, selective entity, correction chains (supersession/suppression), thread drift, temporal structure, mixed-salience accumulation, cross-store stress (restart between write and retrieve; stats reopen; graph reopen). Generated fixtures checked in; generator determinism tested. No DSL beyond scenario needs.
- acceptance:
  - Same seed => byte-identical fixture set (two-run identity test).
  - All listed scenario patterns present; hub scenarios span ≥3 entity kinds.
  - No fixture rule special-cases roles or entity names.
  - Per-query relevance labels present for recall- and pollution-scored scenarios.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace"

### Task_9: [CME] Continuity driver loop + `run continuity` subcommand

- type: impl
- owns:
  - (CME repo) continuity driver module + runner subcommand wiring + runner dependency additions
- depends_on: [Task_6, Task_8]
- description: |
  Fixture-scripted deterministic driver exercising the full adapter contract: remember, prepare/validate_plan/commit, retrieve, correct, forget, link — no meaning inference (ADR-I-0013), every action scripted by the fixture. Captures full retrieval traces/rationale/telemetry per query in serializable form. Wire a `run continuity` subcommand following the existing dataset-run pattern, supporting mock (service-free smoke) and real adapters.
- acceptance:
  - All eight facade operations exercised across the scenario library.
  - Driver reads no wall-clock of its own; deterministic given fixture + config.
  - Mock smoke run completes service-free; live run completes vs local Qdrant.
  - Per-query traces captured for metric computation.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace"
  - kind: test
    required: true
    owner: worker
    detail: "[CME] Mock smoke run (service-free) + live one-scenario run vs local Qdrant; doubles as Qdrant run-to-run reproducibility probe (two runs, diff reports)"

### Task_10: [CME] Seven continuity metrics + registry integration

- type: impl
- owns:
  - (CME repo) continuity metrics module + additive result/summary/registry extensions in the shared core
- depends_on: [Task_3, Task_9]
- description: |
  Implement phase-doc §7 metrics over captured traces + fixture labels: 7.1 continuity recall@k by gap-length bucket (reuse recall machinery); 7.2 entity continuity without flooding (context-pack share, hub-expansion hits, fanout utilization vs cap); 7.3 temporal retrieval quality; 7.4 correction safety (zero suppressed/superseded admitted; supersession replacement — extend existing leakage metrics); 7.5 rationale quality (coverage + category distribution); 7.6 context pollution rate attributed by rationale category; 7.7 fanout discipline (zero over-budget expansions, conservative-fallback activations, selectivity distributions). Integrate with `metric_support`/registry semantics (null, never false zero). Deterministic ordering (BTreeMap/sorted). Measurements, not pass/fail gates. Entity-neutral.
- acceptance:
  - Each §7 metric implemented with a unit test vs hand-computed expectations.
  - Metric code references only fixture labels + telemetry; no entity special-cases.
  - Selectivity/fanout outputs usable for v0.1.5 default tuning (values + config snapshot correlation).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace"

### Task_11: [CME] Restart measurement phase + report format

- type: impl
- owns:
  - (CME repo) continuity restart-orchestration + report-assembly modules
- depends_on: [Task_6, Task_9]
- description: |
  Restart orchestration for cross-store scenarios: mid-scenario drop + reconstruct of CharacterMemory over persistent Oxigraph + sqlite stats (Task_6 reconstruct path), post-restart retrieval re-measurement, restart deltas recorded. Continuity report assembly: run metadata (fixture set + seeds, config snapshot incl. selectivity/fanout knobs, schema versions, timestamp isolated in a metadata block), per-scenario + aggregated metrics, per-query rationale samples, fanout decisions, stats health events, restart observations. Reports diffable across runs: stable key order, nondeterminism confined to the metadata block. Reuse JSONL/summary writer infra.
- acceptance:
  - Two consecutive runs on identical fixture + config produce reports identical outside the metadata block.
  - Report includes all phase-doc report-format elements.
  - Restart scenarios record post-restart observations against persistent stores.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace"
  - kind: test
    required: true
    owner: worker
    detail: "[CME] Live end-to-end run incl. restart scenarios vs local Qdrant + persistent Oxigraph/sqlite; two-run report diff empty outside metadata block"

### Task_12: [CME] Configs, README, and extension documentation

- type: docs
- owns:
  - (CME repo) continuity run configs + README continuity section (+ crate README if house style splits docs)
- depends_on: [Task_10, Task_11]
- description: |
  Run configs for continuity (deterministic provider mandatory, small vector_size, persistent-store paths for restart scenarios). README: prereqs (local Qdrant; persistent paths), commands, report location/reading, extending the scenario library and metrics. Non-anthropomorphic tone. State explicitly: the harness observes and reports; it is not a CI gate; metric thresholds are not pass/fail.
- acceptance:
  - A newcomer can run mock smoke + live continuity eval from docs alone.
  - Extension guide covers adding a scenario and adding a metric.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Docs accuracy vs implemented CLI/config/report format"

### Task_13: [CM] Version bump and roadmap bookkeeping

- type: chore
- owns:
  - Cargo.toml (version field only)
  - docs/roadmap/development_roadmap.md (version table row status + harness-location note only)
- depends_on: [Task_10, Task_11]
- description: |
  Bump `character_memory` version 0.1.2 -> 0.1.4 (covers finished v0.1.3 + this milestone; user-approved). Update the roadmap version table row for v0.1.4 at closeout. Where the roadmap notes the harness location, follow A8 wording: the continuity evaluation harness is implemented in the private companion CharacterMemoryEvals repository (a development aid, not core library functionality) — so future readers neither assume it is unimplemented nor look for it in this repository. No machine-local paths. Tagging policy stays with the user.
- acceptance:
  - Cargo.toml version = 0.1.4; lockfiles re-resolve cleanly in both repos.
  - Roadmap reflects v0.1.4 status and harness location per A8 wording.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CM] cargo check; [CME] cargo check (path dep re-resolves cleanly)"

### Task_14: Independent review and acceptance verification (both repos)

- type: review
- owns: []
- depends_on: [Task_12, Task_13]
- description: |
  Reviewer independently verifies all phase-doc acceptance criteria (lines 304–317) with evidence per criterion; reruns the two-run reproducibility check (reports identical outside metadata block); runs the ADR-I-0018 dependency-direction audit over all [CM] src/ diffs; verifies [CME] changes import character_memory only through the public API; checks entity-neutrality of fixtures/metrics; confirms no library behavior/default change (existing [CM] test suite green and unmodified except mechanical additions); scans all [CM] committed artifacts from this plan for machine-local absolute paths and A8 wording compliance.
- acceptance:
  - Reviewer status APPROVED with per-criterion evidence.
  - Dependency-direction audit clean in [CM]; public-API-only usage confirmed in [CME].
  - A8 compliance confirmed: no local paths in [CM] artifacts; private-repo wording present where [CME] is mentioned.
- validation:
  - kind: test
    required: true
    owner: reviewer
    detail: "Independent two-run reproducibility check vs local Qdrant + persistent stores"
  - kind: review
    required: true
    owner: reviewer
    detail: "Acceptance-criteria checklist + dependency-direction audit + entity-neutrality check + A8 path/wording scan"

## Task Waves (explicit parallel dispatch sets)

Interpretation: tasks in the same wave dispatch in parallel by default when `owns` are disjoint and dependencies are met; waves execute sequentially. Cross-repo tasks ([CM] vs [CME]) never share owns. Task_4 is a gate: its wave completes only when the CME architecture-revision plan (its own plan, in that repo) is done.

- Wave 1 (parallel): [Task_1, Task_2, Task_4]
- Wave 2 (parallel): [Task_3, Task_5, Task_7]
- Wave 3 (parallel): [Task_6, Task_8]
- Wave 4 (parallel): [Task_9]
- Wave 5 (parallel): [Task_10, Task_11]
- Wave 6 (parallel): [Task_12, Task_13]
- Wave 7 (parallel): [Task_14]

Note: Task_4's research can start immediately (Wave 1), but Wave 2's [CME] tasks (Task_5, Task_7) wait for the full Task_4 gate (revision plan approved + executed), which may span multiple working sessions. [CM] Task_3 proceeds independently.

## Rollback / Safety

- [CME] changes are additive (new crate + contract/adapter/config extensions) on top of whatever the architecture revision lands; rollback = revert the feature branch there.
- Task_3 is the only [CM] src/ touch; additive-only with pre-existing tests required to pass unchanged — clean revert.
- No changes to library defaults, retrieval behavior, or public facade methods.
- Eval collections/namespaces use distinct stable names — no collision with LongMemEval/LoCoMo namespaces or [CM] integration-test collections.

## Progress Log (append-only)

- 2026-07-05 12:15 Wave 1 (partial) completed: [Task_1, Task_2]
  - Summary: Task_1 done — ADR-I-0019 created + decisions README index updated; worker validation pass (cargo fmt --check && cargo check), orchestrator review pass (A8 wording/path scan clean). Task_2 done — findings appendix appended to this plan; orchestrator review pass; Task_3 scope confirmed non-empty (three additive telemetry items; no time seam; no facade method).
  - Validation evidence: worker YAML reports (Task_1: fmt/check pass, path/wording rg scan pass); Task_2 appendix content verified in-plan.
  - Notes: Task_4 gate remains open — CME architecture-revision plan drafted in the CME repo, awaiting user approval there. Task_3 dispatched (see Decision Log entry on restart identity).
- 2026-07-05 13:20 Execution paused by user (host resource pressure from codex-monitor watchers).
  - Summary: codex-monitor watcher processes killed; no further dispatch until resume. In flight at pause: [CM] Task_3 (worker, report pending — will be integrated on arrival, no new dispatch). CME plan approved and in_progress; its Task_1 NOT yet executed — first dispatch attempt was blocked by worker sandbox rooted in the [CM] checkout; a CME-rooted worker (worker3 registered, session not yet spawned) or a sandbox-scope decision is needed at resume.
  - Validation evidence: n/a (operational note).
  - Notes: resume checklist — (1) restart codex-monitor watchers for active worker threads (endpoint/thread ids may change), (2) resolve CME worker hosting, (3) dispatch CME Task_1, then continue waves.
- 2026-07-05 13:25 Task_3 completed (report integrated post-pause; no new dispatch).
  - Summary: additive serde-defaulted telemetry landed — FanoutUtilizationTrace (retained/omitted counts around existing graph-expansion pruning) and RationaleCategory (stable enum populated from existing structural decisions); re-exported from crate root; no facade/ranking/default changes. Scope item 3 (fallback event detail) intentionally not implemented: SelectivityTrace.fallback (per-decision) + telemetry fallback_count (aggregate) already suffice — judgment accepted. Worker also fixed the Qdrant skip predicate in tests/support/base.rs to match the real gRPC connection-refused error shape (within owns).
  - Validation evidence: fmt/check/clippy(-D warnings)/test --no-run pass; cargo test pass (339 lib tests, 3 ignored live-service; Qdrant-dependent integration tests exercised the skip-return path — local Qdrant was down, so live integration remains for Reviewer/live runs).
  - Notes: Reviewer-owned required validation for Task_3 (additive-only diff review + ADR-I-0018 dependency-direction audit) is PENDING — deferred by the pause; must run before Task_3 is closed at the Validation Gate. Worker lesson candidate promoted to repo rule (worker.md): match concrete client error shapes in skip-if-unavailable predicates.

- 2026-07-11 12:15 Resume + Task_3 Reviewer validation completed — Task_3 CLOSED.
  - Summary: execution resumed. Reviewer status APPROVED on the Task_3 working-tree diff: additive-only confirmed (pruned link set byte-identical via delegation to unmodified `apply_fanout_limits`; new fields serde-defaulted with legacy-payload deserialization test; facade unchanged — exactly the eight public methods); ADR-I-0018 dependency-direction audit clean (no new inverted edges; no ports/policy import of usecases); absolute-path scan clean.
  - Validation evidence: cargo fmt --check pass; cargo check pass; cargo clippy --all-targets -D warnings pass; cargo test --no-run pass; cargo test pass (lib 339/0 failed/3 ignored; integration 25 passed/0 failed, Qdrant tests via pass-or-skip path).
  - Notes: three MINOR non-blocking follow-ups recorded for the cleanup backlog: (1) fanout utilization computed even when no trace requested (since implemented in e58434c — trace-gated; see the 2026-07-11 20:10 entry), (2) pre-existing ports/policy imports of domain types via `crate::api::types` instead of `crate::domain` — one-time sweep candidate per ADR-I-0018, (3) doc comment on `FanoutUtilizationTrace::selected_cap` clarifying global-truncation interaction. Reviewer lesson candidate promoted to repo rule (orchestrator.md): run the ADR-I-0018 audit diff-scoped for incremental reviews. [CM] Wave 1–2 work now fully validated; changes remain uncommitted pending the commit/PR stage. Task_4 gate still open: cdxm CLI found missing from the machine at resume (source checkout deleted); CME worker hosting decision (reinstall cdxm vs agmsg codex bridge/spawn vs claude-code worker) sent to user via agmsg — CME Task_1 dispatch blocked on that answer.

- 2026-07-11 19:10 Task_4 gate CLOSED: the [CME] architecture-revision plan is completed (its Task_8 independent review APPROVED; plan moved to its plans/completed/). The revised state lives on the CME feature branch eval-harness-architecture-revision, green under the full pinned gate; merge to CME main is pending the user's PR decision. Post-revision layout facts for Tasks 5-12 owns: live adapter crate crates/cmem-eval-adapter-cmem; adapter contract in crates/cmem-eval-core/src/memory_adapter.rs (Character-Memory-shaped main trait); DatasetSpec seam in crates/cmem-eval-runner/src/pipeline.rs; continuity crate home documented as crates/cmem-eval-continuity; reports schema_version 1.0.0 with segregated latency; restart identity via BTreeMap ExternalIdRegistry + deterministic (prefix, run_id, namespace) collection naming with open/reattach lifecycle.

- 2026-07-11 20:10 PR #59 review-fix cycle complete. Copilot raised six findings: two (new public fields on trace structs technically breaking exhaustive construction) resolved by policy reply on the PR (accepted per user-approved A2, pre-1.0 crate, consumers compile green, serde-compat tested; non_exhaustive rejected because the evals repo constructs trace fixtures); four fixed in commit e58434c by the CM codex worker — fanout-utilization computation now gated behind a crate-private default-false query flag (untraced retrievals take the original fast path; closes the Task_3 review backlog item), stale-reason mapping computed once per site, skip-predicate early return. Independent Tier D review (cm-reviewer, per the new delegation routing) APPROVED with zero findings: flag surface pub(crate)-only, call-site census proves no include_trace path omits utilization, parity regression compares full expansion content on a pruning-active fixture, predicate boolean-equivalent, diff-scoped ADR-I-0018 audit clean; full validation green with live Qdrant integration tests (340 unit + 25 integration, no skips). Worker push was blocked by codex approval layer — orchestrator pushes by default for CM tasks now.

- 2026-07-12 01:20 Task_4 merge precondition CLOSED; Task_5 CLOSED as satisfied-by-CME-revision.
  - Summary: CM PR #59 and CME PR #8 both merged by the user; CME main green at e53fc76 (hosted pinned CI passed pre-merge, including the post-review hardening deltas through dd668ef). The [CME] feature-dispatch precondition from the 2026-07-11 gate decision is now met. Task_5 dispatch-time acceptance cross-check performed: the adapter contract and its mock both implement link/correct/forget/prepare/validate_plan/commit (crates/cmem-eval-core/src/memory_adapter.rs — trait at lines 504–512, mock impls at 612–896); LongMemEval/LoCoMo compile unchanged on merged main.
  - Validation evidence: CME hosted CI green on the merged PR; local CME main fast-forwarded to e53fc76.
  - Notes: Wave 2 remaining item Task_7 (controllable-similarity deterministic embedding provider) dispatched to evals-worker on a new CME feature branch. Wave 3 (Task_6 residual, Task_8) queues behind it.

- 2026-07-12 01:35 Task_7 completed and CLOSED (reviewer APPROVED, no blocking findings) — Wave 2 complete.
  - Summary: controllable-similarity deterministic embedding provider landed as a new cmem-eval-core module (controllable_similarity_embedding.rs) with additive crate-root export; existing hash-bucket provider untouched. Fixture contract: BTreeMap clusters with exact f32 base vectors, exact input-to-concept assignments, u64 seed, configurable vector_size/noise, controlled validation errors. Determinism via fixed-width FNV-1a + SplitMix64 and ternary f32 noise addition only.
  - Validation evidence: worker and reviewer independently green under pinned 1.97.0 (fmt --all --check, clippy --workspace -D warnings, test --workspace incl. cross-process byte-identity child probes); reviewer verified all five acceptance points with line-level evidence on range e53fc76..f84b52a (branch feature/controllable-similarity-embedding).
  - Notes: reviewer residuals (non-blocking): multi-platform CI and reassignment sweep across all 128 seeds would amplify confidence. Wave 3 dispatched: Task_6 residual (continuity config validation + persistent-store paths) and Task_8 (fixture schema/generator/scenario library).

## Decision Log (append-only; re-plans and major discoveries)

- 2026-07-11 Decision: Task_4 gate criteria clarified after PR review flagged a contradiction (gate recorded closed while CME main-green acceptance was unmet). Revised criteria: the gate's substantive conditions (revision plan completed per its own lifecycle, independent review APPROVED, full validation green on the revision state, continuity-stakeholder requirements satisfied) are met on the CME feature branch with its PR open and hosted CI green; the residual condition — merge to CME main — is user-controlled and remains a HARD PRECONDITION for dispatching any [CME] feature task of this plan (Tasks 5–12). The gate is therefore "satisfied pending merge": [CM]-side tasks may proceed; no [CME]-side task dispatches until the CME PR merges and main is green. Rationale: the merge decision belongs to the user, and holding [CM]-side design work hostage to it serves no safety purpose; the original acceptance intent (no feature work on an unrevised/unverified CME base) is preserved by the dispatch precondition.
- 2026-07-11 Decision: re-scoping after the Task_4 gate closed (A6 replan check). The CME revision ALREADY LANDED the substance of this plan's Task_5 (adapter contract correct/forget/link + staged writes with mock parity — CME revision Task_2) and most of Task_6 (live adapter methods, external-id round-trip mapping incl. ADR-I-0015 provenance, reattach lifecycle — CME revision Tasks 2-3, with live Qdrant reattach evidence). Plan delta: Task_5 here closes as satisfied-by-CME-revision after an acceptance cross-check at dispatch time; Task_6 re-scopes to residual gaps only — continuity dataset-kind config validation hard-rejecting non-deterministic embedding providers, and persistent Oxigraph/sqlite-stats path configuration for restart scenarios (the CME reattach work exercised Qdrant; graph/stats persistence config likely remains). Wave structure otherwise intact; Task_7 (controllable-similarity provider) unaffected. User approval: within the already-approved plan direction; no owns/acceptance change beyond narrowing.
- 2026-07-05 Decision: initial draft from [CM]-side research (phase doc, philosophy, ADRs, module tree, CI). Recommended in-repo `evals/` crate.
  - User approval: superseded by the revisions below.
- 2026-07-05 Decision: plan revised after user direction + [CME] survey.
  - Trigger / new insight: user redirected harness home to the existing sibling evals repo (directory `CharacterMemoryEvals`); survey found a pure-Rust workspace with reusable runner/report/metric infra and a path dependency on character_memory.
  - Plan delta: harness re-homed to [CME] as a continuity dataset crate; delegation boundary defined (LongMemEval/LoCoMo cover none of the continuity metrics/facade coverage/restart — delegate infra only); adapter-contract expansion chosen over facade-direct driving (A6); restart-identity audit added to Task_2; additive telemetry approved (A2); version bump approved (A3).
  - Tradeoffs considered: in-repo evals/ crate (rejected: duplicate infra, established home exists); expanding the adapter contract vs bypassing it (chosen: house-consistent, keeps mock smoke coverage).
  - User approval: superseded by the revision below.
- 2026-07-05 Decision: second revision after user answers.
  - Trigger / new insight: (a) [CME] cleaned up and on main (former Q1 resolved); (b) public-API restart-identity approach approved (former Q2 resolved); (c) new user constraints — no machine-local absolute paths in [CM] committed artifacts, private-repo-aware wording for public readers ([CME] stays private; eval tooling is a development aid, not core functionality); (d) new gating requirement — [CME] architecture revision, planned and executed in that repo under its own plan, before continuity features land there.
  - Plan delta: added Task_4 architecture-revision gate (renumbering Tasks 5–14); all [CME] feature tasks now depend on Task_4; [CME] owns paths marked "per post-revision layout"; A8 wording/path constraint added and threaded through Task_1/Task_2/Task_13/Task_14 acceptance; repo rule added to `docs/coding-agent/rules/common.md` (sibling-repo reference + wording rules, local-path-free); open questions cleared.
  - Tradeoffs considered: folding the architecture revision into this plan (rejected: user directed it to run in [CME] under its own plan; also keeps this plan's owns/wave structure stable against unknown post-revision layout).
  - User approval: yes (2026-07-05, "Go for it") — plan status moved to in_progress; Wave 1 dispatched.
- 2026-07-05 Decision: restart-identity mechanism refined per Task_2 audit.
  - Trigger / new insight: audit found pure store-side re-association infeasible via public API (no lookup/enumeration facade methods; retrieval rediscovery is relevance-dependent). However, public draft types accept caller-supplied MemoryIds, and RememberOutcome returns persisted ids — so the harness can use stable fixture-derived MemoryIds at write time (plus externally persisted mapping as belt-and-braces), making post-restart re-association unnecessary.
  - Plan delta: restart identity = caller-supplied deterministic MemoryIds + harness-persisted mapping as primary mechanism; store-side rediscovery via retrieval used only as verification. No facade change needed — stays within the user-approved public-API direction. CME architecture plan assumption A1 updated to match. Task_3 confirmed scope: (1) per-relation/object fanout utilization telemetry, (2) stable rationale-category enum per included/omitted object, (3) optional stats health/fallback event detail — all optional, serde-defaulted, additive-only; explicitly NO reference-time seam, NO facade methods. Correction/forget mutation timestamps use wall-clock — CME reports must normalize/exclude mutation metadata from diffable content (noted for Tasks 9–11).
  - Tradeoffs considered: facade lookup method (rejected: violates no-new-facade constraint; unnecessary given caller-supplied ids); retrieval-only rediscovery (rejected as primary: incomplete by design).
  - User approval: within previously approved direction; no new approval needed.

## Notes

- Risks:
  - The Task_4 architecture revision may change [CME] crate boundaries or the adapter-contract shape; Tasks 5–12 owns are declared "per post-revision layout" and re-scope via the Decision Log if the revision lands a materially different structure (replan trigger, per A6).
  - Qdrant run-to-run reproducibility assumed, verified early (Task_9 two-run probe); if unstable, report diffing may need tolerance comparison — replan trigger.
  - Wall-clock reads in retrieval scoring (Task_2) may force a reference-time seam; if a facade-visible parameter is unavoidable, that is a replan + user decision.
  - Restart identity re-association may prove infeasible via public API alone (Task_2 audit) — replan trigger with a facade-level proposal.
  - Cross-repo path dependency: [CME] silently tracks the [CM] working tree; sequence [CM] Task_3 before [CME] Task_10 (metrics consume new telemetry fields), which the waves enforce.
- Edge cases:
  - Months-scale timestamp gaps interacting with recency scoring.
  - Correction chains interleaving supersession and suppression on hub entities.
  - Stats store unhealthy/reopened mid-scenario (conservative fallback activation).
  - Restart with in-memory modes misconfigured (must fail validation, not silently measure nothing).

## Task_2 Findings Appendix

### Time Audit

Production retrieval/scoring path verdict: months-scale-gap fixtures can use caller-provided timestamps end-to-end for stored memory time fields; no retrieval reference-time seam is currently needed. Retrieval ranking in `src/usecases/retrieve.rs` combines vector score, graph proximity, and salience only; it does not compute age/recency from wall-clock time. Temporal-quality metrics therefore need fixture labels and stored timestamps, not "now".

Wall-clock reads found:

| Location | Path role | Finding | Verdict |
|---|---|---|---|
| `src/usecases/retrieve.rs` | Retrieval pipeline | No `Utc::now`, `SystemTime::now`, `Local::now`, or `Instant::now` production reads. Retrieval builds traces/telemetry from caller context, vector search, graph expansion, lifecycle filters, and selectivity stats. | No reference-time seam needed for retrieval. |
| `src/policy/retrieval_selectivity.rs` | Fanout/selectivity scoring | No wall-clock reads. Scoring uses persisted stats counters, configured alpha/gamma, candidate vector score, and lifecycle count scope. | No reference-time seam needed for scoring. |
| `src/policy/graph_expansion.rs` | Bounded graph expansion | No production wall-clock reads. Timeout behavior is modeled from `GraphExpansionFailurePolicy.timeout_ms`; the only `Utc::now` hit at `src/policy/graph_expansion.rs:1106` is in a test helper constructing `MemoryLink`. | No reference-time seam needed. |
| `src/adapters/qdrant/store.rs:826` | Qdrant adapter tests | `Instant::now()` appears in an idle-gap canary test around `upsert_points`, not in production retrieval/scoring. | Not relevant to fixture determinism. |
| `src/api/types/draft.rs:23` | Draft default construction | `DraftDefaults::generated()` uses `Utc::now()`, but callers can use explicit draft timestamps and lower-level fixed defaults; public draft fields expose `created_at`, `updated_at`, `started_at`, `ended_at`, `observed_at`, and `last_touched_at` where applicable. | Harness should supply timestamps/IDs explicitly for deterministic writes. |
| `src/usecases/write_planning.rs:33` | Write-plan defaults | `RememberPlanDefaults::generated()` uses `Utc::now()` for facade `prepare`; fixed defaults exist internally, while public `RememberInput` exposes event timestamps but not operation `created_at`. | No retrieval seam; deterministic plan bytes may require CME-side stable IDs/timestamps through draft/commit paths rather than public `prepare`. |
| `src/usecases/correct_forget.rs:881`, `:917`, `:927`, `:942` | Lifecycle mutation writes | Correction/forget mutation timestamps use `Utc::now()` for replacement/update/link metadata. | Not retrieval scoring; if report diffing includes mutation metadata, CME should normalize metadata or Task_3 should add optional telemetry/report exclusions rather than change behavior. |

### Metrics Computability

Public telemetry surface checked: `src/api/types/retrieval.rs` exposes `RetrieveOutcome`, `ContinuityContextPack`, `RetrievalRationale`, `RetrievalTelemetry`, `RetrievalTrace`, `VectorCandidateTrace`, `GraphRelationTrace`, `GraphExpansionTrace`, `SelectivityTrace`, `LifecycleFilterDecision`, `StaleCandidateOmission`, and `SectionAssignment`. Fixture labels can supply expected relevance sets, gap buckets, temporal query kind, hub ids, scenario ids, and expected replacement ids.

| Metric | Inputs needed | Available from public telemetry + fixture labels? | Gap |
|---|---|---|---|
| 7.1 Continuity recall@k by gap bucket | Query label, expected relevant `MemoryId` set, returned pack members, rank/order, fixture gap bucket. | Yes. Pack members expose IDs by section; trace `section_assignments` gives final section and rank-like assignment details; fixture labels supply expected sets and gap buckets. | None for recall@k. Use deterministic report-side ordering by section then trace assignment/rank when needed. |
| 7.2 Entity continuity without flooding | Hub id/kind labels, context-pack membership, hub-incident labels, graph expansion hits, fanout budget utilization vs cap per relation/object pair. | Partial. Pack and `graph_relations` can identify hub-incident retrieved objects; `SelectivityTrace` exposes relation, object type, chosen fanout, and max fanout; `GraphExpansionTrace` exposes aggregate object/relation counts. | Missing exact per-relation/object actual utilization after graph expansion. Current trace shows budget and aggregate expansion counts, but not per relation/object retained edge counts, omitted-by-fanout counts, or cap utilization per pair. |
| 7.3 Temporal retrieval quality | Temporal query type labels, expected relevant IDs, stored object timestamps, returned pack/trace. | Yes, for labeled correctness. Domain objects in the pack carry public timestamps (`Episode.started_at/ended_at`, `Observation.observed_at`, `MemoryThread.last_touched_at`, object `created_at/updated_at` where applicable), and labels define recency/order/interval expectations. | None for label-based correctness; no retrieval-internal temporal signal category exists to explain why a temporal result was admitted. |
| 7.4 Correction safety | Lifecycle labels, suppressed/superseded IDs, returned pack, lifecycle decisions, replacement mapping. | Yes. `LifecycleFilterDecision` exposes retention state, currentness, superseded_by, action, and reason; pack IDs reveal admitted objects; correction outcomes expose mutation IDs during the run. | None for safety/admission metrics. |
| 7.5 Rationale quality | Per-pack-member rationale presence and rationale category distribution across semantic/entity/thread/temporal/salience/scope. | Partial. `RetrievalRationale.summary` exists; `SectionAssignment.reason` is optional free text; trace distinguishes vector candidates, graph relations, lifecycle, stale omissions, selectivity, and section assignment. | Missing stable rationale category taxonomy per included object. Free-text `reason` and structural trace categories are insufficient for the exact requested category distribution. |
| 7.6 Context pollution rate | Expected relevant labels, pack members, rationale category per admitted noise item. | Partial. Pollution share is computable from pack IDs vs fixture labels. | Pollution attribution by rationale category has the same missing taxonomy as 7.5. |
| 7.7 Fanout discipline | Over-budget expansions, conservative fallback activations, selectivity distributions by entity kind. | Partial. `SelectivityTrace` exposes score, entity/global counts, chosen/max fanout, decision, fallback, relation, object type, and count scope; telemetry exposes fallback counts; `GraphExpansionTrace` exposes bounded failure reason/outcome. | Missing explicit per-relation/object actual fanout utilization and omitted-over-budget counts. Conservative fallback activation is available when traces are enabled; if traces are disabled only aggregate fallback count remains. |

### Restart Identity Audit

Verdict: external harness re-association after dropping and reconstructing `CharacterMemory` over the same persistent stores is not fully feasible using only the public `CharacterMemory` API, unless the harness persists the `MemoryId` mapping externally at write time and treats that mapping as authoritative across restart.

Evidence:

- Public write inputs allow caller-supplied IDs: `EntityDraft.id`, `EpisodeDraft.id`, `ObservationDraft.id`, `MemoryThreadDraft.id`, `DerivedMemoryDraft.id`, and `MemoryLinkDraft.id` in `src/api/types/draft.rs`; `RememberOutcome` returns `persisted_object_ids` and `persisted_link_ids`.
- Public retrieval outputs expose IDs and retained external references through pack objects and traces (`src/api/types/retrieval.rs`). Episode/observation `raw_ref` and episode `source_conversation_id` are public domain fields (`src/domain.rs`), so a query that retrieves the relevant object can rediscover its ID.
- Public facade methods in `src/memory.rs` are limited to `prepare`, `validate_plan`, `commit`, `remember`, `link`, `retrieve`, `correct`, and `forget`. There is no public method to look up by external fixture id/raw ref, list all objects, list diagnostic graph objects, or query by `MemoryObjectRef`.
- Diagnostic graph/vector listing exists only behind crate-internal ports/usecases, not the public facade.

Blocking gap: after restart, if the harness has lost its external-id-to-`MemoryId` map, the public API cannot enumerate or directly query persisted objects by external id/raw ref to rebuild the map. Retrieval can opportunistically surface some IDs, but it is relevance-dependent and cannot guarantee complete re-association for all fixture objects/links.

Feasible public-API pattern without library changes: CME should generate stable fixture `MemoryId`s and supply them in drafts, or persist the mapping it receives from write outcomes in its own report/state. On restart, reuse that external mapping and verify persistence by retrieving scripted objects and checking returned IDs/lifecycle traces. This satisfies restart measurement if the external mapping survives the adapter reconstruction; it does not solve re-association from stores alone.

### Additive-Only Change List For Task_3

Recommended Task_3 scope:

1. Add optional, serde-defaulted public telemetry for per-relation/object fanout utilization on retrieval traces, including root, relation, object type, configured cap, selected cap, retained count, and omitted-by-fanout count where the graph expansion layer can report it without behavior changes.
2. Add optional, serde-defaulted public rationale category data per included or omitted object using a stable enum covering at least semantic, entity, thread, temporal, salience, scope, lifecycle, and graph_bound categories. Populate from existing structural decisions only; do not change ranking or section assignment.
3. Add optional, serde-defaulted retrieval stats health/fallback event details if conservative fallback needs to be attributed beyond `SelectivityTrace.fallback` and aggregate `fallback_count`.
4. Do not add a retrieval reference-time seam in Task_3.
5. Do not add a new public facade method for restart identity unless the Orchestrator replans Task_3; the additive public telemetry work is independent, while restart re-association should be handled in CME through stable fixture IDs or external mapping persistence.
