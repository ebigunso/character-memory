# Plan: v0.1.5 Eval-Driven v0.1 Family Closeout

- status: done
- generated: 2026-07-17
- last_updated: 2026-07-19
- work_type: mixed

Repo references: [CM] = this repository (character-memory, public). [CME] = the companion repository `CharacterMemoryEvals`, checked out as a sibling directory of this repo; it hosts the continuity evaluation harness as a development aid (not core library functionality). [CME] was private for most of this plan's execution and was made public on 2026-07-19; historical log entries reflect the status at their time of writing.

## Goal

- Execute the v0.1.5 phase (`docs/design/roadmap-phases/v0_1_5_eval_driven_v0_1_family_closeout.md`): run the v0.1.4 continuity harness across the full v0.1 family surface, record findings with severity and disposition, fix accepted findings with regression coverage, tune the unmeasured v0.1.2 defaults (alpha, gamma, fanout budgets) from sweep data, resolve the entity-root-only selectivity boundary with evidence, and declare the v0.1 family closed for v0.2 entry.
- Precondition owned by this plan: [CME] run configs cannot yet override CM selectivity/fanout knobs, so sweep plumbing lands first.

## Definition of Done

- Findings are recorded in a committed findings register with ID, scenario/metric, observed vs expected, severity, suspected layer, disposition, and rationale per phase doc §3.1; no critical finding is dispositioned accept-as-designed.
- Every fix-now finding is resolved within existing concepts/signals and covered by a regression test in [CM] `tests/` or a regression scenario in the [CME] scenario library, with before/after report references recorded on the finding.
- Deferred findings carry rationale and a target phase usable as v0.2/v0.4/v0.5 planning input.
- Tuned defaults are shipped consistently in both [CM] default sites (`src/policy/retrieval_selectivity.rs` `DEFAULT_FANOUT_SPECS` and `src/config/app_settings.rs` `default_retrieval_fanout_budgets`, plus alpha/gamma defaults) and documented together with the sweep measurements that justified them.
- The selectivity scope boundary (entity-root-only, phase doc §4.1) is widened or re-affirmed with recorded report evidence.
- The final full harness re-run shows no critical findings, and the [CM] structural suite passes unchanged in meaning (tests pinning old defaults updated deliberately alongside the default changes).
- Tuning constraints hold: relation caps remain hard upper bounds, conservative fallback is not weakened, entity-neutrality is not weakened, no per-entity or per-name tuning.
- No new memory object types, no new public facade methods, no new retrieval signals; fixes stay within existing concepts.
- [CM] version bumped to 0.1.5 in a closeout commit; closeout report recorded and linked from the roadmap or plans archive; v0.2 entry explicitly confirmed against it.
- [CM] validation green: `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --no-run`, plus full `cargo test` with live Qdrant. [CME] validation green per its rules: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, continuity mock smoke.
- No committed [CM] artifact contains machine-local absolute paths; [CME] mentions follow the private-repo wording rule.

## Scope / Non-goals

- Scope:
  - [CME]: run-config plumbing for CM selectivity/fanout overrides; findings-register convention; baseline and sweep eval runs; regression scenarios for fix-now findings; confirmation re-run.
  - [CM]: defect fixes in retrieval/selectivity/guardrails/write path/persistence as dispositioned; disposition of the known hub-root-truncation observation; tuned default values in both default sites plus the tests that pin them; selectivity-boundary decision; documentation of tuned values and measurement basis; closeout report, roadmap update, version bump.
- Non-goals (phase doc §2.2/§8): v0.2 continuity concepts, new memory object types, new public facade APIs, new retrieval signals beyond tuning what exists, harness feature growth beyond what findings require, learned/adaptive tuning loops, automatic finding triage, performance work beyond fixing measured defects, speculative refactoring, a multi-config sweep runner (sweeps run as N manual configs), making [CME] public.
- Disposition rule guardrail: if a finding's correct fix needs a new concept or signal (for example widening selectivity to non-entity roots requiring non-entity-keyed stats), it is deferred with a target phase, not fixed here.

## Context (workspace)

