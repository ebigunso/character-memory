# Plan: Graph adapter contracts are tested once, through the port, on the real adapter

- status: in_progress (approved by the decider 2026-09-16)
- generated: 2026-09-16
- last_updated: 2026-09-16
- work_type: code

## Goal
- Every Oxigraph graph-authority contract has exactly one test, asserted through query and expansion results rather than RDF internals; the test-support module hosts fixtures only; the three retrieval pipeline tests run on the real embedded vector adapter instead of a hand-written port double.

## Definition of Done
- `src/test_support.rs` contains no tests of the graph adapter; the two tests worth keeping (typed-ref query, deterministic timeout substitute) live in `src/adapters/oxigraph/tests.rs`, and the remaining eleven are deleted with their oxigraph counterpart named.
- The `sparql_selectors.rs` and `vocabulary.rs` test modules are gone; `rdf_mapping.rs` keeps only the raw-transcript privacy invariant and the two schema-rejection tests.
- In `src/adapters/oxigraph/tests.rs` no assertion reads `triple_count`, `contains_triple`, `matching_triple_count` or named-graph ownership; the pre-hydration seam tests assert the same bounds through `expand_bounded`; error assertions match variants, not prose; the smoke and hydrate-from-RDF tests are deleted.
- `FixedVectorStore` is deleted; the three retrieval pipeline tests live in the usecase suite and run on the embedded Qdrant Edge adapter under a temporary root.
- The three constructor-glue tests in `src/ports/graph_authority.rs` and the two field-echo tests in `src/models/vector/candidate_record.rs` are deleted.
- The repo validation commands pass and the full suite executes with counts reported.

## Planner-added requirements
- None

## Scope / Non-goals
- Scope: test code under `src/adapters/oxigraph/**`, `src/test_support.rs`, the test module of `src/usecases/retrieve.rs`, `src/ports/graph_authority.rs`, `src/models/vector/candidate_record.rs`.
- Non-goals: production behaviour changes (the only production edit permitted is removing now-callerless `pub(crate)` inspection helpers on the embedded Oxigraph store, `src/adapters/oxigraph/embedded.rs`); the usecase re-anchoring work (next plan); integration tests (previous plan).

## Design
- Chosen: one owning test file per adapter, asserting the port's observable results; the test-support module keeps fixtures and doubles that record or inject failures, never port semantics. Structure: no test re-implements a port. Evolution: follows the 2026-09-14 ruling that deleted the graph fake (lessons.md, same date) and applies it to the vector side. Verification: RDF refactors no longer break tests; the pipeline tests exercise the real vector adapter. Operation: unchanged. Human: a failing test names the contract it protects. Safety: the raw-transcript privacy invariant keeps its dedicated test.
- Alternative: keep the RDF-level assertions as a second layer and only delete exact duplicates. Structure: two observers per contract. Evolution: every quad-shape change edits two tests. Verification: the RDF layer only observes what the query layer already observes, so it adds no defect detection.
- Why chosen: the audit found the query-level assertion present in every test that also asserts quads, so the RDF layer is pure cost.

## Compatibility stance
- surface: none; test code only.
- stance: preserve
- justification: no contract, interface or persisted format changes; the `pub(crate)` inspection helpers on the embedded store may become unused and are removed only if nothing else calls them.

## Context (workspace)
- Related files/areas: `src/adapters/oxigraph/tests.rs` (34 tests; quad assertions at 59, 263, 290, 323, 379, 426, 1492, 1566; ownership 507; seam tests 891, 927; prose 1454; pipeline tests 1330, 1746, 1837; `FixedVectorStore` 1961-2011); `src/adapters/oxigraph/{rdf_mapping,sparql_selectors,vocabulary}.rs`; `src/test_support.rs:591-1149`; `src/ports/graph_authority.rs:432-483`; `src/models/vector/candidate_record.rs:158,196`.
- Existing patterns or references: `src/test_support.rs:825` temporary vector store fixture (cleanup on drop); the embedded adapter's own tests in `src/adapters/qdrant_edge/mod.rs` for constructing a store under a temp root.
- Design record consulted and deviations from its acceptance: lessons.md 2026-09-14 "A test fake that re-implements a port is deleted when the real adapter has an in-memory mode"; HANDOFF test-doubles constraint. No deviation.
- Audit source: `.agent-work/reviewer/test-suite-audit-2026-09-16.md` sections F2, F3 (RDF internals), F4; partition report B and C. Line cites are against main 4a00303 and are re-located by test name after the integration-suite branch lands beneath.

