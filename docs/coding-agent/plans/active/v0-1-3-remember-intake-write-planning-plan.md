# Plan: v0.1.3 Remember Intake Interfaces and Deterministic Write Planning

- status: in_progress
- generated: 2026-07-02
- last_updated: 2026-07-02
- work_type: code

## Goal
- Make the memory write path generation-ready: introduce an inspectable prepare → validate → commit workflow (`RememberInput`, `MemoryCandidate`, `RememberWritePlan`, `CandidateValidation`, `CandidateProvenance`, `RememberDiagnostics`) plus deterministic helpers, so manual writes today and generated candidates in v0.6 share one safe, provenance-gated commit path — with zero semantic inference from raw text.

## Roadmap / Philosophy Context
- v0.1.2 hardened retrieval guardrails *before* intake got easier; v0.1.3 now builds the safe write path *before* v0.2 continuity structures and v0.6 generated candidates must travel through it. It is the last feature phase of the v0.1 family (v0.1.4 eval harness and v0.1.5 closeout exercise this path).
- Implements roadmap cross-version invariant "generated and manual writes share one safe path"; introduces source spans as opaque provenance.
- Philosophy anchors: provenance-gated behavior influence; raw source is evidence not substrate (`raw_ref` opaque, never resolved/persisted); candidates are not memory until validated and committed; entity-neutral helpers; append-only correction semantics; inspectability of why/where memory came from.
- Authority invariant preserved: Qdrant suggests, stats guide fanout, Oxigraph decides.

## Definition of Done
- `prepare()`, `validate_plan()`, `commit()` exist on `CharacterMemory`; prepare/validate persist nothing; `commit()` revalidates against current graph state before writing.
- `remember()` remains available and routes through the same prepare/validate/commit machinery.
- Validation rejects behavior-influencing `DerivedMemory` without episode/observation provenance and `MemoryLink` targets absent from both plan and graph (or defers per explicit policy).
- Idempotency (deterministic IDs + idempotent graph upsert keyed by plan idempotency keys) prevents duplicate writes on retry.
- Deterministic helpers exist for stable IDs, graph IRIs, idempotency keys, source references, source spans, lifecycle/currentness/schema-version defaults, provenance links, and embedding-text fallback — with no inference of preferences, commitments, corrections, character signals, thread membership, or entity identity from raw text.
- `CandidateProvenance` records `CandidateProducerKind` and `RationaleOrigin`; inferred rationale cannot masquerade as caller-provided; missing rationale representable as unavailable.
- Commit distinguishes critical failures (Oxigraph objects/links/provenance/lifecycle) from repairable ones (Qdrant, stats, diagnostics), mirroring the existing `VectorIndexingFailure` pattern.
- Flow works in in-memory and persistent graph modes; integration tests cover the acceptance criteria in the phase doc.
- `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --no-run` pass; `cargo test` passes with live services.

## Scope / Non-goals
- Scope:
  - New public types under `src/api/types/` (write-plan/candidate/provenance/diagnostics surface) with re-exports in `src/api/types.rs` and `src/lib.rs`.
  - New internal write-planning module in `src/internal/repositories/` (plan assembly + validation), commit path refactor around `remember_pipeline.rs`.
  - Facade methods on `CharacterMemory`; `remember()` as wrapper.
  - Unit tests in-module; integration tests `tests/v0_1_3_*_tests.rs` (in-memory + persistent graph modes).
- Non-goals (per phase doc + ADRs):
  - No LLM/model/rule-assisted extraction, summarization, salience scoring, admission control, or privacy classification; no entity/thread/scope inference from natural language.
  - No commit-mode enum (ADR-I-0012); no application review callback framework; no generic `MetaMemory`/durable rationale metadata plane (ADR-D-0016, ADR-I-0015).
  - No v0.6 admission states (Accepted/Deferred/NeedsReview/Rejected/Invalid).
  - No raw-log storage/search or `raw_ref` resolution (ADR-D-0008, ADR-D-0015).
  - No new memory object types; no retrieval-pipeline or stats-semantics changes beyond stats-update plumbing from commit.
  - v0.2 owns continuity scopes/reflection/commitments/open loops; scope IDs are carried opaquely at most.

