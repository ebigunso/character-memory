# Plan: Qdrant Teardown Hardening (light-delta, reshaped after live verification)

- status: approved
- generated: 2026-09-02
- last_updated: 2026-09-02
- work_type: mixed

## Goal
- Retire the observability-phase teardown-transport waiver on evidence, close the two crate-default Qdrant client timeouts, pin the Qdrant image/client pair, and give the orchestrator a safe prefix-scoped prune tool for orphaned collections.

## Definition of Done
- Both repos: every Qdrant client is built through `QdrantConfig` with an explicit deadline (worker.md rule); no client remains on the crate default.
- Both repos: qdrant-client 1.19.0, CM compose/README image pinned to `qdrant/qdrant:v1.19.0`; live tests emit no client/server compatibility warning.
- CM: erased-connect contract canary re-verified against 1.19.0 and renamed version-agnostic; the `is_erased_qdrant_connect_failure` comment cites the verified version, or the coupling is retired if upstream now preserves the source.
- CME: `scripts/qdrant_prune_collections.sh` exists (REST, explicit prefix, dry-run default, `--delete` opt-in) and has pruned the 15 July `cmem_eval_continuity_continuity_v1_*` orphans (user-authorized 2026-09-02).
- Endpoint defaults (`.env.example`, CME test fallback, CME README) say `127.0.0.1:6334`.
- Waiver retirement recorded in the archived observability plan's Decision Log; lessons entries in both repos; one PR per repo, green CI.

