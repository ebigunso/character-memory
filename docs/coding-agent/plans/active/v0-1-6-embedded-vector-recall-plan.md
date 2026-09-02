# Plan: v0.1.6 Embedded Vector Candidate Recall

- status: draft
- generated: 2026-09-02
- last_updated: 2026-09-02
- work_type: mixed

## Goal
- Deliver the phase described in `docs/design/roadmap-phases/v0_1_6_embedded_vector_candidate_recall.md` under ADR-I-0023 through ADR-I-0026: a redesigned vector port contract, a five-field vector record, an embedded SQLite exact-scan vector candidate store as the opt-in local mode, a shared contract suite over both adapters, and the evaluation repository's vector-only baseline moved onto the retrieval trace.

## Definition of Done
- Every acceptance criterion in the phase document's "Acceptance criteria" section holds with recorded evidence.
- Every row of the phase document's deferral-reconfirmation checklist has its evidence produced and cited in the Progress Log.
- Every deletion listed under "Deletions that are deliverables" is gone, with a zero-hit census.
- Both repositories' service-gated suites execute (not skip) under the service-backed CI job.
- One PR per repository per wave, merged by the decider; the evaluation repository's obligations in ADR-I-0026 are all landed.

## Scope / Non-goals
- Scope: the phase document's deliverables and deletions; the evaluation repository obligations in ADR-I-0026.
- Non-goals: the phase document's non-goals (no default flip, no approximate index, no migration tooling, no multi-process embedded access, no public candidate-search facade, no retrieval semantics change in service mode).

## Context (workspace)
- Design memo and audits: `.agent-work/orchestrator/` (v016-port-design-consult.md sections A-G; cm-design-audit.md; cme-design-audit.md; v016-consolidated-triage.md) and the researcher censuses under `.agent-work/researcher/` and the evaluation repository's `.agent-work/evals-researcher/`; all transient, consumed into this plan and the ADRs.
- As-built port: `src/ports/vector_candidate.rs`, `src/models/vector/candidate_record.rs`, `src/models/vector/record.rs`, `src/adapters/qdrant/{store,payload}.rs`, `src/policy/embedding_surface.rs`, `src/usecases/retrieve.rs`, `src/api/types/retrieval.rs`, `src/composition.rs`, `src/config/app_settings.rs`, `src/test_support.rs`.
- Prerequisite landed separately: the evaluation repository's evidence-integrity light-delta (batch outcome duplication, harness-invented rank, unhonored manifest/hash knobs, shared graph-path fallback, live-skip panic switch), branch `chore/evidence-integrity-pre-v016`.
- Repo reference docs consulted: the four ADRs; ADR-I-0018 (dependency direction; ports may import the public retrieval vocabulary under its named exception); ADR-I-0007 (schema versioning); ADR-I-0021 (embedded default pattern); rules in `docs/coding-agent/rules/`.

