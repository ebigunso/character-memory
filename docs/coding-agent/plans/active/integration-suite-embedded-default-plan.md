# Plan: Integration tests run against the embedded default and stop duplicating the facade suite

- status: in_progress (approved by the decider 2026-09-16; Tier D APPROVED 2026-09-16 at 1d8c5fb; pull request open, awaiting merge approval)
- generated: 2026-09-16
- last_updated: 2026-09-16
- work_type: code

## Goal
- A default `cargo test` on a machine without Qdrant executes every integration test that does not need the Qdrant service, so the facade contracts (remember, retrieve, correct, forget, restart safety, stats persistence) have an observer again; the service is exercised only by the explicit parity tests, gated by opt-in rather than silent skip.

## Definition of Done
- No integration test prints "skipping" and returns ok: the `should_skip_qdrant_unavailable` helper, its typed-classification test and the `REQUIRE_QDRANT_TESTS` check on it are gone; the shared setups in `tests/support` construct the embedded vector store under a temporary root and build their settings without any required environment variable (no `.env` needed), and the two service parity tests in `tests/vector_port_contract_tests.rs` keep their explicit opt-in.
- A fresh clone with no `.env` runs the full integration suite green with Qdrant down.
- `tests/write_planning_tests.rs` keeps only the tests whose contract a real persistent store can falsify, and its private setup harness is replaced by `tests/support`; the contracts listed in the audit's duplication table stay protected by their inline twins in `src/memory.rs` and `src/usecases/write_planning.rs`.
- `tests/initialization_tests.rs` no longer contains a test that asserts nothing.
- Every remaining integration test that claims persistence reopens the store before asserting.
- The README test-running paragraph describes the new gating; the PR workflow runs `cargo test` without the service for every pull request and keeps the live job as the service parity run.
- Executed-test counts are reported for a run with Qdrant down and a run with Qdrant up and `REQUIRE_QDRANT_TESTS=1`; both show zero skipped, zero failed.

## Planner-added requirements
- A service-free `cargo test` job in the PR workflow. Needed because: today `cargo test` runs only in the live Qdrant job, which is gated to same-repo non-Dependabot pull requests, so fork and Dependabot PRs get compile-only; once the suite runs without the service there is no reason to keep that hole.

## Scope / Non-goals
- Scope: `tests/**`, the README testing section, `.github/workflows/pr_validation.yaml`.
- Non-goals: any production code change; the inline unit suites (separate plans); adding new contracts (the gaps plan); the evaluation repository's half of the audit (sections F3 evals, F5, F6, F7 evals), which the four plans under `../CharacterMemoryEvals/docs/coding-agent/plans/active/` carry.

## Design
- Chosen: the shared test setups default to the embedded vector store, mirroring the library default since v0.1.6; the service is reached only from the two parity tests that name it. Structure: one setup path in `tests/support`, no per-file harness copies. Evolution: fits the embedded-default direction recorded in ADR-I-0023 to ADR-I-0030 and the README; deleting the skip helper removes a concept. Verification: every test either runs or fails; no third state. Operation: the suite no longer depends on Docker for the default run. Human: a green run means the tests ran. Safety: none affected.
- Alternative: keep service mode and turn the silent skip into a hard failure when the service is down. Structure: unchanged. Evolution: keeps a service dependency the product no longer has by default. Verification: honest but red on every machine without Docker, so the suite would be run less. Operation: Docker remains a prerequisite for `cargo test`.
- Why chosen: the embedded store is the product default and already proven service-free by `tests/vector_port_contract_tests.rs`; the alternative preserves a dependency the product dropped.

## Compatibility stance
- surface: none; test code, README wording and CI only.
- stance: preserve
- justification: no library contract, interface or persisted format changes.

## Context (workspace)
- Related files/areas: `tests/support/basic.rs:51`, `tests/support/persistent.rs:29`, `tests/write_planning_tests.rs:804-893` (three setups forcing `vector_store_mode = "service"`); `tests/support/base.rs:137-167` (skip helper); `tests/initialization_tests.rs`; `tests/vector_port_contract_tests.rs:336,362` (the opt-in pattern to keep); README lines 190-207; `.github/workflows/pr_validation.yaml` lines 73-129.
- Existing patterns or references: `tests/vector_port_contract_tests.rs` builds embedded stores under a `TempDir`; `src/test_support.rs:825` shows the temporary vector store fixture and its cleanup rule (test artifacts cleaned unless retrospect, ruled 2026-09-13).
- Design record consulted and deviations from its acceptance: v0.1.6 embedded vector recall plan (embedded is the default; service remains an explicit mode). No deviation.
- Audit source: `.agent-work/reviewer/test-suite-audit-2026-09-16.md` sections F1 and F2 (first table).

