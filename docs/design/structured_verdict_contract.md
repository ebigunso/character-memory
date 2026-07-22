# Structured Verdict Contract

- status: draft (Task_1 of docs/coding-agent/plans/active/structured-verdict-observability-plan.md)
- date: 2026-07-21
- scope: CharacterMemory (CM) + CharacterMemoryEvals (CME), designed once, implemented in Task_2/Task_3 (CM) and Task_4 (CME)

## Design intent

Structure is authoritative; prose is a projection derived centrally, exactly once, at the type that owns the structure.
This is the contract already established as-built by `RememberDiagnostics.validations` (commit 13bc56f): structured rows are the outcome field, messages are regenerated from them in one place.
Philosophy grounding: retrieval results include score components and rationale (project_philosophy.md 9.2), and unexplained recall is a failure mode (10) — a trace row that cannot name its evidence is unexplained recall in structured clothing.
ADR-I-0012 grounding: `remember()` is a convenience over prepare/validate/commit, so the two paths must expose the same structured outcome, never asymmetric projections.
Repo constraints honored throughout: no backwards-compat shims (fields are retyped in place), no roadmap version labels in production identifiers, sealed CME artifacts stay byte-identical and machine-readable solely through the bounded legacy 1.0.0 dispatch defined in section 6.

Fixed rulings this design operates under: CME report schema bumps to 2.0.0 as a clean break; the dormant governance/reconciliation slice (R2-11) is deleted and F13 dies with it; vector-port findings go to the v0.1.6 phase; lifecycle-mode redesign goes to v0.2; the R2-01 ledger goes to v0.2+ with only a narrow `query_links_by_ids` port method landing now.

## 1. Typed validation-issue vocabulary (F2, composing with as-built F1)

As-built base (pull-forward PR, design around, do not re-decide): `CandidateValidation`, `CandidateValidationStatus`, and `MemoryCandidateKind` live in a domain write-validation module, relocated unchanged from `api::types::write_plan`, because `CustomError::WritePlanValidationRejected { validations: Vec<CandidateValidation> }` must respect ADR-I-0018 (errors import only domain).
Their public path is the flat crate root, re-exported from domain via `lib.rs`; `api::types` holds no re-export aliases and imports from domain like any other layer.
The F2 issue enums below therefore also live in that domain write-validation module and evolve the relocated types in place.

