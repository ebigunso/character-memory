# Plan: Structured Verdict Observability and Workaround Cleanup

- status: done
- generated: 2026-07-21
- last_updated: 2026-09-14
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

- 2026-07-23 PHASE COMPLETE AND MERGED: CM PR #65 squashed as 0408e71; CME PR #15 squashed as ea01f8e (CI fully green on merged main — the merge-order coupling resolved exactly as predicted). Final arcs after the last recorded entry: the R3 audit closures (writer preflight family-wide; attempt identity per the consult ruling — (operation_id, attempt_index), contiguity-checked shape-agnostic dedup, once-per-operation degradation counting, repair_attempt_count); the terminal Copilot rounds (stats hydration reporting, wire-token parity, strict-admission chain down to a streaming whole-document duplicate validator with raw-source dispatch, writer label integrity, sealed-v1 tolerance restored surgically, aggregator identity single-owner, retrieval-payload consistency with typed per-constituent errors, vector-only zero-budget rejection); the Tier A value audit (APPROVED, no DELETE findings, two OVERSIZED trims seeded, precedent-drift warning); worker2's three-round thesis audit ending CLEAN. Loop terminated by user-directed exit rubric (in-PR only for phase-delivered evidence-integrity defects or phase regressions; bm25_only surface validation deferred to seed as the first application). Every commit on both PRs was internally Tier-D approved before push under the push-sequencing rule.
- 2026-07-23 Closeout: lessons batch integrated (typed-from-introduction promoted to worker.md rule; per-branch negative evidence, stateful idempotency testing, family-wide invariant census, agmsg/env tooling notes); evals-reviewer validation-table trigger refined to byte-shape intent; MutationPlan clone trim, deferral-reconfirmation checklist, and exit-rubric deferrals live in FOLLOWUP-SEED.md pending the next phases (teardown hardening first, then v0.1.6).

