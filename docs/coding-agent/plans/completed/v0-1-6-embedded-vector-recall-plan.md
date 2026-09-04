# Plan: v0.1.6 Embedded Vector Candidate Recall

- status: done
- generated: 2026-09-02
- last_updated: 2026-09-04
- work_type: mixed

## Goal
- Deliver the phase described in `docs/design/roadmap-phases/v0_1_6_embedded_vector_candidate_recall.md` under ADR-I-0023 through ADR-I-0028: a redesigned vector port contract, a five-field vector record, an embedded vector candidate store on the in-process build of the service backend (Qdrant Edge) as the default vector mode with the service adapter retained as the explicit service mode, and a shared contract suite over both adapters; the evaluation repository's move of its vector-only baseline onto the retrieval trace is planned there and consumed here as closeout evidence.

## Definition of Done
- Every acceptance criterion in the phase document's "Acceptance criteria" section holds with recorded evidence.
- Every row of the phase document's deferral-reconfirmation checklist has its evidence produced and cited in the Progress Log.
- Every deletion listed under "Deletions that are deliverables" is gone, with a zero-hit census.
- Both repositories' service-gated suites execute (not skip) under the service-backed CI job.
- One or more PRs per wave in this repository, stacked on the planning PR and merged by the decider; the evaluation repository's cross-mode comparison is consumed as evidence when that repository produces it; by decider ruling (ADR-I-0023, 2026-09-03) it is a revisit trigger for the embedded default, not a closeout gate, so its pending state does not hold this plan open.

## Scope / Non-goals
- Scope: the phase document's deliverables and deletions, all in this repository.
- Non-goals: the phase document's non-goals (no index tuning beyond the exact-scan threshold, no migration tooling, no multi-process embedded access, no public candidate-search facade, no retrieval semantics change in service mode for non-empty scopes and non-degenerate queries; ADR-I-0024's empty-scope change, zero candidates for an empty scope and boundary rejection of an empty configured scope, is an intended change and in scope).

## Context (workspace)
- Design memo and audits: `.agent-work/orchestrator/` (v016-port-design-consult.md sections A-G; cm-design-audit.md; cme-design-audit.md; v016-consolidated-triage.md) and the researcher censuses under `.agent-work/researcher/` and the evaluation repository's `.agent-work/evals-researcher/`; all transient, consumed into this plan and the ADRs.
- As-built port: `src/ports/vector_candidate.rs`, `src/models/vector/candidate_record.rs`, `src/models/vector/record.rs`, `src/adapters/qdrant/{store,payload}.rs`, `src/policy/embedding_surface.rs`, `src/usecases/retrieve.rs`, `src/api/types/retrieval.rs`, `src/composition.rs`, `src/config/app_settings.rs`, `src/test_support.rs`.
- Prerequisite in this repository, landed: the toolchain pin moved to the embedded engine's minimum (Rust 1.97.0) in its own change, merged 2026-09-02 as a88c117.
- Prerequisite tracked in the evaluation repository: its evidence-integrity fixes must be merged before this phase cites any harness measurement.
- Repo reference docs consulted: the six ADRs (ADR-I-0023 through ADR-I-0028); ADR-I-0018 (dependency direction; ports may import the public retrieval vocabulary under its named exception); ADR-I-0007 (schema versioning); ADR-I-0021 (embedded default pattern); rules in `docs/coding-agent/rules/`.

