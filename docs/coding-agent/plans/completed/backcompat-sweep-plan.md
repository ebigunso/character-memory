# Plan: Backwards-Compatibility Sweep of the v0.1.5 Merge PRs

- status: done
- generated: 2026-07-21
- last_updated: 2026-07-21
- work_type: mixed

## Goal
- Remove all unnecessary backwards-compatibility code that slipped into CM PR #63 (`feature/v0-1-5-embedded-default`) and CME PR #13 (`feature/v0-1-5-config-overrides`), so only the latest supported surfaces remain, and record the no-backcompat convention as a durable repository rule in both repos.

## Definition of Done
- Every backcompat instance found by the forensic inventories is either removed or explicitly kept with a recorded reason (sealed-fixture/artifact protection is the only anticipated keep reason).
- Both PR branches pass repo validation commands and Copilot re-review after the cleanup commits.
- The no-backcompat convention is recorded in `docs/coding-agent/rules/common.md` of both repositories.

## Scope / Non-goals
- Scope: source trees of both PR branches (src/, tests/, config surfaces); rule files; PR branch commits + push + Copilot re-review.
- Non-goals: merging the PRs (user-owned); changes that would invalidate frozen embedding stores, their hashes, or sealed evidence artifacts; any new feature work; historical documents (completed plans, dated ADR bodies) stay unchanged.

## Context (workspace)
- User ruling (2026-07-21, chat): the library has no consumers yet and is free to change; backwards compatibility is simply not needed.
- Forensic inventories dispatched via agmsg to codex `worker` (CM) and `evals-worker` (CME); a preliminary Claude Explore pattern sweep runs in parallel as a cross-check anchor. Codex inventories are the scrutiny of record.
- Repo rules consulted: `common.md`, `orchestrator.md` (both repos share suite conventions).

## Open Questions (max 3)
- Q1: none currently.

## Assumptions
- A1: Cleanup lands as additional commits on the two open PR branches (the user described the backcompat code as having "slipped in while the code in the two PRs were written").
- A2: Anything whose removal would change on-disk store schema of frozen CME fixtures or invalidate committed evidence is flagged and kept, not removed, unless the user rules otherwise.

## Tasks

