# Plan: Public facade and port contracts with no observer get one

- status: in_progress (approved by the decider 2026-09-16; Tier D APPROVED 2026-09-17 at 19826cf; pull request open, awaiting merge approval)
- generated: 2026-09-16
- last_updated: 2026-09-16
- work_type: code

## Goal
- Every documented public promise the audit found unobserved has one test that fails when it breaks: the idempotency-key override on `prepare()`, rejection of a correction whose target is absent, the inclusive confidence bounds on links, upsert-dimension and delete parity across both vector backends, provider failure classification, score ordering and root truncation in retrieval, the stats-projection object-type filter, forward references inside a write plan, and the settings-to-fanout wiring running offline.

## Definition of Done
- Each gap in the audit's F7 library list, as corrected on 2026-09-16 (the `close()` gap is withdrawn: `tests/vector_port_contract_tests.rs:258` already observes it unconditionally), has one test at the cheapest boundary that observes it, named in the report with the defect it would catch.
- Any new test that fails on current main is committed as `#[ignore = "pending ruling: <observed behaviour>"]`, reported as a product defect with the observed behaviour, and the item pauses for the decider's ruling; the branch stays green and the ignored count is stated.
- The service and embedded vector adapters reject a wrong-width upsert with the same typed classification, and `delete_candidates` parity runs in the contract suite (embedded unconditionally, service under the opt-in).
- Repo validation commands pass; full suite executes with counts reported; the companion evaluation workspace compiles and passes against the reviewed commit.

## Planner-added requirements
- None

## Scope / Non-goals
- Scope: new tests in `src/memory.rs`, `src/usecases/**`, `src/adapters/openai/**`, `tests/vector_port_contract_tests.rs`, `tests/retrieval_guardrails_tests.rs`; production changes only where the decider rules a revealed defect is fixed in this plan, each as its own commit.
- Non-goals: re-anchoring or deleting existing tests (earlier plans; note the constant-echo truncation test at `src/usecases/retrieve.rs:2602` is already deleted by the re-anchoring plan and the new truncation test here asserts a different contract); new product features.

## Design
- Chosen: one test per gap at the boundary the audit named. Structure: no new fixtures beyond the temporary embedded store and the recording transport that exist. Evolution: closes the completeness check the harness lens asks for ("which plausible defect would ship undetected"). Verification: each test names the defect class. Operation: unchanged. Human: names predict the contract. Safety: the provider failure test uses the recording transport, no network.
- Alternative: cover the gaps through one broad facade scenario test. Structure: one test, many contracts. Verification: a failure names nothing; the audit already flagged that shape downstream.
- Why chosen: one contract per test is the lens's placement rule.

## Compatibility stance
- surface: service vector adapter upsert classification, if it gains a client-side dimension check or a harmonized error mapping to match the embedded adapter's typed rejection.
- stance: break
- justification: no external consumer; the companion evaluation repository constructs the library through settings and never upserts a wrong-width vector (its adapter goes through the facade); the orchestrator repins the companion checkout to the reviewed commit before review so the claim is tested, not asserted.

## Context (workspace)
- Related files/areas: `src/memory.rs:44-56` (`prepare`, idempotency key override); `src/usecases/correct_forget.rs` (all corrections seed the target); `src/usecases/link.rs:192` (confidence 1.1 only); `src/adapters/qdrant/store.rs:523` (`qdrant_point_structs`, no dimension check) vs `src/adapters/qdrant_edge/mod.rs:286` (`CollectionMismatch::VectorSize`); `src/adapters/openai/embedding_provider.rs` recording transport (200 only); `src/usecases/retrieve.rs:2631` (ties only); `src/usecases/stats_projection.rs:220`; `src/usecases/write_planning.rs` `collect_plan_ref` pre-pass; `tests/retrieval_guardrails_tests.rs:253` (settings-to-fanout wiring; skip-gated until the integration-suite plan lands). Line cites are against main 4a00303 and are re-located by test name after the lower branches land.
- Existing patterns or references: `tests/vector_port_contract_tests.rs` parity shape; lessons.md 2026-09-04 and 2026-09-06 on close, cleanup and Windows directory removal.
- Design record consulted and deviations from its acceptance: ADR-I-0030 (ports own their postconditions) for the parity assertions. No deviation.
- Audit source: `.agent-work/reviewer/test-suite-audit-2026-09-16.md` section F7, library list, with the 2026-09-16 correction.

## Open Questions (max 3)
- Q1: If the service adapter today lets the server reject a wrong-width vector and maps that to a transport-class error, is parity achieved by harmonizing the mapping to the embedded adapter's `CollectionMismatch` classification, or by a client-side check before the request? Task_2 reports what the service does; the decider rules.
- Q2: If correcting an absent target is accepted today, is rejection the ruling? Task_1 reports the observed behaviour before any fix.

