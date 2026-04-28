# Plan: v0.1 Retrieve And ContinuityContextPack

- status: draft
- generated: 2026-04-28
- last_updated: 2026-04-28
- work_type: mixed

## Goal
- Implement v0.1 retrieval as continuity context, not generic top-k memory search.
- Add backend-free retrieval/context-pack DTOs, hardened graph expansion policy, vector-to-graph retrieval, lifecycle/currentness filtering, deterministic reranking/grouping, rationale, and an injected `CharacterMemory::retrieve` facade.
- Keep this chunk focused on retrieval assembly; do not implement correction, forget, raw storage, reflection scheduling, or a full belief/evidence subsystem.

## Definition of Done
- Backend-free retrieval request, policy, context pack, rationale, and optional trace types exist without Qdrant/Oxigraph/RDF types.
- Graph expansion supports the retrieval controls required before assembly depends on it: relation allowlists, fanout/hub limits, lifecycle/currentness filters, and explicit timeout/failure policy or a documented bounded substitute where timeouts are not practical.
- Vector search builds a natural-language query surface, searches selected v0.1 indexed object types, and treats vector payload filters as candidate hints rather than authority.
- Retrieve pipeline vector-searches, graph-expands/verifies, filters suppressed/deleted and non-current/superseded memories by default, deterministically reranks/dedupes, groups a `ContinuityContextPack`, and records rationale.
- `CharacterMemory::retrieve` is wired through injected v0.1 graph/vector/embedder parts without extending legacy `search_memories`.
- Required Rust checks, deterministic retrieval tests, embedded Oxigraph smoke, gated Qdrant candidate smoke, and Reviewer approval are complete, or blockers/waivers are explicitly recorded.

## Scope / Non-goals
- Scope:
  - Backend-free `RetrievalContext` or equivalent request DTO, retrieval policy, context-pack sections, rationale, and optional trace DTOs.
  - Graph expansion policy hardening before retrieve assembly.
  - Vector candidate search/filter contract updates needed for retrieval.
  - Internal retrieve pipeline over `VectorCandidateStore`, `GraphAuthorityStore`, and `MemoryEmbedder`.
  - Injected `CharacterMemory::retrieve` facade and deterministic facade tests.
  - Deterministic unit/fake-store tests plus embedded Oxigraph and gated Qdrant smoke evidence.
- Non-goals:
  - `correct`, `forget`, suppression/supersession lifecycle mutation APIs.
  - Production raw input storage or source material retrieval.
  - LLM extraction, summarization, reflection scheduling, or reranking via external services.
  - Replacing the old flat `search_memories` beyond keeping it isolated/deprecated.
  - UI/E2E/browser validation.