## Open Questions (max 3)
- None.

## Assumptions
- A1: The embedded adapter supports every operation the facade tests exercise, including persistent reopen — source: `tests/vector_port_contract_tests.rs:218,258` (restart-safe and close-then-reopen tests pass against embedded).
- A2: The inline twins named in the audit table exist and run unconditionally — source: `src/memory.rs:263,296,320,340`; `src/usecases/write_planning.rs:520,553,1843,1894`; verified 2026-09-16 (400 lib tests executed with Qdrant down).

## Tasks

### Task_1: Embedded-by-default shared setup and skip-machinery removal
- type: test
- owns:
  - tests/support/**
  - tests/initialization_tests.rs
  - tests/public_facade_tests.rs
  - tests/retrieval_guardrails_tests.rs
  - tests/vector_port_contract_tests.rs
  - tests/write_planning_tests.rs (setup harness and skip call sites only; test bodies belong to Task_2)
- depends_on: []
- description: |
  Make `tests/support` construct the embedded vector store under a temporary root that is removed at test end and build settings from explicit overrides with no required environment variable (the shape `common_settings()` in the vector port contract suite already uses), delete the skip helper and every call site including the private harness in `tests/write_planning_tests.rs`, and drop the initialization test that asserts nothing. The two service parity tests keep their explicit opt-in; any other test that still needs the service is a finding to report, not a skip to keep.
- acceptance:
  - No `should_skip_qdrant_unavailable` symbol and no "skipping" print remains under `tests/`.
  - `tests/support` reads no required environment variable; the suite runs with no `.env` present.
  - With Qdrant down, every test in the owned files executes and passes; with Qdrant up and the opt-in set, the parity tests execute too.
  - Every temporary store root created by the setups is removed after the test (census of the temp directory after the run).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test --tests with --nocapture shows zero lines matching 'skipping' (Qdrant down, no .env, executed counts reported); REQUIRE_QDRANT_TESTS=1 cargo test --tests (Qdrant up, executed counts reported)"

### Task_2: write_planning integration file reduced to store-falsifiable contracts
- type: test
- owns:
  - tests/write_planning_tests.rs
- depends_on: [Task_1]
- description: |
  Delete the tests whose contract is already protected inline (the audit's duplication table: prepare/validate do not persist, commit revalidation, ungrounded derived memory, missing link target, idempotent retry and divergent key, opaque source refs, no inference helpers, stripped vector candidates, the provenance unit test), keep one of the three "every entry path commits equivalent graph state" spellings, and make the persistent-graph test reopen before asserting. Re-anchor the remaining prose assertion helper onto typed error variants; where no typed variant exists for a case, report it and leave that assertion for the unit-test re-anchoring plan (no production change here).
- acceptance:
  - Every deleted test is listed in the report with the inline test that protects its contract, by file and line.
  - Every remaining test uses `tests/support` (the private harness is removed by Task_1).
  - The persistent-mode test closes and reopens the stores before its assertions.
  - No assertion in the file matches error prose, except cases reported as lacking a typed variant.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --test write_planning_tests (executed count reported, zero skipped)"

### Task_3: README and workflow follow the new gating
- type: docs
- owns:
  - README.md
  - .github/workflows/pr_validation.yaml
- depends_on: [Task_1]
- description: |
  State in the README that the integration suite runs service-free by default and that the two service parity tests need Qdrant plus the opt-in; add a service-free `cargo test` job to the PR workflow for every pull request and keep the live job as the parity run.
- acceptance:
  - README describes the default run and the opt-in in one paragraph, without machine-local paths.
  - The workflow runs `cargo test` without service prerequisites on every pull request and the live job still sets `REQUIRE_QDRANT_TESTS=1`.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "README wording matches the shipped gating; workflow job conditions read as described"

### Task_4: Independent review
- type: review
- owns: []
- depends_on: [Task_2, Task_3]
- description: |
  Verify in a pinned worktree that no contract lost its observer: for each deleted integration test, confirm the named inline twin exists and executes; confirm the skip census is zero with Qdrant down; confirm the temp-store cleanup census; confirm executed-test counts in the evidence are positive.
- acceptance:
  - Reviewer status is APPROVED with the deleted-test to inline-twin mapping independently confirmed.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review against acceptance; independent Qdrant-down run with --nocapture showing zero skips; cleanup census"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2, Task_3]
- Wave 3 (parallel): [Task_4]

Parallel tasks run in separate worktrees (review-worktree pattern) so Cargo commands never share a target directory mid-edit; the orchestrator runs the repo validation commands on the integrated branch after each wave.

## Rollback / Safety
- Single branch `feature/2026-09-16/integration-suite-embedded-default`; test, docs and workflow only; reverting the branch restores the service-gated suite.

## Progress Log (append-only)

- 2026-09-16 Plan drafted from the test-suite audit (F1, F2 first table).
- 2026-09-16 Reviewer pass (Tier A): Task_1 now owns the write-planning harness and skip call sites so Wave 1 compiles; settings built without environment variables so fork PRs can run the suite; typed-variant fallback stated for Task_2; evals half of the audit named in Non-goals.

- 2026-09-16 Decider accepted all eight plans; execution starts, PRs stacked.

- 2026-09-16 Wave 1 completed: [Task_1] (commit fd65ec8)
  - Summary: tests/support opens the embedded Qdrant Edge store under a TempDir from explicit overrides; skip helpers, load_test_settings, tests/initialization_tests.rs and the write_planning private harness deleted; parity opt-in kept.
  - Validation evidence: fmt and clippy clean; Qdrant down and no .env: 2+3+12+19 integration tests executed, 0 skipping, temp census unchanged (2160 before and after); Qdrant up with REQUIRE_QDRANT_TESTS=1: same counts, both parity tests executed ok.
- 2026-09-16 Wave 2 completed: [Task_2, Task_3] (commits 20f2de8, 7957e3c)
  - Summary: write_planning integration tests 19 to 6 with every deletion mapped to its inline observer; source_input_ref and provenance span assertions moved into the inline source-ref test; persistent test closes and reopens; README testing paragraph rewritten; PR workflow gains an unconditional service-free cargo test job and the live job runs the two parity tests.
  - Validation evidence (orchestrator, integrated branch, Qdrant down, no .env): fmt, check, clippy, test --no-run clean; lib 400 passed 5 ignored; public_facade 2, retrieval_guardrails 3, vector_port_contract 12, write_planning 6; zero skipping lines.
  - Notes: Codex workers write only under the repository root; task worktrees moved to .worktrees/ (memory note codex-worktrees-inside-repo-root).

- 2026-09-16 Wave 3 completed: [Task_4] (reviewed 7957e3c, final pin 1d8c5fb)
  - Summary: cm-reviewer APPROVED. Two P2 observer gaps found in the deletion mapping (MissingDerivedSource at validate and commit; caller and ModelProcessor provenance fields) and closed by carrying the assertions into the inline tests (1d8c5fb, test-only). All 13 deletion mappings verified at the owning boundary.
  - Validation evidence (reviewer, pinned worktree): fmt, check, clippy, test --no-run clean; Qdrant down and no .env: lib 400 (401 at the final pin) passed 5 ignored, integration 2+3+12+6, doc 1, zero skipping lines; TEMP census 2663 to 2663 and tempfile roots 2158 to 2158; live parity at http://127.0.0.1:6334 with REQUIRE_QDRANT_TESTS=1: both service_and_embedded tests executed ok; workflow parses and reads as planned.
  - Notes: lesson recorded in lessons.md (deletion mappings compare assertion sets, not test names).

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-16 Decision: Task_2 owns widened to the test module of src/usecases/write_planning.rs for one move. Trigger: the inline source-ref test did not assert plan.source_input_ref or candidate provenance source_spans, so deleting the integration test would have lost those observers. Plan delta: the two assertions moved inline, then the integration test was deleted. Tradeoff: a second observer at the cheaper boundary versus keeping a service-shaped integration test. User approval: orchestrator ruling under the accepted plan.
- 2026-09-16 Decision: the PR workflow keeps its global trigger filters (bases, paths) and adds the service-free job beneath them. Trigger: worker question whether every pull request literally required removing the filters. Rationale: the planner-added requirement targets fork and Dependabot coverage on source-touching pull requests; widening triggers would also widen the live job. User approval: orchestrator ruling under the accepted plan.
- 2026-09-16 Decision: integration tests default to the embedded store; the service is reached only through explicit opt-in parity tests. Trigger: 24 of 38 integration tests skipped silently with Qdrant down. Decider approval: plan accepted 2026-09-16; merge approval pending.

## Notes
- Risks: an integration test may depend on a service-only behaviour not visible from its name; the worker reports each such case instead of re-adding a skip.
- Edge cases: Windows path length for RocksDB roots (use short temp roots as the existing contract suite does).