- 2026-07-21 Scope decision at draft time: vector-port findings deferred to v0.1.6 planning; lifecycle-mode redesign coordinated with v0.2; both recorded in Non-goals.
- 2026-07-21 Tripwire escalation #2 (worker, Task_2 F9/R2-06): the design's `cause: VectorDatabaseError` cannot represent pre-database failures on the shared index path (embed_batch, cardinality verification) — mapping them in would be false classification. RULED: one shared `VectorIndexingCause` enum in errors/domain — Embedding(#[source] Box<CustomError>), CardinalityMismatch { expected, actual }, VectorDatabase(VectorDatabaseError) — used by both F9 maintenance items and the R2-06/RepairMarker::VectorIndex causes; Delete ops simply never construct the non-database variants; no new provider-error taxonomy this phase (boxed typed source is lossless); StatsUpdate keeps its stats-side cause. Design doc to be amended at integration.
- 2026-07-21 Tripwire follow-up (same chunk): the orchestrator's Box<CustomError> refinement was itself wrong — CustomError is neither Clone nor serde-capable, and the containing outcome DTOs are serialized evidence. RULED option B (worker's recommendation): Embedding carries a closed serializable EmbeddingError enumerated from the provider producer sites F10-style (typed TransportStatus reuse, structured response-shape violations, Unrecognized(String) only for external upstream text). The shared VectorIndexingCause enum stands, fully serializable. Orchestrator-side lesson: cause-type refinements must be checked against the containing DTOs' derive obligations before ruling.
- 2026-07-22 Determinism MAJOR (evals-reviewer two-run gate): supersedes_link minted Uuid::new_v4 (correct_forget.rs:909-912), diverging graph_mutated_link_internal_ids across identical runs — violating the v0.1.3 deterministic-UUIDv5 contract. First live use of the Design-Consult Threshold: a design consult grounded the ruling — UUIDv5 via the existing deterministic_uuid under WRITE_PLAN_NAMESPACE with a lifecycle domain tag + (from_id, to_id), no event/rationale/timestamp inputs (retry convergence is the recorded idempotency philosophy; domain-tag partitioning keeps remember/lifecycle ID spaces disjoint so neither collision check can cross-trip). The consult also surfaced a second latent same-class defect: the replacement-ID v4 fallback at :861, fixed in the same commit via the write-planning object-ID derivation convention. Producer-side fix per the reviewer's sustained anti-normalization warning; local dev v4 links are rebuildable, no migration.
- 2026-07-22 Thesis-audit CM fixes complete at 7974d59 (F-01 b212570 multi-cause typed stats errors; F-02 73757ff ruled exception + canary per Amendment 10; F-03 d9d0aee raw-deserialize + TryFrom; full gates incl. live suites green). 7974d59's lessons.md edit breached the orchestrator-integration rule; content ratified post-hoc as orchestrator-adopted (reverting good content would be ceremony over substance), rule restated to both workers with parity noted to evals-worker, who was twice denied the same expansion.
- 2026-07-21 USER-DIRECTED final thesis audit (worker2, read-only, both final PR ranges): NOT CLEAN — six fix-round regressions of the phase's own thesis. F-01 HIGH stats-cause prose (violates design §5's own typed-stats-cause requirement; both repos); F-02 HIGH message-parsing transport shim under the typed HttpConnect kind (fix: source-chain downcast, or ruled+documented exception with canary); F-03 MEDIUM per-field config pre-read special case (fix: raw-representation deserialize + TryFrom); F-04 HIGH stale CME README/rules mandating the 1.0.0 contract (rules fixed by orchestrator 20bf34c; README to worker); F-05 MEDIUM anyhow prose invariant + contains-tests (fix: typed SummaryInvariantError); F-06 LOW roadmap version label in production error/test text. All dispatched; both reviewers' final verdicts widened to include the fix SHAs. Root-cause note: two findings implement my own rulings' letter while missing the thesis — fix-round implementations need the same tripwire scrutiny as feature chunks.
- 2026-07-21 USER-APPROVED teardown-transport waiver (Task_4/Task_5 validation): tests::live_adapter_reattaches_with_external_ids and tests::live_reset_preserves_sibling_namespace_durable_stores may fail ONLY at final cleanup on gRPC delete-response timeouts for this phase's validation runs. Evidence basis: deletions proven committed server-side (immediate REST 404 during the client 'timeout'); loss is Docker Desktop gRPC transport on both localhost and VM-IP routes on a fresh service; retry semantics diff-proven unchanged; substantive assertions still required to pass. Expiry: the waiver dies when the teardown-hardening task (user-confirmed first post-merge item; core fix = verify-deletion-via-REST on gRPC timeout, plus pre-run orphan sweeper and endpoint normalization) lands. Full failure catalog from 2026-07-21: service wedge under concurrent suites, live-run mutex instituted, timeout caps shorter than legitimate serial duration, IPv6-localhost fallback, deterministic delete-response loss.
- 2026-07-21 Task_4 row/summary rulings: (1) outcome records carry a deterministic operation identity; every dependent row carries the full record (independent certification), summaries dedup degradation counts by operation ID — first-row attribution rejected as uncertifying. (2) The legacy 1.0.0 dispatch is bounded to result rows + continuity traces (what the register cites for machine reading); summary and continuity-report readers stay strict 2.0.0-only, with the bounding cited in the dispatch's code comment — derived sealed artifacts are hash-verified, recomputable via the legacy row reader if ever needed.
- 2026-07-21 Task_4 design confirmations A-D, all approved as recommended by evals-worker: (A) DatasetId is a serde-transparent validated newtype in core with the descriptor registry runner-owned (a closed core enum would violate the dataset-independence rule); (B) 2.0 rows persist per-scenario typed EmbeddingBindingRecord, summaries aggregate sorted unique bindings, the untruthful config-derived embedding_provider field is deleted from 2.0 (V1 legacy DTO keeps it); (C) RetrievedContextPack sole constructor with private fields, no renderer-strategy ID and no read-time rerender (speculative surface; persisted context_text authoritative); (D) embedding config separates serializable shared resources from runtime bindings, scenarios build bindings without config rewriting or sentinels. Non_exhaustive removal landed as CM 33aa2a0; Task_4 conversion chunk unblocked, aggregate-gate hold stands until Task_3 settles.
- 2026-07-21 Tripwire #4 (Task_4 pre-implementation): the design self-contradicts — #[non_exhaustive] on the verdict vocabulary enums makes the promised CME compile-error-on-drift impossible (external crates must wildcard). Neither Tier A round caught it. RULED option A scoped to enums: Task_3 removes non_exhaustive from the closed vocabulary enums (TransportStatus keeps Unrecognized(String) as its in-vocabulary escape); non_exhaustive structs stay (read-only downstream, additive evolution); CME matches are exhaustive with no wildcard arms; Task_4's conversion chunk gated on the CM removal SHA. Rationale: under the Compatibility Policy, non_exhaustive is a backwards-compat affordance contradicting the repo philosophy; vocabulary drift must break loudly.
- 2026-07-21 Tripwire #3 (Task_2 F5): the designed SectionAssignmentReason vocabulary missed the graph-only/no-prompt-section producer branch (section_for_object returns None for Entity/MemoryLink and emits explicit omission rows). RULED: add OmittedNoPromptSection { object_type } as a fourth variant — an untruthful variant or dropped rows would both be workarounds. Root cause: the design enumerated reasons from the finding's citations, not from the producer's full branch set; Tier D on Task_2 must verify reason-vocabulary completeness against every section_for_object branch.
- 2026-07-21 Tier A review of Task_1: NEEDS_REVISION, three MAJORs accepted and ruled. (1) Sealed-reader claim was wrong — results.rs:249-285 hard-rejects non-current schema versions; RULED option (a): bounded legacy 1.0.0 read dispatch retained solely for sealed register-cited evidence under the sealed-artifact exemption; trace/report schema constants bump to 2.0.0 in the same break; compatibility claims must cite reader file:line (rule candidate for reviewer.md/worker.md). (2) Wave 2 parallelism undischarged (shared MemoryObjectRef dependency + file conflicts); RULED: Task_2 absorbs relocations, ObjectRef unification, R2-06/07, R2-01 slice; Task_3 becomes pruning/hygiene depending on Task_2; waves restructured (Wave 2 = Task_2; Wave 3 = Task_3 + Task_4). (3) Write-path degradation causes stayed prose in the typed records; RULED: retype error_message fields reusing VectorDatabaseError with F10 kinds. Four MINORs accepted. Drafter is revising; delta re-review to follow.
- 2026-07-21 Tripwire escalation (worker, CM pull-forward) — RULED: the typed rejection error cannot import CandidateValidation from api (ADR-I-0018 forbids errors -> api). Authorized relocating CandidateValidation/CandidateValidationStatus/MemoryCandidateKind unchanged to a domain write-validation module (correct owning layer; Task_1's issue-enum typing evolves them there), with CustomError::WritePlanValidationRejected { validations } and Display-derived prose. Adjustment to the worker's proposal: no api::types re-exports (would re-create the B2-removed shim shape) — api imports from domain, public path is the flat crate root via lib.rs. Rejected workarounds recorded by the worker: api import into errors, string/boxed erasure, duplicate error-only type. First live firing of the Workaround Tripwire rule; escalate-then-rule worked as designed.
- 2026-07-21 Plan approved by user; Q1-Q3 ruled by adopting orchestrator recommendations: Q1 report schema bumps to 2.0.0 (clean break per Compatibility Policy; sealed readers of 1.0.0 artifacts keep their tolerance); Q2 the two pull-forwards land now as independent PRs (CME dead namespace-reset knobs; CM structured validation-rejection error — kept narrow: a typed error variant carrying the existing CandidateValidation rows, so it does not prejudge the Task_1 error taxonomy); Q3 the dormant governance/reconciliation slice (R2-11) is deleted, not gated (recoverable from history).

- 2026-09-02 WAIVER RETIRED: rebuilt-machine verification against Qdrant 1.19.0 did not reproduce the July teardown failure catalog across seven service-up runs; CM live smoke tests and the full 405-test suite passed without leaks, CME live tests passed four of four runs with final cleanup, and the retry macro never fired. The approved qdrant-teardown-hardening-plan.md therefore replaced the expired waiver with explicit client deadlines, pinned client/server versions, IPv4-loopback defaults, and leak-visible best-effort cleanup; REST delete verification and pre-run sweepers were not added because the failures no longer reproduced and sweepers could delete concurrent runs.

## Notes
- Risks: report-schema evolution touching sealed readers (mitigated by Task_1 design-first + Q1); CM/CME wave coupling (mitigated by Task_4 depending on Task_2).
- Edge cases: sealed-artifact tolerances are kept and documented, never "cleaned up".

## Appendix: finding-disposition table and in-flight amendments (moved from the retired design note `docs/design/structured_verdict_contract.md`, 2026-09-14)

The design note that carried the contract of the plan recorded in this file was retired when its two durable rulings became ADR-I-0029 and ADR-I-0030; the per-finding dispositions and the amendments recorded during implementation are preserved here unchanged, as historical records of what was decided in July 2026. v0.1.6 (ADR-I-0023 to ADR-I-0028) changed some of the shapes they name (for example the shared vector-indexing cause enum gained a zero-norm-embedding variant); the code and the governing ADRs are authoritative for the current shape.

The section numbers cited in the table and the amendments refer to the retired note's own sections, which covered: section 1, the typed validation-issue vocabulary (now the closed-vocabulary rule of ADR-I-0029); section 2, the typed error story (the payload and display rules of ADR-I-0029); section 3, the trace identity additions (vector surface, link id, section-assignment reason, telemetry echo); section 4, postcondition ownership (ADR-I-0030); section 5, the library consolidations (indexing service, stats projection service, shared object reference, payload schema manifest, query enum); section 6, the contract that phase set for CharacterMemoryEvals, the public companion evaluation repository whose tooling is a development aid and not core library functionality (typed DTO vocabularies and report schema 2.0.0 with the bounded 1.0.0 dispatch, since retired by CharacterMemoryEvals ADR-I-0005, a record distinct from this repository ADR-I-0005); section 7, this table.

### Finding-disposition table

Legend: resolved-here = designed above and implemented in Task_2/3/4; pull-forward = landing in the pre-phase PRs already dispatched; deferred = owner-assigned, not designed here; dies-with-deletion = removed by the ruled R2-11 deletion.

| ID | Disposition | Section / owner |
| --- | --- | --- |
| CM F1 | pull-forward (rejection half: `WritePlanValidationRejected` + domain relocation) + as-built (success half, 13bc56f) | section 1 retypes the carried rows |
| CM F2 | resolved-here | section 1 |
| CM F3 | resolved-here | section 3 |
| CM F4 | resolved-here | section 3 |
| CM F5 | resolved-here | section 3 |
| CM F6 | resolved-here | section 3 |
| CM F7 | resolved-here | section 2 |
| CM F8 | resolved-here | section 2 |
| CM F9 | resolved-here | section 2 |
| CM F10 | resolved-here | section 2 |
| CM F11 | resolved-here | section 2 |
| CM F12 | resolved-here | section 2 |
| CM F13 | dies-with-deletion | R2-11 ruling |
| CM R2-01 | narrow slice resolved-here; ledger deferred | section 5; owner v0.2+ |
| CM R2-02 | deferred | v0.2 scoped-continuity coordination; only the typed `LifecyclePolicyUnsupported` rejection (section 2) lands now |
| CM R2-03 | deferred | v0.1.6 embedded vector-recall port design |
| CM R2-04 | deferred | v0.2 (strict variant with R2-02); the lossy projection is unchanged this phase |
| CM R2-05 | deferred | v0.1.6 vector-port design pass (query-side hint semantics belong to the same port contract) |
| CM R2-06 | resolved-here (incl. typed repair/indexing causes per MAJOR ruling) | section 5; Task_2 |
| CM R2-07 | resolved-here | section 5; Task_2 |
| CM R2-08 | resolved-here (ObjectRef unification in Task_2 first chunk; mechanical helper replacement in Task_3) | section 5 |
| CM R2-09 | resolved-here | section 4 |
| CM R2-10 | resolved-here (typed failure mode with F7; internal trace/root mode enums in Task_2/3) | section 3 |
| CM R2-11 | dies-with-deletion (ruled: delete, not gate) | Task_3 |
| CM R2-12 | resolved-here | section 5 shape; Task_3 |
| CM R2-13 | resolved-here (manifest + record_type drop); text-column decision deferred | section 5; v0.1.6 for text columns |
| CM R2-14 | resolved-here (hygiene, no contract design needed) | Task_3 |
| CM R2-15 | resolved-here | section 5 |
| CM R2-16 | resolved-here (test-support facade, no contract design needed) | Task_3 |
| CME r1#1 (typed-ingest verdict drop) | resolved-here | section 6, report schema 2.0.0 |
| CME r1#2 (explicit-commit Debug-flattening + asymmetry) | resolved-here | section 6 |
| CME r1#3 (lifecycle maintenance-failure drop) | resolved-here | section 6 |
| CME r1#4 (telemetry DTO gaps) | resolved-here | section 6 |
| CME r1#5 (untyped metrics Value) | resolved-here | section 6 |
| CME r2#1 (stringly core DTOs) | resolved-here | section 6 |
| CME r2#2 (vector_only hidden capability port) | deferred | v0.1.6 embedded vector-recall port design |
| CME r2#3 (embedding runtime binding) | resolved-here | section 6 |
| CME r2#4 (dataset registry) | resolved-here | section 6 |
| CME r2#5 (dead namespace-reset knobs) | pull-forward | independent PR already dispatched |
| CME r2#6 (boolean retrieval flags + magic budgets) | resolved-here | section 6 |
| CME r2#7 (context-pack renderers) | resolved-here | section 6 |
| CME r2#8 (duplicate OpenAI embedding client) | resolved-here | section 6 |
| CME r2#9 (copied atomic-replace helper) | resolved-here | section 6 |
| Copilot ADR-I-0018 edge: default_retrieval_object_types consumed from models | dies-with-deletion | the models-side consumers are the src/models/vector/candidate_record.rs default-type helpers (~120-137, 136, 400) deleted in Task_3's R2-12 scope, with Tier D verifying no live consumer; the canonical default set stays api-owned |
| Copilot ADR-I-0018 edge: RetrievalLifecyclePolicy in policy | deferred | v0.2 (rides the R2-02/R2-04 lifecycle coordination) |

Not in scope of this table by ruling: the CME `history_text` prose-encoded structure remains deferred-unless-a-parser-appears, as recorded in the backcompat plan addendum.

### Amendments (in-flight rulings during Task_2/3/4 implementation)

These rulings, recorded in the plan Decision Log at the time they were made, amend the sections above; the implementation is the authoritative expression.

1. Cause typing (amends section 5 R2-06 / section 2 F9): the vector-side cause is the shared serializable `VectorIndexingCause` enum — `Embedding(EmbeddingError)`, `CardinalityMismatch { expected, actual }`, `VectorDatabase(VectorDatabaseError)` — used by both maintenance items and indexing/repair causes; `EmbeddingError` is a closed serializable payload enumerated from the provider producer sites; the stats side uses its own `StatsUpdateCause`; `Box<CustomError>` was rejected because outcome DTOs are serialized evidence requiring Clone/serde/Eq.
2. Vocabulary closure (amends the section 1 sketch and section 6 exhaustiveness): `#[non_exhaustive]` is REMOVED from the closed verdict vocabulary enums (CandidateValidationIssue, RememberDiagnosticCode, VectorDatabaseErrorKind, TransportStatus, EmbeddingError and its transport kind, VectorIndexingCause, StatsUpdateCause) so CME's exhaustive conversions break loudly on drift; the read-only structs (RetrievalTelemetry, RetrievalTrace, VectorDatabaseError) keep the attribute. The section-1 sketch's `#[non_exhaustive]` contradicted section 6's compile-error promise; closure wins under the Compatibility Policy.
3. GraphFailureMode location (amends section 3 R2-10): the enum lives in domain (mode vocabulary; request-side DTOs are outside ADR-I-0018's api exception for ports/policy), flat crate-root export, api::types imports it for RetrievalGraphLimits.
4. SectionAssignmentReason vocabulary (amends section 3 F5): a fourth variant `OmittedNoPromptSection { object_type }` covers the graph-only/no-prompt-section producer branch; reason vocabularies must be enumerated from the producer's full branch set, not a finding's citations.
5. GraphObjectQuery empty semantics (amends section 5 R2-12): empty targeted input deterministically selects zero objects in every adapter; wildcard-on-empty is prohibited; any future query-all need gets an explicit variant.
6. CME row/summary identity (amends section 6): outcome records carry a deterministic operation identity; every dependent row carries the full record, summaries deduplicate degradation counts by operation ID; the untruthful config-derived `embedding_provider` summary field is deleted in 2.0.0 in favor of per-scenario typed `EmbeddingBindingRecord` aggregation.
7. Legacy-dispatch bound (sharpens section 6 / sealed-reader constraints): the 1.0.0 legacy read dispatch covers result rows and continuity traces only — the artifacts the register cites for machine reading; summary and continuity-report readers are strict 2.0.0-only, with the bound documented at the dispatch site.
8. DatasetId shape (amends section 6 r2#4): a serde-transparent validated newtype in core with the descriptor registry runner-owned; a closed core enum would violate the dataset-independence rule.
9. RetrievedContextPack (sharpens section 6 r2#7): sole constructor with private fields and accessors; no renderer-strategy ID and no read-time rerender — persisted `context_text` is the authoritative evaluated text.
10. HttpConnect classification exception (rules a verified external constraint, thesis-audit F-02): qdrant-client 1.17.0 irretrievably erases the tonic transport source (`channel_pool.rs` wraps it into `Status::internal(format!("Failed to connect to {}: {:?}", ...))`), so no structural downcast can exist at our boundary.
The adapter-contained prefix normalization is a ruled, documented exception — cited at the classification site, pinned by a canary test whose failure means the upstream message contract drifted, and retired automatically when a qdrant-client upgrade preserves the source (checked on every dependency bump).
Forking the client for one error path was rejected on cost; the tripwire's requirement is that unavoidable workarounds be ruled and visible, never silent.
11. Score-breakdown reconstruction invariant (rules the lossy-breakdown review finding): SectionScoreComponents publishes the EFFECTIVE vector input used by scoring plus typed provenance (`vector_score_source`: DirectMatch | DerivedFromRoot { root_score }), and max-merge keeps provenance tied to the winning component; published components must reconstruct the published final_score for both direct and derived rows, enforced by a production-path regression.
12. Write-outcome stats conservation (rules the consumer-boundary Copilot findings, 2026-07-22): every consumer of the stats projection service propagates the returned typed `StatsUpdateStatus` to its public outcome — `remember` via `RememberOutcome`, `correct`/`forget` via `LifecycleMutationOutcome.stats_update_status`, and `link` via the new `LinkOutcome { link, stats_update_status }` (a bare `Vec<StatsUpdateCause>` was rejected as duplicating the owned status contract while dropping attempted IDs).
CME mirrors the field on `LifecycleOutcomeRecord` and the link write records, and counts lifecycle stats failures in the degradation summary; the historical v0.1 phase doc's `link -> MemoryLink` signature stays unchanged as an append-only record.
