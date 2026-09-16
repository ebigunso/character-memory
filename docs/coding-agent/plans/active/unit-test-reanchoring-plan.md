# Plan: Unit tests assert typed contracts and outcomes, not prose, transcripts or serde plumbing

- status: in_progress (approved by the decider 2026-09-16; Tier D APPROVED 2026-09-17 at 3c2f154; pull request open, awaiting merge approval)
- generated: 2026-09-16
- last_updated: 2026-09-17
- work_type: code

## Goal
- A test in the library fails only when a contract breaks: error assertions match typed variants, write-ordering assertions compare persisted state and ordering rather than recorded call transcripts, serde round-trips exist only where a persisted consumer reads the shape, and each duplicated contract keeps one observer at the boundary a consumer sees.

## Definition of Done
- With the graph-adapter plan landed beneath, no test under `src/` asserts an error message substring or `Display` text; every rejection a test asserts has a typed variant, introduced in Wave 1 where one is missing today.
- No test in `src/usecases/correct_forget.rs` or `src/usecases/remember.rs` asserts equality against a full recorded call transcript; ordering contracts are asserted as "graph writes precede vector maintenance" plus persisted state.
- The retrieve, write-planning, remember, link and stats-projection modules have the deletions and folds from the audit applied, with each deletion mapped to its surviving observer.
- The three constructor and composition tests in `src/memory.rs` live in the config or composition test modules; the facade module keeps facade contracts only.
- Retrieval-stats stores share one port-contract suite run over both `SqliteRetrievalStatsStore` and `InMemoryRetrievalStatsStore`; no test reads raw SQL rows or the in-memory store's internal map.
- Selectivity validation is tested once, at the config boundary; the policy module keeps its plan-level tests.
- Serde round-trip tests remain only for enum tokens and hand-written token parsers that reach RDF literals or vector payloads; struct round-trips and constant-equals-literal tests are deleted.
- Qdrant, Qdrant Edge, OpenAI, payload and tie-closure adapter tests carry the re-anchors from the audit (no Debug-string equality, no backoff timing lower bound, no self-derived identity, no upstream `type_name` probes).
- Repo validation commands pass on the integrated branch; the full suite executes with counts reported; no new ignored tests; the companion evaluation workspace compiles and passes against the reviewed commit.

## Planner-added requirements
- None

## Scope / Non-goals
- Scope: test modules under `src/**` not covered by the graph-adapter plan, the prose assertion cases the integration-suite plan handed off in `tests/write_planning_tests.rs`, plus the minimal production additions of typed error variants where a rejection is prose-only today (variants declared in the error enums by Task_1; producers wired by the Wave 2 task that owns the producing file).
- Non-goals: new contracts (gaps plan); integration tests; the Oxigraph module and `test_support` (previous plan).

## Design
- Chosen: introduce every missing typed variant first in one task that owns the error enums, then re-anchor module by module in parallel, deleting rather than rewriting whenever another test already observes the contract. Structure: one owner per error enum during the change; one observer per contract after it. Evolution: rewording a message or reordering a query no longer edits tests; follows worker.md "typed at introduction" and lessons.md 2026-07-21. Verification: tests pin contracts, not implementation. Operation: unchanged. Human: a failing test names the variant. Safety: the raw-transcript and key-absence privacy assertions stay.
- Alternative: leave transcript and prose assertions and only delete exact duplicates. Structure: unchanged. Evolution: refactors of the write sequence keep failing tests that no consumer backs. Verification: cheaper now, costlier at every change.
- Why chosen: the audit found the transcript and prose assertions are the largest source of refactor-punishing failures; the typed vocabularies they should use already exist for most cases, and sequencing the variants first keeps every later task inside its own files.

## Compatibility stance
- surface: error enums that gain a variant where a rejection is prose-only today (candidates: link admission in `DomainValidationError` at `src/domain.rs:250` and `src/domain/write_validation.rs:88`; source-object correction reference checks; missing-timestamp commit values; the sqlite negative counter in `RetrievalStatsStoreError` at `src/errors.rs:252`).
- stance: break
- justification: no external consumer of the crate exists (common.md compatibility policy). The companion evaluation repository is the only in-workspace consumer; whether it matches on any of these enums is established by Task_1's grep over `../CharacterMemoryEvals/crates` and recorded in the Decision Log, and the orchestrator repins the companion checkout to the reviewed commit before the review task so the reviewer consumes a real build.

