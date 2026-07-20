# Plan: Structured Verdict Observability and Workaround Cleanup

- status: approved
- generated: 2026-07-21
- last_updated: 2026-07-21
- work_type: code

## Goal
- Verdicts, diagnostics, and trace evidence are structured end-to-end: the library emits typed, lossless verdict data on every public path, and the evaluation harness propagates it into benchmark records — so infrastructure degradation can never masquerade as memory quality, and no consumer parses prose for load-bearing data.

## Definition of Done
- Every finding from the four 2026-07-20 sweeps (CM F1-F13 + R2-01..R2-16, CME rounds 1-2; consolidated in FOLLOWUP-SEED.md) is fixed, explicitly deferred with a recorded owner (v0.1.6 / v0.2 / rejected), or ruled out with rationale.
- Sealed evidence artifacts remain byte-identical and readable; report-schema evolution is versioned with the sealed-reader constraints designed in, not patched around.
- No test in either repo parses prose for load-bearing data on the touched paths.

## Scope / Non-goals
- Scope (waved below): CM verdict/error/trace structuring; CME verdict propagation into rows/summaries/reports incl. report-schema evolution; dead/dormant surface pruning; duplication consolidation.
- Non-goals / deferred by design: the vector-port findings (CM R2-03 completeness envelope, CME vector_only capability port) — designed once inside the v0.1.6 embedded vector-recall phase; lifecycle mode redesign (R2-02, R2-04's strict variant) — coordinate with v0.2 scoped-continuity, only the advertise-what-works constraint lands here; performance-grade idempotency ledger (R2-01 full solution) — this phase adds the narrow port method + TOCTOU documentation, the ledger is a v0.2+ decision.

## Context (workspace)
- Both repos on merged main (CM 62cdce2, CME 3d78847). Finding bodies: agmsg history 2026-07-20 21:58-22:25Z; index: docs/coding-agent/FOLLOWUP-SEED.md (untracked; delete when this plan absorbs it).
- Research gate satisfied by the four read-only forensic sweeps (codex worker2 / evals-worker).
- Rules in force: Compatibility Policy (no shims, sealed-artifact exemption), Workaround Tripwire (escalate, don't implement through).

## Open Questions (max 3)
- None (Q1-Q3 ruled 2026-07-21, see Decision Log).

## Assumptions
- A1: One PR per repo per wave (small, reviewable), same worker/reviewer routing as the sweep (codex implements, codex Tier D reviews, Claude altitude review on the verdict-schema design doc only).
- A2: Frozen stores and register-cited runs stay byte-identical; new-schema evidence is generated fresh, never by rewriting.

## Tasks

### Task_1: Verdict and error contract design doc (CM+CME, orchestrator-authored)
- type: design
- owns:
  - CharacterMemory: docs/design/ (one new design note), docs/decisions/ (ADR if warranted)
- depends_on: []
- description: |
  Design once, before code: the typed verdict vocabulary (validation issues/warnings with refs, F2/F10), the structured error story (rejection-half of F1, F7/F8/F11/F12 typed error payloads with Display prose), trace identity additions (F3 surface, F4 link_id, F5 typed section/omission reasons, F6 configured filters), and the CME report-schema evolution (Q1 decision) with sealed-reader constraints stated. Tier A review by a Claude reviewer.
- acceptance:
  - Design note enumerates every F/R2/CME finding it resolves, defers, or rejects, with the deferred ones owner-assigned.
  - Report-schema decision recorded with sealed-reader analysis.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier A altitude review (Claude): contract coherence, nothing designed twice, v0.1.6/v0.2 boundaries respected"

### Task_2: CM structured verdicts and errors
- type: impl
- owns:
  - CharacterMemory: src/**, tests/**
- depends_on: [Task_1]
- description: |
  Implement the Task_1 contract in the library: typed validation issues incl. rejection path; typed error payloads (config, collection-compat, bounded-failure, lifecycle facade); trace identity fields; telemetry configured-filter fields; single-owner postconditions (R2-09 conformance at port edges, remove use-case repair passes); F9 per-operation maintenance failures.
- acceptance:
  - All Task_1-assigned CM findings closed; no prose-parsing tests remain on touched paths.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test (lib + affected integration, live where gated)"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D (cm-reviewer): contract-complete propagation audit — every non-error field of every touched verdict type traced to a public sink or recorded intentional drop"

### Task_3: CM dead-surface pruning and duplication consolidation
- type: impl
- owns:
  - CharacterMemory: src/**, tests/**
- depends_on: [Task_1, Task_2]
- description: |
  R2-11 dormant slice per Q3 ruling; R2-12 speculative APIs deleted / cfg(test)-moved, GraphObjectQuery as enum; R2-08 central identity/order methods + single ObjectRef; R2-13 typed payload schema manifest; R2-14/15/16 hygiene (barrels, outcome clones, test support facade); R2-06/07 consolidation (vector-indexing service, stats projection service) as Task_1 assigns them.
- acceptance:
  - Duplicated helpers exist once; deleted surfaces leave no allow(dead_code) residue on touched modules.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "Full CM gate set as Task_2"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D: deletion completeness, no behavior change on live paths"

### Task_4: CME typed DTOs and verdict propagation
- type: impl
- owns:
  - CharacterMemoryEvals: crates/**, configs/** (NOT frozen stores/sealed artifacts)
- depends_on: [Task_1, Task_2]
- description: |
  CME rounds 1-2: typed core enums replacing stringly vocabularies (r2#1) with bounded sealed-artifact decoding retained; write/lifecycle verdict propagation into rows/summaries/reports (r1 MAJORs) per the Task_1 schema decision; metrics-shape admission (r1#5); typed EmbeddingRuntimeBinding (r2#3); dataset registry unification (r2#4); dead reset knobs (r2#5, unless pulled forward); typed retrieval surface policy (r2#6); owned context-pack constructor/renderer (r2#7); shared OpenAI embedding client (r2#8); shared atomic-replace helper (r2#9).
- acceptance:
  - Degraded write/lifecycle/vector state is visible in benchmark records; sealed artifacts byte-identical; sealed readers still pass.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked + synthetic mock smoke + pre/post sealed-artifact hash inventory"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D (evals-reviewer): propagation completeness, sealed integrity, schema-version conformance"

### Task_5: Integration, PRs, closeout
- type: chore
- owns: []
- depends_on: [Task_2, Task_3, Task_4]
- description: |
  Orchestrator: wave integration, PRs (content-named, monitors armed), Copilot reviews, seed-file deletion, plan archive.
- acceptance:
  - Both PRs merged or MERGE-READY per user preference; FOLLOWUP-SEED.md deleted.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "CI green both repos; Copilot clean"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]  (absorbs the domain relocations incl. ObjectRef unification as its first chunk, the R2-06/R2-07 service consolidations, and the R2-01 narrow slice; CM tasks are sequential in the shared checkout)
- Wave 3 (parallel): [Task_3, Task_4]  (Task_3 is pure pruning/hygiene after Task_2's moves land; Task_4 is CME, disjoint repo)
- Wave 4 (parallel): [Task_5]

## Rollback / Safety
- Feature-branch PRs; sealed artifacts never rewritten; schema changes versioned.

## Progress Log (append-only)

- 2026-07-21 Plan drafted from the four-sweep seed (43 findings); awaiting user approval and Q1-Q3 rulings.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-07-21 Scope decision at draft time: vector-port findings deferred to v0.1.6 planning; lifecycle-mode redesign coordinated with v0.2; both recorded in Non-goals.
- 2026-07-21 Tier A review of Task_1: NEEDS_REVISION, three MAJORs accepted and ruled. (1) Sealed-reader claim was wrong — results.rs:249-285 hard-rejects non-current schema versions; RULED option (a): bounded legacy 1.0.0 read dispatch retained solely for sealed register-cited evidence under the sealed-artifact exemption; trace/report schema constants bump to 2.0.0 in the same break; compatibility claims must cite reader file:line (rule candidate for reviewer.md/worker.md). (2) Wave 2 parallelism undischarged (shared MemoryObjectRef dependency + file conflicts); RULED: Task_2 absorbs relocations, ObjectRef unification, R2-06/07, R2-01 slice; Task_3 becomes pruning/hygiene depending on Task_2; waves restructured (Wave 2 = Task_2; Wave 3 = Task_3 + Task_4). (3) Write-path degradation causes stayed prose in the typed records; RULED: retype error_message fields reusing VectorDatabaseError with F10 kinds. Four MINORs accepted. Drafter is revising; delta re-review to follow.
- 2026-07-21 Tripwire escalation (worker, CM pull-forward) — RULED: the typed rejection error cannot import CandidateValidation from api (ADR-I-0018 forbids errors -> api). Authorized relocating CandidateValidation/CandidateValidationStatus/MemoryCandidateKind unchanged to a domain write-validation module (correct owning layer; Task_1's issue-enum typing evolves them there), with CustomError::WritePlanValidationRejected { validations } and Display-derived prose. Adjustment to the worker's proposal: no api::types re-exports (would re-create the B2-removed shim shape) — api imports from domain, public path is the flat crate root via lib.rs. Rejected workarounds recorded by the worker: api import into errors, string/boxed erasure, duplicate error-only type. First live firing of the Workaround Tripwire rule; escalate-then-rule worked as designed.
- 2026-07-21 Plan approved by user; Q1-Q3 ruled by adopting orchestrator recommendations: Q1 report schema bumps to 2.0.0 (clean break per Compatibility Policy; sealed readers of 1.0.0 artifacts keep their tolerance); Q2 the two pull-forwards land now as independent PRs (CME dead namespace-reset knobs; CM structured validation-rejection error — kept narrow: a typed error variant carrying the existing CandidateValidation rows, so it does not prejudge the Task_1 error taxonomy); Q3 the dormant governance/reconciliation slice (R2-11) is deleted, not gated (recoverable from history).

## Notes
- Risks: report-schema evolution touching sealed readers (mitigated by Task_1 design-first + Q1); CM/CME wave coupling (mitigated by Task_4 depending on Task_2).
- Edge cases: sealed-artifact tolerances are kept and documented, never "cleaned up".