## Open Questions (max 3)
- none (the draft's five open questions were ruled by the decider on 2026-09-02 and are recorded in the phase document and the ADRs).

## Assumptions
- A1: `rusqlite` (already a dependency for the statistics store) is sufficient for the embedded adapter; no vector extension is added.
- A2: The evaluation repository's row/summary schema move for the typed backend identity follows that repository's normal clean-schema procedure and is owned by its wave task.

## Tasks

### Task_1: Live-gate hardening in the library test suite
- type: test
- owns:
  - tests/support/base.rs
  - tests/write_planning_tests.rs
  - tests/initialization_tests.rs
  - tests/public_facade_tests.rs
  - tests/retrieval_guardrails_tests.rs
  - .github/workflows/*.yml
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
    detail: "service-up cargo test with the switch set: all nine former skip sites execute; service-down with the switch set: the suites fail, not pass"
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
  - src/adapters/qdrant/store.rs
  - src/usecases/retrieve.rs
  - src/usecases/remember.rs
  - src/usecases/correct_forget.rs
  - src/memory.rs
  - src/test_support.rs
  - src/adapters/oxigraph/tests.rs
- depends_on: []
- description: |
  Introduce the result envelope (canonical candidates plus the typed completeness verdict) and the verdict enum in the public retrieval telemetry vocabulary; make the service adapter map its fetch decision onto the verdict; make the query scope-only with empty-scope-selects-zero and boundary rejection of an empty configured object-type set; record the verdict in retrieval telemetry beside the returned count; update every fake store. No repair, retry, or failure on the verdict.
- acceptance:
  - The envelope and enum match ADR-I-0024's Decision section; the canonical-candidates newtype is unchanged.
  - Telemetry carries the verdict for every retrieval; a retrieval test asserts each variant.
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
  - docs/design/database/vector_payload_design.md
- depends_on: [Task_2]
- description: |
  Shrink the record and the typed manifest to the five fields; drop the hint carriers, the readable text column, the per-field index creation for dropped fields, the test-only field constants and the prose-assertion note constant; replace the service adapter's private enum token mappers and the pipeline's copy with one Display/FromStr per enum in the domain. Schema version ruling: the stored schema version is retained, because every field the new contract reads is present in records written under the current version and the removal only drops fields no reader consumes; existing stored payloads with extra fields are tolerated unread. A version bump is required only if a later change adds a read field that older records lack (the re-entry paths in ADR-I-0024), and that change owns the bump and its backfill.
- acceptance:
  - The manifest test asserts exactly five entries; both text-column producers except `embedding_text` are gone.
  - Zero-hit census across both repositories for the dropped fields and for `content_text` readers (the evaluation repository's reader is removed by Task_5).
  - One token mapping per enum; census shows no copy in adapters or use cases.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; clippy -D warnings; service-up cargo test; ignored qdrant_ lib tests; census commands recorded"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review vs ADR-I-0025; verify the payload design note's supersession note matches what landed"

### Task_4: Embedded SQLite vector candidate store, settings, and parity suite (ADR-I-0023)
- type: impl
- owns:
  - src/adapters/sqlite_vector/**
  - src/adapters.rs
  - src/composition.rs
  - src/config/app_settings.rs
  - src/errors.rs
  - tests/vector_port_contract_tests.rs
  - tests/support/**
  - .env.example
  - README.md
  - docs/design/roadmap-phases/v0_1_6_embedded_vector_candidate_recall.md
- depends_on: [Task_3]
- description: |
  Implement the embedded adapter per the phase document (schema keyed on object id and surface, normalised vector blobs, object-type scope predicate, exact scan returning Exhaustive, restart safety), the `VECTOR_STORE_MODE` and `VECTOR_STORE_PATH` settings with mode-specific validation (service connection string required only in service mode), composition mode switch with `collection_name` as the backend-neutral namespace key, and the port-conformance parity suite run against both adapters (embedded unconditionally, service under the live gate). Extend the vector error vocabulary only where the embedded adapter needs a kind the service adapter lacks. Measure and document corpus-size guidance from an in-phase benchmark.
- acceptance:
  - Embedded mode constructs and retrieves with no service running; the parity suite yields identical admitted sets on the shared fixtures; the tie fixture yields Exhaustive (embedded) and BoundaryTieClosed (service).
  - Restart test passes; repeated runs are byte-identical.
  - Settings docs, single-process expectation, corpus-size guidance, and rebuild-from-graph-authority path are documented.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test with no service (parity suite + embedded tests execute); service-up cargo test with the live switch set; benchmark numbers recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review vs ADR-I-0023; independent service-free and service-up runs"

### Task_5: Evaluation repository: trace-sourced vector-only baseline and telemetry mirror (ADR-I-0026)
- type: impl
- owns:
  - crates/cmem-eval-adapter-cmem/src/lib.rs
  - crates/cmem-eval-adapter-cmem/Cargo.toml
  - crates/cmem-eval-core/src/{config,runtime,results,verdict,metrics}.rs
  - crates/cmem-eval-runner/src/pipeline.rs
  - configs/**
  - docs/**
- depends_on: [Task_2]
- description: |
  Replace the direct vector-service search in the vector-only baseline with the retrieval trace (overfetch-and-slice per kind; item text from the evaluation repository's own ingest records); mirror the completeness telemetry field; add the typed backend identity to result rows through the repository's clean-schema procedure; make the cleanup guard backend-neutral; drop the payload constants, the second vector client's search path, and the second embeddings client's divergent dimension handling. Perform the A/B run with a row-level diff of item identities and ranks against the pre-switch baseline before deleting the old path.
- acceptance:
  - Zero-hit census for vector-service search calls and payload constants in the evaluation adapter.
  - A/B evidence recorded; vector-only rows carry the completeness verdict.
  - Cleanup and namespace guards work for both vector modes.
- validation:
  - kind: command
    required: true
    owner: evals-worker
    detail: "fmt; workspace clippy -D warnings; service-up cargo test --workspace with the live switch set; A/B run artifacts under .agent-work with the diff"
  - kind: review
    required: true
    owner: evals-reviewer
    detail: "Diff review vs ADR-I-0026; verify the A/B diff and that no sealed evidence changed"

### Task_6: Evaluation repository: embedded-mode configuration and cross-mode baselines
- type: impl
- owns:
  - crates/cmem-eval-core/src/config.rs
  - crates/cmem-eval-adapter-cmem/src/lib.rs
  - configs/**
  - docs/**
- depends_on: [Task_4, Task_5]
- description: |
  Add the embedded vector mode to the evaluation backend configuration, run the continuity suite in embedded mode, and record that scenario baselines are identical to service mode under the parity contract (any divergence is a finding).
- acceptance:
  - An embedded-mode configuration exists and runs without a vector service.
  - Cross-mode baseline comparison recorded with zero unexplained divergence.
- validation:
  - kind: command
    required: true
    owner: evals-worker
    detail: "continuity suite in both modes; comparison artifact recorded"
  - kind: review
    required: true
    owner: evals-reviewer
    detail: "Verify the comparison and the register entries"

### Task_7: Fake retirement, closeout docs, and reconfirmation evidence
- type: chore
- owns:
  - src/test_support.rs
  - src/**/tests (fake stores only)
  - docs/coding-agent/plans/active/v0-1-6-embedded-vector-recall-plan.md
  - docs/coding-agent/lessons.md
  - docs/roadmap/development_roadmap.md
- depends_on: [Task_4, Task_6]
- description: |
  Retire the deterministic vector fake and its embedding-bearing record type in favour of the embedded adapter opened in memory (failure-injecting and recording fakes stay); collect the deferral-reconfirmation evidence for all five checklist rows; mark the roadmap row finished; move the plan to completed.
- acceptance:
  - Zero-hit census for the retired fake and record type.
  - All five checklist rows cite evidence in the Progress Log.
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
- owns: []
- depends_on: [Task_4, Task_5]
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
- Wave 2 (parallel): [Task_3, Task_5]
- Wave 3 (parallel): [Task_4]
- Wave 4 (parallel): [Task_6, Task_8]
- Wave 5 (parallel): [Task_7]

Each wave ends with reviewer approval and a PR per touched repository, merged by the decider before the next wave starts; the evaluation repository's sibling checkout is re-pinned to the merged library commit at every wave boundary.

## Rollback / Safety
- Embedded mode is opt-in; the service mode's behavior is unchanged except for the reported verdict and the shrunken record, both covered by the parity suite.
- Stored service-mode payloads with dropped fields remain readable (extra fields tolerated unread); rebuild from graph authority is the recovery path.
- Each wave is a separately revertible PR pair.

## Progress Log (append-only)

Append-only editing rule (applies to both logs below): when appending an entry, anchor the edit on the previous entry and reproduce it (or anchor on the section's tail marker) so the edit inserts rather than replaces, and verify afterward that the log grew.

- 2026-09-02 Planning wave completed: five parallel inputs (design consult, two altitude audits, two forensic censuses) consolidated; decider ruled the five design questions; ADR-I-0023 through ADR-I-0026, the rewritten phase document, and the roadmap section authored on branch `plan/v0-1-6-embedded-vector-recall`. Plan awaits approval.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-02 Decision: the draft's port description was rewritten as an intentional new port contract.
  - Trigger / new insight: the draft (2026-07-20) described filter, diagnostics, and reconciliation capabilities that the structured-verdict phase deleted; thirty of thirty-three payload fields were write-only; the readable text column's only reader was the evaluation repository's direct store access.
  - Plan delta: contract-first waves (envelope and query, then record, then adapter); evaluation baseline moved to the trace; deletions promoted to deliverables.
  - Tradeoffs considered: recorded in the four ADRs' rejected alternatives; the forward-looking keep case for hint fields (immutable time window) is recorded as a re-entry path rather than kept.
  - User approval: rulings on all five questions given 2026-09-02; plan approval pending.
- 2026-09-02 Decision: evidence-integrity defects in the evaluation repository are fixed before this phase, outside this plan.
  - Trigger / new insight: batch ingest produced phantom repair attempts and the evaluated rank was harness-invented; both would corrupt the parity and baseline evidence this phase cites.
  - Plan delta: none inside this plan; recorded as a prerequisite in Context.
  - User approval: yes, 2026-09-02.

## Notes
- Risks: the row/summary schema move in the evaluation repository (typed backend identity) is a clean break under its compatibility policy and must not touch sealed evidence; the exact-scan corpus-size guidance must be measured, not assumed.
- Edge cases: empty object-type scope selects zero in both adapters; `limit == 0` returns an empty exhaustive result; identical-vector tie fixtures must produce Exhaustive versus BoundaryTieClosed, never be encoded as expected parity of the bounded behavior.