## Context (workspace)
- Related files/areas: partition reports A, B and C in `.agent-work/reviewer/test-suite-audit-2026-09-16-partition-reports.md` list every finding by file and line; the audit's section F3 groups them by surface. Line cites are against main 4a00303; after the two lower branches land, findings are re-located by test name.
- Existing patterns or references: `src/usecases/write_planning.rs` validator tests (typed `CandidateValidationIssue` per branch) are the model; `src/domain.rs:250` `DomainValidationError`; `src/errors.rs:405` `CustomError`; `src/errors.rs:252` `RetrievalStatsStoreError`.
- Design record consulted and deviations from its acceptance: ADR-I-0029 (outcome and error typing) for any added variant. No deviation.

## Open Questions (max 3)
- Q1: Do the trace and telemetry snake_case tag tests in `src/api/types/retrieval.rs:785,995` have a consumer in the evals repository that snapshots trace JSON? If none is found by Task_6, both are deleted; if one is, one test stays.

## Assumptions
- A1: Every prose assertion named in the audit has a typed variant or a natural home for one — source: partition report A and B per-finding lines; Task_1 enumerates the prose-only rejections and reports the exceptions.
- A2: The in-memory retrieval stats store is the real adapter's in-memory mode, not a fake — source: `src/ports/retrieval_stats.rs` (adapter suite hosted in the port file); confirmed by the 2026-09-14 ruling scope.

## Tasks