`CandidateValidation.errors`/`warnings` stay two channels (the as-built dual-channel shape that `refresh_validation_warning_messages` and `CandidateValidationStatus` already key off), but their elements become a typed issue enum instead of `String`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateValidation {
    pub candidate_index: usize,
    pub candidate_kind: MemoryCandidateKind,
    pub status: CandidateValidationStatus,
    pub errors: Vec<CandidateValidationIssue>,
    pub warnings: Vec<CandidateValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CandidateValidationIssue {
    #[error("observation echoes episode content {echo_surface:?}; matching episodes: {matching_episode_ids:?}")]
    DuplicateObservationEcho { echo_surface: String, matching_episode_ids: Vec<MemoryId> },
    #[error("candidate references unknown object {referenced:?}")]
    UnknownObjectRef { referenced: MemoryObjectRef },
    // ... remaining variants enumerated in Task_2 from the closed producer vocabulary in src/usecases/write_planning.rs
}
```

Task_2 enumerates the full variant set from the write-planning validator's producer sites; every MemoryId, ObjectType, or surface currently interpolated into the prose becomes a structured field on its variant.
The issue enum references only domain types (`MemoryId`, `ObjectType`, and the domain-side object ref from section 5's R2-08 unification), which is exactly what the domain location permits under ADR-I-0018.
Prose derivation: each variant's `Display` (thiserror) is the single projection; `refresh_validation_warning_messages` keeps deriving `write_plan_validation_warning` messages centrally, now via `issue.to_string()`, so the as-built central-projection invariant is unchanged.
Composition with the as-built rejection error: `CustomError::WritePlanValidationRejected` carries `Vec<CandidateValidation>` as-is, so retyping the row's inner fields flows through the rejection path with no further error-shape change.
Tests assert variants and structured fields; the only string assertions permitted are on the derived projection's code/severity, per the as-built convention.

## 2. Typed error story (F7, F8, F9, F10, F11, F12)

One Display-derivation convention, applied uniformly: every structured error payload implements `Display` once via thiserror on the payload type; `CustomError` variants wrap the payload with `#[error(transparent)]` or a `#[source]` reference; no call site ever `format!`s payload fields into a message, and no test asserts substrings of error prose.

One layering convention, following the write-validation relocation precedent: every structured error payload type lives in `domain` (or the `errors` module itself), never in api/ports/adapters, because ADR-I-0018 permits `errors` to import only `domain`.
Producers above construct these lower-layer types — an adapter building a `CollectionCompatibilityError` is the normal direction; the type living in the adapter would invert it.
Concretely this relocates `GraphExpansionBoundedFailureTrace` (and its reason enum) from api to domain (they live in api/types/retrieval.rs:291,438), and `LifecycleDtoValidationError` from `api::types::lifecycle` to domain, both unchanged, with the flat crate-root export convention and no api/ports re-export aliases; the degraded-success trace and the draft validator reference the domain types thereafter, which keeps the F7 cannot-diverge-by-construction property intact.

`CustomError` changes:

```rust
pub enum CustomError {
    #[error(transparent)]
    ConfigValidation(#[from] ConfigValidationError),                    // F12, replaces ConfigParseError(String) for validation failures

    #[error(transparent)]
    CollectionIncompatible(#[from] CollectionCompatibilityError),       // F11, replaces the DatabaseError(String) producers in qdrant store

    #[error("graph expansion bounded by retrieval policy: {0}")]
    GraphExpansionBounded(GraphExpansionBoundedFailureTrace),           // F7, replaces { reason: String, location: String }

    #[error(transparent)]
    LifecycleDraftInvalid(#[from] LifecycleDtoValidationError),         // F8

    // existing variants unchanged unless named here
}
```

F7: the fail-closed error reuses `GraphExpansionBoundedFailureTrace { reason: GraphExpansionBoundedReason, at: Option<MemoryObjectRef> }` — the exact struct the degraded-success trace already carries — so the two channels cannot diverge by construction.
F8: `CharacterMemory::correct`/`forget` call the canonical `draft.validate()` and surface it as `LifecycleDraftInvalid`; the duplicated re-worded validator inside `correct_forget.rs` is deleted.
Policy-level rejections that are not draft-shape errors (the "unsupported in this lifecycle chunk" branches) get a separate typed `LifecyclePolicyUnsupported { knob: LifecyclePolicyKnob }` variant naming the rejected knob as an enum; redesigning the mode surface itself is R2-02 and stays deferred to v0.2.
F9: per-operation vector-maintenance failure items replace the joined string.

```rust
pub struct VectorMaintenanceFailure {
    pub failures: Vec<VectorMaintenanceFailureItem>,
}
pub struct VectorMaintenanceFailureItem {
    pub operation: VectorMaintenanceOperation,   // Delete | Upsert
    pub objects: Vec<MemoryObjectRef>,
    pub error: VectorDatabaseError,
}
```

`unmaintained_objects` becomes a derived accessor (refs, not bare IDs) (union over failure items), not a stored second copy.
Outcome-embedded-record convention: outcome fields such as `VectorMaintenanceFailure`/`VectorMaintenanceFailureItem` live in `api` (the outcome layer) and may reference `domain`/`errors` types, since api-to-errors is an allowed dependency direction; only payloads embedded in `CustomError` variants must themselves be `domain`/`errors`-resident.
F12: `ConfigValidationError { keys: Vec<&'static str>, reason: ConfigValidationReason }` where the reason enum covers the producer vocabulary in `app_settings.rs` (missing value, out-of-domain value with expected/actual, paired-key violation naming both keys); config tests assert keys and reason variants, never message tokens.
F11: `CollectionCompatibilityError { collection: String, mismatch: CollectionMismatch }` with `CollectionMismatch::{VectorSize { expected, actual }, Distance { expected, actual }, MissingNamedVector { name }, ...}` enumerated from the qdrant compatibility checks.
F10: `RememberDiagnostic.code: String` becomes `code: RememberDiagnosticCode` (`#[non_exhaustive]`, `snake_case` serde so the wire tokens are unchanged); `VectorDatabaseError.kind: String` becomes `kind: VectorDatabaseErrorKind` enumerated from the qdrant producer sites; `status: Option<String>` becomes `status: Option<TransportStatus>`, an enum of the known transport classes with an `Unrecognized(String)` carrier for genuinely unknown upstream codes — preserving raw upstream data is data retention, not a stringly vocabulary.

## 3. Trace identity additions (F3, F4, F5, F6, R2-10)

F3: a public `VectorSurface` enum (mirroring the internal `VectorCandidateMatch.surface` vocabulary) is added, and `VectorCandidateTrace` gains `pub surface: VectorSurface`, so canonicalization over (id, type, surface) is explainable from the trace.
F4: `GraphRelationTrace` gains `pub link_id: MemoryId`; wider link metadata (confidence, rationale) is not added this phase — link_id is the correlation key to the authoritative record and everything else is retrievable through it.
F5: `SectionAssignment.reason: Option<String>` becomes a required typed reason with score components and intended section.

```rust
pub struct SectionAssignment {
    pub object: MemoryObjectRef,
    pub section: ContextPackSection,
    pub rank: Option<usize>,
    pub reason: SectionAssignmentReason,
    pub rationale_categories: Vec<RationaleCategory>,
}

#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SectionAssignmentReason {
    Selected { scores: SectionScoreComponents },
    OmittedByLimit { intended_section: ContextPackSection, scores: SectionScoreComponents },
    OmittedNonActiveThread { thread_status: ThreadStatus },
}

pub struct SectionScoreComponents {
    pub final_score: f32,
    pub vector_score: Option<f32>,                          // the EFFECTIVE vector input used by scoring
    pub vector_score_source: Option<SectionVectorScoreSource>, // DirectMatch | DerivedFromRoot { root_score } (Amendment 11)
    pub graph_score: Option<f32>,
    pub salience_score: Option<f32>,
}
```

`ContextPackSection::Omitted` remains the value of `section` for omitted rows; the erased intent lives in `intended_section`, so no consumer of the section field changes meaning.
The retrieval tests that currently assert exact reason strings migrate to variant/field assertions.
F6: `RetrievalTelemetry` gains `configured_object_types: Vec<ObjectType>` and `configured_lifecycle_policy: RetrievalLifecyclePolicy`, completing the configured-inputs echo; query text and current context stay omitted (privacy drop, ruled intentional in the sweep).
R2-10 (resolved here): the fail-closed-vs-partial graph failure boolean becomes a typed `GraphFailureMode` enum riding the F7 work; the internal `collect_trace`/`apply_selectivity_overrides` boolean plumbing is typed as internal `TraceMode`/`RootFanoutMode` enums during Task_2's touch of those paths — internal-only, no public contract change.

## 4. Postcondition ownership (R2-09)

Ruling: each port owns its stated postconditions; upper layers never repair lower-layer output, because silent repair masks adapter bugs and duplicates policy.
Two enforcement mechanisms, chosen per site by cost:

Canonical candidates (evidence A): `VectorCandidateStore::search_candidates` returns a `CanonicalCandidates` newtype whose only constructor performs canonicalization and de-duplication, so adapters cannot return a non-canonical value and the pipeline cannot re-do it.
The `RetrievePipeline` re-canonicalization pass (retrieve.rs:95-97) is deleted.
The `CanonicalCandidates` newtype is expected to survive the v0.1.6 vector-port redesign or be absorbed into that port's result envelope, not to be a throwaway shape.

Lifecycle-filtered expansion (evidence B): graph expansion is the single owner of lifecycle filtering as declared by `GraphExpansionQuery`; conformance is enforced by port contract tests run against both the Oxigraph adapter and the fake, not by a runtime wrapper — a runtime re-check is exactly the repair pass being removed, and a violation is an adapter defect that should fail tests, not be silently corrected.
Deleted use-case repair passes: the per-object lifecycle re-evaluation (retrieve.rs:458-485), the contradictory-omission removal (492-504), and the decision de-duplication (505-519) in `RetrieveAssembly`.

SPARQL post-filtering (evidence C): `select_objects` query semantics own ref/id/type predicates and ordering; the post-filter and re-sort/dedup in `sparql_selectors.rs:48-87,379-383` are deleted, with the query semantics pinned by adapter tests.

## 5. CM consolidations this phase (R2-06, R2-07, R2-08, R2-13, R2-15; R2-01 narrow slice; R2-12 shape)

R2-06 vector-indexing service: one internal service owns map-to-embedding-input, embed_batch, count verification, positional zip, and upsert, returning typed per-record outcomes; `remember` and `correct_forget` both call it and their duplicated orchestration is deleted.

```rust
pub(crate) struct VectorIndexingService { /* embedder + vector store */ }
impl VectorIndexingService {
    pub(crate) fn index(&self, records: Vec<VectorRecord>) -> VectorIndexingOutcome; // per-record MemoryObjectRef outcomes + typed failure
}
```

The ID-vs-ref divergence between the two callers unifies on `MemoryObjectRef` outcomes with IDs derivable.
Degradation causes are typed, not prose (ruled): `VectorIndexingFailure.error_message` and the `error_message` fields of `RepairMarker::{VectorIndex, StatsUpdate}` become `cause: VectorDatabaseError` carrying the F10-typed kind, with per-record refs supplied by this service's outcome unification; `RepairMarker::StatsUpdate` and `StatsUpdateFailure` follow the same typed-cause shape but with their own stats-side cause type (a typed graph/stats-store error enumerated from the stats producer sites, not `VectorDatabaseError` — a stats-projection failure is not a vector-database error), so no repair or indexing record carries free-form prose.
R2-07 stats projection service: one internal service owns endpoint hydration against `GraphAuthorityStore`, `record_stats_after_write`, and failure health marking; it accepts committed objects/links from all four write paths — `remember`, `link`, `correct`, and `forget` — and every consumer propagates the returned `StatsUpdateStatus` to its public outcome (Amendment 12); `object_type_has_stats_state` moves to a single home on the projection (its two copies are deleted).
R2-08 central identity/order, split across tasks (ruled): the `MemoryObjectRef` unification below is Task_2's first chunk (the F2/F7 payloads reference it), while the mechanical replacement of the ~9 copied `object_type_rank`/`object_identity`/`stable_node_key`/sort helpers happens in Task_3 after the ObjectRef move lands; `MemoryObject` exposes `id()`/`object_type()` accessors and `ObjectType` (plus `RelationType`, `RetentionState` restrictive rank) get canonical stable ordering as documented `Ord`/rank methods in `domain`.
Single neutral ObjectRef: the `{ object_type, id }` pair moves to one serde-deriving struct in `domain`, keeping the public name `MemoryObjectRef` and following the export convention set by the write-validation relocation (flat crate-root re-export from domain via `lib.rs`, no `api::types` alias, api imports from domain); the internal `GraphObjectRef` (reversed field order) plus its conversion are deleted.
R2-13 payload schema manifest: one typed `QdrantPayloadSchema` manifest (field name, kind, indexed flag) is the single source consumed by payload serialization, index creation, and contract tests; the test-only constants become manifest entries; `record_type` is dropped from the manifest (redundant with `object_type`) with existing stored payloads tolerated unread; the dual `embedding_text`/`content_text` column decision is explicitly deferred to the v0.1.6 vector-port design, which owns the payload read contract.
R2-15: `RememberPipelineOutcome`, the internal `VectorIndexingFailure` mirror, and the two `From` impls in `composition.rs` are deleted; the use case constructs the public `RememberOutcome` directly (it already imports the API types, and ADR-I-0018 permits use-case dependency on the api boundary).
R2-01 narrow slice (Task_2): `GraphAuthorityStore` gains `query_links_by_ids(&[MemoryId]) -> Result<Vec<MemoryLink>>`; the collision check in `remember` uses it instead of scanning `list_diagnostic_links()`; the remaining TOCTOU window is documented at the call site; the atomic conditional-upsert/ledger design is deferred to v0.2+.
R2-12 shape (Task_3): speculative vector query/source-reference APIs are deleted or moved to `cfg(test)`, and `GraphObjectQuery` becomes an enum (`ByRefs`/`ByIds`/`ByTypes`) eliminating the invalid mixed-vector states.

## 6. CME contract (r1#1-5, r2#1, r2#3, r2#4, r2#6, r2#7, r2#8, r2#9)

### Typed core DTO vocabularies (r2#1)

`cmem-eval-core` defines its own closed enums for the vocabularies currently modeled as `String`: entity type, thread status, derived type, stability, relation, endpoint/item object type, candidate status, and retrieval decision/count scope.
Serde rename contracts mirror CM's wire tokens (`snake_case`), and the CM adapter converts via exhaustive `From`/`TryFrom` so vocabulary drift is a compile error instead of a silent reparse; the serde_json round-trip shim, Debug-derived output values, and the camel-to-snake converter are deleted.
Sealed-artifact decoding stays bounded exactly as-built for the fixture/frozen-store families that own the `#[serde(default)]`/`Option` tolerance (e.g. telemetry fields, memory_adapter.rs:165-213); sealed result rows and continuity traces are instead served by the legacy 1.0.0 dispatch defined under the schema section below, and neither path is migrated to the typed vocabularies.

### Report schema 2.0.0 (r1#1, r1#2, r1#3, r1#4, r1#5)

`RESULT_SCHEMA_VERSION` (crates/cmem-eval-core/src/results.rs:12) becomes `"2.0.0"`, a clean break per the ruling, and `CONTINUITY_TRACE_SCHEMA_VERSION` (crates/cmem-eval-continuity/src/driver.rs:25) and `CONTINUITY_REPORT_SCHEMA_VERSION` (crates/cmem-eval-continuity/src/report.rs:22) bump to `"2.0.0"` in the same clean break.
Reader factual baseline (convention: every compatibility claim cites its reader file:line): the only result-row reader is `read_jsonl` (results.rs:249-263), whose `validate_row_schema` (results.rs:265-275) hard-rejects any `schema_version` other than the current constant; `validate_summary_schema` (results.rs:277-285) does the same for summaries, and the continuity trace loader rejects non-current versions at driver.rs:120-126 — there is no tolerant 1.0.0 result reader today, so a bare constant bump would orphan sealed evidence.
Ruling: a bounded, clearly-marked legacy 1.0.0 read dispatch is retained for result rows and continuity traces, existing solely so sealed register-cited evidence stays machine-verifiable under the Compatibility Policy's sealed-artifact exemption; it is what keeps the kept telemetry field tolerance (`#[serde(default)]` fields at crates/cmem-eval-core/src/memory_adapter.rs:165-213) reachable.
2.0.0 readers are strict fail-closed; sealed artifacts are never rewritten; new evidence is generated fresh.
Rows carry write and lifecycle verdicts so infrastructure degradation can never masquerade as memory quality:

```rust
pub struct PerQuestionResult {
    // existing identity/retrieval fields...
    pub write_outcomes: Vec<WriteOutcomeRecord>,        // r1#1, r1#2
    pub lifecycle_outcomes: Vec<LifecycleOutcomeRecord>, // r1#3
    pub context_text: String,                            // canonical evaluated text, see r2#7
    pub metrics: MetricsRecord,                          // r1#5, typed admission
}

pub struct WriteOutcomeRecord {
    pub operation: WriteOperationKind,                   // typed ingest | explicit commit
    pub validations: Vec<CandidateValidationRecord>,     // typed mirror of CM rows
    pub repair_markers: Vec<RepairMarkerRecord>,         // typed causes (F10-kind), Debug-flattening deleted
    pub vector_indexing_failure: Option<VectorIndexingFailureRecord>, // typed cause, not error_message prose
    pub stats_update_failure: Option<StatsUpdateFailureRecord>,       // typed cause, not error_message prose
}

pub struct LifecycleOutcomeRecord {
    pub vector_maintenance_failures: Vec<VectorMaintenanceFailureRecord>, // per-operation, mirrors CM F9 shape
    pub stats_update_status: StatsUpdateStatusRecord,                     // Amendment 12: lifecycle stats degradation reaches rows
    pub warnings: Vec<LifecycleWarningRecord>,
    pub requested_targets: Vec<ObjectRefRecord>,
}
```

Path asymmetry (r1#2) dies at the source: `commit_typed_drafts` returns the write outcome record instead of `Result<()>`, and `CommitWriteResult` carries the same diagnostics/repair-marker structure as the typed-ingest path, so both routes feed identical rows.
Run summaries aggregate degradation counts (degraded write count, lifecycle maintenance failure count, repair-marker counts by kind) so a green summary certifies clean infrastructure, not just metric averages.
Telemetry widening (r1#4): the core `RetrievalTelemetry` DTO gains the missing counters and reason summaries (query embedding dimension, returned candidate count, graph-expansion counts with bounded-failure reasons, section pressure) plus the new CM identity fields from section 3 as they land in Task_2; full per-decision traces remain trace-gated, not row-resident.
Metrics-shape admission (r1#5): `metrics` stops being `serde_json::Value`; 2.0.0 rows carry a typed metrics map, and readers fail closed on a non-conforming metrics shape instead of `filter_map(as_object)` silently dropping rows, so `num_questions` always equals the rows that contributed to aggregates; the 1.0.0 sealed-reader path keeps its tolerance.

### Runtime and construction consolidations (r2#3, r2#4, r2#6, r2#7, r2#8, r2#9)

r2#3: a typed per-scenario `EmbeddingRuntimeBinding` enum (`Controllable { fixture, dimension_policy }` | `Frozen { store }` | `Live { provider, model }`) owns both create and restart reconstruction; the `mixed` pseudo-provider string and the cloned-config rewriting in the runner are deleted; suite admission policy (which bindings a dataset accepts) is a separate check on the binding type, not a provider-string match.
r2#4: `BenchmarkRunConfig.dataset` becomes a typed `DatasetId`, and one `DatasetDescriptor` registry (kind, name validation, metric-family requirements) serves both run and summarize; 2.0.0 rows persist the dataset kind so summarize never redispatches on strings; the split name validators collapse into the registry.
r2#6: a typed `RetrievalSurfacePolicy` with explicit named per-section budgets replaces the `include_*` boolean fanout and the hidden 12/8 constants; one exhaustive conversion produces CM's `RetrievalContext`, and dataset presets are values of the policy type, not code branches.
r2#7: `RetrievedContextPack::from_ranked_items` becomes the only constructor, computing `context_text` and char/word counts from one canonical renderer so items, evaluated text, and counts cannot disagree; the mock and live renderers become renderer strategies behind that constructor; the official exporter reads the persisted 2.0.0 `context_text` instead of re-rendering, and when pointed at a sealed 1.0.0 run it uses the legacy 1.0.0 read dispatch read-only.
r2#8: one shared `OpenAiEmbeddingClient` batch API serves both live query (batch of one) and offline generation, with shared response validation (model, index coverage, cardinality) and caller-supplied retry policy, replacing the two divergent implementations.
r2#9: the copied atomic staged-file-replacement helper is extracted into one shared fs utility used by the identity registry and the frozen store.

## 7. Finding-disposition table

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

## Sealed-reader constraints (restated for Task_4 acceptance)

Sealed 1.0.0 evidence artifacts are byte-identical before and after this phase, verified by the pre/post hash inventory in Task_4 validation.
Because the current readers hard-reject non-current versions (results.rs:265-275 and 277-285 for rows/summaries; driver.rs:120-126 for continuity traces), Task_4 adds the bounded legacy 1.0.0 read dispatch for result rows and continuity traces — clearly marked, existing solely for sealed register-cited evidence under the Compatibility Policy's sealed-artifact exemption — rather than any generalized version tolerance.
The fixture/frozen-store serde-default tolerance (memory_adapter.rs:165-213) is retained untouched and is reachable only through that dispatch for sealed result evidence.
2.0.0 readers fail closed on shape violations (metrics admission, unknown vocabulary outside the sealed path); no 2.0.0 writer or reader ever rewrites sealed evidence.
Every compatibility claim in Task_4's report must cite the reader file:line it holds for, per the convention established here.

## Appendix: Task assignment and file ownership

Sequencing rule (ruled): Task_3 depends on Task_2; the two CM tasks run sequentially in the shared checkout; Task_4 runs parallel with Task_3 (plan waves: Wave 2 = Task_2 alone, Wave 3 = Task_3 + Task_4).
File overlap between Task_2 and Task_3 is therefore acceptable; the only hard ordering constraint inside Task_2 is that the domain relocations land as its first chunk because every later payload references them.

Task_2 — CM structured verdicts, errors, and shared-file consolidations:
- First chunk: domain relocations — `MemoryObjectRef` unification (R2-08 ref half), `GraphExpansionBoundedFailureTrace` + reason enum (api to domain), `LifecycleDtoValidationError` (api to domain).
- Findings: F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, R2-01 narrow `query_links_by_ids` slice, R2-06, R2-07, R2-09, R2-10.
- Files: src/domain.rs (module), src/errors.rs, src/api/types/{write_plan.rs, retrieval.rs, lifecycle.rs, draft.rs}, src/usecases/{write_planning.rs, remember.rs, retrieve.rs, correct_forget.rs, link.rs}, src/policy/{graph_expansion.rs, retrieval_selectivity.rs}, src/ports/{graph_authority.rs, vector_candidate.rs, retrieval_stats.rs}, src/adapters/qdrant/store.rs, src/adapters/oxigraph/sparql_selectors.rs (R2-09 evidence C), src/config/app_settings.rs, src/composition.rs, src/memory.rs, and tests on those paths.

Task_3 — pure pruning and hygiene (after Task_2):
- Findings: R2-11 deletion (F13 dies with it), R2-12 including the src/models/vector/candidate_record.rs default-type helpers (~120-137, 136, 400) with a Tier D no-live-consumer verification before deletion, R2-13 manifest, R2-14, R2-15, R2-16, and the R2-08 mechanical rank/identity helper replacement.
- Files: src/usecases/reconciliation.rs (deleted), src/models/vector/{record.rs, candidate_record.rs}, src/ports/source_reference.rs, src/adapters/qdrant/payload.rs, src/adapters/oxigraph/{sparql_selectors.rs governance selectors, vocabulary.rs, embedded.rs}, the src/adapters.rs, src/models/vector.rs, src/policy.rs barrels, src/test_support.rs, src/composition.rs (R2-15 From-impl deletion), tests/support/**, and the helper-copy sites listed under R2-08.

Task_4 — CME (parallel with Task_3): all of section 6 plus the schema bumps and legacy dispatch from the sealed-reader section; files crates/** and configs/**, never frozen stores or sealed artifacts.

## Amendments (in-flight rulings during Task_2/3/4 implementation)

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