## Scope / Non-goals
- Scope: the items above only.
- Non-goals: REST verification on delete timeout (existing exists-then-delete plus one retry never fired in 7 service-up runs); pre-run orphan sweeper in test support (would delete concurrent benchmark runs' `cmem_eval_*` collections; Qdrant exposes no creation time); cross-process live-run mutex (stays orchestrator discipline); any production adapter behavior change.

## Context (workspace)
- Evidence 2026-09-02 (rebuilt machine, Qdrant 1.19.0 at 127.0.0.1): CM canary and 2 live smoke tests pass; CM 405 tests pass (write_planning 12s vs 353s recorded); CME three live tests pass 4/4 runs including final cleanup, zero leaked collections, retry macro never triggered; IPv6 loopback healthy.
- Crate-default (5s) client sites: CM `tests/support/base.rs` `cleanup_collection`; CME `crates/cmem-eval-adapter-cmem/src/lib.rs` `new_internal` (`Qdrant::from_url`). Production store uses 30s plus keep_alive_while_idle (`src/adapters/qdrant/store.rs`).
- Version skew source: `docker-compose.qdrant.yml` and README pin `qdrant/qdrant:latest`.
- Orphan safety precedent: CME `validate_cleanup_target` (refuses missing or too-broad prefixes).
- Repo reference docs consulted: FOLLOWUP-SEED.md teardown section; structured-verdict-observability-plan.md Decision Log (waiver entry 2026-07-21); lessons.md 2026-07-03 idle-mutation stall entry.

## Open Questions (max 3)
- none

## Assumptions
- A1: qdrant-client 1.19.0 is API-compatible for the calls used (list/create/exists/delete collection, upsert, search, delete points, payload index). If not, the worker reports the delta instead of adapting call sites silently.
- A2: The July stall class cannot be induced on demand; retirement evidence is the service-up run census above plus the reviewer's rerun.

## Tasks

### Task_1: CM: explicit cleanup-client deadline, client/image pin, defaults, records
- type: impl
- owns:
  - tests/support/base.rs
  - src/adapters/qdrant/store.rs
  - Cargo.toml
  - Cargo.lock
  - docker-compose.qdrant.yml
  - README.md
  - .env.example
  - docs/coding-agent/lessons.md
  - docs/coding-agent/plans/completed/structured-verdict-observability-plan.md
- depends_on: []
- description: |
  1. `cleanup_collection`: build the client via `QdrantConfig::from_url(url).timeout(Duration::from_secs(30))` (keep best-effort semantics). After a failed delete, check `collection_exists`; if still present, `eprintln!` a warning naming the collection so leaks stop being silent. No panic, no retry loop.
  2. Bump `qdrant-client` to `1.19.0` (Cargo.toml plus lock). Re-run `qdrant_client_1_17_erased_connect_contract_canary`; rename it `qdrant_client_erased_connect_contract_canary`; update the comment in `is_erased_qdrant_connect_failure` to the verified version, or retire the prefix coupling if 1.19.0 preserves a downcastable source (report which).
  3. Pin `qdrant/qdrant:v1.19.0` in `docker-compose.qdrant.yml` and the README docker snippet. `.env.example` becomes `http://127.0.0.1:6334`.
  4. Append a lessons.md entry: 2026-09-02 rebuilt-machine verification, July teardown failure catalog not reproduced, waiver retired, what was and was not changed.
  5. Append to the archived observability plan Decision Log: waiver retired 2026-09-02 with the evidence census (cite this plan).
- acceptance:
  - No `Qdrant::from_url(..).build()` remains in src/ or tests/.
  - Live test output contains no "not compatible with server version" line.
  - Canary passes under 1.19.0; name is version-agnostic; comment cites the verified version.
  - Compose, README, and .env.example updated as specified.
  - Lessons and archived-plan entries appended (append-only; logs grew).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo clippy --all-targets -- -D warnings"
  - kind: command
    required: true
    owner: worker
    detail: "QDRANT_CONNECTION_STRING=http://127.0.0.1:6334 cargo test (service-up), then cargo test --lib -- --ignored --test-threads=1 qdrant_ ; report pass/skip census with endpoint; census test_collection_* via REST before and after shows no leak"
  - kind: review
    required: true
    owner: cm-reviewer
    detail: "Diff review vs acceptance; rerun the live lib tests service-up once"

### Task_2: CME: explicit adapter-client deadline, client bump, prune script, defaults, records
- type: impl
- owns:
  - crates/cmem-eval-adapter-cmem/src/lib.rs
  - crates/cmem-eval-adapter-cmem/Cargo.toml
  - Cargo.lock
  - scripts/qdrant_prune_collections.sh
  - scripts/README.md
  - README.md
  - docs/coding-agent/lessons.md
- depends_on: []
- description: |
  1. `new_internal`: build the client via `QdrantConfig::from_url(&url).timeout(Duration::from_secs(30))` (a named const next to the existing timeouts). Behavior otherwise unchanged.
  2. Bump `qdrant-client` to `1.19.0` in the adapter crate plus workspace lock (CM's twin bump lands in the sibling working tree in the same wave; run validation after the orchestrator relays "CM Task_1 committed locally").
  3. Test fallback at `adapter_config` and README occurrences become `http://127.0.0.1:6334`.
  4. `scripts/qdrant_prune_collections.sh` (bash, curl only): `usage: qdrant_prune_collections.sh <collection-prefix> [--delete]`; endpoint from `QDRANT_REST_URL` default `http://127.0.0.1:6333`; lists matching collections; dry-run unless `--delete`; refuses an empty prefix and any prefix shorter than 12 characters (same spirit as `validate_cleanup_target`'s too-broad guard); prints per-collection result and a final count. Document in scripts/README.md (purpose, both known prefix families `cmem_eval_*` and `test_collection_*`, run only under the live-run mutex).
  5. Validate the script live: dry-run for `cmem_eval_continuity_continuity_v1_` must list exactly 15; then `--delete` them (user-authorized 2026-09-02); dry-run again must list 0. Record the 15 names in the report.
  6. Append a lessons.md entry mirroring CM's (rebuilt machine, waiver retired, prune tool replaces sweeper).
- acceptance:
  - No `Qdrant::from_url(..).build()` remains in crates/.
  - Live test output contains no "not compatible with server version" line.
  - Script exists, executable, dry-run by default, refuses broad prefixes; 15 orphans pruned and verified gone.
  - README fallbacks say 127.0.0.1; lessons entry appended.
- validation:
  - kind: command
    required: true
    owner: evals-worker
    detail: "cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings"
  - kind: command
    required: true
    owner: evals-worker
    detail: "QDRANT_CONNECTION_STRING=http://127.0.0.1:6334 cargo test --workspace (service-up); then the three live_* adapter tests 3 consecutive runs with --test-threads=1; report endpoint plus pass/skip census; collection count unchanged after runs"
  - kind: command
    required: true
    owner: evals-worker
    detail: "Prune script: dry-run lists 15; --delete; dry-run lists 0; a refused short-prefix invocation exits non-zero"
  - kind: review
    required: true
    owner: evals-reviewer
    detail: "Diff review vs acceptance; rerun the three live tests service-up once"

### Task_3: CM review
- type: review
- owns: []
- depends_on: [Task_1]
- description: |
  Offline diff review of the CM branch pinned from the LOCAL repo, plus one service-up rerun of the live lib tests. Exit rubric applies: in-PR fixes only for defects introduced here.
- acceptance:
  - Reviewer status APPROVED or a defect list scoped to this diff.
- validation:
  - kind: review
    required: true
    owner: cm-reviewer
    detail: "Report per reviewer.md (endpoint plus pass/skip census)"

### Task_4: CME review
- type: review
- owns: []
- depends_on: [Task_2]
- description: |
  Same as Task_3 for the CME branch; also sanity-check the prune script's refusal path.
- acceptance:
  - Reviewer status APPROVED or a defect list scoped to this diff.
- validation:
  - kind: review
    required: true
    owner: evals-reviewer
    detail: "Report per reviewer.md (endpoint plus pass/skip census)"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_2]
- Wave 2 (parallel): [Task_3, Task_4]

## Rollback / Safety
- Every change is a local-only worker commit on `chore/qdrant-teardown-hardening` in each repo until the orchestrator pushes after review; revert means dropping the branch.
- The prune script deletes nothing without `--delete`; the only deletion in scope is the 15 named July orphans.

## Progress Log (append-only)

- 2026-09-02 Plan drafted from live verification; user approved all proposed changes and directed liberal use of codex peers.
- 2026-09-02 Wave 1 (CM half) completed: [Task_1] — local commit 1aa3dd6 by cm-worker (Task_2 still pending: evals-worker terminal not yet live after the machine rebuild).
  - Summary: cleanup client on explicit 30s QdrantConfig deadline with leak warning; qdrant-client 1.19.0 (tripwire ruled GO: dev tonic 0.12→0.14, reqwest_012 alias removed, single test call on main reqwest 0.13); canary renamed `qdrant_client_erased_connect_contract_canary`, 1.19.0 channel_pool.rs inspected — prefix coupling retained and comment re-cited; image pinned v1.19.0 (compose + README); .env.example 127.0.0.1; lessons + archived-plan Decision Log appended.
  - Validation evidence (worker report `.agent-work/worker/task1-report.md`): fmt, clippy -D warnings, service-up `cargo test` 405 passed/0 failed/3 ignored at 127.0.0.1:6334, ignored `qdrant_` lib tests 3 passed incl. canary, zero compatibility-warning lines, REST census test_collection_* before=0/after=0, commit hooks (5) passed.
  - Notes: two environment blockers fixed at root by the orchestrator (stale pre-commit hook → pre-commit installed into Python312's own site-packages; codex→orchestrator agmsg truncation → file-based reports). Worker candidates to triage at closeout: rule RB-CAND-AGMSG-FILE-FALLBACK; lessons LESSON-CAND-AGMSG-ARG-SPLIT, LESSON-CAND-STALE-PRECOMMIT-INTERPRETER.
- 2026-09-02 Wave 2 (CM half) completed: [Task_3] — cm-reviewer APPROVED 1aa3dd6 from `.review-worktrees/cm-reviewer`, zero findings.
  - Validation evidence (`.agent-work/reviewer/task3-review.md`): client-construction census clean (no crate-default client left); locked graph has exactly one qdrant-client 1.19.0 / tonic 0.14.6 / reqwest 0.13.4; 1.19.0 `channel_pool.rs:80` still erases the connect source (prefix coupling correctly retained); reviewer service-up rerun of the ignored `qdrant_` lib tests 3/3 at 127.0.0.1:6334 (server 1.19.0), zero compatibility-warning lines, zero leaked collections; `--locked` canary and metadata pass.
  - Notes: promotion step (push + PR) follows per push-after-internal-approval; the plan file and the FOLLOWUP-SEED disposition ride as an orchestrator docs commit on the same branch. Promoted as CM PR #70 (orchestrator docs commit 7f7b855). Copilot findings (2, both phase-introduced, in-PR): cleanup warning must also fire when the existence probe itself errors (→ cm-worker); seed wording wrongly called a tracked file untracked (→ orchestrator).
- 2026-09-02 Wave 1 (CME half) completed: [Task_2] — local commit cdaec3a by evals-worker.
  - Summary: named 30s QdrantConfig deadline in `new_internal`; qdrant-client 1.19.0 + lock; 127.0.0.1 test fallback and README; `scripts/qdrant_prune_collections.sh` (exact-prefix dry-run default, `--delete` opt-in, short-prefix refusal exit 2, curl-only) + scripts/README.md; lessons entry. The 15 July orphans were intentionally NOT pruned by the agent (auto-mode classifier blocks agent-initiated bulk deletes); ebigunso runs the prune.
  - Validation evidence (`.agent-work/evals-worker/task2-report.md`): fmt, clippy -D warnings (workspace), service-up `cargo test --workspace` 305/305 at 127.0.0.1:6334, three serialized runs of the three live adapter tests 9/9, zero compatibility-warning lines, collection count 15→15 throughout; script dry-run listed exactly 15, throwaway `cmem_eval_prune_selftest_<pid>` created and deleted, refusal exit 2.
  - Notes: evals-worker lesson candidate LESSON-CAND-WINDOWS-SANDBOX-DPAPI (CryptUnprotectData 2148073483 on every sandboxed launch) joins the post-task delivery-layer investigation.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-02 Decision: reshape the FOLLOWUP-SEED four-step spec.
  - Trigger / new insight: on the rebuilt machine every July failure mode (delete-response loss, idle-mutation stall, IPv6-localhost stall, 353s write_planning) failed to reproduce across 7 service-up runs; existing CME teardown already does exists-then-delete with one retry; a `cmem_eval_*` sweeper would delete live benchmark runs' collections; the 353s figure was a suite duration under a degraded host, not a code timeout.
  - Plan delta: REST verification, pre-run sweeper, and timeout-cap calibration dropped; explicit client deadlines (rule compliance), client/image version pin (new finding: `latest` tag drifted to 1.19.0), prefix-scoped prune script, and 127.0.0.1 defaults kept.
  - Tradeoffs considered: bump client vs pin image at 1.17/1.18 (rejected: the surviving volume was already opened by 1.19.0; downgrade unsupported).
  - User approval: yes (2026-09-02, "go with all the proposed changes").

## Notes
- Risks: qdrant-client 1.19.0 may change the erased-connect error shape; the canary exists precisely to catch it.
- Edge cases: prune script must never default a prefix; `test_collection_*` pruning is safe only under the live-run mutex.