### Task_1: Typed variants for every prose-only rejection; errors and domain tests trimmed
- type: impl
- owns:
  - src/errors.rs
  - src/domain.rs
  - src/domain/**
- depends_on: []
- description: |
  Enumerate every rejection the audit's prose assertions cover (link admission, source-object correction references, missing commit timestamps, sqlite negative counter, the cases the integration-suite plan handed off, and any other found by a grep of `to_string().contains` over `src` and `tests`), declare the missing variants in the owning enums, and name for each the production site that will produce it; the Wave 2 task owning that site wires the producer. In the same files, delete the struct-level serde round-trips, the constructor read-backs and the display-message tests named in the audit; re-anchor the stable-rank test on distinctness and restrictiveness ordering and the raw-ref fixture test on the serialized object; keep enum token, URN and schema-version tests. Grep the companion repository for matches on the changed enums and record the result.
- acceptance:
  - Each added variant is listed with the production site that will produce it, the Wave 2 task that owns that site, and the test task that will assert it.
  - The crate-wide prose-assertion inventory is in the report with a typed target for every entry.
  - No struct-level serde round-trip or display-message test remains in the owned files; the remaining serde tests cover enum tokens, URNs and schema versions only.
  - The companion-repository grep result for every changed enum is recorded in the Decision Log.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test --lib (executed count reported); rg -n 'to_string\\(\\)\\.contains' src tests (inventory attached)"

### Task_2: Write-path usecases re-anchored
- type: test
- owns:
  - src/usecases/remember.rs
  - src/usecases/write_planning.rs
  - src/usecases/correct_forget.rs
  - src/usecases/link.rs
  - tests/write_planning_tests.rs (the prose assertion cases handed off by the integration-suite plan)
- depends_on: [Task_1]
- description: |
  Wire the producers for the variants Task_1 declared whose producing site is in these files. Apply the audit's findings: typed variants replace prose, transcript equality becomes ordering plus persisted state, the listed deletions and folds are made, the seven-scenario role matrix becomes two scenarios, the 110-line remember test is split.
- acceptance:
  - No `to_string().contains` or `Display`-text assertion remains in the owned modules.
  - No assertion compares a full `calls()` transcript; ordering assertions name the two operations whose order is the contract.
  - Every deleted or folded test is mapped to its surviving observer by file and line.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib usecases::remember; cargo test --lib usecases::write_planning; cargo test --lib usecases::correct_forget; cargo test --lib usecases::link (executed counts reported)"

### Task_3: Read-path usecases re-anchored
- type: test
- owns:
  - src/usecases/retrieve.rs
  - src/usecases/stats_projection.rs
  - src/usecases/vector_indexing.rs
- depends_on: [Task_1]
- description: |
  Wire any producer Task_1 assigned to these files. Apply the audit's findings for retrieve: delete the implementation-pinning tests, fold the rationale-category tests into the classification test, drop config-echo and weight-literal assertions, reduce the completeness matrix to one representative plus NotRequested, drop rationale prose. The three pipeline tests relocated here by the graph-adapter plan keep their contracts.
- acceptance:
  - No config-echo, weight-literal or prose assertion remains in the owned modules.
  - Every deleted or folded test is mapped to its surviving observer by file and line.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib usecases::retrieve; cargo test --lib usecases::stats_projection (executed counts reported)"

### Task_4: Facade keeps facade contracts; config, composition and policy tests own construction and validation
- type: test
- owns:
  - src/memory.rs
  - src/config/**
  - src/composition.rs
  - src/policy/**
- depends_on: [Task_1]
- description: |
  Move the zero-vector-size, endpoint-URL-as-path and sqlite fallback tests out of the facade module into the config or composition test modules; re-anchor the facade's collision assertion on the typed error and drop the fixture-induced stats failure assertion; collapse the config graph-mode and override read-back tests into table-driven forms; delete the selectivity policy validation duplicates and the Default tautology; drop the prose line in the graph expansion root test and the redundant equality block in the fanout utilization test; trim the embedding surface builder test to its per-variant dispatch.
- acceptance:
  - `src/memory.rs` tests construct `CharacterMemory` through the facade only.
  - Selectivity numeric and fanout validation each have exactly one rejection test, at the config boundary.
  - Each moved or deleted test is mapped to its destination or surviving observer.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib memory; cargo test --lib config; cargo test --lib composition; cargo test --lib policy (executed counts reported)"

### Task_5: One retrieval-stats port-contract suite over both stores
- type: test
- owns:
  - src/adapters/stats/**
  - src/ports/retrieval_stats.rs
- depends_on: [Task_1]
- description: |
  Wire the negative-counter producer in the sqlite adapter to the variant Task_1 declared. Replace the paired sqlite and in-memory store tests with one suite that runs the same contract cases over both adapters through the port; keep the sqlite-only durability tests and the `retrieval_stats_edges` derivation tests; assert the negative counter through that variant. No test reads SQL rows or the in-memory state map. Every sqlite file the suite creates is removed at test end.
- acceptance:
  - Each shared contract case executes once per adapter (counts reported per adapter).
  - No raw `SELECT` and no `state.lock()` access remains in tests.
  - Durability across reopen keeps its sqlite-specific tests; temporary database files are removed after each test.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib retrieval_stats; cargo test --lib adapters::stats (executed counts reported)"

### Task_6: API types keep wire-contract tests only
- type: test
- owns:
  - src/api/types/**
- depends_on: [Task_1]
- description: |
  Delete struct-level serde round-trips, display-message tests, constructor read-backs and derive(Default) delegation tests named in the audit; keep the tests whose token names reach RDF literals (`draft.rs:756`) and the source-span and diagnostics logic tests; settle Q1 by searching the evals repository for a trace JSON consumer.
- acceptance:
  - Each remaining serde test in the owned files is mapped in the report to the persisted consumer of its shape.
  - No test constructs a value and asserts the field it just set.
  - Q1 is answered in the Decision Log with the grep evidence.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib api (executed count reported); rg -n 'section_assignment|telemetry' ../CharacterMemoryEvals/crates --type rust (evidence for Q1)"

### Task_7: Vector and embedding adapter tests re-anchored
- type: test
- owns:
  - src/adapters/qdrant/**
  - src/adapters/qdrant_edge/**
  - src/adapters/openai/**
- depends_on: [Task_1]
- description: |
  Apply the audit's findings: delete config plumbing, filter-shape, legacy-field and duplicate tests; assert error classification without message equality; test `http_transport_status` directly instead of through a TCP listener; compare candidates by value rather than Debug string; drop the backoff lower time bound and the `type_name` and panic probes; assert point identity by its properties rather than by re-deriving it; assert the tie-closure loop by monotonic growth and closure rather than the exact schedule; fold the live smoke's search half onto the contract suite and keep its delete leg. The ignored idle-gap canary stays.
- acceptance:
  - No test in the owned modules asserts a message string, a Debug rendering, a lower elapsed-time bound, or a value re-derived by the same function under test.
  - Each deleted test is mapped to its surviving observer.
  - The ignored canary and the live tie-determinism test are unchanged.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --lib adapters::qdrant; cargo test --lib adapters::qdrant_edge; cargo test --lib adapters::openai (executed counts reported)"

### Task_8: Independent review with the companion pinned
- type: review
- owns: []
- depends_on: [Task_2, Task_3, Task_4, Task_5, Task_6, Task_7]
- description: |
  The orchestrator integrates Wave 2, runs the repo validation commands on the integrated branch, and repins the companion evaluation checkout to that commit between reviews. The reviewer then, in a pinned worktree, confirms every deleted test's surviving observer, greps the whole crate for remaining prose and transcript assertions, confirms each added error variant is produced by production code and asserted by a test, runs the full suite with counts, and runs the companion workspace suite against the repinned checkout.
- acceptance:
  - Reviewer status is APPROVED with the grep census attached and the companion workspace compiling and passing against the reviewed commit.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test --no-run; cargo test (full, counts reported) on the integrated branch; companion checkout repinned to that commit and the repin recorded"
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review against acceptance; rg census for to_string().contains and calls() equality under src/; cargo test full run; cargo test --workspace in the repinned companion checkout"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2, Task_3, Task_4, Task_5, Task_6, Task_7]
- Wave 3 (parallel): [Task_8]

Parallel tasks run in separate worktrees so Cargo commands never share a target directory mid-edit; per-task validation is module-scoped and the orchestrator runs the repo validation commands on the integrated branch after each wave.

## Rollback / Safety
- Single branch `feature/2026-09-16/unit-test-reanchoring` stacked on the graph-adapter plan's branch; production changes are the typed error variants, the deletion of the superseded `RationaleOrigin` surface, the `MemoryValidation(String)` variant and the Oxigraph `triple_count` helper, and the usecase error paths rewired onto the typed variants; revertible as one commit. If review load demands, Wave 2 is landed as two stacked PRs (write path and facade; adapters, stats, types) without changing the tasks.

## Progress Log (append-only)

- 2026-09-16 Plan drafted from the test-suite audit (F2 remaining pairs, F3).
- 2026-09-16 Confirmation pass: Task_1 declares variants and names producer sites; each Wave 2 owner wires its producer in its own files; the integration-suite plan's handed-off prose cases are in Scope and Task_2's owns; the inventory grep covers `tests` too.
- 2026-09-16 Reviewer pass (Tier A): typed variants moved into a Wave 1 task that owns the error enums so no Wave 2 task edits outside its `owns`; usecases split into write and read paths; the consumer-comment convention dropped; companion repin made orchestrator-owned; single-filter test commands; cleanup bullet on the stats suite; cite-staleness note.

- 2026-09-16 Wave 1 completed: [Task_1] (92169f0, rebased as 6d35d51 onto the graph-adapter reviewed tip 54f63b5)
  - Summary: variants declared (CustomError::DomainValidation transparent from DomainValidationError, LowInformationCoOccurrence, MissingOriginalSourceReference, OriginalSourceReferenceMismatch with SourceReferenceKind, DeterministicIdCollision; RetrievalStatsStoreError::NegativeCounter); errors and domain serde plumbing tests removed; crate-wide prose inventory with producer and owner per site; companion grep zero hits for CustomError, RetrievalStatsStoreError and SourceReferenceKind matches.
  - Validation evidence: fmt and clippy clean; cargo test --lib 369 passed 5 ignored.
- 2026-09-17 Wave 2 completed: [Task_2 3d5b356, Task_3 e3d5ac5, Task_4 b8831f1, Task_5 a57c58f, Task_6 b5d3584, Task_7 5df6e7b] (integrated tip 5df6e7b)
  - Summary: write-path and read-path usecase tests re-anchored on typed variants, ordering plus persisted state; the retrieve module's semantic vector double rebuilt over the real embedded store at 18 sites; facade module keeps facade contracts; one retrieval-stats port-contract suite runs over both stores; API type tests keep wire-contract and validation assertions; vector and embedding adapter tests re-anchored; the four Oxigraph erasure maps use the transparent DomainValidation variant; the orphaned triple_count helper removed.
  - Validation evidence: per-task module counts in the worker reports; orchestrator full-suite run on the integrated tip recorded in the Wave 3 entry.

- 2026-09-17 Wave 3 completed: [Task_8] (reviewed 8818bc8, final pin 3c2f154 after the three restored wire-token tests)
  - Summary: cm-reviewer APPROVED. Findings: three deleted serde tests had a persisted consumer (the evaluation workspace's JSONL records) and were restored as focused token observers; two qualifications recorded (LowInformationCoOccurrence reachable through test evidence only; companion Windows symlink exception).
  - Validation evidence (reviewer, pinned worktree, Qdrant unused, no .env): fmt, check, clippy, test --no-run clean; lib 322 passed 3 ignored at 8818bc8 and 325 at 3c2f154; integration 2+3+12+6; doc 1; zero skipping; TEMP 2700 to 2699, zero additions; ADR-I-0018 diff-scoped dependency audit clean. Companion: cargo check --workspace --all-targets clean and cargo test --workspace green except the known symlink exception against 8818bc8.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-16 Decision: branch cut from the graph-adapter branch tip 1778456 while that plan is under Tier D review; Wave 1 touches only the error and domain modules, disjoint from the graph-adapter files, and this branch rebases onto the reviewed tip before its own review. Trigger: decider directive to progress the stack.

- 2026-09-16 Decision (Task_1 inventory): CustomError gains a transparent DomainValidation(DomainValidationError) variant so usecase adapters stop erasing domain rejections into MemoryValidation(String); missing commit timestamps reuse CandidateValidationIssue::MissingTimestamp through WritePlanValidationRejected rather than a second vocabulary.
- 2026-09-16 Decision (Task_2 alert): the orphaned cfg_attr-allowed OxigraphGraphAuthorityStore::triple_count helper is deleted rather than suppressed; owns widened for that one deletion.
- 2026-09-16 Decision (Task_3 alerts): SectionScoreComponents keeps its weights private; the score-components test asserts provenance equality, derived below root and consistent ordering, not literal weights. The stats_projection hydration test asserts the EndpointHydration cause by variant, not the production sentence.
- 2026-09-17 Decision (Task_5 alert): the four first-seen and last-seen timestamp tests are deleted without replacement and the port is not widened: no production consumer reads those values back, so a test would be its only observer (necessity gate). Revisit when a consumer appears.
- 2026-09-17 Decision (Task_4 alert): the production MemoryLink-root rejection in policy/graph_expansion.rs becomes CustomError::UnsupportedExpansionRoot { object } with its policy test and the Oxigraph observer updated; the two cfg(test) helper rejections become panics. Dispatched as the Wave 2 follow-up (Task_8 on task/p3-t8).
- 2026-09-17 Decision (Task_6 alert and Q1): CandidateProvenance::rationale_origin, CandidateRationale::origin and the RationaleOrigin enum have no caller in this crate or the companion; deleted with their re-exports in the Wave 2 follow-up (no consumers, common.md policy). Q1 answered yes: the companion persists RetrieveOutcome inside its JSONL records (cmem-eval results.rs PerQuestionResult) and probes the serialized trace by JSON pointer, so src/api/types/retrieval.rs section_assignment_shape_reports_final_section stays and the telemetry token test is deleted; a persisted wire consumer can live outside this crate's adapters (lesson candidate).
- 2026-09-17 Decision: Task_4 was dispatched in parallel with Task_2 although its facade collision assertion depends on Task_2's producer; its required memory-module check was red on its own branch and green at integration. Lesson candidate: a wave that splits a producer from its observer names the integration-owned check in the packet.
- 2026-09-17 Decision (review): three focused wire-token tests are restored because the companion evaluation workspace persists the shapes inside its JSONL records (RememberOutcome, LinkOutcome and LifecycleMutationOutcome carry StatsUpdateStatus; RememberOutcome.vector_indexing_failure carries EmbeddingError; lifecycle diagnostics carry the cascade warning reason): StatsUpdateStatus cause and kind tokens, EmbeddingError::Unrecognized kind and opaque detail, LifecycleMutationWarningReason::CascadeSuppressesCurrentReplacement token. Rule candidate for common.md: a persisted wire consumer can live outside this crate's adapters.
- 2026-09-17 Disposition (review): CustomError::LowInformationCoOccurrence is reachable only through cfg(test) evidence today because the public link path always supplies ExplicitCallerIntent; the variant is accepted as declared-ahead vocabulary on the existing production admission branch, asserted through the test evidence path, and its test moves to the production producer when one appears. The companion runner's OS error 1314 failure is the known Windows symlink-privilege exception (Linux CI authoritative).
- 2026-09-16 Decision: prose-only rejections gain a typed variant rather than keeping a prose assertion, introduced in one Wave 1 task before any re-anchoring. Trigger: worker.md typed-at-introduction rule; reviewer finding that three parallel tasks would otherwise edit the same enums. Decider approval: plan accepted 2026-09-16; merge approval pending.
- 2026-09-17 Copilot follow-up (PR 99): the stable-rank test asserts strictly increasing object and relation ranks (uniqueness alone let two ranks swap unnoticed); `CustomError::MemoryValidation(String)` had no producer or matcher left after the typed conversion and is deleted per the compatibility policy; the rollback line names every production surface this branch changes; the PR body's executed count is the final 325.

## Notes
- Risks: Wave 2 is six parallel workers; the orchestrator integrates and validates once on the integrated branch rather than trusting six module-scoped runs.
- Edge cases: a variant added here changes the evals adapter's error mapping; the repinned companion run catches it.