## Open Questions (max 3)
- None.

## Assumptions
- A1: Every quad-level assertion sits beside a query-level assertion of the same contract in the same test — source: partition report B per-test findings for `tests.rs`; the worker confirms per test before deleting and adds the query-level assertion where it is missing.
- A2: The embedded adapter can stand in for `FixedVectorStore` in the three pipeline tests, which only need a store that returns a known candidate — source: `src/adapters/qdrant_edge/mod.rs` upsert and search tests; verified by Task_3 running them.

## Tasks

### Task_1: Oxigraph module tests consolidated onto port-level assertions
- type: test
- owns:
  - src/adapters/oxigraph/**
- depends_on: []
- description: |
  Strip the RDF-internal halves, delete the ownership, smoke and hydrate-from-RDF tests, re-anchor the two pre-hydration seam tests and the prose assertion, remove the three pipeline tests and `FixedVectorStore` from this module (they are re-created in Task_3), delete the `sparql_selectors` and `vocabulary` test modules, reduce `rdf_mapping` tests to the three earners, and receive the typed-ref query and timeout-substitute tests from `test_support`. Remove any `pub(crate)` inspection helper on the embedded Oxigraph store (`embedded.rs`) that has no remaining caller. Copy the two relocated tests (typed-ref query, deterministic timeout substitute) from `src/test_support.rs` at the wave base commit; Task_2's verbatim hand-over in its report is the orchestrator's integration cross-check.
- acceptance:
  - No test under the owned path calls `triple_count`, `contains_triple`, `matching_triple_count`, or asserts named-graph ownership or error prose.
  - Each deleted test is listed with the surviving test that observes the same contract, by file and line.
  - The typed-ref query and timeout-substitute contracts each have one test in this module.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --lib adapters::oxigraph (executed count reported)"

### Task_2: test_support hosts fixtures only; port and model glue tests deleted
- type: test
- owns:
  - src/test_support.rs
  - src/ports/graph_authority.rs
  - src/models/vector/candidate_record.rs
- depends_on: []
- description: |
  Delete the graph adapter tests and the fixture-reads-its-own-fixture test from `test_support`, keeping the temporary-vector-store cleanup test and the deterministic embedder test; delete the three constructor-glue tests in the graph authority port and the two field-echo tests in the candidate record model.
- acceptance:
  - `test_support` has exactly two tests and no `in_memory_graph_*` test.
  - The typed-ref query test and the deterministic timeout-substitute test are handed over verbatim in the report for Task_1 to host.
  - No test in the owned files asserts a constructor's field equals the value passed to it.
  - Every deleted test is listed with the surviving observer or the reason none is needed.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --lib (executed count reported)"

### Task_3: Retrieval pipeline tests run on the real embedded vector adapter
- type: test
- owns:
  - src/usecases/retrieve.rs
- depends_on: []
- description: |
  Re-create the three pipeline tests (lifecycle mutation excludes stale records, fixed candidate expands with embedded Oxigraph, persistent reopen keeps graph-authority filters) in the retrieve usecase test module using the embedded Qdrant Edge adapter under a temporary root and the in-memory or persistent Oxigraph adapter; drop the rationale summary prose assertion. Stores created by the tests are closed and removed.
- acceptance:
  - The three contracts each have one test in the retrieve module; none uses a hand-written vector store double.
  - No assertion matches rationale summary prose.
  - Temporary roots are removed after each test.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --lib usecases::retrieve (executed count reported)"

### Task_4: Independent review
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Confirm in a pinned worktree that every deleted test's contract has a surviving observer, that no port double remains under `src/`, that the full suite runs with a positive executed count and no new ignored tests, and that the raw-transcript privacy invariant still has its test.
- acceptance:
  - Reviewer status is APPROVED with the deletion mapping independently confirmed.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review against acceptance; grep census for port-shaped doubles under src/; cargo test full run with counts"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_2, Task_3]
- Wave 2 (parallel): [Task_4]

Parallel tasks run in separate worktrees so Cargo commands never share a target directory mid-edit; the orchestrator runs the repo validation commands on the integrated branch after each wave.

## Rollback / Safety
- Single branch `feature/2026-09-16/graph-adapter-test-consolidation` stacked on the integration-suite plan's branch; test code only; revertible as one commit.

## Progress Log (append-only)

- 2026-09-16 Plan drafted from the test-suite audit (F2, F3, F4; partition reports B and C).
- 2026-09-16 Reviewer pass (Tier A): production-edit exception stated in Non-goals; Task_2 to Task_1 handoff made explicit; embedded Oxigraph vs embedded Qdrant Edge disambiguated; cite-staleness note added.
- 2026-09-16 Confirmation pass: Task_1 copies the two tests from the wave base commit rather than from a parallel task's report.

- 2026-09-16 Decider accepted all eight plans; branch cut from the integration-suite branch tip 944acbf (PR open, stacked beneath).

- 2026-09-16 Wave 1 completed: [Task_1, Task_2, Task_3] (pre-rebase task commits 0d9614c, 7114b58, 4cdb313; after the rebase onto the integration-suite tip a61241e they are 02b15d4, 7f90fc5, 1778456), plus the review follow-up 7f9873a
  - Summary: Oxigraph tests consolidated onto port-level assertions (47 to 31 adapter tests); sparql_selectors and vocabulary test modules removed; test_support keeps two tests; ports and models glue tests removed; the three retrieval pipeline tests run on the embedded Qdrant Edge adapter; Task_2's assertion gaps carried into Task_1's surviving tests. Review follow-up: FixedVectorCandidateStore in src/memory.rs tests replaced by the real temporary embedded store; the surviving link-query test writes links onto an empty object store again.
  - Validation evidence (orchestrator, post-rebase tip 1778456): fmt, clippy, test --no-run clean; cargo test lib 374 passed 5 ignored, integration 2+3+12+6, doc 1, zero skipping. Follow-up: cargo test --lib memory 40 passed; link-only test passed.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-16 Decision (review): src/memory.rs FixedVectorCandidateStore, a pre-existing port double outside Wave 1 ownership, is removed in this plan (follow-up 7f9873a) because the Definition of Done states no port double under src. The retrieve module's RecordingVectorStore (src/usecases/retrieve.rs ~3233-3300, a semantic double despite its name, about 18 construction sites, some needing completeness injection) is carried into the unit-test re-anchoring plan, whose Task_3 owns that module: rebuild it as a wrapper over the real embedded adapter injecting only completeness and failures, or seed real records where that suffices. Acceptance 3 of this plan is read as the files Wave 1 touched plus src/memory.rs.
- 2026-09-16 Decision: the typed-ref query test relocated from test_support maps onto the existing selection-table case in the Oxigraph tests (same contract) instead of a standalone duplicate; triple_count on the embedded store is kept because src/usecases/correct_forget.rs still calls it, other callerless inspection helpers were removed.
- 2026-09-16 Decision: the vector-side port double follows the graph fake out; pipeline tests run on the real embedded adapter. Trigger: audit F4. User approval: pending.

## Notes
- Risks: Wave 1 tasks compile against each other (Task_1 removes tests Task_3 re-creates; Task_2 deletes two tests Task_1 hosts); the orchestrator runs the full suite after the wave, not per task, and checks both handoffs against the reports.
- Edge cases: the embedded store under Windows path limits; use short temp roots.
