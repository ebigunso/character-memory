# Plan: Value-audit cleanup

- status: draft
- generated: 2026-09-24
- last_updated: 2026-09-24
- work_type: code

## Goal
- Production recalls the latest occasion of a participant who is present in nearly every experience, as ADR-I-0036 states, because the participant fanout default is one to five on the path every caller uses. It is also the only default table.
- Code that never runs, that nothing reads, or that only repeats another test is gone, as ruling 77 decided. Nothing else about what comes to mind changes, and one proof at the final tip shows it.
- The prospective slice then starts from a smaller crate. Its BEFORE readings are known to hold at this plan's final tip.

## Definition of Done
- Production builds the participant-occasion budget at one to five from one default table, owned by the settings and validated once there. The policy constructor cannot fail. A regression test through `CharacterMemory::new_with_embedding_provider` fails at c0ed9e21 and passes at Task_1's tip.
- Task_1's BEFORE and AFTER numbers are in this plan's Decision Log, with a paragraph on what they mean for the character and a verdict. The falsifier held.
- Every verdict item for the library is done: LP1 to LP14 and LT1 to LT22. LP15, LT23 and LT24 stay as they are. Each task report lists:
  - each item, with its files and line counts before and after;
  - each deleted test, with the behavior or code it covered.
- One behavior-free proof holds at the integrated final tip (Task_12's):
  - what the instrument records (returned ids, sections, order, scores, trace rank and fanout utilization) equals Task_1's AFTER;
  - trace, report and header fields change only where fields are deleted.
  This single proof replaces the audit verdicts' per-task measurement requirement (ruling 77 authorizes the value-audit change). Each intermediate tip is covered by its own review and gates, not by this proof.
- The per-task gates passed: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` with counts, and `pre-commit run --all-files`. The companion also builds against each task's tip.
- The evals dependency below has landed on the evals cleanup branch, and the companion's `cargo test` passes against this plan's final tip. Both must be true before this stack merges.
- A Tier A completion value audit found nothing the verdicts deleted still in place, and nothing added that the verdicts did not ask for.

## Planner-added requirements
- Task_1's regression test must be seen failing at c0ed9e21 before it passes at the tip. Needed because: the defect survived behind scene tests that pass while production is wrong. A test that never failed at the parent does not show that it reaches the production path.
- Before Task_1's AFTER numbers are read, the families whose probes present a keyed participant are listed from their definitions. The reading also reports every memory that an added occasion displaced. Needed because: the falsifier treats participant-heavy families differently from the rest. If the families were sorted after the numbers were seen, the falsifier could not fail. The falsifier counts only gains, so without the displacement report the verdict could not say what the character lost at a full cap.
- The one-value `GraphFailureMode` and the two `failure_mode` fields go together with `FailClosed`. There are two fields: the public one on `RetrievalGraphLimits` and the one on the port's failure policy. Needed because: once `FailClosed` is gone, the setting can only name what the library always does. LP4 deletes that shape, and ruling 77 deletes "the fail-closed graph mode", not just one of its values.
- Each `#[cfg(test)]` pipeline constructor creates its own store with `Box::leak(Box::new(InMemoryRetrievalStatsStore::new()))`. This is test-only: each test gets a fresh store, nothing is shared, and the 66 call sites are unchanged. A `ponytail:` comment at the leak names the ceiling (per-test memory until the process exits) and the upgrade path (fixture-owned stores, if test memory ever matters). Needed because: the no-op store discarded writes, and an in-memory store keeps them. With a shared store, one test's counts would steer another test's selectivity decisions depending on test order. The tests would then no longer behave as they did at the parent.
- The evals dependency also covers two other uses in the same `adapter.rs` test module: `CURRENT_SCHEMA_VERSION` (`adapter.rs:2773`, `:5355`) and `Settings::get_oxigraph_path` (`adapter.rs:3039`). Needed because: LP13 keeps only `DEFAULT_SCHEMA_VERSION` and LP14 makes the getter crate-private, so both uses stop compiling. The owns set stays the one the request names.
- Before Task_6 changes the shape of `io_kind`, it searches the companion's committed evidence for `io_kind`. Needed because: sealed evidence is the one exception to the no-backcompat rule. The companion also re-reads persisted remember outcomes, which can carry a vector indexing failure (`errors.rs:526-527` names the path).

## Scope / Non-goals
- Scope: the library verdict items of ruling 77 (LP1 to LP14, LT1 to LT22), the one README line LT3 names, and this plan. The companion change is named here as a dependency and is carried by the evals cleanup plan.
- Non-goals:
  - LP15, the internal embedder port. It is kept, and its risk to test fidelity is logged in ruling 77.
  - LT23 and LT24, which are kept.
  - The unhealthy statistics marker that never clears, a known gap logged in ruling 77.
  - Any evals production change. The evals cleanup plan owns it.
  - Record edits. ADR-I-0011's decision holds as ruling 77 reads it. ADR-I-0007 line 44 names all three schema-version constants; see Notes.
  - Embedder doubles in LT12. The embedder port has no in-memory adapter to wrap, and it is LP15's subject.
  - Measuring any task other than Task_1. The behavior-free tasks are proven equal once, at the final tip; they are not measured.

## Design
- Chosen: the fix first and alone, then ten behavior-free tasks in two lanes whose owns sets do not overlap, then the test-double consolidation, which is also behavior-free.
  - Task_1 changes behavior and carries the plan's one measurement.
  - The behavior-free tasks are grouped by the files they must edit, not by audit item. Every public item that is removed must also leave the re-export list in `src/lib.rs`, and rustfmt reflows that list as a block. So every task that edits `lib.rs` sits on one serial spine: Task_2, Task_4, Task_6, Task_8 and Task_10. A second lane of tasks whose owns avoid the spine's files runs beside it: Task_3, Task_5, Task_7, Task_9 and Task_11.
  - LT12, as Task_12, lands last, after every task that deletes tests or constructors in the files its doubles live in.
  - Structure: the end state has one fanout table and one validator, and no second implementation of the stats port.
  - Evolution: the concept count drops: one config enum, one graph mode, the telemetry echo, three constants and one feature flag go. The one behavior change sits at the bottom of the stack.
  - Verification:
    - BEFORE reuses the consolidation slice-end AFTER at c0ed9e21, so the plan needs no extra BEFORE run.
    - One behavior-free proof at the integrated final tip (Task_12's) compares with Task_1's AFTER, on the fields the instrument records. It says nothing about intermediate tips, which are covered by their own review and gates. If it differs, the orchestrator bisects over the retained wave tips, runs only there, and reruns the final comparison after any correction.
    - Per task, the gates and a companion build against the tip catch a removed surface the companion's production uses (A4) loudly and at once.
    - Test doubles wrap the real adapter's in-memory mode.
  - Operation: no runtime cost changes. The SQLite stats edge table loses two columns and a MIN/MAX upsert.
  - Human: each stacked PR has one reason. Every item says where in the audits it came from.
  - Safety: two settings getters that returned secrets as plain `&str` stop being public (LP14), and the `println!` in library code goes (LP8).
- Alternative A: the cleanup first and the fix last, on the smaller crate.
  - Structure: the end state is the same.
  - Evolution: the fix sits on top of the stack, where a revert does not conflict. In the chosen order, reverting Task_1 would conflict with Task_2, Task_10 and Task_11, which touch the same files.
  - Verification:
    - The slice-end AFTER at c0ed9e21 could not be reused as BEFORE. A new BEFORE would be needed at the cleanup tip, run twice.
    - The behavior-free proof would compare against pre-fix readings at the cleanup tip, and the fix's AFTER would be a separate run on top. The number of runs is the same, but one of them would be spent on a BEFORE that the chosen order gets for free.
  - Operation and safety: the same.
  - Human: a reviewer of the fix reads it against less code.
- Alternative B: one task per verdict item, about thirty tasks.
  - Structure: the same end state.
  - Evolution: narrower diffs, but the `lib.rs` spine grows from five serial tasks to about twelve.
  - Verification: the same single final proof, but a bisect over about thirty tips instead of eleven, and about twice the stack churn.
  - Human: many PRs are only one to five lines.
- Alternative C: a behavior-free proof at every stacked tip, eleven runs.
  - Structure, evolution, operation, human and safety: the same.
  - Verification: each task's equality is shown directly, at about eleven times the run cost. Tasks 5, 7, 9 and 12 change no compiled library code, so their runs could show nothing. The chosen single proof covers only the integrated final tip. It accepts review and gates as the evidence for intermediate tips, in place of the audit text's per-task measurement (ruling 77).
- Why chosen: fix-first reuses a reading that already exists and makes the one final proof a re-confirmation of the AFTER the prospective slice needs. The revert cost it adds is theoretical: the fix is a one-line default and a validator move, and nothing reverts it without re-measuring anyway. Grouping by file puts two workers in parallel wherever the files allow and keeps each PR to one reason.
- Measurement (orchestrator rule, Measurement Of Behavior Changes):
  - Instrument: the companion's situated suite, all families, through the calibration binary `calibrate_cue_floors`. It uses the companion commit that produced the consolidation slice-end AFTER, pinned in the dispatch brief.
  - BEFORE: that AFTER, at c0ed9e21.
  - AFTER: Task_1's final reviewed tip, identifiers opposed to time in both orders, run twice.
  - Falsified if either of these happens:
    - In a participant-heavy family, a retrieval gains more than one occasion per ubiquitous participant. A ubiquitous participant is a present participant whose participant-occasion budget was chosen at its minimum at BEFORE.
    - Any other family moves, in returned ids, sections, order, scores, trace rank or fanout utilization.
  - Observed fields: the falsifier and the final proof compare only what the instrument records, which is the list above. The calibrator does not observe `memory_scenes` or `admitted_by`; library tests cover them.
  - The prospective BEFORE is Task_1's AFTER. That is valid because the integrated final tip reads the same, and the prospective slice builds on that final tip. It holds for the families this instrument records; a new prospective family keeps its own baseline precondition.
  - Fit: ADR-I-0036 (participant experiences at one to five); the orchestrator rule, Measurement Of Behavior Changes; common.md, Compatibility Policy.

## Compatibility stance
- surface:
  - `RetrievalTelemetry` loses these fields, and the public types go with them:
    - the five `configured_*` fields;
    - `query_embedding_dimension`;
    - `selectivity` (`SelectivityTelemetry`);
    - `section_pressure` (`SectionPressureSummary`);
    - every `graph_expansion` counter except `bounded_failure_count` (`GraphExpansionBoundedFailureSummary`).
    `RetrievalRationale` loses `vector_candidate_count`.
  - `RetrievalGraphLimits` loses `timeout_ms` and `failure_mode`. `GraphFailureMode` and `GraphExpansionBoundedReason::Timeout` are removed.
  - `CustomError` loses `LowInformationCoOccurrence`, `EnvFileNotFound`, `EnvLoadError`, `MissingEpisodicField`, `InvalidSemanticMemory` and `UnsupportedOperation`. It also loses `SerializationError` if the compiler finds no producer.
  - Other error surfaces:
    - `ConfigValidationReason` loses `PairedKeyViolation`.
    - `EmbeddingError` loses `MissingApiKey`.
    - `CandidateValidationIssue` loses `MemoryLinkRejectedByAdmissionPolicy`.
    - `IoErrorKind` becomes a string inside `VectorDatabaseErrorKind::Io` and `RetrievalStatsStoreError`.
  - Removed public items:
    - `CandidateCount`, and `RememberDiagnostics.candidate_counts`;
    - the builders `with_schema_version`, `graph_iri`, `with_repair_needed`, `with_warning` and `unmaintained_objects`;
    - the constants `EPISODIC_MEMORY_SCHEMA_VERSION` and `CURRENT_SCHEMA_VERSION`;
    - `RetrievalStatsHealthFailMode` and its config key;
    - the `test-fixtures` feature.
  - Settings getters other than `get_embedding_vector_size` and `get_selectivity_*` become crate-private.
  - The stats SQLite table `entity_edge_index` loses `first_seen_at` and `last_seen_at`.
- stance: break
- justification: the library has no external consumers (common.md, Compatibility Policy). The one locatable consumer is the companion evaluation repository. Its production code reads none of these surfaces: it reads kept telemetry fields by name or JSON path, `DEFAULT_SCHEMA_VERSION` and the `get_selectivity_*` getters (grep at evals 1b8f02a). Its test modules in `adapter.rs` and `pipeline.rs` read `configured_object_types`, `CURRENT_SCHEMA_VERSION` and `get_oxigraph_path`, and the evals dependency rewrites them. The calibrator serializes `RetrievalGraphLimits::default()` into its report header (`calibrate_cue_floors.rs:1065`), which then loses two fields; that is a deletion. Stats stores are regenerated on every evals run. There is no migration and no shim.

## Context (workspace)
- Line numbers are at 4808cf07. c0ed9e21 adds one rustdoc line to `src/api/types/retrieval.rs`, so line numbers after it in that file are one higher.
- Inputs:
  - the orchestrator's verdicts on the four value audits (Library section and Constraints; binding);
  - the audits `audit-lib-prod` (LP items) and `audit-lib-tests` (LT items), with their file and line evidence;
  - the rulings log, items 66 and 77.
- Design records: ADR-I-0006 (bounded expansion; its timeout is only an example control), ADR-I-0007 (schema versioning), ADR-I-0008 (the conservative stats fallback is required, not a switch), ADR-I-0011, ADR-I-0019 (companion coordination), ADR-I-0024 (the vector verdict sits beside the returned candidate count), ADR-I-0029 (opaque backend detail), ADR-I-0036 (participant fanout one to five). No deviation.
- Neighbour plans: the consolidation plan (its slice-end measurement at c0ed9e21 provides BEFORE); the prospective plan (it stacks on this plan's tip and uses Task_1's AFTER as its BEFORE); the evals cleanup plan (it carries the evals dependency).

## Open Questions (max 3)
- None. The verdicts are binding, and the drafting decisions are in the Decision Log.

## Assumptions
- A1: c0ed9e21 differs from 4808cf07 by one rustdoc line in `src/api/types/retrieval.rs`. Source: `git diff --stat`. The dispatch brief checks it again.
- A2: The consolidation slice-end AFTER at c0ed9e21 finishes, and its verdict does not reopen the design, before Task_1's AFTER is read. Source: project state. If the verdict reopens the design, this plan waits and is re-based.
- A3: Production reaches the policy only through `composition.rs:176-183`, which always passes all three budgets from Settings. So the settings default is the one production uses. Source: audit LP1; `get_retrieval_fanout_budgets`. Checked by Task_1's regression test.
- A4: The companion's production code uses no surface this plan removes. Source: grep of the companion at 1b8f02a. Checked by the companion build against every task's tip, and by the companion `cargo test` before the stack merges.
- A5: The situated suite writes no I/O error, so changing `IoErrorKind` to a string changes no reading. Checked by the final-tip proof.
- A6: Task_5, Task_7, Task_9 and Task_12 change no compiled library code, so they need no run of their own. A bisect still uses the integrated wave tips they close, because those contain the production tasks merged beside them. Their review checks that no non-test code changed.

## Tasks

### Task_1: Production recalls the latest occasion of a ubiquitous participant
- type: impl
- effort: 3 worker-hours
- worker: cm-worker
- owns:
  - src/policy/retrieval_selectivity.rs
  - src/config/app_settings.rs
  - src/composition.rs
  - src/usecases/retrieve.rs (the `#[cfg(test)]` pipeline constructor's policy construction only)
  - src/memory/retrieval_floor_tests.rs (the policy construction at :1409 only)
  - tests/retrieval_state_tests.rs
- depends_on: []
- description: |
  LP1. Branch from c0ed9e21 (see Integration).

  The defect: commit 3d7a734e raised the participant minimum only in the policy's own table (`retrieval_selectivity.rs:475-497`). Production never reads that table, because `composition.rs:179` passes all three budgets from Settings, whose default is still 0 to 5 (`app_settings.rs:514-533`).

  The fix:
  - Keep one default table, the one in the settings, with participant experiences at one to five.
  - Validate once, in the settings (`app_settings.rs:554-583`). Delete the policy's re-validation of alpha, gamma, min not above max and unsupported targets (`retrieval_selectivity.rs:27-70`, `:313-359`), along with its duplicate key names and its own default table.
  - Make the policy constructor infallible, and delete `new` and `try_new`.
  - The `#[cfg(test)]` `from_parts` and the test pipeline constructor build the policy from the same settings table.

  Add a facade regression test through `CharacterMemory::new_with_embedding_provider`, with default settings. A participant present in every experience brings their latest occasion. Show it failing at c0ed9e21.
- acceptance:
  - One default fanout table exists, owned by the settings. The policy holds no default of its own and validates nothing. Its constructor returns `Self`.
  - The facade regression test fails at c0ed9e21 and passes at the tip, with healthy, populated statistics, not the missing or unhealthy fallback. The report shows both runs.
  - Every changed test expectation is listed with before and after.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test (passed, failed and ignored counts before and after in the report); pre-commit run --all-files; the regression test run at c0ed9e21 and recorded"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer: one table, one validator, every production and test construction path traced to it, and the regression test reaching production composition"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement, run by evals-worker2 at Task_1's final reviewed tip, with the orchestrator pinning the library checkout. Instrument: the situated suite, all families, through calibrate_cue_floors, on the companion commit and inputs that produced the consolidation slice-end AFTER. BEFORE is that AFTER at c0ed9e21, reused and not re-run. AFTER uses identifiers opposed to time in both orders and is run twice, and the two runs must be byte-identical apart from the recorded commit. Before AFTER is read, the participant-heavy families are listed from their definitions. Falsified if a participant-heavy family gains more than one occasion per ubiquitous participant in any retrieval (a present participant whose participant-occasion budget was chosen at its minimum at BEFORE), or if any other family differs from BEFORE in returned ids, sections, order, scores, trace rank or fanout utilization. These are the fields the instrument records: it does not observe memory_scenes or admitted_by, and library tests cover those. The reading reports, per participant-heavy retrieval, the occasions gained and every memory displaced. The AFTER is kept as the reference for the final-tip behavior-free proof and as the prospective BEFORE."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Verdict: the BEFORE and AFTER numbers, a paragraph on what they mean for the character (a ubiquitous companion's latest occasion comes back without crowding out what is known about them) and a verdict, direction confirmed or questioned, in this plan's Decision Log. A questioned direction reopens the design, not the acceptance line."

### Task_2: Retrieval telemetry reports only what someone reads
- type: impl
- effort: 4 worker-hours
- worker: cm-worker
- owns:
  - src/api/types/retrieval.rs
  - src/api/types.rs
  - src/lib.rs
  - src/usecases/retrieve.rs
  - src/policy/retrieval_selectivity.rs
  - src/memory/retrieval_scene_tests/descriptions.rs
  - src/memory/retrieval_scene_tests/presence.rs
  - src/memory.rs (the assertion at :740 only)
  - tests/retrieval_state_tests.rs
  - tests/retrieval_guardrails_tests.rs
- depends_on: [Task_1]
- description: |
  LP3 and LT14.

  Delete:
  - the five `configured_*` echoes of the caller's own context;
  - `query_embedding_dimension`;
  - `rationale.vector_candidate_count`;
  - `section_pressure` and `SectionPressureSummary`;
  - the whole of `SelectivityTelemetry` and the plan's telemetry in `retrieval_selectivity.rs:95` and `:450-460`;
  - all of `GraphExpansionTelemetry` except `bounded_failure_count`, together with `GraphExpansionBoundedFailureSummary`.

  Locations are in audit LP3.

  Keep:
  - every field companion production reads: `unique_graph_root_candidate_count`, `selected_graph_root_count`, `graph_root_omission_count`, `vector_recall_completeness`, the rationale omission counts and reasons, `graph_verified_count` and `summary`;
  - the JSON path `graph_expansion.bounded_failure_count`, exactly;
  - `returned_vector_candidate_count` (ADR-I-0024).

  `SectionAssignment.rank` comes from the section pressure's `included_count` today (`retrieve.rs:1063-1064`). A private per-section count keeps it one-based.

  Test readers of deleted fields do one of two things. If they checked counts the trace already carries, they read the trace (the selectivity decisions, the section assignments). If they pinned only an echo, they are deleted.

  LT14: `topic_only_applies_section_limits_and_preserves_query_text` (`descriptions.rs:4-74`) asserts two things: per-section count limits, and that the query text is trimmed and embedded once. It no longer asserts an exact JSON projection of ids.
- acceptance:
  - Every field above is gone from the public types and `lib.rs`. Every kept field and path is unchanged in name and value.
  - Every selected member keeps its one-based rank within its section. The report shows a retrieval with several sections, before and after.
  - The report lists each rewritten or deleted test with the field it read.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer: the kept list matches the companion's production reads and JSON paths; nothing kept that the verdict deletes; section rank preserved"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip: with the library checkout pinned to the tip by the orchestrator, the companion builds calibrate_cue_floors on the plan's instrument commit (A4). The behavior-free proof itself runs once, at Task_12's tip."

### Task_3: The link admission guard that could not fire is gone
- type: impl
- effort: 1.5 worker-hours
- worker: cm-worker2
- owns:
  - src/usecases/link.rs
  - src/usecases.rs
  - src/usecases/write_planning.rs
  - src/domain/write_validation.rs
  - src/errors.rs
- depends_on: [Task_1]
- description: |
  LP2 and LT2. Delete:
  - `admit_link`, `LinkAdmissionEvidence` and `LinkAdmissionDecision`, including the `#[cfg(test)]` evidence variant `LowSelectivityCoOccurrenceOnly` (`link.rs:10-21`, `:66-79`, `:125-140`), and their re-export (`usecases.rs:11`);
  - the write-planning branch (`write_planning.rs:1563-1567`);
  - `CustomError::LowInformationCoOccurrence` (`errors.rs:423-429`) and `CandidateValidationIssue::MemoryLinkRejectedByAdmissionPolicy` (`write_validation.rs:99`);
  - the two guard tests (`link.rs:363-436`).

  ADR-I-0011's decision holds because nothing creates co-occurrence links (ruling 77). No record changes.
- acceptance:
  - None of the named items exists. The link path admits every link it admitted at the parent.
  - The report lists both deleted tests.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer2: every production link path passed ExplicitCallerIntent, so nothing that was accepted changes"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_4: Graph expansion keeps one utilization count and no clockless timeout
- type: impl
- effort: 4 worker-hours
- worker: cm-worker
- owns:
  - src/policy/graph_expansion.rs
  - src/domain/retrieval.rs
  - src/domain.rs
  - src/lib.rs
  - src/api/types/retrieval.rs
  - src/ports/graph_authority.rs
  - src/usecases/retrieve.rs
  - src/adapters/oxigraph/embedded.rs
  - src/adapters/oxigraph/shared.rs (the live `fail_if_closed(query.failure_policy.mode, …)` reader at :78 and its import; partial-result node-limit evidence stays)
  - src/adapters/oxigraph/tests.rs
  - src/memory/retrieval_scene_tests/evidence.rs
  - tests/public_facade_tests.rs
  - tests/retrieval_guardrails_tests.rs
  - tests/retrieval_lifecycle_evidence_tests.rs
  - tests/retrieval_resolution_tests.rs
  - tests/retrieval_time_tests.rs
- depends_on: [Task_2]
- description: |
  LP10: record fanout utilization only in the adapter's pass (`bounded_incident_link_refs`). Delete the counting in the policy's `bounded_expansion` and its trace-mode-dependent `cap + 1` (`graph_expansion.rs:443`, `:568-584`, `:612-630`, `:681`). The adapter already forces `TraceMode::Disabled` before calling the policy (`embedded.rs:395`); if that line only existed to skip the policy's counting, remove it too and report it. The `fanout_utilization_*` policy tests observed only the policy's output. Keep or move coverage so that one adapter-level test pins utilization.

  LP11 (ruling 66): delete these, together with their test-only uses (listed in audit LP11 and found by grep):
  - `timeout_ms`, both the public field and the port's, and the `Some(0)` check (`graph_expansion.rs:320-335`);
  - the `Timeout` reason, both the public `GraphExpansionBoundedReason::Timeout` and the port's;
  - `GraphFailureMode::FailClosed` and the fail-closed branch (`graph_expansion.rs:891-901`). The then one-value `GraphFailureMode` and both `failure_mode` fields go with it (planner-added).
- acceptance:
  - Utilization is counted in one place. The production trace's `fanout_utilization` is unchanged, which the final-tip proof shows. The utilization helpers `bounded_incident_link_refs` uses stay, and outcomes are the same with the trace on and off.
  - `timeout_ms`, `Timeout`, `FailClosed`, `GraphFailureMode` and `failure_mode` no longer exist. Graph limits default as before for every remaining field.
  - The report lists each deleted test with the mode or reason it exercised, and each mechanical edit (for example `timeout_ms = None` lines) by file.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer: the embedded adapter's utilization path is the one production uses; no remaining caller could set a zero timeout or fail-closed; the bounded-failure path is otherwise unchanged"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_5: Scene and facade tests stop repeating each other
- type: test
- effort: 3 worker-hours
- worker: cm-worker2
- owns:
  - src/memory.rs (test code only)
  - src/memory/scene_tests.rs
  - tests/scene_surface_tests.rs
  - src/adapters/qdrant/store.rs (the test module only)
  - README.md (the Running tests paragraph only)
- depends_on: [Task_2]
- description: |
  LT3: delete the four service tests that never run:
  - the scene-surface service variant, with its service branches (`scene_surface_tests.rs:82-107`, `:264`, `:371-387`);
  - `service_wrong_width_upsert_rejects_with_collection_mismatch` (`store.rs:636-650`);
  - the idle-gap canary (`store.rs:946-1017`);
  - the live equal-score tie test (`store.rs:1188-1243`).
  Then make the README's count of service-reaching tests (`README.md:385`) true.

  LT6: delete `episode_and_thread_with_the_same_id_keep_distinct_vectors_on_write_and_forget` (`memory.rs:746-838`) and `remember_keeps_distinct_vectors_for_episode_and_observation_with_the_same_uuid` (`scene_tests.rs:256-295`).

  LT7: delete `setting_words_recall_the_episode_when_the_summary_does_not_name_the_place` (`scene_tests.rs:373-419`).

  LT8: delete `every_scene_embedding_is_prepared_before_the_write_turn` (`scene_surface_tests.rs:389-431`).

  LT11: delete the `injected_facade_*` smoke tests (`memory.rs:201-221`, `:702-744`). Fold the four injected lifecycle tests (`memory.rs:860-1141`) into at most one facade test of correct and forget.

  LT15: the integration tests assert batching, surface kinds and that keys are never embedded. The text format is left to `embedding_surface.rs`. Rename `scene_without_words_keeps_the_exact_legacy_embedding_text` after what it checks.
- acceptance:
  - Every named test is deleted or folded as stated. The report maps each one to the test that already covers its behavior.
  - The README's count of tests that reach a Qdrant service matches the tests that do.
  - No non-test code changes, and no remaining assertion is weakened.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer2: each deletion's behavior is covered where the report says; the folded lifecycle test keeps every distinct assertion"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_6: Error types say only what something produces
- type: impl
- effort: 2 worker-hours
- worker: cm-worker
- owns:
  - src/errors.rs
  - src/lib.rs
  - src/adapters/qdrant/store.rs
  - src/adapters/qdrant_edge/mod.rs
  - src/adapters/stats/sqlite.rs
- depends_on: [Task_3, Task_4, Task_5]
- description: |
  LP5: delete `EnvFileNotFound`, `EnvLoadError`, `MissingEpisodicField`, `InvalidSemanticMemory`, `UnsupportedOperation` and `ConfigValidationReason::PairedKeyViolation`. Remove `SerializationError` and let the compiler decide: if an implicit `?` conversion needs it, restore it and report the producer.

  LP12: `IoErrorKind` becomes a string where it is held. Delete the enum, its mapping and its two tests (`errors.rs:27-117`, `:538-554`). First, search the companion's committed evidence for `io_kind` (planner-added), and report what it found.
- acceptance:
  - The named variants and `IoErrorKind` no longer exist. The report states whether `SerializationError` stayed, and why.
  - I/O failures keep the standard kind's text in the string.
  - The report states the result of the evidence search. If a sealed artifact carries `io_kind`, stop and report before changing the shape.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer: no producer of a deleted variant exists in src, tests or the companion; the string keeps the classification text"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_7: Recency and range tests pin behavior once, by order
- type: test
- effort: 2 worker-hours
- worker: cm-worker2
- owns:
  - tests/public_facade_tests.rs
  - tests/retrieval_time_tests/recency.rs
- depends_on: [Task_4]
- description: |
  LT5: delete `public_correct_and_forget_hide_stale_memories_from_normal_retrieval` (`public_facade_tests.rs:2227-2335`).

  LT9: delete these four tests:
  - `explicit_recency_floor_reserves_under_topic_pressure` (`recency.rs:647-679`);
  - `recency_preserves_original_saturated_topic` (`recency.rs:189-199`);
  - `a_recency_reservation_uses_the_latest_occasion` (`public_facade_tests.rs:1849-1877`);
  - `a_range_replaces_the_recency_window` (`public_facade_tests.rs:1606-1645`).

  LT13: the exact `final_score` floats and exact root lists (`recency.rs:98-102`, `:168-177`, `:256-276`) become checks of relative order and thresholds.
- acceptance:
  - The four LT9 tests and the LT5 test are gone. The report names the test that covers each one.
  - No exact float score or exact id list stays pinned where order or admission is the behavior. Every order and threshold those numbers implied is still asserted.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer2: relative checks still fail on the regressions the exact values caught"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_8: Write-side builders and constants have callers or are gone
- type: impl
- effort: 2.5 worker-hours
- worker: cm-worker
- owns:
  - src/lib.rs
  - src/api/types.rs
  - src/api/types/write_plan.rs
  - src/api/types/lifecycle.rs
  - src/usecases/remember.rs
  - src/usecases/write_planning.rs
  - src/usecases/correct_forget.rs (the test at :2371 only)
  - src/domain/write_validation.rs
  - src/domain.rs
  - src/domain/schema.rs
  - src/domain/tests.rs
- depends_on: [Task_6]
- description: |
  LP6: delete `RememberDiagnostics.candidate_counts`, `CandidateCount` and `with_candidate_count`, together with the copy in `remember.rs:122-125`.

  LP13: delete these builders; the test uses at `write_plan.rs:855` and `correct_forget.rs:2371` read the data directly:
  - `RememberPlanDefaults::with_schema_version` and `graph_iri`;
  - `RememberDiagnostics::with_repair_needed`;
  - `CandidateValidation::with_warning`;
  - `VectorMaintenanceFailure::unmaintained_objects`.

  Keep one schema-version constant, `DEFAULT_SCHEMA_VERSION`, and delete `EPISODIC_MEMORY_SCHEMA_VERSION` and `CURRENT_SCHEMA_VERSION`.

  LT20: the fixture builders in `domain/tests.rs:77-171` reuse `test_support::representative_fixtures`, and the constant tautology test (`:228-233`) is deleted.
- acceptance:
  - None of the named items exists. `DEFAULT_SCHEMA_VERSION` is the only schema-version constant.
  - `domain/tests.rs` builds its fixtures from `test_support`. The report counts the lines before and after.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer: no production, test or companion-production caller of a deleted item"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_9: Pipeline, write and helper tests keep what earns its place
- type: test
- effort: 2.5 worker-hours
- worker: cm-worker2
- owns:
  - src/usecases/retrieve.rs (test code only)
  - src/test_support.rs
  - src/memory/retrieval_floor_tests.rs
  - tests/support/
  - tests/write_planning_tests.rs
  - tests/public_facade_tests.rs
  - tests/retrieval_guardrails_tests.rs
  - tests/retrieval_last_interaction_tests.rs
  - tests/retrieval_scope_tests.rs
- depends_on: [Task_4, Task_7]
- description: |
  LT10: delete `retrieve_pipeline_expands_embedded_vector_candidate_with_embedded_oxigraph` (`retrieve.rs:3216-3299`) and `retrieve_pipeline_after_persistent_reopen_uses_graph_authority_filters` (`:3301-3372`).

  LT16: delete `core_commit_flow_works_in_in_memory_graph_mode` (`write_planning_tests.rs:146-165`) only. Its persistent twin stays.

  LT17: remove the `about_derived_memory_fanout` parameter from `try_setup_persistent_character_memory` (`tests/support/persistent.rs:9-34`). All seven callers pass `None`.

  LT18: delete `deterministic_embedder_uses_explicit_text_without_external_services` (`test_support.rs:616-641`). Keep the drop-cleanup test.

  LT22: fold `a_large_activity_shares_roots_with_the_topic` (`retrieval_floor_tests.rs:529-565`) into the next test as a default-floors case.
- acceptance:
  - Every named test is deleted or folded as stated. The report names each deleted test's coverage.
  - The persistent setup helper has no fanout parameter.
  - No non-test code changes.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer2: coverage claims hold; the folded floor case keeps its default-floors assertion"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_10: Configuration, the OpenAI provider and dev-dependencies carry nothing unused
- type: impl
- effort: 3 worker-hours
- worker: cm-worker
- owns:
  - src/config/app_settings.rs
  - src/config.rs
  - src/config/embedding_provider_settings.rs
  - src/adapters/openai/embedding_provider.rs
  - src/composition.rs
  - src/errors.rs
  - src/lib.rs
  - src/api/embedding.rs
  - src/models/vector.rs
  - src/usecases/vector_indexing.rs (the test at :231 only)
  - tests/vector_port_contract_tests.rs
  - Cargo.toml
  - Cargo.lock
- depends_on: [Task_8]
- description: |
  LP4: delete `RetrievalStatsHealthFailMode`, its setting, its getter and its re-export. A failed SQLite open always falls back to `InMemoryRetrievalStatsStore::unhealthy` (ADR-I-0008). The fallback test stops setting the removed key, and its name drops "configured".

  LP8: `OpenAIEmbeddingProvider::new(api_key, model)`. Delete `EmbeddingProviderSettings`, the second blank-key check with `EmbeddingError::MissingApiKey` and its test, and the `println!` (`embedding_provider.rs:46`). Preflight's `require_openai_api_key` stays the one check. The transport trait stays.

  LP14: settings getters become `pub(crate)`, except `get_embedding_vector_size` and `get_selectivity_*`. A getter left with no caller goes, as the lint requires.

  LT4: remove the `test-fixtures` feature, the crate's dev-dependency on itself, `zero_norm_record_fixture` and its re-export. Inline the values at `vector_port_contract_tests.rs:458`, `:517` and `vector_indexing.rs:231`.

  LT21: `MockEmbeddingProvider` becomes a small struct in the composition tests, and `mockall` is dropped.
- acceptance:
  - The named types, the setting, the feature, the self dev-dependency and `mockall` are gone. `cargo tree` shows no `mockall`.
  - The OpenAI provider has one constructor with no second key check and prints nothing.
  - Outside the crate, only `get_embedding_vector_size` and `get_selectivity_*` remain callable. The report lists every getter's new visibility, or its deletion.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files; cargo tree -e dev | grep mockall returns nothing"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer: preflight still rejects a blank OpenAI key before construction; the unhealthy fallback is unconditional; no secret-returning getter is public"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_11: The stats port has one implementation and records only what it reads
- type: impl
- effort: 3 worker-hours
- worker: cm-worker2
- owns:
  - src/adapters/stats.rs
  - src/adapters/stats/noop.rs
  - src/adapters/stats/sqlite.rs
  - src/adapters/stats/in_memory.rs
  - src/ports/retrieval_stats.rs
  - src/policy/retrieval_selectivity.rs (the test module only)
  - src/usecases/retrieve.rs (the `#[cfg(test)]` constructor only)
  - src/usecases/remember.rs
  - src/usecases/link.rs
  - src/usecases/correct_forget.rs
- depends_on: [Task_8, Task_9]
- description: |
  LP7 and LT1: delete `NoopRetrievalStatsStore` (`stats/noop.rs`) and its re-export. The four `#[cfg(test)]` pipeline constructors each create their own store with `Box::leak(Box::new(InMemoryRetrievalStatsStore::new()))` (planner-added). This is test-only: each test gets a fresh store, nothing is shared, and the 66 call sites are unchanged. A `ponytail:` comment at the leak names the ceiling (per-test memory until the process exits) and the upgrade path (fixture-owned stores, if test memory ever matters). They are at `retrieve.rs:58-67`, `remember.rs:37-45`, `link.rs:35-41` and `correct_forget.rs:53-61`.

  LP9: delete `first_seen_at` and `last_seen_at` from the stats edge, the two SQLite columns, the MIN/MAX upsert and the in-memory max (locations in audit LP9).

  LT19: keep the unit test `stats_projection.rs:403`. Keep one pipeline test per entry point, checking only that a stats failure reaches the outcome (link, correct and forget, remember). Delete the exact cause-list duplicates (`link.rs:466-560` with `DualFailingStatsStore`, `correct_forget.rs:2106-2155` and `:2218`, `remember.rs:880`) where another test already covers them.
- acceptance:
  - The stats port has two implementations, SQLite and in-memory, plus recording and failure doubles in tests. Each implicit test constructor leaks its own fresh store, with the `ponytail:` comment. Production composition's shared store and tests that share a store on purpose are unchanged.
  - Stats edges carry no timestamps anywhere. A fresh SQLite store initializes, records and reopens.
  - Each write entry point keeps one test that a stats failure reaches its outcome. The report lists the deleted tests.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer2: every test built with the pipeline constructors still sees empty, healthy statistics; no read path used the timestamps"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"

### Task_12: One test double per port wraps the real in-memory adapter
- type: test
- effort: 6 worker-hours
- worker: cm-worker2
- owns:
  - src/test_support.rs
  - the `#[cfg(test)]` modules of src/usecases/remember.rs, src/usecases/correct_forget.rs, src/usecases/link.rs, src/usecases/retrieve.rs, src/usecases/stats_projection.rs, src/usecases/vector_indexing.rs, src/memory.rs and src/policy/retrieval_selectivity.rs
  - src/memory/write_turn_tests.rs
- depends_on: [Task_10, Task_11]
- description: |
  LT12, for the graph, vector and stats ports. `test_support` gets one wrapper per port, each around the real adapter's in-memory mode, with hooks to record calls, to fail on a chosen method, and to gate.

  Every existing recording, failure and gating double moves onto its wrapper. The audit lists them: the graph doubles at `remember.rs:1184`, `correct_forget.rs:3400`, `link.rs:569`, `write_turn_tests.rs:550` and `retrieve.rs:3480`, plus the stats and vector doubles found by grepping for `impl ... for` on the three ports. Stubs that diverge from the real store delegate to it, for example `query_anniversaries` returning an empty list. `ErrorGraphStore` becomes a failure hook.

  Embedder doubles are out of scope (see Scope). No production code changes.
- acceptance:
  - Under `#[cfg(test)]`, each of the three ports has exactly one wrapper type, in `test_support`, and no other test implementation. The report lists each former double and the hooks that replace it, with line counts before and after.
  - Every test that used a divergent stub passes against delegation. Any expectation that changed because the stub diverged is listed with before and after.
  - The test count is unchanged, and no assertion is weakened. `VectorRecallOverride`'s corruption and metadata controls, and `TemporaryVectorCandidateStore`'s cleanup, survive in the wrappers.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts before and after; pre-commit run --all-files"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer: edits confined to test code; hooks preserve each double's recorded, failed and gated behavior; no second port implementation"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Companion build against this task's tip, as in Task_2's build item"
  - kind: command
    required: true
    owner: orchestrator
    detail: "The plan's one behavior-free proof, run by evals-worker2 at the integrated final tip (Task_12's): the situated suite, all families, through calibrate_cue_floors, on the plan's instrument commit, identifiers opposed to time in both orders. It replaces the audit verdicts' per-task measurement requirement (ruling 77), covers the final tip only, and claims nothing about intermediate tips. It is compared with Task_1's AFTER after stripping the recorded commit and an exact allowlist of the field paths the stack deletes, listed from the task reports (for example, the calibrator header's native_graph_limits loses timeout_ms and failure_mode). The reports must then be byte-identical on what the instrument records: returned ids, sections, order, scores, trace rank and fanout utilization, with the kept counters. memory_scenes and admitted_by are not observed; library tests cover them. If the reports differ, the orchestrator bisects over the retained integrated wave tips and runs only there. The cause is named, the task it came from reopens, and the final comparison is rerun after the correction."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Completion value audit by a Claude Tier A agent over the whole stack: nothing the verdicts deleted is still there, nothing was added that the verdicts did not ask for, and every planner-added requirement earned its place"

## Cross-repo dependency: the companion's test modules stop reading removed surfaces
This change is not work of this plan. The evals cleanup plan carries it on its branch, and it is named here so this stack does not merge without it.
- owns (companion repository):
  - crates/cmem-eval/src/adapter.rs (the `#[cfg(test)]` module only)
  - crates/cmem-eval-runner/src/pipeline.rs (the `#[cfg(test)]` module only)
- depends_on: none. It compiles against c0ed9e21 and against this plan's tip, so it can land in either order.
- what:
  - The checks that read `rationale.telemetry.configured_object_types` (`adapter.rs:2900`, `:3239`; `pipeline.rs:2513`) either observe some other way that the adapter passes its configured object types, or are deleted if the echo was all they tested.
  - `CURRENT_SCHEMA_VERSION` becomes `DEFAULT_SCHEMA_VERSION` (`adapter.rs:2773`, `:5355`).
  - The persistence-path test (`adapter.rs:3039`) observes the path without `Settings::get_oxigraph_path`.
- acceptance: the companion's `cargo test` passes against both c0ed9e21 and this plan's final tip.
- gate for this plan: before the stack merges, the orchestrator pins the companion checkout to the evals cleanup branch tip that contains this change, and runs the companion's `cargo test` against this plan's final tip.

### Task_13: A person reached through an experience is hub-checked on the links actually traversed
- type: impl
- effort: 3 worker-hours
- worker: cm-worker
- owns:
  - src/policy/graph_expansion.rs
  - src/adapters/oxigraph/shared.rs (the bounded incident-link call path only)
  - the test modules of those two files, and one integration test file for the regression
- depends_on: [Task_4]
- description: |
  Found during the obligations instrument work (evidence in the evals repository at `.agent-work/worker/obligations-hub-limit-evidence/`; investigation at `.agent-work/reviewer/hub-limit-investigation.md`). When an experience or belief found by topic reaches a person who is not a root, both `bounded_incident_link_refs` and the materialized `bounded_expansion_plan` count every eligible incident link against the hub limit (64) before applying the fanout bound (16). A person the character knows well, with more than 64 links, then records HubLimit and recall comes back degraded, although the traversal read at most 16 links. This contradicts ruling 66 and `docs/design/database/graph_schema_design.md:131`.

  The fix: apply the existing fanout selection first, then hub-check and, if needed, truncate the selected prefix, in both places. Keep the deterministic order, the root-only overrides, subject and participant eligibility, and trace-evidence exclusion. No new selector, API or setting, and no raised limit. A selected prefix genuinely over the hub limit (a caller fanout above 64) still records the bound.
- acceptance:
  - A regression test through the real adapter: an experience root reaching a person with 71 incident links returns the same objects and links as with 16, and records no HubLimit. It fails at the Task_4 tip and passes after.
  - A test where the selected prefix really exceeds the hub limit still records it.
  - The trace's fanout utilization is unchanged apart from the removed false bound.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts; pre-commit; the regression shown failing at the Task_4 tip"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer, who found the defect: both paths fixed, the root-only protections unchanged, a real over-limit prefix still bounded"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Measurement, run by an evals worker: the recorded failing query from the obligations evidence (evals b4a8ab5 fixture, library before and after this task) records zero bounded failures after and returns the same pack as before apart from the degraded flag. The final-tip behavior-free proof then allows exactly this change: bounded-failure counts may only fall."

### Task_14: Graph expansion reads only the links and occasions it selected
- type: impl
- effort: 3 worker-hours
- worker: cm-worker2
- owns:
  - src/adapters/oxigraph/shared.rs (link hydration and the occasion-metadata call path)
  - src/adapters/oxigraph/embedded.rs (only if a wrapper becomes unused)
  - src/adapters/oxigraph/tests.rs (the read-counter tests)
- depends_on: [Task_13]
- description: |
  Found by the consolidation slice-end timing: retrieval is about 1.59 times slower after consolidation Task_3 (interleaved, same load, and about 1.6 times where results are unchanged); the other steps are about 1.0. The investigation (`.agent-work/reviewer/observedin-read-cost-investigation.md`) found two costs on the expansion path:
  - `hydrate_links_by_id_sets_from_store` hydrates every link in the store and then filters to the selected IDs, once per expanded root. This predates Task_3, which multiplied it by adding a traversal level. It contradicts ruling 67 (read compact columns, decide in Rust, hydrate only what is kept).
  - Task_3's occasion-metadata call re-queries memories whose occasions the same expansion already fetched: three SELECTs per expanded root, where one is new information.

  The fix:
  - Hydrate by ID with the existing `hydrate_links_by_ids_from_store`, keeping the endpoint and lifecycle filters, and delete the full-store wrappers that become unused.
  - Filter the metadata request through the per-expansion occasion map, extend the map with fetched rows, and derive future memories from the whole map.
  - No cache across requests, no port change, ObservedIn and the as-of cut unchanged.
- acceptance:
  - A real-adapter read-counter test: one episode, its observation and their ObservedIn link. RDF quads read equal the expected named graphs and stay identical after 200 unrelated episodes are written; the unfixed code reads 66, then 3,066.
  - The SELECT count for the one-root keyless fixture is 8, down from 9. With both objects as vector roots it is 14, down from 16.
  - Every complete retrieval outcome in the existing suites is unchanged.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test with counts; pre-commit; the read-counter tests shown failing at the Task_13 tip"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review by cm-reviewer2, who found the costs: reads bounded by the selected IDs, the future filter derived from the whole map, outcomes unchanged"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Timing, notebook grade: the interleaved harness on the fixed 50-query keyless and time subset, consolidation parent 67d6735 against this task's tip. Direction confirmed if the ratio returns to about 1.0 or below; if a residual stays above 1.1, attribute it before closing. The final-tip behavior-free proof covers the outputs."

## Integration
- Branch: `feature/2026-09-24/value-audit-cleanup`, cut from `origin/feature/2026-09-23/consolidation-records` at c0ed9e21. It holds this plan. Each task is one PR, stacked in task-number order on the one below (`gh stack link`).
- Lanes:
  - cm-worker takes Task_1, then the `lib.rs` spine: Task_2, Task_4, Task_6, Task_8 and Task_10.
  - cm-worker2 takes the second lane: Task_3, Task_5, Task_7, Task_9, Task_11 and Task_12.
  - cm-reviewer reviews cm-worker's tasks, and cm-reviewer2 reviews cm-worker2's, apart from Task_12, which cm-reviewer takes to balance the load. No reviewer reviews its own diff.
- Both tasks in a wave branch from the previous wave's top tip. Nothing is ever rebased. Once the lower-numbered task is approved, the higher-numbered task plain-merges that approved tip into its own branch, and its PR's base is the lower-numbered task's branch. Their owns do not overlap, so the merge is clean. The orchestrator then runs `cargo check` and `cargo test` at the merged tip, and the companion build.
- Measurement, builds and the proof: evals-worker2 runs them on one companion instrument commit, pinned in the dispatch brief, the same commit that produced the consolidation slice-end AFTER. The orchestrator alone pins the library checkout the calibrator links against, between runs. The runs are:
  - BEFORE at c0ed9e21, reused;
  - AFTER at Task_1's tip;
  - a companion build at each later task's tip;
  - one behavior-free proof at the integrated final tip (Task_12's), compared with Task_1's AFTER.
  The wave tips are retained for bisecting a final mismatch, and the final comparison is rerun after any correction.
- Workers commit locally and do not push. A PR opens only after its Tier D approval (Push Sequencing).
- The prospective implementation stacks on Task_12's tip, and Task_1's AFTER is its BEFORE.
- The companion's test-module change lands on the evals cleanup branch (cross-repo dependency above). The evals cleanup stacks on the prospective-obligations line.

## Task Waves (explicit parallel dispatch sets)
- Wave 1: [Task_1]
- Wave 2 (parallel): [Task_2, Task_3]
- Wave 3 (parallel): [Task_4, Task_5]
- Wave 4 (parallel): [Task_6, Task_7]
- Wave 5 (parallel): [Task_8, Task_9]
- Wave 6 (parallel): [Task_10, Task_11]
- Wave 7: [Task_12]

Task_1 lands and is measured before Wave 2 is dispatched, so the final-tip proof has its reference. In each wave the two owns sets do not overlap, and every file shared across waves is ordered by depends_on. Task_12 lands after every task that deletes tests or constructors in the files its doubles live in, which are Task_3, Task_5, Task_8, Task_9, Task_10 and Task_11.

## Rollback / Safety
- Each task is one PR and reverts on its own, in reverse stack order. Reverting Task_1 restores the participant minimum of zero; it conflicts with Task_2, Task_10 and Task_11, which later touch the same files, and it needs a new measurement.
- Task_11 drops two columns from the stats SQLite table, and there is no migration. A stats store written before Task_11 fails its first edge write and turns selectivity off until it is recreated (the marker that never clears, ruling 77). No consumer holds such a store, the companion regenerates its stores on every run, and the README already says that a superseded stats store must be recreated.

## Progress Log (append-only)

- None yet.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-24 Decision: the library cleanup follows ruling 77 (value-audit cleanup, 2026-09-24).
  - Trigger / new insight: four read-only audits (library and evals, production and tests) found one live defect and, in the library, about 1,100 lines removable at high confidence. The participant fanout minimum is zero in production and one everywhere else, because a fix changed only the policy's default table, which production never reads. The orchestrator's verdicts on the audits are binding, and ruling 77 records the rule applied: keep what guards the public API, stored data, the honesty of a measurement, or what a decision record requires; delete what guards against states no realistic actor produces, what nothing calls or reads, and tests that repeat another test's behavior.
  - Plan delta (what changed): new plan. Task_1 is the fanout fix, the only behavior change, measured against the consolidation slice-end AFTER. Eleven behavior-free tasks follow, the last of them the test-double consolidation, and one proof at the final tip shows that they read the same as Task_1's AFTER. The companion's test-module change is a cross-repo dependency.
  - Tradeoffs considered: see the Design alternatives.
  - User approval: under the v0.2 standing authorizations; ruling 77 was made on the decider's request.
  - Record proposed: none.
- 2026-09-24 Decision: choices made while drafting.
  - Trigger / new insight: while drafting, it became clear that most public-item removals share `src/lib.rs` and rustfmt reflows its re-export list, that the companion's `adapter.rs` test module also uses `CURRENT_SCHEMA_VERSION` and `get_oxigraph_path`, that a static in-memory stats store would keep writes the no-op store discarded, and that `SectionAssignment.rank` is computed from the section pressure summary LP3 deletes.
  - Plan delta (what changed):
    - Tasks are grouped by file, with a serial `lib.rs` spine and a disjoint second lane.
    - The companion dependency covers the two extra uses in the same module.
    - Each test pipeline constructor gets its own store.
    - The section rank keeps a private count.
    - `GraphFailureMode` goes whole.
    - Embedder doubles stay outside LT12.
    - ADR-I-0007 stays unedited.
  - Tradeoffs considered: keeping `get_oxigraph_path` public to spare the companion one test rewrite was rejected, because the verdict limits the public getters to three and the companion's production does not use it.
  - User approval: decided under the standing instruction, and logged for presentation.
  - Record proposed: none.
- 2026-09-24 Decision: revised after cm-reviewer's REQUEST_CHANGES on 619ac45f (R1 to R4), as the coordinator ruled.
  - Trigger / new insight:
    - R1: `shared.rs:78` is a live reader of the failure mode that Task_4 deletes, and Task_4 did not own it.
    - R2: a constructor-local store cannot be borrowed into the pipeline it returns.
    - R3: the calibrator does not observe `memory_scenes` or `admitted_by`.
    - R4: equality at the final tip does not prove equality at intermediate tips, because changes can cancel.
  - Plan delta (what changed):
    - Task_4 owns `src/adapters/oxigraph/shared.rs`.
    - Each test pipeline constructor leaks a fresh `InMemoryRetrievalStatsStore`, with a `ponytail:` comment naming the ceiling and the upgrade path. The 66 call sites are unchanged.
    - The falsifier and the proof compare only the fields the instrument records (returned ids, sections, order, scores, trace rank, fanout utilization). `admitted_by` is dropped from the falsifier, and library tests cover it.
    - The single proof is scoped to the integrated final tip and replaces the audit text's per-task measurement requirement under ruling 77. Intermediate tips rest on their own review and gates. The wave tips are retained for a bisect, with the final comparison rerun after any correction. The prospective-BEFORE sentence is restated to match.
    - Worker-brief notes folded in as one-line acceptance fixes:
      - Task_1's regression test uses healthy, populated statistics;
      - Task_4 keeps the utilization helpers `bounded_incident_link_refs` uses, and keeps trace-on/off parity;
      - Task_12 keeps `VectorRecallOverride`'s controls and `TemporaryVectorCandidateStore`'s cleanup.
      The other notes stay for the dispatch briefs.
  - Tradeoffs considered:
    - Caller-owned stores (the reviewer's suggestion) were rejected, because they would change 66 test call sites for no behavioral gain.
    - Adding an `admitted_by` observer to the instrument was rejected, because it would need a new BEFORE for a field that library tests already pin.
  - User approval: ruled by the coordinator.
  - Record proposed: none.

- 2026-09-24 Task_13 added: a hub-limit defect contrary to ruling 66.
  Trigger / new insight: the obligations instrument at evals b4a8ab5 failed closed with seven HubLimit failures. cm-reviewer traced them to a person reached through an experience, not as a root: the hub limit counted the person's whole eligible neighbourhood (71 links) before the fanout bound (16) chose what to traverse. Ruling 66's fix covered only people who are roots.
  - Plan delta (what changed): Task_13 follows Task_4, which edits the same file. It is a behavior change on reported health (the false degraded flag goes, recall content is unchanged), so it carries its own small measurement, and the final-tip proof allows bounded-failure counts only to fall. Wave 2 went out before Task_1's measurement by the orchestrator's ruling, because the final-tip proof covers it and no Wave 2 task depends on the fanout default.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none; the design doc already states the contract.

- 2026-09-24 Task_14 added, and Task_13's allowance made precise.
  Trigger / new insight: consolidation Task_3 made retrieval about 1.59 times slower, which interleaved timing confirmed and attributed to that step alone. The investigation found a pre-existing whole-store link hydration per expanded root (contrary to ruling 67), which Task_3 multiplied, plus redundant occasion-metadata queries. Task_13's review showed that selecting before the hub check also returns an occasion that a person root's aboutness links used to starve out.
  - Plan delta (what changed): Task_14 fixes both read costs without changing outcomes, and is timed against the consolidation parent. The final-tip proof allows bounded-failure counts only to fall, and memories only to be recovered where a false bound starved them; each recovery is reported, never masked. Unchanged-output claims for Task_13 are scoped to the recorded non-root and obligations fixtures.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.

## Notes
- Risks:
  - At a full cap, Task_1's added occasion can displace another memory in participant-heavy families. The falsifier counts only gains, so the reading reports displacements, and the verdict judges them for the character.
  - The per-task companion build links the calibrator against every task tip. If a task removes something companion production uses, that build fails. A4 says no such use exists, and the failure would be loud.
- Edge cases:
  - A participant with no occasions still brings none. The minimum of one applies only to occasions that exist.
  - With stats missing or unhealthy, the conservative fallback already chose one occasion (README, retrieval statistics). Task_1 makes the healthy low-selectivity path agree with it.
- For the decider's batch at closeout: ADR-I-0007 line 44 names `EPISODIC_MEMORY_SCHEMA_VERSION`, `CURRENT_SCHEMA_VERSION` and `DEFAULT_SCHEMA_VERSION`, and after Task_8 only the last exists. The record is accepted, and its sentence goes on to say that the exact names may vary, so this plan leaves it unedited.