### Task_1: Remove backcompat surfaces in CM (PR #63 branch)
- type: impl
- owns:
  - CharacterMemory: src/**, tests/**
- depends_on: []
- description: |
  On `feature/v0-1-5-embedded-default`, remove every inventory-confirmed backwards-compatibility surface: deprecated shims, legacy aliases/re-exports, serde old-name tolerance, compat-motivated dual APIs, migration code for never-shipped formats. Latest surface only. Commit on the branch (no push).
- acceptance:
  - All inventory items in CM are removed or recorded as kept-with-reason in the worker report.
  - No `#[deprecated]`, compat-comment, or legacy-alias surface remains in src/ or tests/.
  - Tests updated to target the surviving surfaces only (assert kinds + load-bearing tokens, not phrasing).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer (codex): confirm removals complete per inventory, no behavior regressions, no new compat surfaces"

### Task_2: Remove backcompat surfaces in CME (PR #13 branch)
- type: impl
- owns:
  - CharacterMemoryEvals: crates/**, src/**, tests/**, fixtures config surfaces (NOT frozen stores or committed evidence artifacts)
- depends_on: []
- description: |
  On `feature/v0-1-5-config-overrides`, remove every inventory-confirmed backcompat surface (old CM API shims, serde old-field tolerance, legacy config keys/flags, dead dual paths). Frozen real-embedding stores, their hashes, and sealed evidence artifacts are untouchable; flag any item that would require touching them. Cargo runs use `--locked`. Commit on the branch (no push).
- acceptance:
  - All inventory items in CME are removed or recorded as kept-with-reason.
  - Frozen stores/hashes/evidence artifacts are byte-identical before and after.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked (mock/offline scope; live Qdrant not required for this sweep)"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by evals-reviewer (codex): removals complete, sealed artifacts untouched (hash check)"

### Task_3: Record the no-backcompat convention as a repo rule (both repos)
- type: docs
- owns:
  - CharacterMemory: docs/coding-agent/rules/common.md
  - CharacterMemoryEvals: docs/coding-agent/rules/common.md
- depends_on: []
- description: |
  Orchestrator-authored (rule files are orchestrator-only): add a common rule stating that until the library has external consumers, backwards compatibility is not a goal; changes replace old surfaces outright, and compat shims/aliases/migration paths must not be introduced. Mirror in CME per the both-repos precedent for cross-cutting conventions.
- acceptance:
  - Rule present in both repos' common.md, one-line-per-sentence, no hard wrap.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Rule wording consistent across repos; frontmatter last_updated bumped"

### Task_4: Integration, push, Copilot re-review
- type: chore
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Orchestrator: wave-integration checklist, push both branches, request Copilot re-review via the REST fallback if needed, keep PR monitors armed.
- acceptance:
  - Both PR branches pushed; CI green; Copilot re-review requested and clean or findings resolved.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "gh checks green on both PRs; Copilot review state verified"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_2, Task_3]
- Wave 2 (parallel): [Task_4]

## Rollback / Safety
- All changes are commits on feature branches; revert by dropping the cleanup commits. No force pushes.

## Progress Log (append-only)

- 2026-07-21 Forensic inventory dispatches sent to codex worker (CM) and evals-worker (CME) via agmsg; Claude Explore preliminary sweep launched in parallel.
- 2026-07-21 Explore preliminary inventory complete. CM: (A) `assistant_behavior_note` serde alias + dual const (`src/domain.rs:64,76-79`) + alias test (`src/api/types/draft.rs:769-770`); (B) removed `service` graph-mode migration hints (`src/config/app_settings.rs:84-88,290-293`) + two hint tests; (C) dormant `migrate_current_schema` seam + tests (`src/domain/schema.rs:19-72`); (D) serde defaults for older trace payloads (`src/api/types/retrieval.rs:222,259,324-328,497,518` + test at 920-944); (E) Qdrant `schema_version` payload field (`src/adapters/qdrant/payload.rs:85`) + missing-legacy-field diagnostic tolerance (`store.rs:1048-1069`). CME: (F) legacy-telemetry Option/default tolerance (`cmem-eval-core/src/memory_adapter.rs:164-177` + test 1352-1373); (G) legacy embedding byte-compat guard tied to sealed fixtures (`deterministic_embedding.rs:74-118`). Zero hits: #[deprecated], compat renames, path-preserving re-exports, compat config keys/CLI flags.

- 2026-07-21 Wave 1 CM side complete: Task_1 done by codex worker (3cdb77a impl/tests, 39f629e living docs); orchestrator commits e316048 (ADR-I-0021 latest-only wording) and b5bbb29 (Compatibility Policy rule + lessons). Worker validation all pass: fmt/check/clippy -D warnings/test --no-run, 359 lib tests, 2 public facade + 3 retrieval guardrails + 19 write-planning live tests incl. the remember/manual-plan equivalence test. Residual risk (worker lesson candidate): removing an env key leaves developer-local .env files stale — live runs must pass current keys explicitly; the user's local .env still carries the removed OXIGRAPH_CONNECTION_STRING and was intentionally left untouched. Task_2 CME-local edits done, aggregates now un-held against b5bbb29. cm-reviewer Tier D dispatched over 3cdb77a..b5bbb29.

- 2026-07-21 Wave 1+2 complete; both Tier D reviews APPROVED.
  - CM: worker fix cacdddd (equivalence test strengthened to full-contract comparison; roadmap staleness); orchestrator CI fix 25434df. cm-reviewer APPROVED at cacdddd, no remaining findings.
  - CME: Task_2 a294f05 + rule c911989; reviewer MAJOR (topology test partial-field assertions) fixed as 4cbc1b0 (complete object/link MemoryCandidate equality). evals-reviewer APPROVED at 4cbc1b0 with sibling pinned b5bbb29; sealed artifacts verified byte-identical throughout.
  - Cross-repo pattern: both workers independently under-asserted equivalence tests; both reviewers independently caught it — recorded as a shared lesson in both repos' lessons.md.
  - Remaining: push both branches, Copilot re-review, PR monitors, worktree cleanup.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-07-21 Decision: proceed without a separate plan-approval round trip.
  - Trigger / new insight: the user's chat directive (2026-07-21) prescribes the full scope: sweep both PRs for backcompat code, remove it, record the convention as a common rule.
  - Plan delta: plan authored directly from the directive; inventories refine task contents but not scope.
  - Tradeoffs considered: waiting for formal approval adds an async round trip with no scope uncertainty to resolve.
  - User approval: directive itself; plan summary sent to the user via agmsg for visibility and veto.

- 2026-07-21 Decision: disposition of the preliminary inventory.
  - Remove outright: A (alias + dual const + alias test), B (removed-mode migration hints revert to the standard invalid-value error path), C (dormant migration seam + tests) — all source-only, no store or fixture entanglement.
  - Verify-then-remove (Codex workers): D and F are a matched pair (library emits traces, CME tolerates old shapes) — remove both IF no committed/sealed artifact is re-deserialized by current production or test paths other than the compat tests themselves; otherwise flag back. E: verify nothing in CME's live adapter/reattach logic filters on `schema_version` and that Qdrant collections remain rebuildable derived state; if confirmed, remove the missing-legacy-field tolerance, and keep or drop the `schema_version` payload field per what the reattach verification shows.
  - Keep: G (legacy embedding byte-compat guard) — it enforces the seal on the frozen embedding fixtures; it is fixture-integrity protection, not consumer backcompat.
  - Tradeoffs considered: removing D/F without the artifact-read check could break evidence tooling silently; the check is cheap.
  - User approval: within the standing directive; dispositions recorded for veto.

- 2026-07-21 Decision: dispositions for the codex worker's full CM inventory (deeper than the preliminary pass).
  - Approved removals beyond the preliminary list: compat-motivated `api::types` domain re-exports incl. the one-time internal import sweep to `crate::domain` (resolves the grandfathered debt noted in orchestrator.md); lifecycle + `count_scope` serde old-payload defaults (same class as item D); rename of the repurposed service-era `oxigraph_connection_string`/`OXIGRAPH_CONNECTION_STRING` to `oxigraph_path`/`OXIGRAPH_PATH`; the legacy `remember(RememberDraft)` facade with its `RememberPipeline` closure and orphaned `RememberOptions` (prepare→validate→commit is the sole write path; test coverage migrates, not disappears); the legacy `TextEmbeddingAda002` variant pending a CME cross-check for references.
  - Keep (ruled current design, not backcompat): end-to-end `schema_version` persistence (ADR-I-0007 forward-migration provision), `require_current_schema_version` strict guard, `#[non_exhaustive]` forward-compat attributes, pub(crate) barrel re-exports (architecture hygiene, out of scope).
  - Storage ruling: local runtime stores (Qdrant collections, Oxigraph dirs) are rebuildable derived state with no consumers — reset is authorized; the dormant Qdrant reconciliation mapper's missing-legacy-field tolerance is removed and diagnostics require canonical fields.
  - Docs ruling: in-PR docs (README, .env.example, closeout report, roadmap wrapper promises) are corrected by the worker as part of the change since PR #63 is unmerged; ADR-I-0021 wording and the ADR-I-0012 amendment note are orchestrator-authored after the worker's commits land (sequenced to avoid concurrent edits in the shared checkout). Historical phase docs stay append-only unchanged per ADR-I-0021.
  - Scope delta: Task_1 `owns` expands to README.md, .env.example, docs/roadmap/development_roadmap.md, docs/roadmap/v0_1_5_closeout_report.md; ADRs excluded (orchestrator-owned). Removing a public facade (`remember`) exceeds shim deletion — flagged to the user via agmsg for veto before push.
  - User approval: within the standing directive; veto window open until push.

- 2026-07-21 Decision: sealed-evidence stop condition fired on F; D placed on hold.
  - Trigger / new insight: evals-worker evidence — all eight register-cited v0-1-5-baseline results/traces files (gitignored, hash-recorded, sealed) omit the three graph-root counter fields, and current CME readers (`read_jsonl` results.rs:249-263, `read_continuity_traces` driver.rs:100-132) deserialize them; Task_6 sealed results.jsonl also omits them.
  - Plan delta: F is KEEP under the sealed-artifact exemption, reclassified with a sealed-evidence comment so future sweeps don't re-flag it; library-side C-serde-defaults/D removals are HELD pending evals-worker's determination of whether those reader paths deserialize library types or CME-local mirrors — if library types, the library defaults are load-bearing for sealed evidence and are kept + reclassified likewise. ada-002 removal is GO on both sides (CME references are current code/tests only; no fixtures or provenance). H1 stays GO (no CME reads of the Qdrant schema_version payload).
  - Tradeoffs considered: requiring current shape in CME would force regenerating sealed baselines — defeats the seal; keeping tolerance costs nothing.
  - User approval: consistent with the sealed-artifact exemption the user's convention anticipated.

- 2026-07-21 Decision: USER VETO on item E; corrected disposition after design-intent review.
  - Trigger / new insight: user ruling — remember() was always intended as the consumer convenience API internally running prepare/validate/commit. Confirmed by philosophy §9.1 (remember is the primary lifecycle verb), ADR-I-0012 (accepted: remember() is the convenience wrapper; equivalence validation required), roadmap ("keep remember() as a convenience wrapper"), README:148. The v0.1.3 phase doc's "legacy" phrasing refers to internals/shape: shipped remember(RememberDraft) still runs the pre-plan-era RememberPipeline (vectors derived from committed objects, diverging from plan-path indexing) and the intended remember(input, options) shape was never implemented (RememberOptions is its orphan).
  - Plan delta: E changes from remove to REWORK — implement remember(RememberInput, RememberOptions) as a thin prepare→validate_plan→commit composition per ADR-I-0012, remove the divergent RememberPipeline path and draft-typed signature, wire RememberOptions minimally, add the ADR equivalence test; README/roadmap wrapper lines updated to the new signature, not deleted. ADR-I-0012 needs no amendment — the rework implements it.
  - Tradeoffs considered: leaving remember(RememberDraft) as-is would keep a divergent old write path alive (contradicts both the user intent and the Compatibility Policy); full removal contradicted the design record.
  - User approval: yes — direct chat correction 2026-07-21.
- 2026-07-21 Decision: C/D hold lifted. evals-worker type-provenance determination: CME evidence readers deserialize only CME-local mirror types (PerQuestionResult, memory_adapter::RetrievalTelemetry, ContinuityQueryTrace); no library type on sealed paths. Library C/D serde-default removals proceed as originally ruled.

- 2026-07-21 Decision: CME migration off RememberDraft uses a CME-side current-plan helper, not a new library seam.
  - Trigger / new insight: evals-worker evidence — CME's adapter used remember(RememberDraft) for arbitrary typed batches + enrichment (entities/threads/deriveds/links); the new remember(RememberInput) convenience only emits primary episode+observation candidates, so mechanical replacement would ignore batch members, synthesize unregistered objects, and drift controlled-similarity fixture topology.
  - Plan delta: Task_2 gains a bounded CME adapter helper converting typed drafts to the current public RememberWritePlan candidate surface, then validate + commit with explicit CommitOptions; constraints — exact embedding-text reproduction, in-plan vector targets, explicit timestamps, topology-equivalence test, mock smoke. CM stays settled at b5bbb29 (no new import API).
  - Tradeoffs considered: a CM typed-import seam would reopen the settled library after validation and duplicate what the plan surface already provides; ADR-I-0012 names application-owned plan composition as the intended workflow.
  - User approval: within standing directive; recorded for closeout review.

## Notes
- Risks: removals that silently change persisted-store schema in CME; mitigated by the sealed-artifact flag rule and byte-identity acceptance in Task_2.
- Edge cases: ordinary `#[serde(rename)]` for wire-name conventions is not backcompat; do not remove.