## Open Questions (max 3)
- none (the draft's five open questions were ruled by the decider on 2026-09-02 and are recorded in the phase document and the ADRs).

## Assumptions
- A1: The embedded engine is `qdrant-edge` pinned exactly at 0.8.0 (beta); its API is guarded by a contract canary, and the pin is bumped only with a re-run of the canary and the parity suite.
- A2: The evaluation repository plans and tracks its own work; this plan consumes two of its outputs only: the trace-sourced baseline's A/B evidence (deferral-reconfirmation row 5) and the cross-mode comparison consumed as closeout evidence and as a revisit trigger for the embedded default.

## Tasks

### Task_1: Live-gate hardening in the library test suite
- type: test
- owns:
  - tests/support/base.rs
  - tests/write_planning_tests.rs
  - tests/initialization_tests.rs
  - tests/public_facade_tests.rs
  - tests/retrieval_guardrails_tests.rs
  - .github/workflows/*.yaml
- depends_on: []
- description: |
  Add one environment switch honored by the shared test support that turns every service-unavailable skip into a panic, set it in the CI job that provisions the vector service, and delete the prose-matched timeout skip (`is_qdrant_timeout_signature`) or replace it with a typed match on the existing transport classification.
- acceptance:
  - No test passes by skipping when the switch is set; the CI service-backed job sets it.
  - No test gates on error prose.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "service-up cargo test with the switch set: every former skip site executes (census the skip sites before and after; the count is not fixed); service-down with the switch set: the suites fail, not pass"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review; confirm the CI job sets the switch"

### Task_2: Port contract: completeness envelope and scope-only query (ADR-I-0024)
- type: impl
- owns:
  - src/ports/vector_candidate.rs
  - src/models/vector/candidate_record.rs
  - src/api/types/retrieval.rs
  - src/adapters/qdrant.rs
  - src/adapters/qdrant/store.rs
  - src/adapters/qdrant/tie_closure.rs
  - src/usecases/retrieve.rs
  - src/usecases/remember.rs
  - src/usecases/correct_forget.rs
  - src/memory.rs
  - src/test_support.rs
  - src/adapters/oxigraph/tests.rs
- depends_on: []
- description: |
  Introduce the result envelope (canonical candidates plus the typed completeness verdict) and the verdict enum in the public retrieval telemetry vocabulary; extract the service adapter's private tie-closure loop (fetch decision, fetch bound, cohort closure, canonical construction) into `src/adapters/qdrant/tie_closure.rs` as crate-visible shared logic that takes an engine-neutral fetch callback, so the embedded adapter (Task_4) calls it rather than re-implementing it; make the service adapter map the shared fetch decision onto the verdict; make the query scope-only with empty-scope-selects-zero and boundary rejection of an empty configured object-type set; record the verdict in retrieval telemetry beside the returned count; update every fake store. No repair, retry, or failure on the verdict.
- acceptance:
  - The envelope and verdict express the four situations in ADR-I-0024's Decision section (its appendix shape is a non-binding reference); the canonical-candidates newtype is unchanged.
  - Query-side zero-norm rule (ADR-I-0024) implemented in the service adapter: a zero-norm query scores every candidate zero and returns a truthful verdict, with a unit test and a parity fixture that Task_4 inherits.
  - Telemetry carries the verdict for every retrieval; a retrieval test asserts each variant.
  - The tie-closure loop lives in `src/adapters/qdrant/tie_closure.rs`, the service adapter calls it, and its existing unit tests (fetch decision, all-tied cohort at the bound) move with it; nothing in `store.rs` closes a cohort on its own.
  - Fetch-decision unit tests assert closed and open verdicts including the all-tied cohort at the bound.
  - Zero-hit census: no match-or-unknown condition, no filter type beyond object-type scope.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; service-up cargo test; ignored qdrant_ lib tests"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review vs ADR-I-0024; confirm no pipeline path inspects the verdict for control flow"

### Task_3: Vector record read contract (ADR-I-0025)
- type: impl
- owns:
  - src/models/vector/record.rs
  - src/adapters/qdrant/payload.rs
  - src/adapters/qdrant/store.rs
  - src/policy/embedding_surface.rs
  - src/domain.rs
  - src/usecases/retrieve.rs
  - src/usecases/vector_indexing.rs
  - src/errors.rs
  - src/models/vector.rs
  - src/models/vector/candidate_record.rs
  - src/api/types/retrieval.rs
  - src/api/**
  - src/lib.rs
  - docs/design/database/vector_payload_design.md
  - docs/design/database/schema_cheat_sheet.md
  - docs/design/database/README.md
  - docs/design/database/graph_schema_design.md
- depends_on: [Task_2]
- description: |
  Shrink the record and the typed manifest to the five fields; drop the hint carriers, the readable text column, the per-field index creation for dropped fields, the test-only field constants and the prose-assertion note constant; replace the service adapter's private enum token mappers and the pipeline's copy with one Display/FromStr per enum in the domain. Also own ADR-I-0024's zero-norm rule on the write side: the vector indexing service rejects a zero-norm record embedding as a typed per-record indexing failure (adding the error-vocabulary variant it needs) before any adapter sees it, with a unit test on the service and a parity-suite fixture that Task_4 inherits, so no adapter ever normalises a zero vector. Consolidating the surface enum to one domain definition touches its internal definition and re-export and the public copy, all owned here. Publish the maximum number of embedding surfaces per object kind as public policy next to the surface policy that defines it (ADR-I-0026), with a test that the published value matches the builders. Update the database documentation that advertises the old record: the schema cheat sheet, the database README, and the graph schema design note's cross-store section lose the hint fields, the graph URI, the lifecycle-hint drift diagnostics, and the readable text column, and point at the five-field contract. Schema version ruling: the stored schema version is retained, because every field the new contract reads is present in records written under the current version and the removal only drops fields no reader consumes; existing stored payloads with extra fields are tolerated unread. A version bump is required only if a later change adds a read field that older records lack (the re-entry paths in ADR-I-0024), and that change owns the bump and its backfill.
- acceptance:
  - The manifest test asserts exactly five entries; both text-column producers except `embedding_text` are gone.
  - Zero-hit census across both repositories for the dropped fields and for `content_text` readers (the evaluation repository removes its reader under its own plan; its zero-hit census is consumed as closeout evidence, not ordered here).
  - One token mapping per enum; census shows no copy in adapters or use cases.
  - A zero-norm record embedding yields a typed per-record indexing failure and never reaches an adapter (ADR-I-0024); unit test present.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; service-up cargo test; ignored qdrant_ lib tests; census commands recorded"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review vs ADR-I-0025; verify the payload design note's supersession note matches what landed"

### Task_4: Embedded Qdrant Edge vector candidate store, settings, and parity suite (ADR-I-0023)
- type: impl
- owns:
  - src/adapters/qdrant_edge/**
  - src/adapters.rs
  - src/composition.rs
  - src/config/app_settings.rs
  - src/errors.rs
  - Cargo.toml
  - Cargo.lock
  - tests/vector_port_contract_tests.rs
  - tests/support/**
  - .env.example
  - README.md
  - docs/design/roadmap-phases/v0_1_6_embedded_vector_candidate_recall.md
- depends_on: [Task_3]
- description: |
  Implement the embedded adapter on `qdrant-edge` pinned at 0.8.0 per the phase document: one engine shard directory per collection under `VECTOR_STORE_PATH`, cosine distance at the configured vector size, the indexing threshold shipped at its exact-scan setting (zero) with no optimise call, the five-field payload with keyword indexes on object id (the delete selector) and object type (the scope predicate), the object-type scope as a filter, and search through the shared tie-closure loop Task_2 extracts into `src/adapters/qdrant/tie_closure.rs` and the canonical constructor, with the verdict mapping (Exhaustive when the shard is unindexed per the adapter's own threshold configuration and the loop closed the cutoff cohort, with `scanned` from a filtered count of the scope, never from returned rows; BoundaryTieClosed when an indexed shard's cutoff cohort closed, describing the index's returned prefix and never global recall; BoundaryTieOpen whenever the bound is reached with the cohort open, on an exact scan too; NotRequested at limit zero or empty scope). The adapter constructs only object payloads and uses only the general shard type; every engine call (upsert, delete, search, index build, and shutdown) runs on a dedicated blocking owner the adapter creates at construction and that holds the shard, never on an async executor thread; the adapter's drop signals the owner so the shard's final drop happens on the owner's thread, and no port or facade method is added. Add a contract canary test in the pattern of the service client's erased-connect canary pinning the engine facts the adapter relies on (zero threshold means unindexed; object-payload precondition; shard-directory precondition; crate-local type provenance). Implement the `VECTOR_STORE_MODE` and `VECTOR_STORE_PATH` settings with mode-specific validation (service connection string required only in service mode), the composition mode switch with `collection_name` as the backend-neutral namespace key, the adapter-owned marker recording collection name and record schema version, and the port-conformance parity suite run against both adapters (embedded unconditionally, service under the live gate). Extend the vector error vocabulary only where the embedded adapter needs a kind the service adapter lacks. Produce the dependency-weight report (unstripped and stripped release deltas, effect of feature trimming) and the latency benchmark (exhaustive scan at the configured dimension across corpus sizes; executor responsiveness under a concurrent scan).
- acceptance:
  - Embedded mode constructs and retrieves with no service running; the parity suite yields identical admitted sets and orderings on the shared fixtures while both adapters are below their indexing thresholds; the tie fixture yields Exhaustive (embedded) and BoundaryTieClosed (service), both through the shared tie-closure loop, with no engine ordering relied on.
  - A recall comparison of the embedded adapter above its indexing threshold against its exhaustive setting is recorded on the benchmark corpus (informational this phase; index tuning is a later decision).
  - The collection name is validated to the phase document's allowlist before any directory is touched, and a path-confinement test proves separator and parent-directory inputs cannot escape the configured directory.
  - ADR-I-0027 holds: every engine call (shard open and load with its lock backoff, payload index creation, upsert, delete, search, the filtered scope count, index build, shutdown) runs on the adapter's dedicated blocking owner, which opens the shard itself; a write is acknowledged only after the engine's flush; the facade drop only signals the owner; a constructor meeting a locked directory waits with a bounded backoff; the close-then-reopen test, the hard-exit test (exit without dropping the shard, reopen from a second process, find every acknowledged write), and the responsiveness benchmark (construction and reopen with a lock-backoff wait, a concurrent scan, a write burst, a build, a close) pass; the contract canary pins the three engine facts ADR-I-0027 rests on: no persistence before flush (a probe that writes, skips flush and drop, and reopens empty), no log replay on load, and the directory lock.
  - Restart test passes; repeated runs are byte-identical; reopening a shard with a mismatched vector size or distance raises the collection-compatibility error, and reopening one whose marker carries an unsupported record schema version raises the clear failure ADR-I-0007 requires, each covered by its own test.
  - Embedded mode with no `VECTOR_STORE_PATH` is a configuration error at construction, never an implicit default; covered by a settings test.
  - The contract canary passes on the pinned engine version and is documented as the gate for every engine bump.
  - The dependency-weight report records unstripped and stripped release deltas and the result of feature trimming; the latency guidance is measured and documented with the single-process expectation and the rebuild-from-graph-authority path.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test with no service (parity suite + embedded tests + canary execute); service-up cargo test with the live switch set; benchmark and weight numbers recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review vs ADR-I-0023; independent service-free and service-up runs; tie-closure reuse verified by reading, not by test names"

### Task_7: Fake retirement, closeout docs, and reconfirmation evidence
- type: chore
- owns:
  - src/test_support.rs
  - src/memory.rs (inline fake-store tests only)
  - src/models/vector.rs (fake-only re-export)
  - src/models/vector/candidate_record.rs (fake type retirement and inline tests)
  - src/models/vector/record.rs (fake-only conversions and inline tests)
  - src/policy.rs (fake-only test re-export)
  - src/usecases/correct_forget.rs (inline fake-store tests only)
  - src/usecases/retrieve.rs (inline fake-store tests only)
  - src/api/types/retrieval.rs (doc comments only)
  - src/ports/vector_candidate.rs (doc comments only)
  - src/composition.rs, src/config/app_settings.rs (public constructor doc comments only; deferred from the Task_4 Copilot review)
  - src/adapters/qdrant/store.rs, src/adapters/qdrant/payload.rs, src/adapters/qdrant_edge/mod.rs (Task_8 audit dispositions only)
  - tests/vector_port_contract_tests.rs, tests/support/** (point-id parity fixture and fake-retirement fallout)
  - README.md (stale filtering line)
  - docs/design/roadmap-phases/v0_1_6_embedded_vector_candidate_recall.md (closeout status)
  - docs/coding-agent/plans/**
  - docs/coding-agent/lessons.md
  - docs/roadmap/development_roadmap.md
  - Cargo.toml
  - Cargo.lock
- depends_on: [Task_4]
- description: |
  Retire the deterministic vector fake and its embedding-bearing record type in favour of the embedded adapter opened on a temporary shard directory, as the phase document specifies, so tests exercise the persistence path (failure-injecting and recording fakes stay); collect the deferral-reconfirmation evidence for all five checklist rows; bump the package version to 0.1.6 in the manifest and lockfile as prior milestone closeouts did; mark the roadmap row finished; move the plan to completed.
- acceptance:
  - Zero-hit census for the retired fake and record type.
  - All five checklist rows cite evidence in the Progress Log.
  - Task_8 audit dispositions landed: one point-identity derivation (the v5 derivation) shared by both adapters with a parity assertion and no compat shim; one shared read function for the record contract returning the closed error vocabulary, with the service adapter's stringly database errors for payload and scroll-limit faults removed; the canary pins that an indexed shard searched exactly returns the exhaustive result; the service zero-norm verdict's `scanned` is the size of the final scroll response whose records were actually scored; the duplicate vector-config validation is reduced to one or justified in place; the stale header comments and README line are corrected; the public facade constructor and `Settings::new` rustdoc describe both vector-store modes in backend-neutral terms (embedded default with a store path; explicit service mode with a connection string).
  - The public completeness type documents all four verdict meanings and the `scanned` and `fetched` counters, stating that `scanned` is the number of scoped records actually scored and `fetched` is the final prefix size rather than a cumulative count, and that a closed boundary verdict is deterministic only for the index-returned prefix, never population-level completeness; the port trait's doc comment states the verdict guarantees (tie closure through the shared loop; exhaustive only when every scoped record was scored through a path the adapter knows to be exhaustive, including an unindexed scan or a full-scope scroll, with a closed cohort; open at the bound) so a future adapter cannot label an index-produced prefix exhaustive (deferred from the Task_2 review to avoid re-cascading the stack mid-wave).
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "full cargo test with no service and service-up with the live switch set"
  - kind: review
    required: true
    owner: reviewer
    detail: "Closeout review; Definition of Done census"

### Task_8: Design-value audit at the pre-merge milestone gate
- type: review
- owns:
  - docs/coding-agent/plans/active/v0-1-6-embedded-vector-recall-plan.md
- depends_on: [Task_4]
- description: |
  Altitude review (Claude) against philosophy and roadmap: nothing designed twice across the two adapters, no hint field re-entered without its predicate, the evaluation repository holds no store-private knowledge, the ADR boundaries respected.
- acceptance:
  - Audit verdict recorded in the Decision Log with any EARNS-ITS-PLACE / OVERSIZED / DELETE findings dispositioned.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Audit report consumed; dispositions recorded"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_2]
- Wave 2 (parallel): [Task_3]
- Wave 3 (parallel): [Task_4]
- Wave 4 (parallel): [Task_8]
- Wave 5 (parallel): [Task_7]

Task identifiers 5 and 6 were evaluation-repository work and moved to that repository's own plan; identifiers are not reused.

Each wave ends with reviewer approval and a PR stacked on the previous wave's PR (GitHub stack on the planning PR); the next wave branches from the approved tip without waiting for a merge, and the decider merges the entire stack in one go at phase end (Decision Log, 2026-09-03 and 2026-09-04).
Notification duty: any wave that changes a public vocabulary the evaluation repository converts exhaustively (the vector database error kinds in Wave 3, the telemetry field in Wave 1) is announced to that repository before merge; how and when that repository adopts the change is planned there, and this plan only consumes the resulting compatibility evidence at closeout.

## Rollback / Safety
- Embedded mode is the default and the default-mode construction test asserts it; the service mode's behavior is unchanged except for the reported verdict, the shrunken record, and the intended empty-scope change (an empty object-type scope selects zero instead of searching unfiltered, and an empty configured scope is rejected at the boundary), all covered by the parity suite and the retrieval tests.
- Stored service-mode payloads with dropped fields remain readable (extra fields tolerated unread); rebuild from graph authority is the recovery path.
- Each wave is a separately revertible PR.

## Progress Log (append-only)

Append-only editing rule (applies to both logs below): when appending an entry, anchor the edit on the previous entry and reproduce it (or anchor on the section's tail marker) so the edit inserts rather than replaces, and verify afterward that the log grew.

- 2026-09-02 Planning wave completed: five parallel inputs (design consult, two altitude audits, two forensic censuses) consolidated; decider ruled the five design questions; ADR-I-0023 through ADR-I-0026, the rewritten phase document, and the roadmap section authored on branch `plan/v0-1-6-embedded-vector-recall`. Plan awaits approval.

- 2026-09-04 Wave 1 done: Task_1 (a633b2e, PR #74) and Task_2 (5b30856, PR #75) approved by the Tier D reviewer; Task_2 needed one revision (checked backend-limit conversion with a BoundaryTieOpen regression at the u32 cap; public re-exports) and carries one post-review fix (dimension check before the zero-norm scroll). PRs stacked on the planning PR as stack #76; merges pending. Wave 2 (Task_3) dispatched on a branch stacked on Task_2.
- 2026-09-04 Wave 2 done: Task_3 (36ef8c8, PR #77) approved by the Tier D reviewer after two revisions on the zero-norm rejected-record fixture (final shape: one definition behind the non-default `test-fixtures` feature, enabled for integration tests by a self dev-dependency, doc-hidden). The evaluation repository's exhaustive error-vocabulary conversion is recorded as its re-pin obligation (ADR-I-0023), not a finding. PR appended to stack #76.
- 2026-09-04 — Merge shape refined: the stack (planning PR at the bottom, one PR per wave above it) is merged in one go at phase end; GitHub's bottom-up stack order blocks interim merges of wave PRs while the planning PR is a draft, which is the intended shape. Waves proceed by branching from the previous wave's approved tip.
- 2026-09-04 Wave 3 done: Task_4 (01de5d0, PR #78) approved by the Tier D reviewer with no findings after one revision (both-adapter zero-norm parity through the shared fixture; a populated-shard canary leg). During the review the stack's base was found to predate main's Rust 1.97 bump; the planning branch was rebased onto main and the stack cascaded (content-identical by range-diff), so the toolchain pin is 1.97 everywhere. Measured: exhaustive scan 3/11/48 ms at 100/1000/5000 records of 1536 dimensions; release footprint delta about 24.3 MB; no crate features to trim. Stack #76 is #72, #74, #75, #77, #78.
- 2026-09-04 — Task_8 altitude audit (Claude, stack tip 01de5d0): APPROVED; ADR-I-0023/0024/0025/0027 boundaries verified holding by reading; zero hint-field re-entry; zero store-private knowledge exported.
  - Findings: MEDIUM point identity derived twice (service in-house hash vs embedded v5 derivation; a shard-to-server sync would double points); MEDIUM record read side implemented twice with divergent error classification (service stringly `DatabaseError` escapes the closed vocabulary); LOW filter convention structurally different but equivalent; LOW service zero-norm `scanned` taken from rows returned; LOW embedded exactness rests on `exact: true`, not on the persisted optimizer config; LOW stale header comments and README line.
  - Design-value: every phase addition EARNS-ITS-PLACE except the duplicate vector-config validation (OVERSIZED, minor), the `test-fixtures` feature (OVERSIZED advisory; shape ruled in Wave 2, kept), the in-house point-id hash (DELETE advisory), and the scoring fake (DELETE, already Task_7).
  - Disposition: all findings folded into Task_7 (owns and acceptance extended above); the point-identity unification is a behaviour change for existing service collections, acceptable under the Compatibility Policy (rebuild from graph authority), pending the decider's veto.
- 2026-09-04 Task_7 implementation closeout: the deterministic scoring fake, its embedding-bearing `VectorCandidateRecord`, conversion hooks, and fake-only tests were deleted; all former success-path users now open the embedded adapter in a temporary shard fixture, while recording and failure-injection fakes remain. `rg -n "FakeVectorCandidateStore|VectorCandidateRecord|cosine_similarity" src tests` returned zero hits. Both adapters now use the single v5 point-ID derivation and the single typed payload reader in `qdrant/payload.rs`; the service zero-norm path counts its exact filtered scope; the embedded canary compares an indexed `exact: true` search with the exhaustive result; and the retained pre-open vector-config validation is justified because it avoids retrying an incompatible shard.
- 2026-09-04 Deferral-reconfirmation checklist evidence:
  1. Canonical-candidates newtype survival: `rg -n "CanonicalCandidates" src tests` shows the newtype remains the `VectorCandidateRecall.candidates` envelope field and is constructed only at adapter/tie-closure and deliberate test-double boundaries; `canonical_candidates_dedupe_identity_at_highest_score_and_totally_order_ties` remains in the service-free suite.
  2. Dual text columns: `rg -n "content_text|embedding_text|CONTENT_TEXT|EMBEDDING_TEXT" src tests` shows no `content_text` column in this repository and only the required five-field `embedding_text` record/provenance path. The companion repository still has its direct Qdrant `content_text` reader, so its before/after vector-only identity-and-text A/B evidence is explicitly pending in that repository.
  3. Search completeness: `backend_fetch_cap_reports_an_open_boundary_without_allocating_rows`, `retrieval_telemetry_preserves_every_vector_recall_completeness_verdict`, the live service boundary tests, and `service_and_embedded_admit_identical_candidates_in_identical_order` cover open, telemetry, closed, and cross-adapter exhaustive-versus-closed behavior through the shared tie loop.
  4. Hint filter semantics: `rg -n "VectorCandidateFilter|CandidateFilter|match_or_unknown|matches_or_unknown|MatchUnknown" src tests` returned zero hits; the query carries only object-type scope, with its empty-scope behavior pinned by ADR-I-0024, its predicate rules pinned by ADR-I-0028, and its behavior covered by the port contract tests.
  5. Evaluation baseline capability: this repository exposes singleton-scoped traced candidates, completeness telemetry, and `max_embedding_surfaces`; `rg -n "SearchPointsBuilder|QDRANT_OBJECT_ID_FIELD|QDRANT_OBJECT_TYPE_FIELD|QDRANT_CONTENT_TEXT_FIELD" crates/cmem-eval-adapter-cmem/src/lib.rs` still finds the companion repository's direct Qdrant baseline, so its trace migration, row-level identity/rank A/B diff, and post-switch zero-hit census are explicitly pending in that repository.
- 2026-09-04 Task_7 validation closeout: `cargo fmt --all -- --check` passed; `cargo clippy --all-targets --all-features -- -D warnings` passed on Rust 1.97; bare `cargo test` passed 396 library tests, 31 integration tests, and one doc test with five intentional ignores after the point-identity and temporary-directory-cleanup fixtures were added; the live-switch `cargo test` passed with both service/embedded parity tests executing; and the four ignored service-Qdrant tests passed explicitly under the live endpoint. The package is 0.1.6, the phase and roadmap are finished, and this plan is moved to completed. Independent reviewer approval and stack merge remain Orchestrator gates.
- 2026-09-04 Task_7 review revision: the service scroll-width overflow now returns the typed `Conversion` kind; `rg -n "SURFACE_FIELD" src tests` returns zero hits; completeness docs and ADR-I-0024 define exhaustive recall by proving every scoped record was scored through a known-exhaustive path; the test close acknowledgment follows the `EdgeShard` destructor; and every embedded vector-port integration fixture performs bounded root cleanup after dropping its facade. The service-free suite, strict clippy, strict rustdoc generation, and formatting gate all pass; the final full-suite Windows `.tmp*` census was 148 before and 148 after (zero delta). No live rerun was needed because the delta changes local conversion classification, documentation, and embedded test lifecycles only.
- 2026-09-04 Task_7 service-up cleanup revision: the indexed exact-recall canary awaits both direct-store closes before removing and asserting its temporary root, and the remaining direct point-identity and zero-norm adapter stores also close explicitly. The focused canary, bare full suite, service-up full suite with `REQUIRE_QDRANT_TESTS=1`, strict clippy, strict rustdoc generation, and formatting gate pass; the final Windows `.tmp*` censuses were 149 before and 149 after for both full-suite modes (zero delta).
- 2026-09-04 Task_7 telemetry and decision-record follow-up: the service zero-norm path derives `scanned` from the final scroll response whose scoped records it scored, with no concurrent count request; `fetched` is documented as the final prefix size rather than a cumulative total. ADR-I-0024 now governs completeness only, while accepted ADR-I-0028 governs prefilters and requires fully populated or backfilled current columns before activation. `Settings::new` documents that facade construction consumes and validates the retained vector location, and the Orchestrator rule metadata carries the review date. The ADR-I-0023 through ADR-I-0028 temporal-word census and stale-reference census returned zero hits. Formatting, strict all-target/all-feature clippy, and strict all-feature rustdoc passed. The final bare and service-up full suites each passed 396 library tests, 31 integration tests, and one doc test with five intentional ignores; the changed ignored live zero-norm regression also passed explicitly. The recursive `.tmp*` census stayed 4 before and 4 after the final bare run and 0 before and 0 after the service-up run. Two earlier bare runs exposed unrelated load-sensitive SQLite and Edge lock timing failures; each exact rerun passed, followed by the clean full-suite run recorded here.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-02 Decision: the draft's port description was rewritten as an intentional new port contract.
  - Trigger / new insight: the draft (2026-07-20) described filter, diagnostics, and reconciliation capabilities that the structured-verdict phase deleted; thirty of thirty-three payload fields were write-only; the readable text column's only reader was the evaluation repository's direct store access.
  - Plan delta: contract-first waves (envelope and query, then record, then adapter); evaluation baseline moved to the trace; deletions promoted to deliverables.
  - Tradeoffs considered: recorded in the ADRs' rejected alternatives; the forward-looking keep case for hint fields (immutable time window) is recorded as a re-entry path rather than kept.
  - User approval: rulings on all five questions given 2026-09-02; plan approval pending.
- 2026-09-02 Decision: evidence-integrity defects in the evaluation repository are fixed before this phase, outside this plan.
  - Trigger / new insight: batch ingest produced phantom repair attempts and the evaluated rank was harness-invented; both would corrupt the parity and baseline evidence this phase cites.
  - Plan delta: none inside this plan; recorded as a prerequisite in Context.
  - User approval: yes, 2026-09-02.
- 2026-09-02 Decision: the embedded vector store is the in-process edition of the service engine (Qdrant Edge), not an in-house exact scan and not a SQLite vector extension.
  - Trigger / new insight: the decider restated the objective as a portable memory that plugs into any foundation model and tracks years to decades of continuous character development; the draft's in-house exact scan violated the library-over-in-house principle; two feasibility spikes on the same probe set measured the two library-backed candidates (numbers in ADR-I-0023).
  - Plan delta: Task_4 rewritten for the in-process engine (shard directory per collection, threshold-based exactness, shared tie-closure loop, contract canary, dependency-weight report); the toolchain pin moved to 1.97.0 as a prerequisite; approximate indexing leaves the non-goals; named-vector coexistence and shard-to-server sync are recorded as available but not exercised this phase.
  - Tradeoffs considered: recorded in ADR-I-0023's rejected alternatives (in-house scan; sqlite-vec, lighter and deterministic but exhaustive-only in its stable release with a single-maintainer approximate-index future; LanceDB; in-memory only; default flip now); a scale probe was offered and declined as not worth the effort, so the interactive-latency assumption at decade scale is a recorded revisit trigger rather than a measurement.
  - User approval: yes, 2026-09-02.
- 2026-09-02 — Durability rule for the embedded adapter (review round 22).
  - Finding: the engine persists a write only on flush and does not replay its log on load; process-level probes that skipped the shard drop reopened with zero of two hundred points.
  - Ruling: the blocking owner flushes after every write and acknowledges only then; the signal-only facade drop stays; no port or facade method is added; a hard-exit test joins Task_4's acceptance.
  - Tradeoff: one synchronous disk sync per write, measured by the write burst in the benchmark; batching behind an acknowledgement is the named upgrade.
- 2026-09-03 — Decision records restructured on the decider's review.
  - ADR-I-0023 now decides the default: embedded is the default vector mode from this phase, licensed by the phase's own parity suite and service-free integration path (defaults-match-evidence, ADR-I-0021); ADR-I-0003's vector default is partially superseded; the evaluation repository's cross-mode run becomes a revisit trigger, not a gate. Task_4's default-mode test asserts embedded and its README and phase-document deliverables lead with the local path.
  - The blocking-owner and per-write-flush rules moved out of ADR-I-0023 into ADR-I-0027 as their own decision.
  - ADR-I-0024 narrowed to the completeness verdict and the prefilter rule (unknown never matches); the scope-only query is a current state, the type shape is a non-binding appendix, and the two predicate paths are notes binding on no phase.
- 2026-09-03 — Merge shape: the records and plan are not merged ahead of the implementation, because ADR-I-0023's embedded default rests on evidence the implementation produces. Implementation waves land as PRs against the planning branch (`plan/v0-1-6-embedded-vector-recall`), which stays the readable home of the records; the phase merges to main as one change once the decision is solidified by its evidence.
- 2026-09-04 — ADR-I-0024 split by governing claim.
  - Trigger / new insight: completeness evidence and prefilter admission have different warrants, failure modes, and revisit triggers; keeping both in one record obscured which rule governed a change.
  - Plan delta: ADR-I-0024 now decides completeness verdicts and counters only; accepted ADR-I-0028 decides that prefilters never match unknown and may activate only after their current columns are fully populated or backfilled. Phase, payload, roadmap, and ADR cross-references follow that boundary.
  - Tradeoff: one additional decision record and explicit dependency links in exchange for independent amendment and auditability.
  - User approval: yes, 2026-09-04.

## Notes
- Risks: the row/summary schema move in the evaluation repository (typed backend identity) is a clean break under its compatibility policy and must not touch sealed evidence; the latency guidance and the stripped dependency weight must be measured, not assumed; the engine is beta, so its pin is exact and its bump is gated by the canary.
- Edge cases: an empty object-type scope or `limit == 0` issues no search and reports the not-requested verdict in both adapters; identical-vector tie fixtures must produce Exhaustive versus BoundaryTieClosed, never be encoded as expected parity of the bounded behavior; the parity suite includes non-unit query and record vectors so score equality across adapters (both engines normalise cosine internally) is asserted, not assumed.