- Phase design: `docs/design/roadmap-phases/v0_1_5_eval_driven_v0_1_family_closeout.md`; roadmap §11; philosophy §12 success criteria.
- [CM] tunables: alpha/gamma defaults 1.0 (`src/config/app_settings.rs:430-436`, policy default `src/policy/retrieval_selectivity.rs:69-73`); fanout budgets About→DerivedMemory 0/20, Involves→Episode 0/5, PartOfThread→DerivedMemory 0/15 duplicated at `app_settings.rs:438-457` and `retrieval_selectivity.rs:404-423`; conservative fallback is a fixed formula `max_fanout.min(min_fanout.max(1))` (`retrieval_selectivity.rs:312-314`), not an independent tunable; candidate limits default 48/12 are per-call `RetrievalCandidateLimits::default()` (`src/api/types/retrieval.rs:70-77`), not Settings.
- [CM] selectivity boundary: entity-only guard at `retrieval_selectivity.rs:172-174` plus stats-context load condition at `usecases/retrieve.rs:104-118`; stats counters are entity-keyed, so widening implies a new signal (deferral candidate).
- [CM] root ordering: `usecases/retrieve.rs:1026-1060` sorts score desc then object_type_rank (Episode 0 … Entity 2) then id, truncating at max_graph_roots; the v0.1.4 run evidence recorded that default 12 truncates hub entity roots (known finding F-SEED-1); truncation is observable via `RetrievalTelemetry` root counters.
- [CM] tuning-relevant telemetry confirmed present: `FanoutUtilizationTrace`, `SelectivityTrace`, `SelectivityTelemetry.fallback_count`, `RationaleCategory` (`src/api/types/retrieval.rs`).
- [CM] tests pinning defaults: `usecases/retrieve.rs:2743` (max_graph_roots 12), `app_settings.rs:625-631` (alpha/gamma/budgets); regression-fixture pattern in `tests/retrieval_guardrails_tests.rs:15-116`.
- [CME] state (surveyed 2026-07-17): main clean, harness merged (its PR #9), pinned Rust 1.97.0. Run surface: `run continuity --dataset <fixture> --config <toml> --out/--summary-out/--trace-out/--report-out [--scenario id] [--adapter mock|real]`. `RetrievalConfig` exposes only mode/top-k/include flags/max_vector_candidates/max_graph_roots; the adapter `settings()` fn sets no selectivity/fanout overrides — sweep plumbing required. Report schema 1.0.0 carries per-scenario + aggregate metrics, rationale samples, fanout/selectivity decisions, stats-health events, restart observations, full config snapshot, and a hardcoded hub tuning observation. Eight scenarios: long-gap-recall, recurring-hub-entity, selective-entity, correction-chains, thread-drift, temporal-structure, mixed-salience-accumulation, cross-store-stress. Run artifacts are gitignored (`runs/*`, `reports/*` "unless manually curated"); no diff tooling — comparison is manual with the README normalization/hash recipe. v0.1.4-era full live reports exist locally (round5-live-a/b).
- Known measurement caveat from v0.1.4 runs: conservative fallback dominated hub decisions (7/9), so gamma sweeps may be inert where stats fall back; sweeps must check `stats_health_events` and may need warmed stats.
- Live-run environment: local Qdrant via WSL2 VM IP (Docker Desktop localhost gRPC proxy flake); controllable-similarity provider needs no OpenAI key; distinct run_ids/store paths per sweep config.
- Repo reference docs consulted: `docs/coding-agent/rules/common.md`, `docs/coding-agent/rules/orchestrator.md` (both repos' equivalents for [CME] via researcher).

## Open Questions (max 3)

- None currently. (Q1 findings home, Q2 candidate-limit scope, Q3 tuning ADR form: all resolved 2026-07-17 — see Decision Log.)

## Assumptions

- A1: The disposition authority is the user; the Orchestrator drafts dispositions and the user confirms fix-now vs defer vs accept-as-designed at the findings review checkpoint (Task_5 gate).
- A2: Adding TOML keys to the [CME] run config and mapping them onto existing public `character_memory` Settings keys is harness enablement, not library change; [CM] gains no new Settings keys unless a fix-now finding requires one, which would be a replan trigger.
- A3: Sweeps execute as N config files + N sequential runs with distinct run_ids and store paths; comparison is manual/report-based (no sweep runner, per YAGNI).
- A4: Live eval runs use local Qdrant (WSL2 VM IP), persistent Oxigraph paths, sqlite stats, controllable-similarity embeddings; no external LLM/embedding calls.
- A5: Fixes land on a [CM] feature branch; merges, PRs, and tagging stay user-controlled. [CME] work lands on feature branches there with the same user-controlled merge policy.
- A6: The known v0.1.4 observation (equal-score root ordering deprioritizes entities; default max_graph_roots=12 truncates hub roots) is pre-registered as finding F-SEED-1 and goes through the same disposition workflow as harness-revealed findings.
- A7: Code identifiers use stable domain names, never version labels; committed [CM] artifacts contain no machine-local absolute paths and word [CME] mentions per the private-repo rule.

## Tasks

### Task_1: [CME] Plumb CM selectivity/fanout overrides into run config

- type: impl
- owns:
  - (CME repo) crates/cmem-eval-core/src/config.rs
  - (CME repo) crates/cmem-eval-adapter-cmem/src/lib.rs
  - (CME repo) configs/continuity_retrieval.toml
  - (CME repo) README.md (config reference section only)
- depends_on: []
- description: |
  Add optional run-config keys for selectivity_smoothing_alpha, selectivity_gamma, and the three relation/object fanout budgets (min/max), validated and passed to character_memory Settings via config set_override in the adapter settings() fn (public Settings::new(Config) path; the env route is crate-private in CM). Keys live in an adapter-scoped table so the backend-neutral core stays CM-type-free. Values must appear in the report metadata.config snapshot so sweep reports are self-describing. Absent keys preserve shipped CM defaults exactly.
- acceptance:
  - A config setting each knob reaches the live adapter's Settings and is visible in the report config snapshot.
  - A config omitting all new keys produces byte-identical behavior to today (regression: mock smoke unchanged).
  - Invalid values (negative alpha, min > max) fail config validation loudly.
  - README documents the new keys.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace, plus service-free continuity mock smoke"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review: adapter-scoped keys only, core stays backend-neutral, defaults preserved when keys absent"

### Task_2: [CME] Findings register convention and template

- type: docs
- owns:
  - (CME repo) new committed findings register doc + .gitignore allowlist entry if placed under reports/
- depends_on: []
- description: |
  Establish the committed findings register per phase doc §3.1: fields for finding ID, scenario and metric, observed vs expected, severity (critical/major/minor), suspected layer, disposition (fix-now/defer/accept-as-designed), rationale, target phase when deferred, and before/after report references. Pre-register F-SEED-1 (hub entity root truncation under equal-score ordering with default max_graph_roots=12) from the v0.1.4 evidence. Record the rule that harness/fixture defects are fixed in the harness and do not count against the library, and that critical findings cannot be accept-as-designed.
- acceptance:
  - Register template exists with all §3.1 fields and disposition rules quoted.
  - F-SEED-1 recorded with its v0.1.4 evidence reference.
  - Register location resolves Q1 (user-confirmed) and is committed, not gitignored.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Template matches phase doc §3.1/§3.2/§3.3; F-SEED-1 present"

### Task_3: [CME] Baseline eval runs and findings intake

- type: research
- owns:
  - (CME repo) findings register entries + curated baseline evidence
  - (CME repo) baseline run configs (new files only)
- depends_on: [Task_1, Task_2]
- description: |
  Run the full 8-scenario live suite at two regimes: (a) shipped CM defaults including candidate limits 48/12, and (b) the established eval regime (max_graph_roots=max_vector_candidates=48), both at shipped alpha/gamma/budgets. Also run the mock suite as a determinism cross-check. For every metric anomaly, lifecycle leak, pollution source, fanout breach, missing rationale, persistence drift, or restart delta: record a finding with severity and suspected layer. Answer phase doc §4.1 with evidence: does entity-root-only selectivity leave measurable hub flooding through non-entity roots (inspect hub scenario reports for non-entity root expansion behavior)? Record stats-health/fallback activation patterns to inform the sweep design (warm vs cold stats). Live-evidence claims state scenario scope, config identity, and CM sibling commit per [CME] rules.
- acceptance:
  - Two live baseline reports plus mock cross-check exist with recorded hashes per the README normalization recipe.
  - Every observed anomaly is a register entry with severity and suspected layer; zero-finding sections are explicitly recorded as clean.
  - §4.1 question answered with report evidence attached to a register entry.
  - Draft dispositions proposed for all findings.
- validation:
  - kind: test
    required: true
    owner: worker
    detail: "[CME] Full live suite twice per regime (reproducibility: identical outside metadata block) vs local Qdrant + persistent stores"
  - kind: review
    required: true
    owner: orchestrator
    detail: "Findings completeness and severity/layer plausibility review before the disposition gate"

### Task_4: [CM] Disposition gate and fix-now scoping (user checkpoint)

- type: design
- owns:
  - docs/coding-agent/plans/active/v0-1-5-eval-driven-closeout-plan.md (Decision Log + task re-scoping only)
- depends_on: [Task_3]
- description: |
  Present the findings register with draft dispositions to the user for confirmation (A1). Apply disposition rules: fix-now only when behavior contradicts a v0.1 family acceptance criterion or philosophy invariant and the fix needs no new concepts/signals; defer with target phase otherwise; accept-as-designed only for documented tradeoffs and never for critical findings. Scope each confirmed fix-now finding into concrete Task_5/Task_6 work items with owns, or record a replan if fixes fall outside the pre-declared owns.
- acceptance:
  - Every finding has a user-confirmed disposition recorded in the register.
  - Fix-now items are mapped to Task_5/Task_6 scopes; deferred items carry rationale + target phase.
  - Decision Log records the gate outcome.
- validation:
  - kind: review
    required: true
    owner: user
    detail: "Disposition confirmation on the findings register"

### Task_5: [CM] Fix-now defect fixes with regression tests

- type: impl
- owns:
  - src/usecases/**
  - src/policy/**
  - src/adapters/**
  - tests/**
- depends_on: [Task_4]
- description: |
  Fix each confirmed fix-now finding within existing concepts and signals, each with a reproducing regression test first (pattern: tests/retrieval_guardrails_tests.rs — StoreFixture, explicit UUIDs, traced RetrievalContext, telemetry assertions, Qdrant-unavailable skip guard). Includes F-SEED-1 remediation if dispositioned fix-now (root ordering and/or candidate-limit default change at src/usecases/retrieve.rs:1026-1060 and src/api/types/retrieval.rs:70-77, updating the pinning tests deliberately). Preserve all v0.1 family invariants: Oxigraph decides truth and final inclusion, Qdrant stays candidate recall only, stats stay derived/rebuildable, provenance for behavior-influencing derived memory, suppressed/superseded excluded by default, no entity special-casing. Zero-diff close if no fix-now findings land in this repo.
- acceptance:
  - Every [CM] fix-now finding has a fix plus a regression test reproducing the original failure.
  - No new public facade methods, no new retrieval signals, no default changes beyond dispositioned items (tuning lands in Task_7).
  - Full existing suite passes with only deliberate pin updates.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CM] cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run"
  - kind: test
    required: true
    owner: worker
    detail: "[CM] cargo test with live Qdrant (WSL2 VM IP) and EMBEDDING_MODEL set"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review vs dispositions; invariant preservation; ADR-I-0018 diff-scoped dependency-direction audit; entity-neutrality check"

### Task_6: [CME] Harness-side fixes and regression scenarios

- type: impl
- owns:
  - (CME repo) crates/cmem-eval-continuity/src/** (generator/fixture/driver/metrics as dispositioned)
  - (CME repo) crates/cmem-eval-continuity/fixtures/** (regeneration)
- depends_on: [Task_4]
- description: |
  Fix findings dispositioned as harness/fixture defects (these do not count against the library), and add regression scenarios to the permanent scenario library for [CM] fix-now findings where a library-side unit/integration test cannot express the behavior (long-horizon, cross-store, or metric-level reproductions). Follow the scenario-extension workflow: generator constructor, fixture regeneration with checked seed, determinism/byte-identity tests. Zero-diff close if dispositions require nothing here.
- acceptance:
  - Each harness-defect finding fixed with a test.
  - Each register entry needing a scenario-level reproduction has one in the committed fixture set with relevance labels.
  - Fixture determinism tests pass (same seed, byte-identical).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace, plus continuity mock smoke"
  - kind: review
    required: true
    owner: reviewer
    detail: "Scenario/label entity-neutrality; no fixture rule special-cases names or roles"

### Task_7: [CME→CM] Default tuning sweep and shipped-default update

- type: impl
- owns:
  - (CME repo) sweep config files (new files only) + findings register tuning section
  - src/config/app_settings.rs
  - src/policy/retrieval_selectivity.rs
  - src/api/types/retrieval.rs (candidate-limit defaults only, per Q2 resolution)
  - src/usecases/retrieve.rs (pinning tests only)
  - docs/design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md (§13 values only)
  - docs/decisions/implementation/ (new tuning ADR per Q3 resolution)
- depends_on: [Task_5, Task_6, Task_12]
- description: |
  On the defect-fixed library, with tuning targets constrained by Task_12's preserve/bound guidance per surface type: sweep candidate values for alpha, gamma, and the three fanout budgets (and candidate limits if Q2 confirms) across the fixture library as N configs with distinct run_ids/store paths. Compare continuity recall (gap-bucket recall fractions), sampled_context_pollution_rate, and fanout discipline (fanout_over_budget_count must stay 0, cap utilization, fallback activations). Account for the fallback-dominance caveat: verify swept parameters actually bind by checking stats_health_events, using warmed-stats runs where needed. Prefer values improving recall without raising pollution or breaching caps. Then update [CM] shipped defaults consistently in BOTH default sites plus dependent tests, update the v0.1.2 phase doc §13 documented values, and record chosen values with the comparison data in the tuning ADR. Constraints: relation caps stay hard upper bounds, conservative fallback not weakened, entity-neutrality preserved, no per-entity tuning. If measured data supports keeping a current default, keep it and record the evidence — no change for change's sake.
- acceptance:
  - Sweep comparison table (config → recall/pollution/discipline metrics) recorded in the register with report references.
  - Both [CM] default sites agree; pinning tests updated deliberately; docs reflect shipped values.
  - Tuning ADR records values + measurement basis + constraints held.
  - A post-update [CME] run at new shipped defaults confirms the expected metric movement.
- validation:
  - kind: test
    required: true
    owner: worker
    detail: "[CME] sweep runs live vs local Qdrant with per-config run_ids; [CM] full command set + cargo test with live Qdrant after default update"
  - kind: review
    required: true
    owner: reviewer
    detail: "Default-site consistency, constraint preservation, sweep-data-to-chosen-value traceability"

### Task_8: [CM] Selectivity scope boundary resolution (§4.1)

- type: design
- owns:
  - docs/decisions/implementation/ (amendment or note within the Task_7 tuning ADR)
  - src/policy/retrieval_selectivity.rs (only if widening is dispositioned fix-now)
  - src/usecases/retrieve.rs (only if widening is dispositioned fix-now)
- depends_on: [Task_3, Task_4]
- description: |
  Resolve the v0.1.2 entity-root-only selectivity boundary using Task_3 evidence. If no measurable hub flooding leaks through non-entity roots: re-affirm the boundary and record the evidence. If flooding is measurable: assess whether widening is possible within existing signals (stats counters are entity-keyed; a non-entity-keyed stats extension is a new signal and therefore a deferral to a later phase with rationale). Expected outcome is re-affirm or defer-with-evidence; code changes only if a within-signals widening exists and is dispositioned fix-now at Task_4.
- acceptance:
  - Boundary decision (widen/re-affirm/defer) recorded with report evidence references.
  - If widened: regression coverage added and entity-neutrality preserved; if deferred: target phase and rationale recorded.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Evidence supports the decision; new-signal boundary respected"

### Task_12: Memory-surface contribution analysis (F-BASE-2 pre-tuning)

- type: research
- owns:
  - (CME repo) findings register F-BASE-2 analysis section
- depends_on: [Task_4]
- description: |
  Before any pollution-targeted tuning: from the Task_3 baseline traces, rationale samples, and per-scenario packs, analyze which memory object types and surfaces genuinely contribute to shaping current character behavior from past memories, and which are noise. Specifically: (a) for same-event surface groups (episode + observation + derived memory), assess what each surface adds — factual grounding, the character's inner perspective, distilled meaning — against the philosophy's continuity goals (§12 success criteria, "memory should be lived, not merely logged"); (b) classify the admitted sampled-negatives by surface type and rationale category to determine whether they are true noise or mislabeled behavior-shaping context; (c) audit the fixture relevance labels themselves — labels that misclassify inner-perspective surfaces as negatives are a fixture-semantics finding, not library pollution; (d) produce concrete tuning guidance for Task_7: which surface types/sections are safe to bound aggressively, which must be preserved, and what the pollution metric should mean going forward. Analysis is altitude work: route to a Claude agent per the delegation rules, grounded in docs/project_philosophy.md and the roadmap invariants.
- acceptance:
  - Per-surface-type contribution assessment recorded with trace evidence references.
  - Fixture label audit concluded: labels affirmed, or a finding entry raised for mislabeling.
  - Task_7 receives explicit preserve/bound guidance per surface type and section.
- validation:
  - kind: review
    required: true
    owner: user
    detail: "Analysis conclusions and tuning guidance reviewed before Task_7 dispatch"

### Task_13: [CME] Fixture semantics fixes, label corrections, distinct-surface scenario, event-level pollution

- type: impl
- owns:
  - (CME repo) crates/cmem-eval-continuity/src/** (generator, fixture, metrics as ruled)
  - (CME repo) crates/cmem-eval-continuity/fixtures/** (regeneration)
  - (CME repo) reports/v0-1-5-findings-register.md (F-FIXTURE-1 entry + F-BASE-1 re-disposition)
- depends_on: [Task_4, Task_12]
- description: |
  Apply the user rulings from the Task_12 review: (1) fix the correction-chains scenario per the F-BASE-1 diagnosis — forget issues explicit targets for both delivery-v1 source surfaces with cascade disabled so the v3 replacement survives, restoring the scenario's stated post-correction expectation; (2) correct the two pollution labels — archive-january and hub-memory-0 move out of irrelevant_external_ids (unlabeled, or an ordering-scored label class); (3) add one scenario with genuinely distinct episode/observation/derived texts so per-surface contribution becomes measurable; (4) add event-level pollution (dedupe by episode root) alongside the retained surface-level metric; (5) record F-FIXTURE-1 and the F-BASE-1 re-disposition in the register. Regenerate fixtures with determinism tests. If cheap, expose the omitted object id on suppressed-omission trace decisions; otherwise record the gap.
- acceptance:
  - Corrected correction-chains scenario retrieves delivery-v3 and excludes v1/v2 surfaces in a live run.
  - Label corrections and the new scenario land with regenerated deterministic fixtures (byte-identity tests).
  - Event-level pollution reported alongside surface-level in run and summarize paths with registry parity.
  - Register carries F-FIXTURE-1 and the F-BASE-1 re-disposition with diagnosis evidence references.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CME] cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace (standing waiver applies), plus continuity mock smoke"
  - kind: test
    required: true
    owner: worker
    detail: "[CME] Live correction-chains + new-scenario runs demonstrating corrected expectations"
  - kind: review
    required: true
    owner: reviewer
    detail: "Fixture/label entity-neutrality; metric registry parity; scenario semantics vs diagnosis evidence"

### Task_14: [CME] Binding-scale fixture (hub-scale scenario + graph-only probe)

- type: impl
- status: CLOSED (approved; commit 3d60823; reviewer c483903..3d60823 no findings)
- owns: crates/cmem-eval-continuity/src/{generator,fixture}.rs, fixtures/**, mechanical counts in driver.rs/pipeline.rs/README, register Task_14 section
- depends_on: [Task_4 gate resolution]
- description: Tenth scenario making parameters bind: hub at 48 incidents, 4 salience levels, 4 clusters + orthogonal graph-only-reachable labeled probe; deterministic at seed 20260712.
- acceptance (evidenced): fixture SHA 45EFE4F8…2F87 byte-identical cross-process; shipped-default live run measured roots 48/12/36, scored/fallback 2/1, probe recall 0.0 (trace C2A1D325…196D at CM 7949173) — the phase's headline measurement.
- validation: worker pinned gates + live run PASS; reviewer independent regeneration + live reproduction PASS (byte-identical).

### Task_15: [CME] Binding re-sweep (supersedes Task_7 recommendation)

- type: research
- status: CLOSED (approved after bistability replan; commits 37addf3→e1eab3c→eb2f4f4→0a05813; reviewer APPROVED e1eab3c..0a05813)
- owns: sweep configs, register Task_15 section
- depends_on: [Task_14]
- description: Ten-config single-factor matrix (roots/alpha/gamma/caps) + root-saturation diagnostic on the binding corpus at CM 7949173.
- acceptance (evidenced): all knobs inert (byte-identical returned sets) or harmful (roots↑ = context cost, no recall); roots48 zero-omission probe miss isolated the loss to post-expansion admission; diagnostic bistability discovered → F-SEED-2; corpus-conditional recommendation recorded; roots_96 dropped as contract-impossible (Decision Log).
- validation: worker matrix + hashes PASS; reviewer 40/40 hashes, stratification reproduction, both-attractor criterion PASS.

### Task_16: [CM] Drop Oxigraph service mode (embedded persistent default)

- type: impl
- status: CLOSED (approved; commit 7949173 + CI-fix d657e23; reviewer a23fcda..d657e23 no remaining findings)
- owns: src/adapters/oxigraph/** (http.rs deleted), composition/config, compose files (deleted), README/.env/roadmap/CI, ports/graph_authority.rs comment (authorized fold-in)
- depends_on: [user Oxigraph ruling 2026-07-18]
- description: Remove the unvalidated HTTP service adapter; Persistent default; migration-hinting rejections.
- acceptance (evidenced): 30 removed helpers proven HTTP-only, 32 retained proven used; test-count delta accounted (−5 +2); CI env defect found by review, fixed (d657e23), clean-room reproduced.
- validation: worker + reviewer full gates incl. live suite PASS (359 unit census).

### Task_17: [CM] ADR-I-0021 (orchestrator-authored)

- type: docs
- status: CLOSED (committed 6cd53ee + wrap-fix c2f86db; fact-checked by cm-reviewer within the Task_16 verdict)
- owns: docs/decisions/implementation/ADR-I-0021*, decisions README
- depends_on: [Task_16]
- description: Records the embedded-persistent default decision, four options, deployment-shape analysis, and the two roadmap consequences. Authoring moved from worker to orchestrator by user rule (Decision Log; now orchestrator.md policy).

### Task_18: [research] Qdrant deployment-mode analysis

- type: research
- status: CLOSED (Decision Log 2026-07-18: v0.1.6 embedded vector phase; SQLite exact-scan first, LanceDB escalation, Qdrant Edge revisit; closeout carries the roadmap note; ADR-I-0003 revisit clause flagged as triggered)
- owns: none (research output only)

### Task_19: [CM] Deterministic vector admission (F-SEED-2 fix)

- type: impl
- status: CLOSED (approved; commit 19d650e; reviewer b0bc972..19d650e no findings)
- owns: src/adapters/qdrant/store.rs, models/vector/**, ports/vector_candidate.rs (comment, authorized), usecases/retrieve.rs (trace ranks), test_support, tests
- depends_on: [user fix-now ruling on F-SEED-2]
- description: Tie-cohort closure with bounded doubling overfetch (cap max(K*16,K+4096)), shared canonicalizer (score/type/id/surface), trace ranks from the canonical vector, documented cap degradation.
- acceptance (evidenced): permutation test asserts full pack+trace equality; live all-tied regression 5x consecutive PASS both sides; Task_9 later confirmed single-attractor byte-identical pairs end-to-end.

### Task_20: [CM] Continuity situation catalog (orchestrator-authored)

- type: docs
- status: CLOSED (docs/design/continuity_situation_catalog.md; fcca439, reframed durable/state-independent in 43a54bb per user direction)
- description: 18 mechanism-independent situations across companion/small-circle/independent-entity spectra; R/B tier taxonomy; embedding-realism principle; usage protocol (phases name served situations; scenario library owns coverage mapping).

### Task_21: [CME] Frozen real-embedding store

- type: impl
- status: CLOSED (approved; commits 0cfe000 + provenance-guard b1a9f40; reviewer 52da12a..b1a9f40 no remaining findings after one MAJOR bounce)
- owns: cmem-eval-core frozen_embedding + config, continuity fixture schema v3 (consolidated: embedding provider tags + new ScenarioPattern variants + abstention empty-relevant coupling), runner generate/validate CLI, adapter wiring, smoke fixtures
- depends_on: [user embedding rulings]
- description: (model, exact-UTF-8-SHA-256)-keyed committed store; offline OpenAI generation CLI (sole network step); deterministic loading provider failing loudly on cache miss; ranked-cosine intent validation; v2 byte-compat; adapter-boundary provenance guard (source=open_ai_api enforced; cfg(test)-only test path) added by bounce.
- acceptance (evidenced): full admission matrix actionable; v2 regeneration byte-identical (45EFE4F8…2F87); cosine ordering target>near-miss>background verified offline.

### Task_22: [CME] Five catalog scenarios (suite → 15 scenarios / 23 queries)

- type: impl
- status: CLOSED (approved; commits d4b258a + count-fix b457f84; reviewer b1a9f40..b457f84 no remaining findings)
- owns: generator/fixtures (canonical continuity_v3.json; v2 removed), task22 real store (71×3072, user-approved generation), configs/continuity_retrieval.toml (mixed provider, granted), counts, register
- depends_on: [Task_21]
- description: graded-similarity + combined-life (real embeddings, genuine authored texts), temporal-patterns, entrenched-correction, autobiographical.
- acceptance (evidenced): four cosine orderings >0.01 margins; combined-life live at shipped defaults ~130s, zero leakage, baselines recorded (long-gap recall 0.5@5, event pollution 0.111, context reduction +0.66); reviewer live trace parity.

### Task_23: [CME] Benchmark-adapted scenarios (LongMemEval-S + LoCoMo, 18 scenarios)

- type: impl
- status: CLOSED (approved; commits be6f814 + proof-contract fix e21ce55; reviewer 0cfe000..e21ce55 no remaining findings; executed by evals-worker2 on side branch, orchestrator-pushed after remote verification)
- owns: new crates/cmem-eval-benchmark-convert (one-way deps), continuity_benchmarks_v1 fixture/manifest/store (635×3072), metrics.rs abstention pollution-only scoring, attribution file, register
- depends_on: [Task_21; user rulings: original text both datasets, out-of-band provenance, conversion guards, 18-instance roster]
- description: Converter with byte-identical official-source regeneration (--check); update pairs as Remember+Correct with old-turn negatives; abstention pattern (empty-relevant); speaker-swapped trap documented; selection manifest with machine-derived vs curator-asserted proof split (bounce).
- acceptance (evidenced): 18/18 ranked-cosine orderings (intent gate caught one real authoring error at generation); four raw-source byte-exact spot checks; licenses verified against upstream texts.

### Task_24: [CME] Benchmark frozen-store runtime-surface fix (Task_9b blocker)

- type: impl
- status: CLOSED — APPROVED 2026-07-19 (evals-reviewer, combined range 98b818e..a2d137a after one MAJOR bounce adding the live cross-repository normalization drift regression and one LOW dimension-preflight fix). Strict 635-way bijection (468 reused / 167 regenerated / 167 dropped), fixture bytes unchanged, finding recorded as F-HARNESS-3 in the final register.
- owns: converter/manifest/runtime helper/preflight, regenerated store
- depends_on: [Task_9b stop finding]
- description: CM's clean_text whitespace collapse makes runtime lookup text differ from source-exact text on real chat data; manifest now enumerates normalized runtime surfaces; store = exactly the runtime set (strict-bijection redirect, Decision Log).

### Task_9b: [CME] Expanded confirmation (33-scenario suite, repeated-run)

- type: research
- status: CLOSED — completed 2026-07-19 and APPROVED (evals-reviewer, range 8087879..8b14d71, independent live reproduction). Canonical half: byte-identical pair at shipped defaults, all ten prior Task_9 scenario metrics reproduced exactly. Benchmark half (post-Task_24 fix): 18/18 twice, byte-identical pairs (traces E19036EE…), zero invariant violations, first-live baselines recorded as measurements; mocks 23/23 and 18/18.
- owns: task9b configs, register expanded-confirmation section
- depends_on: [Task_22, Task_23, consolidation merge 98b818e; Task_24 for the benchmark half]
- description: Both fixtures live twice each at shipped defaults; benchmark values recorded as first-live baselines; findings only for invariant violations; prior-scenario metric drift = finding.

### Task_25a: [CM] Over-engineering sweep (report-only)

- type: review
- status: CLOSED 2026-07-19 — NO CHANGES. One 2-line proposal (redundant currentness guard in cascade-warning accumulation) declined at the orchestrator altitude filter as defense-in-depth worth keeping; all other examined seams were live, pre-existing, roadmap-backed, or protected machinery.
- owns: none (report-only)
- depends_on: [user sweep directive 2026-07-19]

### Task_25b: [CME] Over-engineering sweep (report-only)

- type: review
- status: CLOSED 2026-07-19 — four proposals (P1-P4) with no-change conclusions; orchestrator filter approved P2 (folding P1), P3, P4; declined-by-design items recorded (preflight-provider sharing rejected as a provenance-boundary risk).
- owns: none (report-only)
- depends_on: [user sweep directive 2026-07-19]

### Task_25c: [CME] Approved simplifications

- type: impl
- status: CLOSED — APPROVED 2026-07-19 (evals-reviewer, combined range ba1ebe5..1c3aa4b after one LOW README bounce, final gate under the standing environmental teardown waiver). Schema v3 sole fixture contract with v2 rejected actionably (Task_21 v2-acceptance mandate superseded); converter clone elimination; frozen-store deep-clone elimination; all committed artifacts byte-identical (fixture/manifest/store hashes unchanged).
- owns: (CME) fixture.rs, generator.rs, metrics.rs, benchmark-convert lib.rs, frozen_embeddings.rs, README
- depends_on: [Task_25b filter verdict]

### Task_9: [CME] Confirmation re-run

- type: research
- owns:
  - (CME repo) findings register final section + confirmation run configs/evidence
- depends_on: [Task_7, Task_8]
- description: |
  Re-run the full harness (all scenarios, live, tuned shipped defaults, plus mock determinism cross-check) against the fixed and tuned library. Confirm: every fix-now finding's metric moved as recorded (before/after references completed), no other metric regressed beyond recorded tolerances, no critical findings remain, fanout_over_budget_count is 0, correction-safety metrics show zero leakage. Two-run reproducibility holds (identical outside metadata block).
- acceptance:
  - Final report shows no critical findings; before/after references complete on every fix-now entry.
  - No unexplained metric regression vs baseline.
  - Reproducibility check passes.
- validation:
  - kind: test
    required: true
    owner: worker
    detail: "[CME] full live suite twice + mock cross-check; hashes recorded with scenario scope, config identity, CM sibling commit"

### Task_10: [CM] Closeout declaration, version bump, roadmap bookkeeping

- type: chore
- owns:
  - Cargo.toml (version field only)
  - Cargo.lock (re-resolution only)
  - docs/roadmap/development_roadmap.md (v0.1.5 row + closeout link)
  - docs/coding-agent/plans/ (closeout report placement per Q1 resolution)
- depends_on: [Task_9]
- description: |
  Write the public closeout report: disposition summary (counts by severity/disposition), deferred-findings table with target phases (v0.2/v0.4/v0.5 planning input), tuned defaults with measurement basis reference, selectivity-boundary resolution, the deferred Oxigraph deployment-mode design decision (drop the Docker/service-mode assumption or pivot to it — all v0.1 family live evidence covers the embedded-persistent path only; see Decision Log 2026-07-18), and the explicit v0.2 entry confirmation against phase doc §6. Bump character_memory 0.1.4 → 0.1.5. Update the roadmap v0.1.5 row to Finished with the closeout link. Private-repo wording and no machine-local paths (A7). Tagging stays with the user.
- acceptance:
  - Closeout report satisfies every phase doc §6 bullet and is linked from the roadmap or plans archive.
  - Cargo.toml = 0.1.5; [CM] and [CME] lockfiles re-resolve cleanly.
  - v0.2 entry explicitly confirmed in the report.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "[CM] cargo check --locked after re-resolution; [CME] cargo check (path dep re-resolves)"
  - kind: review
    required: true
    owner: orchestrator
    detail: "§6 checklist walked item by item; A7 wording/path scan"

### Task_11: Independent review and acceptance verification (both repos)

- status: CLOSED — all three legs APPROVED 2026-07-19. Tier D [CM] cm-reviewer at 850ba93 (per-criterion phase-doc §7 walk, full live suite, whole-branch ADR-I-0018 audit, entity-neutrality, wording/path scans, version coherence, closeout factual spot-checks). Tier D [CME] evals-reviewer at ba1ebe5 (after one MAJOR census-reconciliation bounce: F-HARNESS-3 promoted to a formal record, F-BASE-5 retired as a plan label, register closed at exactly eleven CONFIRMED findings, all hashes independently reproduced). Tier A Claude altitude review APPROVED with four advisories, all encoded into the v0.2 phase doc and roadmap before completion.
- type: review
- owns: []
- depends_on: [Task_10]
- description: |
  Tier D (defect/compliance): [CM] reviewer verifies every phase doc §7 acceptance criterion with evidence, reruns the structural suite with live Qdrant, runs the diff-scoped ADR-I-0018 dependency-direction audit and entity-neutrality check over all [CM] diffs, verifies both default sites agree, and scans committed artifacts for machine-local paths / A7 wording. [CME] reviewer independently reruns the confirmation suite (two-run reproducibility), verifies register completeness (every finding has disposition + evidence; no critical accept-as-designed), and verifies public-API-only consumption. Tier A (altitude, milestone gate): review the closeout report and deferred-findings routing for v0.2-entry soundness — is the v0.1 family actually safe to build scoped continuity on?
- acceptance:
  - Both Tier D reviewers report APPROVED with per-criterion evidence.
  - Tier A review raises no unresolved v0.2-entry objection.
  - All phase doc §7 acceptance criteria confirmed.
- validation:
  - kind: test
    required: true
    owner: reviewer
    detail: "Independent [CME] confirmation re-run reproducibility check; independent [CM] live-Qdrant suite run"
  - kind: review
    required: true
    owner: reviewer
    detail: "§7 acceptance checklist + dependency-direction audit + entity-neutrality + A7 scan + Tier A closeout-soundness review"

## Task Waves (explicit parallel dispatch sets)

Interpretation: tasks in the same wave dispatch in parallel by default when owns are disjoint and dependencies are met; waves execute sequentially. Cross-repo tasks never share owns. Task_4 is a user checkpoint: no fix/tuning dispatch until dispositions are confirmed.

- Wave 1 (parallel): [Task_1, Task_2]
- Wave 2 (parallel): [Task_3]
- Wave 3 (parallel): [Task_4]
- Wave 4 (parallel): [Task_5, Task_6, Task_8, Task_12, Task_13]
- Wave 5 (parallel): [Task_7]
- Wave 5b (parallel, added at the measurement-hardening gate): [Task_14, Task_16, Task_17, Task_18]
- Wave 5c (sequential): [Task_15]
- Wave 5d (sequential, F-SEED-2 remedy): [Task_19] — Task_20 orchestrator-authored alongside
- Wave 5e (sequential, scope addition): [Task_21]
- Wave 5f (parallel, two workers): [Task_22, Task_23]
- Wave 6 (sequential): [Task_9 original confirmation] then consolidation merge (98b818e) then [Task_9b expanded confirmation, with Task_24 as its in-flight blocker fix]
- Wave 7 (parallel): [Task_10]
- Wave 8 (parallel): [Task_11]
- Wave 9 (post-acceptance sweep, user-directed): [Task_25a, Task_25b] parallel report-only, then [Task_25c]

Executed-wave note: Waves 5b–5f and Task_9b/Task_24 were added during execution by recorded user rulings and replan triggers (see Decision Log); the task records above carry their closure evidence. Task_9's original run completed before the suite expansion; Task_9b re-confirms over the expanded 33-scenario suite.

Note: Task_8's design portion (evidence assessment) can start in Wave 4 alongside fixes; any Task_8 code change shares files with Task_5 owns and would sequence within the wave. Task_7 runs after fixes so tuning measures the corrected library.

## Rollback / Safety

- [CM] fixes and default changes land on a feature branch; default changes are value-level and cleanly revertible; pinning tests document the old values in history.
- [CME] plumbing is additive (optional keys, defaults preserved when absent); regression: mock smoke byte-identical without the keys.
- Eval runs use distinct run_ids and store paths; no collision with prior artifacts; gitignored local run data is never deleted (repo rule).
- No changes to the public facade, memory object types, or retrieval signals; invariant list enforced at Task_5/Task_11 review.

## Progress Log (append-only)

- 2026-07-18 12:09 EXECUTION HALTED (user directive: free CPU). All three agents confirmed idle with recorded stop points; Qdrant container stopped. State at halt: Task_21 CLOSED/APPROVED (pushed through b1a9f40); Task_22 in progress — five scenarios authored, embeddings generated (user-approved network step), fixture.rs tests-only grant exercised, combined-life live retry NOT yet run (no artifacts written; orphaned collection cmem_eval_task22_…54c706d8… noted for prune); Task_23 in progress on side branch at 0cfe000 — targeted converter/abstention-metric/frozen-store validations passed, repo-wide gate run interrupted, tree left as-is uncommitted. Resume checklist: (1) docker start charactermemory-qdrant-1 + readiness wait, (2) resume evals-worker (combined-life retry, fresh identities, backgrounded+polled), (3) resume evals-worker2 (rerun full gates, commit), (4) resume evals-reviewer (idle, no pending review), (5) prune orphaned collection during next maintenance window.
- 2026-07-19 Task_9b partial: canonical half PASSED completely (byte-identical pair; all ten prior Task_9 scenario metrics reproduced EXACTLY at shipped defaults; zero invariant/registry issues — regime ruling: all Task_9b runs at shipped defaults 48/12 via derived configs, committed general-purpose config untouched). Benchmark half BLOCKED at first live event: frozen-store cache miss — the adapter's runtime embedding surface for benchmark-lme-update-01493427 remember:0002 is not among the 635 stored texts (runtime surface composition diverges from the manifest's enumeration; canonical store unaffected, so converter-fixture-specific). Harness-layer defect (does not count against the library), dispatched fix-now to evals-worker2 on feature/v0-1-5-benchmark-fix off 98b818e: root-cause the surface-composition divergence, fix at the enumeration/converter root keeping embedded text byte-exact, regenerate only new store entries, add a runtime-surface-coverage preflight regression (the gap that let this reach live item 1/18), register finding with before/after refs. Task_9b holds with canonical evidence accepted; benchmark A/B + mocks resume after the fix merges.
- 2026-07-19 Waves 5e/5f COMPLETE + consolidation: Task_21 CLOSED (APPROVED after one MAJOR bounce — adapter-boundary provenance guard, cfg(test)-only test path). Task_22 CLOSED (APPROVED after one LOW count bounce): suite now 15 scenarios/23 queries at schema v3; five genuine-text scenarios; 71-vector real store; combined-life live-verified ~130s with zero leakage — long-gap recall 0.5@5, event pollution 0.111, context reduction +0.66 recorded as baselines. Task_23 CLOSED (APPROVED after one LOW proof-contract bounce): 18 benchmark-adapted scenarios (byte-identical converter regeneration from official sources), 635-vector real store with 18/18 intent orderings (the intent gate caught one real authoring error at generation time), abstention pattern + pollution-only scoring, attribution + out-of-band provenance. Worker2's push blocked by its safety layer (unverified remote) — orchestrator pushed after verifying origin; side branch at e21ce55. Consolidation merge 98b818e by orchestrator (register append-both conflict resolved; fmt/clippy/full workspace green; pushed). Task_9b expanded confirmation dispatched: canonical 15 + benchmark 18, each twice (byte-identical pairs), benchmark values as baselines, findings only for invariant violations, prior-10-scenario metrics must reproduce Task_9 exactly.
- 2026-07-18 Task_9 CLOSED — evals-reviewer APPROVED combined range 0a05813..52da12a after one LOW wording bounce (inline F-SEED-3 attribution for the 0.875 medium recall). Independent repeated-run reproduction confirmed both byte-identical pairs; register final at 10 CONFIRMED findings. Push authorized through 52da12a. Phase confirmation evidence sealed for the pre-expansion suite; the expanded confirmation (post Task_22/23) remains before Task_10.
- 2026-07-18 Task_9 worker portion COMPLETE (CME 655834d, push held; reviewer dispatched with independent repeated-run reproduction required, sibling at 19d650e). Determinism confirmed: default and diagnostic pairs byte-identical; F-SEED-2 closed by the deterministic survivor (Attractor A, 49 objects, probe rank 19; no B recurrence); Task_15 aggregates reproduced exactly; inertness spot points hold; correction safety all-zero; workspace suite 204/204 incl. both live adapter tests unwaived. Register FINAL: 10 CONFIRMED findings, no drafts/critical/open; F-SEED-1 accept-as-designed; F-SEED-3 (admission/ranking credit for graph-only evidence) major defer→v0.2 joined with F-BASE-2 residual; F-BASE-2 fixed-in-part. Task_21 (frozen embedding store) starting in parallel per queue. Remaining before closeout: Task_9 verdict, Task_21→22→23 (embedding store, catalog scenarios, benchmark-adapted scenarios incl. abstention schema v3), expanded confirmation, Task_10, Task_11.

- 2026-07-18 Wave 5b COMPLETE: [Task_14, Task_16, Task_17, Task_18] all CLOSED. Task_16+ADR-I-0021 APPROVED at d657e23 after one MEDIUM bounce (CI env var removed while Settings::load requires it — restored with local path value; reviewer clean-room verified) and one split-ruled LOW (ADR bullet wrap fixed c2f86db; sentence-per-line paragraphs ruled compliant). CM push authorized through d657e23 on feature/v0-1-5-embedded-default. Task_15 in flight with a materially changed diagnosis: roots48 ran with ZERO root omissions and all hub entities expanded, probe recall still 0 — root truncation is NOT the binding constraint; roots_96 point dropped (contract-impossible and moot); sweep budget redirected to a trace-level decomposition of where the probe is lost (fanout caps vs expansion ordering vs section admission), one labeled diagnostic run authorized. The F-SEED-1 remedy question is now fanout/admission-shaped, not root-limit-shaped.
- 2026-07-18 Wave 5b (cont.): Task_14 CLOSED — evals-reviewer APPROVED c483903..3d60823 with no findings (fixture contract, determinism at seed 20260712, mechanical-only count changes, and the headline 48/12/36 probe-recall-0.0 live measurement independently verified at CM 7949173); push authorized through 3d60823. Task_16 review: one MEDIUM bounced to CM worker (CI dropped OXIGRAPH_CONNECTION_STRING while Settings::load still requires it — reviewer reproduced hosted-CI failure shape) and one LOW split-ruled (ADR bullet mid-item wrap fixed in c2f86db; sentence-per-line paragraphs stand per common.md). Environment incident logged: shared Qdrant saturated at ~90 retained collections; force-recreate recovered rather than shed them (persistent volume), so targeted deletion of completed-run collections is the remedy; maintenance coordinated across both review pipelines; Task_15 resuming.
- 2026-07-18 Wave 5b (partial): Task_16 worker portion done (CM 7949173 on feature/v0-1-5-embedded-default: HTTP adapter + service helpers/tests + both compose files removed, Persistent default with migration-hint errors, docs/CI/port-comment updated, full live suite green with clean test-count delta; one authorized fold-in). ADR-I-0021 authored by orchestrator and committed as 6cd53ee; cm-reviewer dispatched on a23fcda..6cd53ee incl. ADR fact-check. Task_14 worker portion done (CME 3d60823: ten-scenario fixture with hub-scale scenario — 48 incidents, 4 salience levels, orthogonal graph-only probe; deterministic; mechanical count updates only). HEADLINE MEASUREMENT (live, CM 7949173, shipped defaults): probe recall 0.0 — 48 unique roots / 12 selected / 36 omitted, the relevant graph-only dormant memory never enters the pack; selectivity now scores (2 scored / 1 fallback). Root truncation demonstrably costs recall under binding pressure; F-SEED-1 is live and measurable. evals-reviewer dispatched on Task_14 (sibling flipped to 7949173); Task_15 binding re-sweep dispatched (roots {12,24,48,96} + alpha/gamma/caps single-factor, probe recall as headline metric, ordering-vs-limit distinction called out as the decision-relevant output).
- 2026-07-18 Wave 5 (sweep evidence) COMPLETE: Task_7 CLOSED — evals-reviewer APPROVED range cd70d61..c483903 after one LOW docs bounce (corpus-conditionality wording). Independent verification: 40/40 artifact hashes, 300/300 config-snapshot fields, all comparison values recomputed, stratification counts reproduced (default 29 scored/40 fallback; roots24/48 35/46), two independent live full-suite reproductions at exact CM a23fcda with matching hashes, returned-set diff exactly the six hub derived surfaces, workspace suite unwaived. Sweep conclusion stands as recorded: all swept knobs inert on this corpus (recall 1.0, pollution 0.296, over-budget 0 everywhere); recommendation explicitly bounded to the canonical corpus and cold-stats regime; Task_15 will supersede it with binding measurements. Push authorized through c483903. Task_18 closed same day (v0.1.6 embedded vector phase decision). In flight: Task_14 (binding fixture, one count-only owns grant issued), Task_16 (service-mode removal), ADR-I-0021 authored by orchestrator (uncommitted, awaiting Task_16 branch).
- 2026-07-18 Wave 4 COMPLETE: [Task_5, Task_6, Task_12, Task_13] all CLOSED (Task_8 documentation folds into the Task_7 tuning ADR by design). Task_13 APPROVED at combined range b46ce14..cd70d61 after one docs bounce (three findings: F-FIXTURE-1 byte-identical overclaim — originated in the Task_12 analysis draft; README authoring guidance mandating non-empty negatives — would have re-taught the mislabeling defect; stale scenario count). Reviewer independently reproduced: corrected correction-chains (only delivery-v3, replacement recall 1.0, pollution 0/0), surface-contribution (six distinct surfaces), full 9/9 two-run live pair byte-identical, fixture regeneration byte-identical, workspace suite UNWAIVED incl. live reattach. CME push authorized through cd70d61. Review worktrees pruned; two lessons recorded (verify supplied evidence claims against canonical artifacts; production-default negative regressions). Wave 5 dispatched: Task_7 sweep portion to evals-worker (~12 single-factor live runs around shipped defaults incl. max_graph_roots {12,24,48}, warm/cold split required or analytically stratified, preserve-list constraints hard, recommendation-only — CM default update follows user review of sweep data).
- 2026-07-18 Wave 4 (partial, cont. 3): Task_5 CLOSED — cm-reviewer APPROVED at a23fcda after one bounce (echo-warning scope MEDIUM; delta verified draft-content-only with production-default negative regression, no scope creep, ADR-I-0018 clean, full live suite 361 unit + 25 integration, zero runtime skips at 127.0.0.1:6334). CM push authorized for feature/v0-1-5-write-path-diagnostics; review worktree pruned. Wave 4 remaining: Task_13 review (in flight), Task_8 (documentation, folds into Task_7 ADR after re-measure).
- 2026-07-18 Wave 4 (partial, cont. 2): Task_6 CLOSED — evals-reviewer APPROVED, no findings; independent reproduction of the 21/12/9 hub counters, README-prose hash reproduction of the Task_3 pairs, and a full committed-config live reproducibility pair (8/8, byte-identical across two runs). Preliminary F-BASE-1 finding withdrawn under the packet-chronology ruling. Task_5 phase 2 bounce fix landed (a23fcda, content-text-only echo comparison + production-default negative regression; full live suite green on worker side) — cm-reviewer verifying the delta; their formal NEEDS REVISION at 2c13d7a crossed in flight with the fix and covers the same single MEDIUM (all other audit points already pass: additive-only, deterministic warning IDs, ADR-I-0018 clean, entity-neutral). Worker lesson (trace compared fields through default constructors) recorded in lessons.md by orchestrator. Sibling clone flipped to 2c13d7a on reviewer handshake; Task_13 review begins.
- 2026-07-17 Wave 4 (partial, cont.): Task_5 phase 2 worker portion done (CM commit 2c13d7a on feature/v0-1-5-write-path-diagnostics, push held; cm-reviewer dispatched on pinned worktree): cascade-suppresses-current-replacement lifecycle warning + byte-exact echo-surface validation warning, warn-only, serde-defaulted, full live suite green. Task_13 worker portion done (CME commit 0021e03, push held; evals-reviewer queued behind Task_6 review): corrected forget semantics live-verified against CM 2c13d7a — only delivery-v3 returned, replacement recall 1.0, pollution 0/0; labels unlabeled per user option; nine-scenario fixture with pairwise-distinct surface texts (seed 20260712, deterministic); sampled_event_pollution_rate with explicit-relevance-over-root-negativity precedence and run/summarize parity; F-FIXTURE-1 + F-BASE-1 re-disposition recorded; final workspace suite passed WITHOUT the reattach waiver. Review CM sibling clone synced to 2c13d7a for live reproduction.
- 2026-07-17 Wave 4 (partial): Task_6 worker portion done (commits bed868c + b46ce14, push held; reviewer dispatched on pinned worktree). Root counters projected as Option-None backend-neutral primitives; tuning observation now derived from measured values, root-type-neutral; README canonicalization contract pinned and reproduces the Task_3 pair hashes; all Task_4 dispositions recorded. New measured evidence: hub scenario shipped regime selects 12 of 21 unique roots (9 omitted) — truncation confirmed at scale, type attribution still via typed traversal roots. F-SEED-1 final disposition deliberately deferred until Task_13's corrected labels land and a re-measure exists (hub pollution 0.5 was partly label-driven, so harm assessment would be premature). Task_12 CLOSED (user reviewed analysis; rulings in Decision Log). Task_5 phase 1 diagnosis CLOSED (fixture verdict); Task_5 phase 2 (F-BASE-5 warn-only diagnostics) dispatched to CM worker. Task_13 dispatched to evals-worker.
- 2026-07-17 Wave 2 COMPLETE: Task_3 completed and CLOSED (orchestrator review pass). Baseline evidence: full 8-scenario live suites twice per regime (shipped 48/12 vs eval 48/48) at CM 85b5f84, reproducible (traces and metadata-free report content byte-identical within regime; row-hash sentinel gap recorded as F-HARNESS-2), plus mock cross-check. Register now holds 7 OPEN findings with draft dispositions: F-BASE-1 (critical draft, correction retrieval returns only a stale pre-correction observation, replacement recall 0, pollution 1.0 — draft fix-now), F-BASE-2 (major, aggregate sampled pollution 0.52 and negative context reduction across six scenarios — draft fix-now via existing knobs), F-BASE-3 (major, §4.1 answered YES: episode roots expand hub-incident edges outside selectivity, hub_context_share 1.0 — draft defer to v0.2 per new-signal constraint), F-BASE-4 (minor, conservative fallback dominates cold selectivity, mean 4.5–5.25 activations/query — draft accept-as-designed + warm/cold sweep split), F-SEED-1 (major, evidence now shows candidate-limit effect but not directly type-specific truncation — draft accept-as-designed conditional on F-HARNESS-1), F-HARNESS-1 (major harness defect, root-selection counters not projected — draft fix-now in Task_6), F-HARNESS-2 (minor harness docs defect — draft fix-now in Task_6). Fanout discipline, rationale coverage, persistence/restart, and metric registry sections audited clean in both regimes. Task_3 owns expansion (one lessons.md entry) exercised; push authorized post-review. Task_4 user disposition gate opened.
- 2026-07-17 Wave 1 COMPLETE: [Task_1, Task_2] both CLOSED.
  - Task_1 closed after one reviewer bounce (two MEDIUM findings: non-atomic fanout budget deserialization contradicting README optionality claims; fail-open DTO admission missing deny_unknown_fields). Fix delta f395fa4..caf3a31 re-reviewed and APPROVED with no findings: atomic leaf budget tables with path-qualified missing-key diagnostics, deny_unknown_fields across the full override DTO hierarchy with typo/unsupported-target regressions, README contract aligned. Push of [CME] feature/v0-1-5-config-overrides authorized post-approval.
  - Validation evidence: worker and reviewer independently green on pinned gates (fmt, strict clippy, workspace suite with exactly one test filtered under the recorded environmental waiver, both 8-scenario mock smokes); omission-path hashes byte-identical to pre-change baseline (results 9146…13A, traces FDD1…23E); reviewer evidence at CME caf3a31 / CM sibling 85b5f84.
  - Notes: review worktrees pruned (control-main retained); environmental teardown flake tracked as environment debt outside the plan (follow-up chip offered to the user); worker lesson committed repo-locally (caf3a31, nested config admission class). Wave 2 dispatched: Task_3 baseline runs (shipped regime 48/12 vs eval regime 48/48, two live runs each + mock cross-check) and findings intake with draft dispositions for the Task_4 user gate.

- 2026-07-17 Wave 1 (partial): Task_2 completed and CLOSED (orchestrator review pass). Findings register landed at [CME] reports/v0-1-5-findings-register.md (commit 94bfe3f on feature/v0-1-5-config-overrides, exact .gitignore allowlist): full §3.1 record contract, quoted §3.2/§3.3 rules, finding template, F-SEED-1 pre-registered (major draft, retrieval layer, OPEN pending Task_4) with round5-live-a/b evidence references. Worker validation evidence: tracked/not-ignored proven via git ls-files + check-ignore, fmt gate, no-local-paths scan. Task_1 worker portion done (commit f395fa4, same branch, held unpushed): overrides plumbed and proven — omission path byte-identical (result hash 9146…13A, trace hash FDD1…23E match pre-change baseline), per-knob Settings mapping proven via exact invalid-budget diagnostics, full pinned gates + two 8-scenario mock smokes green, live adapter tests vs CM main 85b5f84. Task_1 remains open on the required evals-reviewer Tier D verdict (dispatched).
- 2026-07-17 Plan approved by user ("Plan looks good"); execution mode confirmed as standard plan-driven dispatch (goal mode declined by Orchestrator recommendation). Status moved to in_progress; Wave 1 dispatching to evals-worker via agmsg (Task_1 first, Task_2 sequential on the same worker — parallelism not worth a second CME session for a small docs task).

## Decision Log (append-only; re-plans and major discoveries)

- 2026-07-19 Canonical finding-census reconciliation (Task_11 CME audit MAJOR): the working label F-BASE-5 (introduced in this log for the user-directed write-path warning diagnostics) is RETIRED as a finding identity — that work was delivered as a library change under Task_5 phase 2 (reviewer-approved) and is recorded in the closeout report's library-changes section, not as an eval finding. The benchmark frozen-store runtime-surface defect is promoted to formal register record F-HARNESS-3. Canonical census: eleven findings = ten F-BASE/F-SEED/F-HARNESS/F-FIXTURE records + F-HARNESS-3; register status flips to closed. Closeout-report table updated to name F-HARNESS-3 (CM commit 4ff8655).

- 2026-07-19 Rule-suite schema v2 migration (user-directed): PR #62 authored on chore/rule-suite-schema-v2 off main, five-round Copilot review cycle (findings each verified against source before acceptance: ADR-I-0018 exception wording, lifecycle refresh-source gaps, provenance honesty, models in the grandfathered inventory, freshness metadata), all threads resolved, squash-merged by orchestrator under user authorization as main 18fe55c. feature/v0-1-5-embedded-default REBASED onto merged main and force-pushed (head fc97a3c); conflicts resolved by porting phase rule-content onto the v2-formatted files, keeping the ADR-corrected audit wording. Src content identical across the rebase. SHA translation map for historical evidence (old→new): 2c13d7a→bfba898, a23fcda→c3d7539, 7949173→2683f2b, 19d650e→e7499df, 6cd53ee→3975d55, c2f86db→4a711cd, d657e23→28f2e93, b0bc972→5b2f96f, 7ec043b→cb99f6b, fcca439→7d5854d, 43a54bb→f0e7187, 7e2cda7→62c1ae4. Register evidence keeps recorded old SHAs (valid at capture); future citations use new SHAs; CME sibling clone re-pinned to e7499df; team notified. PR-monitor lesson from this cycle already encoded (arm on open).
- 2026-07-19 CharacterMemoryEvals made PUBLIC by the user. CM-side wording updated (commit 7e2cda7 on feature/v0-1-5-embedded-default): rules common.md reference + wording rule now describe the public companion repo (historical records stay as written), roadmap v0.1.4 row + §18, ADR-I-0022 descriptive wording; ADR-I-0019 receives a dated update note preserving its decision-time privacy rationale as history. CME-side self-description sweep + public-attribution adequacy check dispatched to evals-worker as a micro-task. Licensing note: public visibility keeps the LoCoMo CC BY-NC posture valid (non-commercial sharing with attribution) but makes attribution adequacy load-bearing — verified as part of the sweep.

- 2026-07-18 Deferred design decision recorded (user-directed): the Oxigraph deployment story is currently split — the library documents Docker-backed service mode as the application default, but all live validation evidence (CM integration tests, the entire continuity eval harness incl. restart scenarios) exercises only the embedded-persistent path; the HTTP service adapter path has no eval coverage. A later phase must either drop the Docker/service-mode Oxigraph assumption entirely or pivot validation and defaults to the service mode. Not in v0.1.5 scope (accepted for this phase); must appear in the Task_10 closeout report's deferred-decisions section so v0.2+ planning inherits it explicitly.
- 2026-07-18 Task_21 CLOSED-pending-review (0cfe000: frozen store with tagged v3 blocks, offline generate/validate CLI, cache-miss preflight with regeneration instructions, test_fixture-provenance live rejection, v2 byte-compat regeneration proven, full admission matrix, unwaived suite green); reviewer packet dispatched; worker2 gate fired (Task_23 implementation begins on side branch); Task_22 dispatched to evals-worker (five catalog scenarios; real embeddings for graded-similarity + combined-life; one coordinated live window). Task_22 owns expanded twice (orchestrator-approved): canonical config switch to mixed/text-embedding-3-large/3072 (preflight precedes adapter selection) and generator-bin default filename continuity_v3.json with v2 file removal in-commit.
- 2026-07-18 Second evals worker added (user-provisioned evals-worker2, codex, CME-rooted). Parallelization structure: Task_21 amended to consolidate ALL fixture.rs schema churn in its v3 commit (embedding provider enum + new ScenarioPattern variants + abstention empty-relevant allowance with two-way pattern coupling), making Task_22 (evals-worker: generator.rs + canonical fixture) and Task_23 (evals-worker2: converter bin + separate benchmarks fixture file + metrics.rs abstention scoring + attribution notes) file-disjoint. Task_23 runs on side branch feature/v0-1-5-benchmark-adapted off Task_21's commit; consolidation merge orchestrator-owned (v0.1.4 Wave-3 precedent). Live-run windows for both workers coordinated through the orchestrator (shared-Qdrant saturation lesson). Provenance markers ruled strictly out-of-band (user: dataset/test integrity over marking; embedded text stays byte-exact source).
- 2026-07-18 License call OVERRIDDEN by user: LoCoMo original text authorized for adapted fixtures (NC encumbrance is contained to the eval fixtures in the private repo — the raw dataset is already committed there — does not touch the library or other suite parts, and is reversible by removal/rephrasing on need). Containment hygiene required: explicit provenance markers on every LoCoMo-derived fixture for mechanical future removal, plus CC BY-NC 4.0 attribution notes; MIT attribution for LME likewise.
- 2026-07-18 Benchmark-adaptation survey integrated; orchestrator calls (user may override the license call): LME (MIT) text + vectors committable with attribution; LoCoMo (CC BY-NC 4.0) adapted BY SHAPE ONLY with freshly authored text — vectors inherit NC as Adapted Material, so committing either encumbers the fixtures against future commercial intent. Task_23 scope fixed: converter bin reusing the existing dataset crates (evidence indexes gold_turn_ids / LoCoMoEvidenceSessionIndex map mechanically to relevance labels; committed pruned distractor samples; turn-level Remember mapping with synthesized intra-session timestamps), curated subset ~15 instances (4 knowledge-update, 4 temporal, 5 multi-evidence assembly, 2 single-hop controls) plus 3 abstention instances gated on an explicit schema change (fixture v3: empty relevant sets allowed for abstention queries with pollution-only scoring; version bump + parser tests). New ScenarioPattern variants for assembly and abstention (metric-family keys); label-granularity and distractor-sample policies documented in the register by the worker with reviewer verification. Sequencing confirmed: Task_21 (frozen store) gates the converter; survey also flagged datasets/README.md's no-large-files rule contradicted by checked-in datasets (pre-existing CME debt, recorded, not this phase's to fix).
- 2026-07-18 SCOPE ADDITION (user-ruled): adapt LongMemEval-S and LoCoMo scenario data into the deterministic continuity harness where valuable (their scoring/judging pipelines stay out of scope; the frozen-embedding store removes their live-embedding blocker). Assessed in-phase subset: knowledge-update phrasing feeding the late-correction scenario; temporal question shapes feeding the temporal-patterns scenario; NEW multi-evidence/multi-session assembly scenario (required-set recall, existing set-based metrics suffice); NEW abstention scenario (absent-knowledge queries, requires the one fixture-contract extension allowing empty relevant sets as the legitimate abstention exception); session-structured history skeletons for combined-life; adversarial phrasing seeding graded-similarity. Later phases: speaker-scoped separation (v0.2), summarization/answer-quality judging (behavioral tier), full-volume histories (scale phase / v0.6 ingestion), multimodal (v1.0). Survey researcher dispatched (dataset formats, evidence-label convertibility, license posture for committing text and vectors, converter feasibility, curated-subset criteria); output gates the Task_22/Task_23 dispatch. Task_23 [CME] added: benchmark-adapted scenarios (converter or curated authoring per survey), joining Wave 5f with Task_22.
- 2026-07-18 Text-authoring requirement added to the embedding scope (user-ruled): when generating real embeddings, every embedded text must be checked and revised so its content actually carries the semantics the scenario needs — placeholder/synthetic prose that only worked under declared cluster assignments does not transfer to real embedding geometry. Concretely for Task_22: relevant targets must be genuinely semantically related to their queries at the intended strength, near-miss distractors genuinely near (same domain, different referent), unrelated background genuinely unrelated, sparse references genuinely sparse (low lexical/semantic overlap with the target, anchored only through entities), and same-event surfaces (episode/observation/derived) written as fact vs reading vs distilled meaning rather than echoes. Any existing scenario migrated to real embeddings gets the same text revision pass. Embedding-strength expectations should be validated at generation time (an offline check that measured cosine relationships match the scenario's intent — e.g. near-miss pairs land between background and target similarity) so authoring errors surface before eval runs.
- 2026-07-18 SCOPE ADDITION (user-ruled): before closeout, (1) the continuity situation catalog is committed as a design document (done: docs/design/continuity_situation_catalog.md, fcca439 — 18 situations across companion/small-circle/independent-entity spectrums, tiered R/deterministic vs B/behavioral, coverage status, scenario backlog); (2) all now-addable scenarios from the catalog get built and tested in-phase — new Task_21/Task_22; (3) evaluation embeddings must be REAL embedding-model vectors where semantic geometry matters, generated ONCE per text in an offline step, persisted retrievably, and loaded at run time (preserves determinism and no-external-calls-at-eval-time; synthetic clusters remain acceptable for purely structural scenarios). New tasks: Task_21 [CME] frozen-embedding store — offline generation CLI (explicit network step), persisted vector store keyed by (model, text) committed alongside fixtures, a loading EmbeddingProvider that fails loudly on cache miss with regeneration instructions, config validation wiring. Task_22 [CME] five catalog scenarios: graded-similarity discrimination (real embeddings), combined-life competition (single namespace, interleaved patterns, real embeddings), temporal pattern classes (intervals/recurrence/one-off-vs-repeated), late correction on entrenched memory, autobiographical continuity. Task_9's confirmation extends to cover the expanded suite once Task_22 lands (the in-flight Task_9 run still closes F-SEED-2 after-references and is kept). Waves: 5e [Task_21] → 5f [Task_22] → 6 [Task_9 expanded confirmation] → 7/8 unchanged.
- 2026-07-18 Task_19 CLOSED — cm-reviewer APPROVED b0bc972..19d650e with line-level evidence (tie closure with saturating growth/cap, shared canonicalizer single-sourced across adapter/fake/trace paths, permutation test asserting full pack+trace equality, 5x live all-tied regression, ADR-I-0018 clean, full live suite 362/3-ignored census). CM push authorized (19d650e + ADR-I-0022 docs commit 7ec043b). ADR-I-0022 authored by orchestrator and committed: retains all defaults with the two-generation measurement basis, records the Task_8 selectivity-boundary resolution (retained as measured limitation, widening deferred to v0.2), cross-references the admission-ranking deferral and the determinism fix. Task_9 dispatched: repeated-run confirmation (full suite 2x, F-SEED-2 diagnostic 2x expecting single-attractor byte-identity, mock cross-check, 3 sweep spot points re-verified) against CM src 19d650e.
- 2026-07-18 Wave 5c COMPLETE: Task_15 CLOSED — evals-reviewer APPROVED e1eab3c..0a05813 after the bistability replan (final state: 10-row matrix with comparability bounded by F-SEED-2, both diagnostic attractors recorded with invariant qualitative conclusion, F-SEED-2 dispositioned fix-now with Task_19 remedy and Task_9 repeated-run after-references, LF-pinned reproducible config hashes, corrected Preserve/provenance wording). CME push authorized through 0a05813. Zero-executed-test evidence rule promoted to CM worker.md (second cross-agent recurrence). Remaining critical path: Task_19 fix + review → tuning ADR (orchestrator) folding Task_8 boundary record → Task_9 repeated-run confirmation (post-fix CM state, ten-scenario fixture, each config twice) → Task_10 closeout/version bump → Task_11 two-tier final review.
- 2026-07-18 Determinism finding DISPOSITIONED fix-now (user). Diagnosis (CM forensic, file:line evidenced): all CM retrieval-path sorts are total-ordered; the defect is the Qdrant admission boundary — CM requests exactly max_vector_candidates, so an arbitrary subset of a larger equal-score cohort is discarded before CM can canonicalize; raw Qdrant order additionally leaks into vector-candidate trace ranks. Fix dispatched as Task_19 (CM, same branch): adapter-side tie-cohort closure (grow fetch until the kth score's cohort is closed), canonicalization with the shared total key, bounded overfetch with documented degradation, trace-rank canonicalization, and the three-layer regression set (service-backed >K equal-score repeated search 5x, reversed-insertion store pin, permuted-candidate pack/trace assertions). Post-fix verification: Task_9 gains a repeated-run protocol (each config twice) plus 2-3 sweep points re-run twice to confirm the inertness conclusions survive; exact-search fallback recorded as a revisit trigger only. HNSW-approximate variance beyond ties: not observed in any paired run; recorded, not built against.
- 2026-07-18 NEW FINDING (replan): nondeterministic retrieval pack composition under equal-score ties. The Task_15 diagnostic (top_k_episodes=64 regime) is bistable within one healthy environment — worker A/B reruns reproduced BOTH the recorded outcome (rank17/49 items/trace EC71FACD…) and the reviewer's (rank16/51/trace C0FD93F6…), each internally stable; boundary items memory-46/memory-37 flip at the admission cut. Contradicts the v0.1 backend acceptance criterion "retrieval behavior is deterministic under fixed fixtures". Likely fix-now class (stable secondary sort keys are within existing concepts) pending diagnosis + user disposition. CM forensic diagnosis dispatched (ordering-operation audit of the retrieval path at 7949173). Task_15 evidence handling: diagnostic row re-recorded as both attractors (qualitative loss-point conclusion invariant across them); matrix comparability stands on the reviewer's independent verification; reviewer re-review standard adjusted accordingly. The bistability does NOT affect the user's defaults/F-SEED-1 rulings.
- 2026-07-18 FINAL TUNING/F-SEED-1 RULINGS (user): (1) Keep ALL shipped defaults — binding-corpus evidence shows alpha/gamma/all three fanout caps inert (byte-identical returned sets) and roots-12 optimal on its axis (raising roots only worsens context reduction, recovers nothing); tuning ADR records values + both sweep generations + corpus-conditionality; the phase's tuning criterion closes as measured-and-confirmed-current. (2) F-SEED-1 closes accept-as-designed (root bounding measured harmless at saturation); the isolated real gap — graph-reachable relevant memories lose pack admission/ranking to vector-scored items for lack of ranking credit for graph-only evidence (probe survives expansion, returned only at Episode rank ~16-17 under top_k 64) — becomes a NEW register finding (major, retrieval), dispositioned defer to v0.2, explicitly joined with the F-BASE-2 admission-gate residual as one v0.2 design item; the hub-scale probe scenario is its permanent regression fixture. Rulings stand independent of the Task_15 evidence bounce (rank 16 vs 17 does not change the mechanism); evidence finalization pending: Task_15 reviewer returned CHANGES REQUESTED — MAJOR diagnostic irreproducibility (recorded row likely captured pre-prune under service saturation; rerun + matrix-stability cross-check dispatched), plus CRLF config-hash, Preserve-contract-wording, and provenance-wording items.
- 2026-07-18 Task_18 CLOSED (research): Qdrant should gain an embedded mode, housed as a dedicated small backend phase v0.1.6 ("embedded vector candidate recall") immediately after v0.1.5 — not v0.2 (orthogonal to continuity concepts) and not v0.4; doing it before v0.2 avoids mirroring every future payload-surface addition across two adapters retroactively. Technology posture: SQLite exact-scan adapter first (the port's filter contract maps natively to SQL; exact scan over character-memory-scale corpora is honest and deterministic; zero heavyweight deps; rusqlite direction already exists via the stats store), LanceDB recorded as the embedded-ANN escalation path, Qdrant Edge (in-process, beta mid-2026) as a revisit candidate at GA. Qdrant service mode stays fully supported for cloud/multi-process shapes. v0.1.5 records only: the closeout-report roadmap note (draft text in the Task_18 research output) and the fact that ADR-I-0003's revisit clause ("two stores too heavy for target users") is now triggered. Decision adopted by orchestrator under the user's delegation ("decide on what phase is appropriate"); user may override at closeout review.
- 2026-07-18 Wave 5→6 gate RESOLVED (user rulings, final): (1) BUILD the binding-scale fixture in-phase and re-sweep — new Task_14 (hub incidents 6→40-60, varied salience, 3-5 embedding clusters, plus a graph-only-reachable labeled probe so root truncation can cost recall; per the coverage audit's D items) and Task_15 (re-sweep on the binding fixture; alpha/gamma/caps/roots axes; tuned defaults AND F-SEED-1 dispositioned from evidence that binds). Key scoping fact: stats are write-derived, so the binding corpus alone warms them — the reattach/warm runner leg is NOT built. (2) Oxigraph: DROP service mode (Option A) — new Task_16 (CM removal: http.rs, HTTP-only shared.rs helpers, Service mode variant/default flip to Persistent with migration-hinting rejection, compose files, tests block; README/.env.example/roadmap doc updates) and Task_17 (ADR-I-0021 per the decision-packet skeleton incl. the deployment-shape analysis, Considered Option 4, and two roadmap items: demand-conditional remote-graph-authority phase; embedded vector-recall option). (3) NEW analysis (user-directed): should Qdrant also move out of Docker for the target use cases — what an embedded/in-process vector story needs behind the existing VectorCandidateStore port, and which phase should house that work; research-only this phase (Task_18). F-BASE-2 residual defer to v0.2 confirmed earlier. Task_7 closes at its current honestly-bounded evidence once the reviewer verdict lands; Task_15 supersedes its recommendation with binding measurements. Waves: 5b [Task_14, Task_16, Task_17, Task_18] → 5c [Task_15] → 6 [Task_9 confirmation vs post-removal CM + new fixture] → 7 [Task_10] → 8 [Task_11]. (Q1 defaults) tuning defaults only change if the swept parameters can be made to actually bind — scoping requested for a minimal warm-stats measurement leg (reattach existing namespace, retrieval-only second pass); if bounded, do it in-phase and re-sweep; if genuine harness growth, defer default tuning with the inertness evidence recorded. (Q2 F-SEED-1) finalize accept-as-designed ONLY if a fixture-coverage audit shows the canonical scenarios sufficiently represent real character-continuity usage (esp. realistic entity/root scale vs the fixture's degree-6 hubs); audit dispatched to a Claude researcher; otherwise stays open. (Q3 F-BASE-2 residual) DECIDED: defer to v0.2 (score-floor/admission gate is a new signal; scoped-continuity phase owns it; measured 0.296 baseline is the planning input). (Q4 Oxigraph) decision pending one more analysis: consumer-app deployment shapes (local apps without Docker; cloud multi-replica realities incl. process-local sqlite stats and the prior service-remote-sparql plan) — decision-packet researcher extended. Task_7 stays open pending Q1 resolution; its review continues on current evidence (LOW corpus-conditionality bounce accepted).
- 2026-07-18 Decision (supersedes the deferral above; user-directed): the Oxigraph deployment-mode question WILL be decided within v0.1.5. Decision point: the Wave 5→6 gate — presented to the user together with the Task_7 sweep table, before Task_9 dispatches, because Task_9 must validate whatever ships (service-mode spot-check if pivoting; CM-side removals before closeout if dropping). A decision-packet Researcher is dispatched (service-mode surface inventory, parity audit, per-option phase cost, operational tradeoffs, draft ADR skeleton). The resulting ADR rides the same doc wave as the tuning ADR; Task_10's deferred-decisions section requirement converts to recording the DECIDED outcome.

- 2026-07-17 Decision: Task_4 disposition gate CLOSED with one replan (user, 2026-07-17). Confirmed dispositions: F-BASE-1 diagnose-then-fix (targeted diagnosis of fixture correction semantics vs library lifecycle propagation decides the guilty layer before the fix dispatches; stays draft-critical and open until root-caused); F-SEED-1 stays OPEN pending measured root counters, F-HARNESS-1 fix-now in Task_6 (project existing facade counters, derive the tuning observation from telemetry); F-BASE-3 defer to v0.2 with recorded evidence (non-entity selectivity requires non-entity-keyed stats, a new signal); F-BASE-4 accept-as-designed with the warm/cold stats split required in Task_7 sweep design; F-HARNESS-2 fix-now in Task_6 (document the distinct-run identity normalization).
  - F-BASE-2 REPLAN (user correction): parameter tuning must NOT be the first response. New Task_12 (memory-surface contribution analysis) runs before Task_7: analyze from baseline traces and the philosophy which memory object types/surfaces (episode facts vs observation inner-perspective vs derived memory) genuinely shape character behavior from past memories, and which are noise; re-examine fixture relevance labels in the same light (labels misclassifying behavior-shaping surfaces as negatives is itself a finding candidate). Task_7 tuning targets derive from Task_12's conclusions, not raw pollution deltas. F-BASE-2 disposition recorded as fix-now-after-analysis.
  - Plan delta: Task_12 added; Task_7 depends_on gains Task_12; Wave 4 becomes [Task_5, Task_6, Task_8, Task_12]. Lesson recorded in docs/coding-agent/lessons.md (assess memory-type contribution before tuning away pollution).
  - User approval: yes (2026-07-17, correction message).

- 2026-07-17 Decision: Task_12 analysis reviewed by user; three rulings plus the F-BASE-1 diagnosis verdict integrated.
  - F-BASE-1 RE-DISPOSITIONED as fixture/harness defect (does not count against the library). Diagnosis (CM worker, file:line evidenced): the scenario corrects v1→v2→v3 all provenanced to the v1 Episode, then forgets the v1 Episode with default cascade (apply_to_derived_from_target=true), which correctly suppresses current v3; the separately registered v1 observation was never targeted and stays legitimately current. Library upheld explicit-target, source-retention, provenance-cascade, and ADR-I-0013 no-inference contracts. Harness fix (new Task_13): forget with explicit targets for both v1 source surfaces and cascade disabled so v3 survives. Trace gap noted: suppressed-omission decisions do not expose the omitted object id (fold into Task_13 if cheap, else record).
  - NEW LIBRARY PRINCIPLE (user, Q2 ruling): no manipulation of memories after they are written (no retrieval-time/pack-assembly dedup). Instead, the write path warns in validation when an item matches a known recall-harming failure mode, or refuses only for very critical cases. Applied this phase as two additive warn-only diagnostics (new finding F-BASE-5, user-directed fix-now, CM side): (a) lifecycle-mutation warning when a forget/correct cascade would suppress a currently-current supersession replacement (the F-BASE-1 footgun); (b) write-plan validation warning when an observation/derived candidate's content text is byte-identical to its source episode candidate (echo surface, known context-triple-counting mode). Warn-only: blocking would change explicit-operation semantics; escalation to refusal stays a user decision.
  - Q1 ruling: event-level pollution metric added in [CME] (dedupe by episode root) with surface-level retained for comparability. Q3 ruling: fix the two fixture labels (archive-january temporal-contrast, hub-memory-0 recurrence — ordering-scored or unlabeled) and add one scenario with genuinely distinct episode/observation/derived texts; F-FIXTURE-1 recorded in the register.
  - Plan delta: Task_5 re-scoped to the F-BASE-5 diagnostics (F-BASE-1 no longer a CM fix); new Task_13 [CME] carries the fixture semantics fixes, label corrections, new scenario, and event-level pollution metric; Task_13 joins Wave 4 (dispatches when evals-worker frees up after Task_6); Task_9 confirmation runs against the corrected fixture.
  - User approval: yes (2026-07-17 rulings).
- 2026-07-17 Decision: Task_13 owns expanded a second time (orchestrator-approved): [CME] crates/cmem-eval-runner/src/pipeline.rs, mechanical scenario-count expectation updates only (8→9), forced by the user-ruled new distinct-surface scenario raising the canonical fixture count. No runner logic changes. Incidental evidence: the environmentally-waived live reattach test passed in this round's full workspace suite.
- 2026-07-17 Decision: Task_13 owns expanded once in flight (orchestrator-approved): [CME] docs/coding-agent/lessons.md, one entry. Trigger: live validation exposed a relevance-vs-provenance-root overlap in the new event-level pollution metric — delivery-v3 is relevant by external_id but attributes to the negative-labeled delivery-v1 episode root; explicit per-object relevance labels now take precedence over root-derived negativity, with a regression pinning that exact shape. Without this, event-level counting would silently classify every correction replacement as pollution.
- 2026-07-17 Decision: Task_6 owns expanded once in flight (orchestrator-approved): [CME] crates/cmem-eval-core/src/memory_adapter.rs (additive optional counter fields, None-default, CM-type-free primitives) and crates/cmem-eval-adapter-cmem/src/lib.rs (population + tests). Trigger: ContinuityQueryTrace embeds the core RetrievedContextPack and the adapter's retrieval_telemetry fn is the only facade-to-core conversion, so F-HARNESS-1's root-counter projection cannot land in the continuity crate alone. Same class as the v0.1.4 Task_10 projection-layer expansions; null-never-false-zero preserved; no other scope change.
- 2026-07-17 Decision: Task_3 owns expanded once in flight (orchestrator-approved): [CME] docs/coding-agent/lessons.md, exactly one lesson entry (canonical-hash evidence procedures must pin every normalization sentinel literally so third parties can reproduce hashes). Trigger: the worker's final canonical-hash gate caught a draft register defect (identity-neutral row hash lacking an explicit sentinel) before commit; improvement-loop mandates a repo-local lesson. Same class as the Task_1 bounce lesson precedent; no other scope change.
- 2026-07-17 Decision: scoped validation waiver for the Task_1 bounce. [CME] live test live_adapter_reattaches_with_external_ids fails in this environment with a post-success final-delete/check retry timeout; the reviewer reproduced the identical signature on pinned [CME] main aa5dfd9 (control worktree, same Qdrant endpoint, readiness 200), and worker isolated reruns fail identically on both endpoints. Classified pre-existing environmental (Windows Qdrant teardown contention class, cf. the v0.1.4 persist-retry precedent). Waiver: the workspace test gate for this bounce may filter exactly that one test; all other tests must pass; filter command and waiver reference recorded in the worker report. The flake is tracked as environment debt, not a library or diff finding. User approval: orchestrator-issued with recorded evidence (per validation-strictness waiver policy).

- 2026-07-17 Decision: user resolved the three open questions. Q1: full findings register (with run evidence) lives in [CME] as a curated committed doc; [CM] gets the public closeout report linked from the roadmap/plans archive. Q2: per-call candidate-limit defaults (max_vector_candidates 48, max_graph_roots 12) are in scope as tuning of what exists (default value change, not a new signal); measured in the sweep alongside alpha/gamma/budgets. Q3: tuning basis documented as a new implementation ADR (next free ADR-I number at execution time); ADR-I-0010's decision stands untouched. Plan approval itself still pending.
  - User approval: Q1–Q3 yes (2026-07-17); plan approval pending.
- 2026-07-17 Decision: initial draft from parallel Researcher reports (CM tunables/telemetry/test surface; CME run/report/sweep capability). Key drivers: sweep plumbing gap in [CME] run configs (alpha/gamma/budgets not overridable today) makes Task_1 the phase precondition; findings register has no existing home (Task_2); conservative-fallback budget is a fixed formula, not a tunable — treated as out of tuning scope unless a finding dispositions otherwise (adding a Settings key would be a replan); selectivity widening likely collides with entity-keyed stats (expected §4.1 outcome: re-affirm or defer). User approval: pending.

## Notes

- Risks:
  - Findings are unknown until Task_3; Task_5/Task_6 scopes are conditional and may trigger a replan if fixes fall outside pre-declared owns (e.g. write-path or adapter defects needing new Settings).
  - Fallback dominance may mask gamma effects in sweeps; mitigation: warmed-stats runs and stats_health_events checks (Task_7).
  - Fanout defaults duplicated in two [CM] source sites; Task_7 acceptance pins consistency.
  - Local Qdrant gRPC flake via Docker Desktop proxy; all live gates use the WSL2 VM IP.
  - [CME] CI validates against public sibling main; sweeps against unmerged [CM] fixes diverge from CI evidence — evidence claims must state CM commit provenance, and final confirmation (Task_9) should run against the merged/mergeable [CM] state.
- Edge cases:
  - Tuning may legitimately conclude current defaults are best; the phase still closes with the evidence recorded.
  - A critical finding whose fix needs a new concept blocks closeout (cannot defer a critical, cannot accept-as-designed): that is an explicit user decision point / replan.
  - Restart scenarios must re-verify after default changes (defaults affect retrieval used by post-restart verification).

- 2026-07-19 PLAN COMPLETE. Task_11 APPROVED on all three legs (CM Tier D at 850ba93; CME Tier D at ba1ebe5 after the census reconciliation; Tier A altitude with advisories encoded pre-completion). Final states: [CM] feature/v0-1-5-embedded-default at 4ff8655 (0.1.5 bump, closeout report, deferral routing; pushed), main at 18fe55c (schema PR #62 merged). [CME] feature/v0-1-5-config-overrides at ba1ebe5 (register closed, eleven CONFIRMED findings; pushed). User-controlled residuals: CM feature-branch PR/merge to main, CME feature-branch merge to main, tagging, stale feature/v0-1-5-write-path-diagnostics branch deletion, v0.1.6-vs-v0.2 sequencing decision.

- 2026-07-19 POST-COMPLETION ADDENDUM (user-directed before final phase acceptance): (1) stale branch feature/v0-1-5-write-path-diagnostics deleted (origin + local). (2) Sequencing DECIDED: v0.1.6 embedded vector recall runs BEFORE v0.2; detailed planning after this phase closes. (3) Task_25a/25b dispatched: over-engineering sweep of the phase's implemented code in both repos (report-only proposals, orchestrator altitude filter, bounded fixes with review to follow); phase is deemed done only after the sweep resolves.

- 2026-07-20 SWEEP COMPLETE — PHASE FINALLY ACCEPTED. Task_25a [CM]: NO CHANGES (one 2-line proposal declined at the altitude filter as defense-in-depth worth keeping; the phase's CM code passed the adversarial simplicity audit intact). Task_25b/25c [CME]: three approved simplifications landed and APPROVED (48e65d0 + README fix 1c3aa4b, one LOW bounce): schema v3 sole fixture contract with v2 hard-rejected actionably (the Task_21 v2-acceptance mandate formally superseded — rationale expired with the last v2 artifact), converter clone elimination, frozen-store deep-clone elimination; all committed artifacts byte-identical; final gate under the standing environmental teardown waiver (recorded terms unchanged). Declined-by-design simplifications recorded (preflight-provider sharing rejected as a provenance-boundary risk). Residual user decisions: CM feature/v0-1-5-embedded-default PR/merge to main, CME feature/v0-1-5-config-overrides merge to main, tagging. v0.1.6 planning follows phase close per user decision.

- 2026-07-19 Post-acceptance PR review rounds (merge PRs #63/[CME]#13): Copilot findings addressed — CM: lifecycle warning types re-exported at crate root + derived-memory echo regression (e9e8f31), archived task records finalized (c46795f, f963435), API-surface accounting corrected (da6f922), closeout determinism caveat retained. CME: README example paths aligned to committed filenames, validate_store tightened to full bijection (extras rejected), preflight split ruled (coverage universal, provenance live-only) — fix commit cb49148 at the PR head, all threads resolved. This addendum is the archive's record of those rounds.

- 2026-07-21 MERGE-PR REVIEW CYCLES COMPLETE (consolidated record, rounds 3-10 on [CME]#13 and the later rounds on [CM]#63). [CM]#63 (final head e309de9): lifecycle warning re-exports + derived-echo regression, archive-record finalization, API-surface accounting, determinism caveat retained, cascade warning gated to effective suppression (archive paths regressed negative), test-lock audit (five prose/format locks loosened, contracts retained) — clean pass. [CME]#13 (final head 4515134): dimension gating (--allow-nonstandard-dimensions + real-run width preflight), atomic staged store writes, temporal/entrenched metric-family routing with strengthened parity, ada dimensions-parameter rejection, full config-container fail-closed census, model normalization at admission, canonical-pair exactness regression, upstream MIT notice redistributed, speaker attribution via 36 scenario-local entities with byte-exact source text proven and baselines honestly refreshed (supersession note; new pair byte-identical; short@10 0.865->0.719 and pollution up under speaker entities — recorded as measurements), unused lookup entries dropped (646-key bijection), test-lock audit, config-guidance wording — clean pass. Environment: Qdrant teardown-contention class recurred across rounds (waiver held to the one named test; a second-test recurrence was resolved by pruning 116 accumulated collections, not waiver creep) — the teardown-hardening chip is the top post-phase follow-up. Test-lock rule encoded in both repos' practice and CM worker.md.