## Context (workspace)
- Related files/areas:
  - `src/api/types/domain.rs`
  - `src/api/types/draft.rs`
  - `src/api/types.rs`
  - `src/lib.rs`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal/infrastructures/external_services/**`
  - `src/internal/infrastructures/graph/**`
  - `tests/**`
  - `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap.md`
  - `docs/coding-agent/plans/active/v0-1-remember-and-link-pipelines-plan.md`
- Existing patterns or references:
  - Canonical v0.1 objects and lifecycle/currentness fields live in `src/api/types/domain.rs`.
  - Backend-free write drafts live in `src/api/types/draft.rs`; retrieval DTOs should follow the same public API boundary or a new direct file under `src/api/types/`.
  - `CharacterMemory::remember` and `CharacterMemory::link` now use injected graph/vector/embedder parts. `CharacterMemory::search_memories` remains deprecated legacy behavior and must not become the v0.1 retrieve path.
  - `VectorCandidateStore::search_candidates` currently supports embedding, limit, and object-type filters only.
  - `GraphExpansionQuery` currently supports root, depth, node count, and allowed object types only. The roadmap explicitly carries forward the need for relation allowlists, fanout/hub limits, lifecycle/currentness filters, and timeout/failure policy.
  - Qdrant payload relationship/lifecycle fields are filter hints; graph authority remains the source of truth for relationships, provenance, lifecycle, currentness, supersession, and expansion verification.
- Repo reference docs consulted:
  - `docs/design/roadmap-phases/v0_1_starter_episodic_memory.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/decisions/implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md`
  - `docs/decisions/implementation/ADR-I-0006-bounded-graph-expansion.md`
  - `docs/coding-agent/plans/active/v0-1-remember-and-link-pipelines-plan.md`

## Open Questions (max 3)
- Q1: Should the public request type be named `RetrievalContext`, `RetrieveRequest`, or `RetrieveDraft` for v0.1?
- Q2: Should optional retrieval trace live inside `ContinuityContextPack`, or be returned as a separate debug structure/outcome wrapper?
- Q3: Should timeout policy be represented in the provider-neutral graph contract now, or should this chunk use deterministic node/fanout limits and record timeout support as adapter-specific follow-up?

## Assumptions
- A1: Retrieval defaults exclude `RetentionState::Suppressed`, `RetentionState::Deleted`, and non-current/superseded derived memories unless policy explicitly includes them.
- A2: Default vector search covers `Episode`, `Observation`, `DerivedMemory`, `MemoryThread`, and `Entity`; links are graph-only.
- A3: Graph verification/filtering wins over stale vector payload hints. Stale or unresolved vector candidates should be omitted from the pack and optionally recorded in trace/rationale.
- A4: Retrieve facade should use injected v0.1 parts; default production constructor rewiring remains a separate migration cleanup.

## Tasks

### Task_1: Select retrieve API and policy boundary
- type: design
- owns:
  - `docs/coding-agent/plans/active/v0-1-retrieve-continuity-context-pack-plan.md`
- depends_on: []
- description: |
  Decide request/pack/trace naming, default retrieval policy, graph expansion failure behavior, and facade boundary before implementation edits.
- acceptance:
  - Decision records public retrieval request naming and whether trace is embedded or separate.
  - Decision records default lifecycle/currentness filtering and explicit policy knobs for including archived/non-current data.
  - Decision records graph expansion policy requirements: relation allowlists, fanout/hub limits, allowed object types, lifecycle/currentness filters, and timeout/failure behavior.
  - Decision records dependency direction: public DTOs stay backend-free; retrieve pipeline depends on provider-neutral graph/vector/embedder contracts; Qdrant/Oxigraph/RDF remain infrastructure details.
  - Decision records how stale vector candidates and graph expansion failures appear in rationale/trace.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Record retrieve API, policy, graph expansion, stale-candidate, and trace decisions before implementation edits."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review boundary decision against roadmap, ADR-I-0005, ADR-I-0006, and remember/link facade shape."

### Task_2: Add retrieval and context-pack DTOs
- type: impl
- owns:
  - `src/api/types.rs`
  - `src/api/types/**`
  - `src/lib.rs`
- depends_on: [Task_1]
- description: |
  Add backend-free retrieval request, policy, continuity context pack, rationale, section, and optional trace DTOs.
- acceptance:
  - DTOs represent query text/current context, candidate limits, graph limits, section limits, lifecycle policy, trace flag, and object-type defaults.
  - `ContinuityContextPack` groups active threads, relevant episodes, salient observations, derived memories, preferences, open loops, commitments, character signals, and rationale without backend-specific fields.
  - Rationale/trace DTOs can report vector candidate score, graph relation/proximity, lifecycle filter decisions, stale candidate omission, and final section assignment.
  - DTOs preserve source/raw references already present on canonical objects but do not introduce raw transcript storage.
  - Pure DTO tests cover serialization, defaults, and policy behavior.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: worker
    detail: "Run targeted retrieval DTO/context-pack tests added by this task."

### Task_3: Harden graph expansion policy
- type: impl
- owns:
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/infrastructures/graph.rs`
  - `src/internal/infrastructures/graph/**`
- depends_on: [Task_1]
- description: |
  Extend graph expansion query/adapter behavior with retrieval-safe bounds and filters before retrieve assembly depends on it.
- acceptance:
  - `GraphExpansionQuery` or companion policy supports relation-type allowlists, max fanout per node, hub limits, lifecycle/currentness filters, and explicit timeout/failure policy or documented deterministic substitute.
  - Fake graph and embedded Oxigraph behavior honor the new bounds deterministically.
  - Expansion excludes suppressed/deleted and non-current/superseded records by default when requested by policy.
  - Tests cover hub/high-fanout scenarios, relation allowlists, lifecycle/currentness filtering, and bounded failure behavior.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: worker
    detail: "Run targeted graph expansion policy tests, including embedded Oxigraph coverage."

### Task_4: Extend vector candidate search filters
- type: impl
- owns:
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/infrastructures/external_services.rs`
  - `src/internal/infrastructures/external_services/**`
- depends_on: [Task_1]
- description: |
  Add the vector search request/filter shape needed by retrieval while preserving graph authority for verification.
- acceptance:
  - Search request can express query embedding, candidate limit, object-type defaults, retention/currentness hint filters, and optional thread/entity/time hints where supported by Qdrant payloads.
  - Qdrant candidate adapter treats filters as candidate prefilters only; graph verification remains required before context-pack inclusion.
  - Deterministic vector fake and Qdrant mapping tests cover filter construction and candidate ordering.
  - Live Qdrant candidate smoke remains prerequisite-gated.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: worker
    detail: "Run targeted vector search/filter and Qdrant payload/adapter tests added by this task."

### Task_5: Implement internal retrieve pipeline
- type: impl
- owns:
  - `src/internal/repositories.rs`
  - `src/internal/repositories/**`
  - `src/internal/models/vector.rs`
  - `src/internal/models/vector/**`
- depends_on: [Task_2, Task_3, Task_4]
- description: |
  Implement vector-to-graph retrieval assembly with deterministic filtering, reranking, grouping, rationale, and optional trace.
- acceptance:
  - Pipeline embeds a natural-language query with `VectorSurface::Query`, searches vector candidates, graph-expands/verifies candidates, and omits unresolved/stale candidates from final pack by default.
  - Lifecycle/currentness filters exclude suppressed/deleted and non-current/superseded memories by default.
  - Reranking is deterministic with stable tie-breaks and documented score components.
  - Grouping populates the context-pack sections from canonical objects without raw transcript storage.
  - Tests cover vector-to-graph flow, stale candidate omission, lifecycle/currentness filtering, deterministic reranking, section limits, rationale, and optional trace.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: worker
    detail: "Run targeted fake-store retrieve pipeline tests added by this task."

### Task_6: Wire injected retrieve facade and legacy isolation
- type: impl
- owns:
  - `src/lib.rs`
  - `src/api/types.rs`
  - `src/api/types/**`
  - `tests/**`
  - `README.md`
- depends_on: [Task_5]
- description: |
  Expose `CharacterMemory::retrieve` through injected v0.1 parts without extending the legacy flat `search_memories` path.
- acceptance:
  - `CharacterMemory::retrieve` accepts the selected backend-free retrieval request and returns `ContinuityContextPack` or selected outcome wrapper.
  - Existing legacy `search_memories` remains isolated/deprecated and is not used by the v0.1 retrieve path.
  - Deterministic facade tests cover injected retrieve behavior and legacy search isolation.
  - README examples are updated only if the public runnable surface is production-usable enough to document without misleading users.
  - No correction/forget behavior is introduced.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"
  - kind: command
    required: true
    owner: worker
    detail: "Run targeted injected retrieve facade tests added by this task."

### Task_7: Final retrieval review and plan lifecycle
- type: review
- owns:
  - `.github/workflows/**`
  - `docs/coding-agent/plans/active/**`
  - `docs/coding-agent/plans/completed/**`
- depends_on: [Task_6]
- description: |
  Run required smoke evidence, review retrieval implementation, complete plan lifecycle updates, and draft the next Correction And Forget Lifecycle plan without implementing correction/forget.
- acceptance:
  - Required deterministic validation evidence from Tasks 1-6 is recorded.
  - Embedded Oxigraph retrieve/expansion smoke passes.
  - Live Qdrant candidate smoke passes locally or via CI evidence.
  - Reviewer approves no correction/forget/raw-storage scope creep and confirms rationale/trace behavior is inspectable.
  - Next concrete plan for Correction And Forget Lifecycle is drafted from the landed retrieval shape.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "Run embedded Oxigraph retrieve/expansion smoke."
  - kind: command
    required: true
    owner: worker
    detail: "Run live Qdrant candidate smoke locally before PR creation or provide CI job evidence before merge."
  - kind: review
    required: true
    owner: reviewer
    detail: "Review retrieval implementation against roadmap, ADRs, validation evidence, and non-goals."
  - kind: review
    required: true
    owner: orchestrator
    detail: "Confirm evidence completeness, no correction/forget scope creep, and next-plan independence."

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (design gate): [Task_1]
- Wave 2 (public DTOs and storage policy foundations): [Task_2, Task_3, Task_4]
- Wave 3 (retrieve pipeline): [Task_5]
- Wave 4 (facade and legacy isolation): [Task_6]
- Wave 5 (smoke, review, next-plan draft): [Task_7]

## E2E / Visual Validation Spec

- Not applicable. This is Rust library retrieval behavior with no UI/user-flow surface.

## Rollback / Safety
- Keep retrieval DTOs and public facade backend-free.
- Harden graph expansion policy before retrieve assembly uses expansion in production paths.
- Treat vector payload filters as hints; graph verification remains authoritative.
- Default filters exclude suppressed/deleted and non-current/superseded memory from context packs.
- Keep correction, forgetting, raw storage, and reflection out of this plan.

## Quality Routing Note
- Routing level: L2
- In-scope docs: Rust API/internal pipeline architecture, bounded graph expansion, vector/graph retrieval contracts, data-integrity filtering, deterministic fake-store validation, smoke evidence.
- Out-of-scope docs: UI/E2E, frontend/browser checks, correction/forget lifecycle mutation, production raw storage, external LLM extraction/reranking.
- Top risks: data-integrity filtering, graph expansion performance/boundedness, stale vector payload hints, public API shape, legacy search drift.
- Risk profile: medium-high because this chunk assembles user-facing context from two stores and must exclude suppressed/deleted/superseded memories correctly.
- Required checks: `cargo fmt --check`, `cargo check`, `cargo test --no-run`, targeted retrieval/graph/vector/facade tests, embedded Oxigraph smoke, live Qdrant candidate smoke, Reviewer gate.
- Optional recommended checks: `cargo clippy --all-targets -- -D warnings`.

## Progress Log

- 2026-04-28 Plan drafted.
  - Summary: Created the next concrete plan for v0.1 retrieve and `ContinuityContextPack` from the landed remember/link write surface, storage contracts, vector records, and graph adapter foundation.
  - Validation evidence: Researcher plan-fill report plus current remember/link implementation validation evidence.
  - Notes: Draft status; requires user approval before execution.

## Decision Log

- 2026-04-28 Decision: Draft retrieve/context-pack plan after remember/link implementation
  - Trigger / new insight: Remember/link now provides graph-authoritative objects/links and selected vector records, making vector-to-graph retrieval the next roadmap chunk.
  - Plan delta: Added `v0-1-retrieve-continuity-context-pack-plan.md` as the next active concrete plan.
  - Tradeoffs considered: Hardening graph expansion before retrieval assembly adds upfront work but prevents hub/fanout/lifecycle semantics from being baked into a fragile context-pack pipeline.
  - User approval: pending.

## Notes
- Risks:
  - Graph expansion around hub entities can become noisy or expensive without fanout and relation controls.
  - Qdrant lifecycle/currentness hints can be stale; graph verification must decide final inclusion.
  - Deterministic reranking can become overcomplicated; keep score components inspectable and testable.
- Edge cases:
  - Suppressed/deleted memories must be omitted from normal packs.
  - Non-current/superseded derived memories must be omitted unless policy includes historical context.
  - Vector candidates whose graph objects are missing should not enter the pack and should be traceable when trace is enabled.
  - Section limits must be stable and should not reorder ties nondeterministically.
