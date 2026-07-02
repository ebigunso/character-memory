# Plan: Stabilize v0.1.2 Retrieval Guardrail Tests

- status: done
- generated: 2026-07-02
- last_updated: 2026-07-03
- work_type: code
- branch: fix/2026-07-02/v0-1-2-test-slowness-flakiness (off main; independent of v0.1.3 feature branch)

## Goal
- Eliminate the 45–70s per-test wall time and order-sensitive failures in tests/v0_1_2_retrieval_guardrails_tests.rs while preserving meaningful live-Qdrant end-to-end coverage, so full `cargo test` is a reliable validation gate again (required by the paused v0.1.3 plan's Task_5).

## Definition of Done
- Each guardrail test completes in a small fraction of current time under healthy local Qdrant (target: <15s each; measured evidence recorded).
- All three tests pass repeatedly (≥3 consecutive runs) with default parallelism, or are explicitly serialized with documented rationale.
- All three tests pass repeatedly with `--test-threads=1`.
- Test failures distinguish Qdrant visibility/ranking issues from graph/stats persistence issues via targeted diagnostics.
- No production behavior change without a focused test; no bypass of payload index validation that could hide schema drift.
- `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --no-run` pass; full `cargo test` green with live Qdrant.

## Scope / Non-goals
- Scope: Qdrant store initialization path (collection/index setup, timeouts), guardrail test fixtures and readiness checks, test-support env-var hygiene, minimal SQLite stats tuning only if Task_1 evidence implicates it.
- Non-goals: retrieval ranking redesign; replacing Qdrant/Oxigraph; benchmarking infrastructure; fixing unrelated tests beyond shared test-support hazards; changes to v0.1.3 feature work (separate branch).

## Context (workspace)
- Researcher-verified cost shape: every facade open runs `QdrantVectorCandidateStore::new` + `init_collection` (list_collections → conditional create → collection_info → per-field create_field_index loop); each test opens 2 facades + 1 cleanup client; upsert/delete use wait=true with 30s timeouts; embedding dim is 3072.
- Smallest test (3 vectors, 1 remember, 1 retrieve) reproduced at 69.66s — data volume is not the driver; init or near-timeout gRPC ops are.
- Order sensitivity: shared live daemon contention + assertions that depend on Qdrant candidate recall returning the entity root (trace absent if vector search misses it).
- Hazard found: tests/support/base.rs calls process-global `std::env::set_var("GRAPH_STORE_MODE", ...)` while tests run multi-threaded in one process.
- Key files: src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs, src/lib.rs, tests/support/{base,basic,persistent}.rs, tests/v0_1_2_retrieval_guardrails_tests.rs, src/internal/infrastructures/retrieval_stats.rs.
- Local env note: QDRANT_CONNECTION_STRING pinned to 127.0.0.1 (see lessons.md IPv6 entry); pin stays.

## Open Questions (max 3)
- Q1: If Task_1 timing shows collection/index init is intrinsically slow on this Qdrant version, is a test-only "assume initialized on reopen" constructor acceptable, or must the production reopen path itself get cheaper? (Default: prefer fixing the production init path if it does redundant work; test-only shortcuts are a fallback.)
- Q2: May guardrail selectivity assertions be reworked to make the entity root unambiguously top-ranked (test seeding change), keeping one end-to-end recall smoke test? (Default: yes.)
- Q3: If default-parallel stability cannot be achieved without weakening coverage, is serializing this one test target acceptable? (Default: yes, with rationale documented in the test file.)

## Assumptions
- A1: Fix strategy selection is deferred to the post-Task_1 decision checkpoint; Tasks 3–4 scope may be trimmed or redirected based on timing evidence (built-in replan point, user consulted only if the delta is material).
- A2: Qdrant stays at qdrant/qdrant:latest from docker-compose.qdrant.yml; no daemon config changes without user approval.
- A3: Reviewer dispatches use the Claude Fable 5 model (user directive).

## Tasks

### Task_1: Phase-timing diagnosis of guardrail tests
- type: test
- owns:
  - tests/v0_1_2_retrieval_guardrails_tests.rs (temporary instrumentation; final state must be clean or keep only bounded, useful diagnostics)
- depends_on: []
- description: |
  Instrument the smallest test (stats_persist_across_facade_reopen) and the fixture setup with phase timings (facade open/init_collection, remember/upsert, reopen, retrieve/search, cleanup). Run single test, single-threaded trio, and parallel trio; attribute wall time per phase. Deliver a timing table identifying the dominant phase(s) and whether they are init-bound, wait=true-bound, search-bound, or contention-bound.
- acceptance:
  - Timing table for all three execution modes with dominant phase identified.
  - Explicit conclusion: which of the four candidate fix strategies (cheaper init; readiness checks; env hygiene; serialization) the evidence supports.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --test v0_1_2_retrieval_guardrails_tests stats_persist_across_facade_reopen -- --nocapture (plus single-threaded and parallel trio runs; timings captured in report)"

### Task_2: Contain process-global env mutation in test support
- type: impl
- owns:
  - tests/support/base.rs
  - tests/support/basic.rs
  - tests/support/persistent.rs
- depends_on: []
- description: |
  Replace `std::env::set_var("GRAPH_STORE_MODE", ...)` (and any sibling process-global mutations) with direct config-builder overrides, matching the pattern persistent.rs already uses. Cast numeric overrides to i64 per lessons.md. No test semantics change.
- acceptance:
  - No `std::env::set_var`/`remove_var` remains in test support (or any residual use is guarded and justified in a comment).
  - All integration test targets still compile and pass their non-guardrail suites.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --test initialization_tests && cargo test --test v0_1_public_facade_tests"

### Task_3: Apply the evidence-selected slowness fix
- type: impl
- owns:
  - src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs
  - src/lib.rs (constructor wiring only, if needed)
- depends_on: [Task_1]
- description: |
  Implement the fix the Task_1 evidence supports for single-test slowness. Expected direction (per research): make facade-reopen initialization cheaper — skip redundant create/index churn when the collection already matches expected config, and/or right-size collection creation params for the workload; adjust operation timeouts only with justification. Root-cause fix preferred over test-only bypass (Q1 default). No skipping of schema/index validation that guards production correctness.
- acceptance:
  - Smallest guardrail test completes in <15s under healthy local Qdrant (evidence attached).
  - Production semantics preserved: fresh-collection creation, index ensure-on-first-open, and config mismatch detection still covered by existing or new unit tests.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --lib && cargo test --test v0_1_2_retrieval_guardrails_tests stats_persist_across_facade_reopen -- --nocapture (timed)"

### Task_4: Deflake write→read visibility and root-ranking assertions
- type: test
- owns:
  - tests/v0_1_2_retrieval_guardrails_tests.rs
  - tests/support/base.rs (readiness helper only)
- depends_on: [Task_1]
- description: |
  Add a bounded post-write readiness check (expected point IDs visible before retrieve; short timeout; targeted diagnostics on failure) and re-seed the selectivity test so the entity root is unambiguously top-ranked (Q2 default), keeping end-to-end recall coverage in at least one test. Failure messages must distinguish vector-visibility misses from graph/stats defects.
- acceptance:
  - Three consecutive single-threaded trio runs pass.
  - Failure injection (e.g., temporarily wrong point ID) produces a diagnostic naming the missing vector points.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --test v0_1_2_retrieval_guardrails_tests -- --test-threads=1 (x3, all green)"

### Task_5: Parallel-execution stabilization and full-suite gate
- type: test
- owns:
  - tests/v0_1_2_retrieval_guardrails_tests.rs (serialization guard if needed)
  - tests/support/base.rs (shared guard helper if needed)
- depends_on: [Task_2, Task_3, Task_4]
- description: |
  Verify default-parallel stability. If contention persists, add a minimal serialization guard for live-Qdrant collection-lifecycle tests with rationale documented (Q3 default). Confirm the full suite is green.
- acceptance:
  - Three consecutive default-parallel `cargo test --test v0_1_2_retrieval_guardrails_tests` runs pass.
  - Full `cargo test` green with live Qdrant.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --test v0_1_2_retrieval_guardrails_tests (x3) && cargo test (full suite, Qdrant up)"

### Task_6: Final review gate
- type: review
- owns: []
- depends_on: [Task_5]
- description: |
  Reviewer (Claude Fable 5) verifies: root-cause fix vs symptom patch (lessons.md ethos), no production-correctness checks bypassed, diagnostics quality, repeated-run evidence completeness, and that instrumentation left in the tree is bounded and useful.
- acceptance:
  - Reviewer status is APPROVED.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Independent diff review + spot re-run of the guardrail target"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_2]  (disjoint owns: Task_1 touches only the guardrail test file; Task_2 touches only tests/support/*)
- Decision checkpoint: Orchestrator reviews Task_1 timing table, confirms/adjusts Task_3–Task_4 scope (Decision Log entry; user consulted if strategy changes materially).
- Wave 2 (parallel): [Task_3, Task_4]  (disjoint owns: src store/facade vs test files)
- Wave 3 (parallel): [Task_5]
- Wave 4 (parallel): [Task_6]

## Rollback / Safety
- All work on fix/2026-07-02/v0-1-2-test-slowness-flakiness; revert = drop branch.
- Production-path changes (Task_3) are the only non-test surface; they require preserved unit coverage and are isolated in one commit for easy revert.
- No Qdrant daemon/compose config changes without explicit approval.

## Progress Log (append-only)

- 2026-07-02 Wave 1 completed: [Task_1, Task_2]
  - Summary: Task_1 timing matrix collected (Orchestrator completed after Worker runs were disrupted); instrumentation reverted. Task_2 env hygiene landed (no set_var remains in tests/support/*).
  - Validation evidence: Task_1 — open_1=28.3s, remember=60.1s, open_2=1.1s, retrieve=0.003s, cleanup≈0s (single test, quiet machine; Qdrant 1.16.3). Task_2 — fmt/check/clippy pass; initialization_tests pass (9.74s); v0_1_public_facade_tests fails identically with and without Task_2 changes (stash A/B: 116.7s vs 176.7s, same vector-indexing partial failure) → pre-existing, tracked under Task_3.
  - Notes: Parallel Worker dispatch caused shared-resource interference (one cargo target dir + one live Qdrant): dirty rebuilds and exit-130 interruptions. Remaining waves dispatch sequentially.

- 2026-07-03 Plan closeout: PR #54 created; all CI checks green including Live service integration tests (8m13s) on Linux — confirming the guardrail tests pass with the shipped hardening and that the residual idle-stall is local-environment-specific as classified.
  - Summary: Shipped Task_2 (env hygiene), Task_3 (timeout API fix + keepalive + canary), Task_4 (outcome assertions + readiness diagnostics), docs/lessons/gitignore. Task_3b/3c/3d closed as diagnosis-complete (falsification chain in Decision Log); Task_5 superseded by CI arbitration; Task_6 review performed pre-PR (Fable 5, CHANGES_REQUESTED → all findings applied) plus green CI as independent evidence.
  - Validation evidence: local — fmt/check/clippy ✅, cargo test --lib 303/303 ✅, initialization_tests ✅; CI — Compile check, Test target compilation, Clippy, Rust formatting, Live service integration tests, GitGuardian all pass (run 28601708495).
  - Notes: local <15s guardrail acceptance waived per user decision (documented environment constraint; see lessons.md idle-stall entry). Canary test qdrant_channel_survives_idle_gap_before_mutating_upsert re-checks the machine after environment updates.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-07-02: Plan drafted from Researcher investigation. Diagnosis-first structure chosen: Task_1 timing evidence gates the fix strategy to avoid speculative changes to the production Qdrant init path.
- 2026-07-02: Decision checkpoint (per A1). Evidence: REST probe — create=1.75s, index(wait)=1.54s, upsert(wait=true, 3×3072)=0.05s; Python gRPC probe — create=2.29s, index(wait)=2.35s, upsert(wait=true)=0.07s; Rust path — open_1=28.3s (~6 sequential index-field creations at ~2s daemon cost each + overhead), remember upsert=60.1s (~2×30s), while Python does the identical upsert in 0.07s. Conclusions: (1) "cheaper reopen init" hypothesis rejected — open_2 is already 1.1s; first-open cost is daemon-side sequential index creation; (2) remember/upsert stall is in the Rust client layer (qdrant-client/tonic channel or request config), NOT the daemon, proxy, or payload size. Task_3 redirected: primary objective is root-causing the Rust-client mutation stall; secondary is batching/parallelizing or trimming first-open index creation. Task_4 readiness-check scope unchanged. Sequential Worker dispatch henceforth (shared cargo target dir + shared live Qdrant made parallel Workers interfere).
- 2026-07-02: Task_3 partial fix landed (removed per-request .timeout() from mutation/scroll builders: isolated upsert 2.41s→0.048s; first-open init 28s→7.7s) but the integration-path stall persists. Task_4 diagnostic surfaced the hidden defect: remember reports vector_indexing_failure (server-cancelled "Timeout expired") while tests previously discarded RememberOutcome; Qdrant ends up empty → telemetry assertions fail downstream. Working hypothesis at the time: current-thread runtime starvation of the tonic driver by blocking Oxigraph/SQLite calls. Task_3b added to verify before fixing; Task_4 remains open for its readiness/reseed scope.
- 2026-07-02: Task_3b falsified the runtime-starvation hypothesis with a decisive contrast: after a 10s blocking gap, the next upsert times out identically on current-thread (78.5s) AND multi-thread (79.6s) runtimes. Refined root cause: the tonic/gRPC channel goes half-open after ~10s of idleness (Docker Desktop port-proxy dropping idle connections is the prime suspect; consistent with the earlier two-week-container hang and the IPv6 lessons entry); the next mutating call stalls to gRPC deadline. Back-to-back probes (Rust post-Task_3, Python, REST) never idle → always fast. Fix direction: enable HTTP/2 keepalive (keep_alive_while_idle + keepalive interval) in qdrant client config. Task_3c added; Task_3b closed as verification-complete/no-fix (correct stop per its falsification gate).

- 2026-07-02: Task_3c blocked: qdrant-client 1.17.0/1.18.0 expose only keep_alive_while_idle (already default-true, inert without a ping interval tonic never sets). Orchestrator follow-up evidence: (a) REST and Python-gRPC probes both survive a 12s idle gap (0.10s/0.11s upserts) → daemon and generic gRPC path fine; (b) qdrant-client points ops use allow_retry=true — the 60s failure is TWO 30s timeouts, the second on a freshly created channel; (c) semver update of hyper/h2/hyper-util (hyper 1.6.0→1.10.1) did not change behavior. Task_3d added: decisive experiments (new-client-after-gap, async vs blocking gap, read vs mutate) then adapter-level idle-aware client refresh if supported.
- 2026-07-02: Task_3d matrix falsified the final in-code hypothesis: (1) a COMPLETELY NEW client/store after the gap still times out (60.0s); (2) tokio::time::sleep gap also times out (60.0s) — not runtime-blocking-specific; (3) reads after the gap are fast (1.5ms) while the next mutation times out. Combined with Python/REST being immune, the defect is now classified ENVIRONMENTAL: Windows Docker Desktop port-proxy interacting pathologically with tonic/h2 mutation traffic after ~10s wall-clock idle, process-wide, unfixable at the adapter layer without grotesque workarounds (heartbeat mutations). Production code is NOT defective — CI (Linux Docker, iptables port publishing, no userspace proxy) and production deployments do not traverse this proxy. Replan proposed to user: attribute decisively via a no-proxy probe (native Qdrant or Docker Desktop mirrored networking), fix the local environment, document in troubleshooting/lessons, and keep Task_2 (env hygiene) + Task_4 (readiness diagnostics + outcome assertions) as the in-repo hardening. Execution paused pending user decision.
- 2026-07-03: Attribution probes falsified the proxy theory too: the idle-gap canary fails under Docker host networking AND against a native Windows Qdrant binary (no Docker/WSL), while Python gRPC against the same native binary is fast (0.03s). Server logs prove failing requests arrive; one native run succeeded on the client's automatic retry (30.0s first attempt + 0.66s retry). Dependency/toolchain drift excluded (June-green Cargo.lock unchanged; pinned rustc 1.95.0; orchestrator's experimental hyper/h2 update reverted). Classification: machine-specific loopback/tonic interaction, cause not pinned; deep-dive (packet capture, h2 tracing) declined as diminishing returns. USER DECISION: ship the unconditionally valuable hardening (Task_2 env hygiene; Task_3 timeout-API fix; Task_4 outcome assertions; canary test), create PR, let Linux CI arbitrate; if green, document the constraint (lessons.md entry added) and close with local <15s acceptance waived as environment-constrained. Task_3d disposition: no idle-refresh implemented (falsified); canary + explicit keepalive shipped. Task_4 readiness helper shipped; selectivity reseed deferred (unverifiable locally). Task_5 parallel-stabilization superseded by CI arbitration. Reviewer (Fable 5) returned CHANGES_REQUESTED pre-PR; findings 1–4 applied (gitignore /data/, this entry, lessons reconciliation, canary cleanup ordering), 5–6 addressed as comments.

### Task_3d: Idle-aware Qdrant client recovery in the adapter
- type: impl
- owns:
  - src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs
- depends_on: [Task_3c]
- description: |
  Run discriminating experiments first (see dispatch prompt), then implement the evidence-supported adapter-level recovery (expected: refresh the Qdrant client when the store has been idle beyond a threshold, before issuing the next operation). Keep Task_3/Task_3c uncommitted changes.
- acceptance:
  - Experiment matrix recorded (new-client-after-gap; tokio vs thread sleep; read vs mutate after gap).
  - Idle-gap regression test passes with post-gap upsert <1s.
  - stats_persist_across_facade_reopen passes in <15s.
  - No behavior change for non-idle paths; unit coverage for the refresh policy.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --lib && CARGO_TERM_PROGRESS_WHEN=never cargo test --lib qdrant_channel_survives_idle_gap_before_mutating_upsert -- --ignored --nocapture && CARGO_TERM_PROGRESS_WHEN=never cargo test --test v0_1_2_retrieval_guardrails_tests stats_persist_across_facade_reopen -- --nocapture (timed)"

### Task_3c: Enable gRPC keepalive to survive idle gaps (status: blocked — superseded by Task_3d)
- type: impl
- owns:
  - src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs
- depends_on: [Task_3b]
- description: |
  Reproduce the idle-gap failure (10s sleep → upsert times out), then enable keepalive in qdrant_candidate_config (keep_alive_while_idle; explicit HTTP/2 keepalive interval if the client exposes it) and prove the same sequence passes in milliseconds. Keep Task_3's uncommitted timeout removals.
- acceptance:
  - Idle-gap reproduction fails before, passes (<1s upsert) after, both timings recorded.
  - stats_persist_across_facade_reopen passes in <15s with Task_4's outcome assertion active.
  - Unit test coverage for the new client config (mirroring the existing candidate_client_config test).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --lib && CARGO_TERM_PROGRESS_WHEN=never cargo test --test v0_1_2_retrieval_guardrails_tests stats_persist_across_facade_reopen -- --nocapture (timed)"

### Task_3b: Verify and fix async-runtime starvation of the Qdrant channel
- type: impl
- owns:
  - src/internal/infrastructures/graph/oxigraph_authority_store.rs (blocking-call wrapping only)
  - src/internal/infrastructures/retrieval_stats.rs (blocking-call wrapping only)
  - src/internal/infrastructures/external_services/qdrant_vector_candidate_store.rs (only if channel-recovery config is the chosen fix)
- depends_on: [Task_3]
- description: |
  Verify the starvation hypothesis with a minimal live-gated reproduction (current-thread runtime; init → blocking sleep/RocksDB write → upsert), then fix the root cause: wrap blocking Oxigraph/SQLite operations in tokio::task::spawn_blocking (or block_in_place where appropriate) so gRPC channels stay driven. Do not change public API or test runtimes as the primary fix (multi-thread test flavor would mask the library defect for downstream current-thread users).
- acceptance:
  - Reproduction test demonstrates the failure mode before the fix and passes after.
  - stats_persist_across_facade_reopen completes in <15s and passes (with Task_4's outcome assertion active).
  - No blocking DB call remains directly on the async runtime in the touched adapters.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --lib && CARGO_TERM_PROGRESS_WHEN=never cargo test --test v0_1_2_retrieval_guardrails_tests stats_persist_across_facade_reopen -- --nocapture (timed)"

## Notes
- Risks:
  - Test-only init shortcuts could hide production schema/index drift — guarded by Task_3 acceptance.
  - Readiness polling with generous timeouts can mask performance regressions — bounded timeouts required.
  - Serialization (if needed) fixes order sensitivity but not slowness — acceptable only alongside Task_3.
  - Qdrant `latest` tag means daemon behavior may shift across pulls; record version in Task_1 evidence.
- Edge cases:
  - Reopen against an existing collection with mismatched vector config must still fail loudly.
  - Cleanup path (collection delete) failures should not poison subsequent tests.