## Context (workspace)
- Related files/areas: src/lib.rs (facade), src/api/types/draft.rs (`RememberDraft`, `DraftDefaults`, `RememberOutcome`), src/api/types/domain.rs (canonical objects + `validate()`), src/api/types/lifecycle.rs (`SourceProvenanceReference`), src/internal/repositories/remember_pipeline.rs (current write path), src/internal/repositories/graph_authority_store.rs (`GraphObjectQuery::by_refs` for existence checks), src/internal/repositories/link_pipeline.rs (`LinkAdmissionEvidence` pattern), src/internal/repositories/source_reference.rs (parked seam), tests/support/*.
- Existing patterns: `foo.rs` module layout; drafts with `with_*` builders + `into_domain_with_defaults`; internal→public outcome conversion via `From` at facade boundary; `CustomError::MemoryValidation`; per-call pipeline construction.
- Repo reference docs consulted: docs/roadmap/development_roadmap.md §9, docs/design/roadmap-phases/v0_1_3_remember_intake_interfaces_deterministic_write_planning.md, docs/project_philosophy.md, ADR-D-0002/0008/0012/0015/0016/0017, ADR-I-0001/0005/0007/0008/0012/0013/0015, docs/design/database/*.

## Open Questions (max 3)
- Q1: Extend the existing public `RememberOutcome` in place (pre-1.0 breakage acceptable, tests updated together) or introduce a distinct commit-outcome type? (Default assumption: extend in place.)
- Q2: Missing `MemoryLink` target policy — strict reject only, or also an explicit defer option (link candidate dropped into a diagnostics/unresolved set, never committed)? (Default assumption: strict reject in v0.1.3; defer documented as future option.)
- Q3: Do `CandidateProducerKind`/`RationaleOrigin` persist into Oxigraph (RDF mapping + schema-version bump) or stay plan/diagnostics-only write-time metadata? (Default assumption: plan/diagnostics-only; no schema bump.)

## Assumptions
- A1: Idempotency contract — plan carries an idempotency key; deterministic object/link IDs + idempotent graph upsert make retries safe. Semantics: same key + same deterministic plan → idempotent no-op on already-committed writes; same key + divergent candidate IDs/content → rejected with a diagnostic; graph-success-with-repairable-vector/stats-failure retried under the same key must not duplicate graph writes. No persisted operation ledger in v0.1.3.
- A2: Existing `remember(RememberDraft)` signature stays source-compatible; new input/options types are additive. Explicit API-shape decision: `RememberInput` is reachable via `prepare()` only; `remember()` keeps its current draft-based signature (phase-doc `remember(input, options)` shape is treated as suggested, delta reconciled in Task_6).
- A3: The plan path honors the same link-admission rules as `link()` (invariant: one safe shared write path).
- A4: `RememberWritePlan` is serializable (serde) so applications can build approval workflows around it.

## Tasks

### Task_1: Public write-plan type surface
- type: impl
- owns:
  - src/api/types/write_plan.rs (+ optional src/api/types/write_plan/ subfiles)
  - src/api/types/draft.rs (`RememberOutcome` extension per Q1 default)
  - src/api/types.rs (re-export additions)
  - src/lib.rs (pub use additions only)
- depends_on: []
- description: |
  Define `RememberInput`, `MemoryCandidate` (episode/observation/entity/thread/derived-memory/link + vector-index and stats-update candidates), `RememberWritePlan`, `CandidateValidation`, `CandidateProvenance` with `CandidateProducerKind` and `RationaleOrigin`, `SourceSpan`, `RememberDiagnostics`, and options types, following the phase-doc API shapes near-verbatim and existing draft/builder conventions. Decide/settle the `RememberOutcome` extension per Q1 default. Serde-serializable (A4). No roadmap version labels in identifiers.
- acceptance:
  - Types compile, serialize/deserialize round-trip, and follow existing naming/builder conventions.
  - Structural unit tests: source-span validity, producer-kind/rationale-origin invariants (inferred rationale cannot claim caller origin), missing rationale representable.
  - Re-exported flat from crate root like existing API types.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test (unit tests, no services needed)"

### Task_2: Deterministic prepare helpers
- type: impl
- owns:
  - src/api/types/write_plan.rs (helper impls only; type definitions land in Task_1)
  - src/api/types/write_plan/ helper subfiles (e.g., src/api/types/write_plan/helpers.rs)
- depends_on: [Task_1]
- description: |
  Deterministic construction helpers: stable IDs, graph IRIs (reuse `graph_uri()`), idempotency keys, source references/spans, one-input-one-episode construction, observation wrapping, caller-hint linking (entity/thread/opaque scope IDs, participants, timestamps, raw refs), retention/currentness defaults, schema-version assignment (reuse `DEFAULT_SCHEMA_VERSION`), provenance links, embedding-text fallback from caller content. Build on `DraftDefaults`. Same input + fixed defaults → identical plan. No semantic inference (ADR-I-0013).
- acceptance:
  - Helpers are pure/deterministic; property-style unit test proves same-input-same-plan.
  - No helper interprets raw natural language beyond verbatim carriage.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test (unit tests)"

### Task_3: Write-plan validator
- type: impl
- owns:
  - src/internal/repositories/write_planning.rs (validator section)
  - src/internal/repositories.rs (module wiring/re-exports)
- depends_on: [Task_1]
- description: |
  Implement the phase-doc validation rule set: IDs/object types/schema versions present; behavior-influencing `DerivedMemory` provenance gating (targets exist in plan or graph); `MemoryLink` target existence-or-in-plan (strict reject per Q2 default); lifecycle/currentness consistency, specifically (verbatim from phase doc) "suppressed memories are not current" and "superseded memories are not current unless explicitly historical"; vector/stats candidates reference graph-authoritative plan-or-graph objects only; source-span structural validity; idempotency-key presence and same-key/divergent-content rejection (A1); producer-kind/rationale-origin conflation rejection; `raw_ref` opacity. Use `GraphObjectQuery::by_refs` for existence checks; mirror `LinkAdmissionEvidence` decision pattern; honor `link()` admission rules (A3).
- acceptance:
  - Every validation rule has at least one rejecting unit test (using `FakeGraphAuthorityStore`).
  - Invalid plans cannot reach commit.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test (unit tests with fakes)"

### Task_4: Commit path, facade methods, remember() wrapper
- type: impl
- owns:
  - src/internal/repositories/remember_pipeline.rs
  - src/internal/repositories/write_planning.rs (commit section)
  - src/lib.rs (facade methods + outcome From impls)
- depends_on: [Task_2, Task_3]
- description: |
  Add `prepare()`, `validate_plan()`, `commit()` to `CharacterMemory` following the per-call pipeline construction pattern. `commit()` revalidates against current graph state, writes Oxigraph first (critical), then Qdrant vectors and stats (repairable; reported in outcome/diagnostics with repair markers). Implement the full idempotency contract from A1 (idempotent exact retry; divergent same-key rejection; no graph-write duplication when retrying after repairable failures). Refactor `remember()` to run prepare+validate+commit over the same machinery with unchanged public behavior (A2).
- acceptance:
  - prepare/validate persist nothing (verified by tests); commit revalidates.
  - `remember()` behavior parity: existing unit tests pass unchanged or with deliberate documented migration.
  - Outcome carries vector/stats status, repair markers, diagnostics; critical vs repairable split enforced.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test (unit tests)"

### Task_5: Integration tests and acceptance sweep
- type: test
- owns:
  - tests/v0_1_3_write_planning_tests.rs
  - tests/support/ (minimal additions only)
- depends_on: [Task_4]
- description: |
  Map every phase-doc acceptance criterion to at least one integration test: prepare-without-persist, validate-without-persist, commit-revalidation, ungrounded DerivedMemory rejection, missing link-target rejection, idempotent exact retry (no duplicates) plus divergent same-key rejection, source ref/span preservation without raw_ref resolution, no-inference guarantees, in-memory + persistent graph modes, Qdrant-unavailable skip gating preserved, authority split preserved (vector/stats failures repairable, never behavior-influencing truth), and an application-space approval flow per ADR-I-0012 (prepare → inspect/filter candidates → commit the approved plan; removed candidates do not persist).
  Persistent graph mode uses the embedded file-backed Oxigraph store (tests/support/persistent.rs); only Qdrant needs a compose service. Use docker-compose.oxigraph.test.yml only if service-mode graph tests are added.
- acceptance:
  - Acceptance-criteria-to-test traceability listed in the test file header or module docs.
  - Full suite green with live Qdrant started first (see lessons.md).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "docker compose -f docker-compose.qdrant.yml up -d && cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test"

### Task_6: Docs alignment
- type: docs
- owns:
  - README.md (write-path section)
  - docs/roadmap/development_roadmap.md (v0.1.3 status cell only)
  - docs/design/roadmap-phases/v0_1_3_remember_intake_interfaces_deterministic_write_planning.md (implementation-note deltas only)
- depends_on: [Task_4]
- description: |
  Document the prepare/validate/commit workflow and explicit non-goals; reconcile any shipped-API deltas with the phase doc; update roadmap status.
- acceptance:
  - No contradiction between shipped API and design docs; non-goals stated.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Docs consistency check against shipped API during final review"

### Task_7: Final review gate
- type: review
- owns: []
- depends_on: [Task_5, Task_6]
- description: |
  Reviewer verifies acceptance criteria, ADR-constraint conformance (provenance gating, authority split, no inference, opacity of raw_ref), validation evidence completeness, and API-shape fidelity to the phase doc.
- acceptance:
  - Reviewer status is APPROVED.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Independent review of diffs + validation evidence; spot-run cargo test with services if warranted"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2, Task_3]
- Wave 3 (parallel): [Task_4]
- Wave 4 (parallel): [Task_5, Task_6]
- Wave 5 (parallel): [Task_7]

## Rollback / Safety
- All work on a feature branch; no shared-state git mutations without Orchestrator control.
- New API surface is additive; `remember()` parity preserved, so reverting the branch restores current behavior.
- No schema-version bump expected (per Q3 default); if Q3 flips, activate the dormant migration seam deliberately in a plan update.

## Progress Log (append-only)

- 2026-07-02 Wave 1 completed (implementation): [Task_1]
  - Summary: Public write-plan type surface implemented (src/api/types/write_plan.rs new; RememberOutcome extended additively in draft.rs; re-exports in types.rs/lib.rs). Worker returned blocked-on-validation; Orchestrator completed evidence.
  - Validation evidence: cargo fmt --check PASS; cargo check PASS; cargo clippy --all-targets -- -D warnings PASS; cargo test --no-run PASS; cargo test --lib PASS (307/307, 2 ignored). Full cargo test: v0_1_2 Qdrant-backed integration tests fail identically on baseline HEAD (stash test) — pre-existing environmental issue, not a Task_1 regression. Root cause partially fixed (localhost→127.0.0.1 for Qdrant gRPC in local .env; see lessons.md); residual flakiness/slowness in tests/v0_1_2_retrieval_guardrails_tests.rs remains under parallel execution. Live-suite green evidence pending user waiver decision or stabilization; independently re-required at Task_5.
  - Notes: Qdrant container crashed (exit 255) mid-diagnosis and was recreated with a fresh volume (test-only data). Worker lesson candidates: Qdrant skip-gate does not classify timeouts as skip; tests generate data/retrieval-stats.sqlite3 in workspace.

- 2026-07-03 Task_1 closed as done; branch rebased onto main (c8dc279) after PR #54 merged the guardrail stabilization (timeout API fix, outcome assertions, env hygiene, canary test; Linux CI live-service tests green). The environmental blocker on Task_1's full-suite evidence is resolved via CI arbitration policy from that plan (local idle-stall documented in lessons.md). Post-rebase validation on this branch: fmt/clippy PASS, cargo test --lib 307/307 PASS. Wave 2 dispatching sequentially (Task_2 then Task_3) per shared-resource lesson from the stabilization plan (parallel Workers contend on cargo target dir).

- 2026-07-03 Wave 2 completed: [Task_2, Task_2b (added), Task_3]
  - Summary: Deterministic prepare helpers (write_plan/helpers.rs) with same-input-same-plan property tests and no-inference guarantees; Task_2b replaced the initial hand-rolled ID mixer with standard UUIDv5 (namespace 5f18dc72-f839-58f8-8ff3-c841298cc789 = UUIDv5(DNS, "character-memory.write-plan"), length-prefixed framing; Cargo.toml gained uuid "v5" feature) because persistent stable IDs require proven collision behavior (ADR-I-0001); write-plan validator (internal/repositories/write_planning.rs) implementing all ten phase-doc rule groups with 17 rejecting/side-effect-free unit tests using FakeGraphAuthorityStore, link-admission parity via admit_link (A3).
  - Validation evidence: each task ran cargo fmt --check, cargo check, cargo clippy --all-targets -- -D warnings, cargo test --lib — final state 330 passed / 0 failed / 3 ignored.
  - Notes: Task_3 carries a scoped pre-wiring dead-code allowance for the validator surface; Task_4 must remove it when wiring the commit path. Task_3 interpreted "explicitly historical" as retention_state Archived for superseded candidates — Task_7 review should confirm against the phase doc.

- 2026-07-03 Waves 3–5 completed: [Task_4, Task_5, Task_6, Task_4b (added), Task_7]
  - Summary: Task_4 wired prepare/validate_plan/commit through the facade (graph-critical/vector-stats-repairable split, A1 idempotency, remember() parity, dead-code allowance removed; one justified cross-owns barrel re-export in repositories.rs). Task_5 added tests/v0_1_3_write_planning_tests.rs with per-criterion traceability (16→19 tests). Task_6 aligned README, roadmap status cell, and phase-doc implementation notes (Q1–Q3/A1–A2 + Archived interpretation). Task_7 Reviewer (Fable 5) returned CHANGES_REQUESTED (F1 plan-path vector fallback leak [major], F2 stats-candidate semantics [documented per ADR-I-0008 decision], F3 traceability mismatch [major], F4–F7 minors); Task_4b fixed all findings (VectorWriteIntent enum for exact plan-target fidelity; genuine remember() wrapper test; docs alignment; hygiene). Re-review verdict: APPROVED — gate satisfied modulo CI-arbitrated full-suite item.
  - Validation evidence: Worker + independent Reviewer runs — cargo fmt --check, cargo clippy --all-targets -- -D warnings PASS; cargo test --lib 333/333 PASS; cargo test --test v0_1_3_write_planning_tests 19/19 PASS with live Qdrant (Reviewer confirmed targeted full-body execution of the three new tests). Full-suite green delegated to Linux CI per recorded policy (local idle-stall constraint, lessons.md).
  - Notes: Reviewer minors accepted as non-blocking (outcome-field vs direct-store assertions; legacy test name). Reviewer lesson candidate on skip-masking recorded for troubleshooting staging.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-07-02: Plan drafted from dual Researcher reports (docs + codebase). Defaults chosen for Q1–Q3 pending user confirmation.
- 2026-07-02: Applied consolidated findings from two independent plan reviews: (1) resolved Wave 2 owns collision by moving deterministic helpers into the public write_plan module (Task_2) so Task_2/Task_3 owns are disjoint files; (2) added Clippy warning-deny gate to all validation commands (CI parity; see lessons.md); (3) corrected Task_5 service requirements — persistent graph mode uses the embedded Oxigraph store, only Qdrant compose is required; (4) specified the idempotency contract in A1 and threaded it through Task_3/Task_4/Task_5; (5) added draft.rs to Task_1 owns for the RememberOutcome extension; (6) recorded the remember() API-shape decision in A2; (7) enumerated verbatim lifecycle currentness rules in Task_3; (8) added ADR-I-0012 approval-flow test to Task_5.
- 2026-07-02: User approved the plan with Q1–Q3 defaults confirmed (extend RememberOutcome in place; strict-reject missing link targets; producer/rationale-origin metadata plan/diagnostics-only). User directive: Reviewer dispatches use the Claude Fable 5 model.

## Notes
- Risks:
  - `RememberOutcome` reuse/extension is the earliest cross-task coupling point — settle in Task_1.
  - Cross-store idempotency has no existing substrate; deterministic-ID approach must be proven by the retry integration test.
  - Commit revalidation has no cross-store transaction; ordering (Oxigraph → Qdrant → stats) must guarantee no ungrounded behavior-influencing memory.
  - Exposing vector/stats candidates in a public plan must not leak Qdrant internals.
- Edge cases:
  - Partially successful commit (graph ok, vectors/stats failed) + retry with same idempotency key.
  - Plans referencing objects created earlier in the same plan.
  - Opaque scope IDs carried without modeling scopes (v0.2 boundary).