## Assumptions
- A1: The integration-suite plan has landed, so `tests/retrieval_guardrails_tests.rs:253` runs offline and Task_3 only has to confirm it executes — source: that plan's Definition of Done; if not landed, Task_1 adds an offline facade test in `src/memory.rs` for the same wiring and Task_3 is a no-op recorded in the report.
- A2: The recording transport in the OpenAI adapter tests can enqueue an arbitrary status and body — source: `src/adapters/openai/embedding_provider.rs` test module (`enqueue_success_response`); Task_2 extends it if not.

## Tasks

### Task_1: Facade and usecase gaps
- type: test
- owns:
  - src/memory.rs
  - src/usecases/**
- depends_on: []
- description: |
  Add one test each for: `prepare()` honours the idempotency-key override and fresh defaults produce distinct plan ids; a correction whose target is absent from the graph is rejected before writes; link confidence 0.0 and 1.0 are accepted and values just outside are rejected; a section with distinct scores orders by descending final score; graph-root truncation retains the highest-scoring roots; Entity and MemoryThread are excluded from object-state projection; a plan candidate may reference a candidate declared later in the plan. A test that fails on main is committed ignored with the pending-ruling reason and reported, not fixed.
- acceptance:
  - Seven tests exist (eight if the A1 fallback applies), each naming its contract; none asserts prose or transcripts.
  - Each test that failed on main is committed `#[ignore = "pending ruling: ..."]`, reported with the observed behaviour, and marked pending ruling.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib memory; cargo test --lib usecases (executed counts reported; ignored tests listed with their reasons)"

### Task_2: Vector-port parity and provider failure gaps
- type: test
- owns:
  - tests/vector_port_contract_tests.rs
  - src/adapters/qdrant/**
  - src/adapters/openai/**
- depends_on: []
- description: |
  Add to the contract suite: a wrong-width upsert is rejected by both backends with the same typed classification, and `delete_candidates` removes every surface of an object on both backends. Add to the provider tests: a non-2xx response and a malformed body are classified through the typed embedding error. Report what the service adapter does today for the wrong-width case before changing it (Q1); if the classifications differ, the parity test is committed ignored pending the ruling.
- acceptance:
  - The parity tests run against embedded unconditionally and against the service under the opt-in, with counts reported for both.
  - The provider tests cover one non-2xx status and one malformed body, asserting variants.
  - Q1 is answered in the Decision Log with the observed service behaviour and the endpoint used.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --test vector_port_contract_tests (Qdrant down); REQUIRE_QDRANT_TESTS=1 cargo test --test vector_port_contract_tests (Qdrant up, endpoint stated); cargo test --lib adapters::openai (executed counts reported)"

### Task_3: Settings-to-fanout wiring observed offline
- type: test
- owns:
  - tests/retrieval_guardrails_tests.rs
- depends_on: []
- description: |
  Confirm the existing wiring test at `tests/retrieval_guardrails_tests.rs:253` executes in a Qdrant-down run and asserts the configured budget through the facade's telemetry and result set; add a test only if it does not.
- acceptance:
  - The wiring test executes in a Qdrant-down run (executed count includes it) and asserts the configured budget through the facade.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test --test retrieval_guardrails_tests (Qdrant down; executed count reported)"

### Task_4: Rulings on revealed defects
- type: design
- owns:
  - docs/coding-agent/plans/active/facade-contract-gaps-plan.md
- depends_on: [Task_1, Task_2]
- description: |
  For each test committed pending ruling, the orchestrator states the design intent it serves, the alternative rejected, and the ruling (fix in this plan, defer with a doc-parked deferral, or accept the behaviour and reshape the test), and records it in the Decision Log. A fix ruled in becomes a follow-up task appended to this plan with explicit `owns`; a fix ruled out of this plan is carved into a separate plan and the ignored test is deleted or reshaped by a follow-up task appended to this plan with explicit `owns`.
- acceptance:
  - Every pending-ruling test has a Decision Log entry with a ruling and its blast radius; no test remains ignored when this PR is linked into the stack.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Decision Log carries one ruling per revealed defect; no production change without a ruling; zero pending-ruling ignores at stack-link time"

### Task_5: Independent review with the companion pinned
- type: review
- owns: []
- depends_on: [Task_3, Task_4]
- description: |
  The orchestrator runs the repo validation commands on the integrated branch and repins the companion evaluation checkout to that commit. The reviewer, in a pinned worktree, confirms each gap has a test that observes it (suggested technique: plant a defect, watch the test fail, restore with a pre-edit hash check, for at least the absent-target and forward-reference tests), confirms parity evidence names the endpoint and a pass census, runs the full suite with counts, and runs the companion workspace suite against the repinned checkout.
- acceptance:
  - Reviewer status is APPROVED with the observer evidence for each gap and the companion run attached.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test --no-run; cargo test (full, counts reported) on the integrated branch; companion checkout repinned to that commit and the repin recorded"
  - kind: review
    required: true
    owner: reviewer
    detail: "Pinned worktree at the reviewed commit; diff review against acceptance; observer evidence per gap; live parity endpoint and pass/skip census; cargo test --workspace in the repinned companion checkout"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_2, Task_3]
- Wave 2 (parallel): [Task_4]
- Wave 3 (parallel): [Task_5]

Parallel tasks run in separate worktrees so Cargo commands never share a target directory mid-edit; the orchestrator runs the repo validation commands on the integrated branch after each wave.

## Rollback / Safety
- Single branch `feature/2026-09-16/facade-contract-gaps` stacked on the unit-test re-anchoring branch. Ruled-in fixes land as separate commits on this branch. This PR is linked into the stack only after every revealed defect has a ruling and no pending-ruling ignore remains; otherwise the open items are carved out to a follow-up plan first (2026-09-15 stack-merge incident).

## Progress Log (append-only)

- 2026-09-16 Plan drafted from the test-suite audit (F7, library list).
- 2026-09-16 Confirmation pass: Task_1 test count allows the A1 fallback; Task_4's test edits go through an appended task with owns.
- 2026-09-16 Reviewer pass (Tier A): `close()` gap withdrawn (already observed by `tests/vector_port_contract_tests.rs:258`); verify-fail-restore demoted to a suggested reviewer technique; A1 fallback moved to Task_1; Task_3 confirms the existing wiring test; companion repin made orchestrator-owned; pending-ruling holding shape and stack-link rule stated.

- 2026-09-17 Wave 1 completed: [Task_1 738962c after rebase, Task_2 on task/p4-t2 5407cff cherry-picked, Task_3 verify-only]
  - Summary: seven facade and usecase gap tests (prepare idempotency-key override and distinct default ids; absent correction target rejected before writes; inclusive link confidence bounds; descending final-score ordering; highest-scoring roots survive truncation; Entity and MemoryThread excluded from object-state projection; forward references inside a write plan); in-crate vector-port parity for wrong-width upsert and delete_candidates over both backends; provider non-2xx and malformed-body classification; the existing fanout wiring test verified executing offline (3 of 3 guardrails tests with Qdrant down).
  - Validation evidence: worker gates fmt and clippy clean; usecases 131 passed, memory filter 37; live port suite 4 of 4 and vector integration 12 of 12 executed against Qdrant v1.19.0 at http://127.0.0.1:6334 with zero collections left; Qdrant down afterwards. Orchestrator full-suite run on the integrated tip recorded in the Wave 3 entry.
- 2026-09-17 Wave 2 completed: [Task_4] zero pending-ruling ignores; rulings below.

- 2026-09-17 Wave 3 completed: [Task_5] (reviewed 19826cf; code identical after the rebase onto the re-anchoring log commit)
  - Summary: cm-reviewer APPROVED, no findings. Absent-target, forward-reference and score-order mutations each failed their intended assertion and passed after SHA-256-verified restoration.
  - Validation evidence (reviewer, pinned worktree): fmt, check, clippy, test --no-run clean; lib 338 passed 2 ignored; integration 2+3+12+6; doc 1; live port suite 4 of 4 and vector integration 12 of 12 against Qdrant v1.19.0 at http://127.0.0.1:6334 with REQUIRE_QDRANT_TESTS=1, collections empty before and after. Companion: cargo check --workspace --all-targets clean and cargo test --workspace green except the known symlink exception against 19826cf.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-17 Decision: branch cut from the unit-test re-anchoring tip f418f27 while that plan's follow-up and Tier D review are still open; Wave 1 here adds tests only, and this branch rebases onto the reviewed tip before its own review. Trigger: decider directive to progress the stack with the Codex workers, one of which was free.

- 2026-09-17 Decision (Q1): the service vector adapter gains the same pre-flight dimension check as the embedded adapter, rejecting a wrong-width record with CollectionIncompatible::VectorSize before any request; observed before the change: the server rejected with a Response/InvalidArgument transport error (expected dim 2, got 3). Alternative rejected: mapping the server's prose to the typed variant. Design intent: ADR-I-0030, ports own their postconditions and the port contract is backend-agnostic. Blast radius: a 13-line guard in the service upsert path; no other behaviour change; the companion constructs the library through settings and is unaffected.
- 2026-09-17 Decision (Q2): no product change; correcting a target absent from the graph is already rejected with GraphExpansionRootNotFound (exact type and id) before any graph, vector or stats write, and the new test observes it.
- 2026-09-16 Decision: a new test that fails on main is committed ignored with the pending-ruling reason and pauses its item for a ruling rather than being fixed by the worker; the PR joins the stack only with zero such ignores. Trigger: orchestrator design-altitude rule (2026-09-13) and the 2026-09-15 stack-merge incident. Decider approval: plan accepted 2026-09-16; merge approval pending.
- 2026-09-16 Decision: the `close()` gap is withdrawn from this plan and from the audit. Trigger: plan review found the unconditional close-and-reopen test in the vector port contract suite.

## Notes
- Risks: the absent-target and wrong-width tests are the ones most likely to fail on main; Q1 and Q2 prepare the rulings.
- Edge cases: the service wrong-width case may fail at the server, not the client; Q1 covers it.
