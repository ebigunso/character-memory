# Plan: OpenAI Embedding Batching

- status: in_progress
- generated: 2026-05-04
- last_updated: 2026-05-04
- work_type: code

## Goal
- Replace serial OpenAI embedding calls in the provider bulk path with true batched embeddings API requests while preserving the existing public embedding provider trait and memory write semantics.

## Definition of Done
- `OpenAIEmbeddingProvider::bulk_generate_embeddings` sends batches of input texts to the embeddings API instead of awaiting one request per text.
- Returned embeddings are validated for count, order, and vector dimensionality before indexing.
- Empty batch and empty text behavior is explicit and covered by tests.
- Remember/correction indexing paths continue to use the existing `MemoryEmbedder::embed_batch` flow without public API changes.
- Required Rust checks and focused provider/pipeline tests pass or have explicit recorded waivers.

## Scope / Non-goals
- Scope:
  - `src/internal/infrastructures/external_services/openai_embedding_provider.rs`
  - adjacent unit tests in the same module
  - `src/internal/repositories/remember_pipeline.rs` tests only if integration coverage needs adjustment
  - `src/internal/repositories/correction_forget_pipeline.rs` tests only if replacement indexing coverage needs adjustment
- Non-goals:
  - Changing the public `EmbeddingProvider` trait.
  - Changing embedding model defaults.
  - Introducing a new embedding vendor or local embedding backend.
  - Running live OpenAI benchmark jobs as part of ordinary validation.
  - Tuning retrieval budgets or eval configs.

## Context (workspace)
- Related files/areas:
  - `src/api/embedding.rs`
  - `src/internal/infrastructures/external_services/openai_embedding_provider.rs`
  - `src/internal/repositories/remember_pipeline.rs`
  - `src/internal/repositories/correction_forget_pipeline.rs`
  - `src/lib.rs`
- Existing patterns or references:
  - The repository already batches vector records at the remember pipeline boundary.
  - `MemoryEmbedder::embed_batch` already delegates to the provider bulk method.
  - The current OpenAI provider bulk method loops through `generate_embedding` and awaits one HTTP call per text.
  - The roadmap keeps OpenAI as a default adapter, not a philosophical dependency.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`
  - `docs/decisions/implementation/ADR-I-0002-natural-language-embedding-surfaces.md`
  - `docs/roadmap/development_roadmap.md`

## Open Questions (max 3)
- None.

## Resolved Decisions
- Use one OpenAI embeddings request for the full bulk input when it is within documented input-array count constraints.
- Split automatically only when the documented input-array count limit is exceeded.
- Do not add token estimation or token-limit preflight guards in this scope.
- Restore response order by returned embedding `index`.
- Fail the whole batch on API, parse, token-limit, or rate-limit errors.
- Defer adaptive retry/backoff.

## Assumptions
- A1: The OpenAI embeddings endpoint accepts an array of input strings and returns one embedding per input.
- A2: The first implementation should fail the whole batch on API or parse failure, matching the current all-or-nothing bulk trait behavior.
- A3: Provider-specific batching should stay inside the OpenAI adapter so tests and alternative embedding providers remain unchanged.
- A4: Live benchmark speedup should be measured after the core provider change lands, not inside this implementation plan.
- A5: Token estimation and token-limit preflight guards are intentionally deferred. Supporting future embedding model configurations beyond the OpenAI adapter will affect the right abstraction, so this plan should not add OpenAI-specific token counting yet.

## Tasks

### Task_1: Define Batch Request Semantics
- type: design
- owns:
  - `docs/coding-agent/plans/active/openai-embedding-batching-plan.md`
  - `src/internal/infrastructures/external_services/openai_embedding_provider.rs`
- depends_on: []
- description: |
  Finalize the internal batching policy before implementation: empty input handling, per-item validation, response order handling, dimensionality checks, and whether the initial implementation uses a fixed internal batch size.
- acceptance:
  - Plan decision log records the batch size and retry boundary.
  - Plan decision log records empty batch and empty text behavior.
  - Plan decision log records that token estimation and token-limit preflight guards are intentionally deferred.
  - Plan decision log records response ordering expectations.
  - No public trait or model default changes are introduced by this decision.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Decision log updated with batching semantics and tradeoffs"

### Task_2: Implement OpenAI Bulk Request Path
- type: impl
- owns:
  - `src/internal/infrastructures/external_services/openai_embedding_provider.rs`
- depends_on: [Task_1]
- description: |
  Replace the serial loop in `bulk_generate_embeddings` with batched OpenAI embeddings requests. Reuse the existing client, model, API key, error type, and vector-size expectations.
- acceptance:
  - `bulk_generate_embeddings` sends array inputs to the embeddings endpoint.
  - Empty batch returns an empty vector without an HTTP request.
  - Blank text entries return a clear embedding generation error before the HTTP request.
  - The implementation may split only by documented input-array count limits; it must not add token estimation or token-limit preflight guards in this scope.
  - Response parsing validates embedding count and vector dimensions.
  - Returned embeddings preserve input order.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test openai_embedding_provider --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::repositories::remember_pipeline --lib"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review provider error handling, response parsing, and trait compatibility"

### Task_3: Add Focused Provider Coverage
- type: test
- owns:
  - `src/internal/infrastructures/external_services/openai_embedding_provider.rs`
- depends_on: [Task_2]
- description: |
  Add or refine service-free tests around request construction and response parsing. Prefer in-module test seams over adding new HTTP mocking dependencies unless the plan is explicitly updated.
- acceptance:
  - Tests cover empty batch behavior.
  - Tests cover blank text rejection in a batch.
  - Tests cover parsing multiple embeddings from one response.
  - Tests cover response count mismatch or invalid embedding shape.
  - Tests do not require a real OpenAI API key.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test openai_embedding_provider --lib"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review test seam for minimality and no live-network dependency"

### Task_4: Closeout Validation
- type: review
- owns: []
- depends_on: [Task_2, Task_3]
- description: |
  Run repository-required checks and review the final change against write-path performance intent and provider abstraction boundaries.
- acceptance:
  - Required validation evidence is complete or explicitly waived.
  - Reviewer confirms the implementation removes serial per-text OpenAI calls in the bulk path.
  - Reviewer confirms no retrieval-budget or benchmark-result semantics changed.
- validation:
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo check"
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo test --no-run"
  - kind: review
    required: true
    owner: reviewer
    detail: "Full diff review against this plan"

## Task Waves (explicit parallel dispatch sets)

Interpretation:
- Tasks listed in the same wave are intended to be dispatched in parallel by default when `owns` are disjoint and dependencies are met.
- Waves are executed sequentially.

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. This plan does not touch UI or browser-facing flows.

## Rollback / Safety
- Revert the provider implementation to the previous serial loop if batched requests produce incompatible API behavior.
- Keep the `EmbeddingProvider` trait unchanged so callers and test providers are insulated from rollback.

## Progress Log (append-only)

- 2026-05-04 00:00 Wave 1 completed: [Task_1]
  - Summary: Batch request semantics resolved and recorded in the decision log.
  - Validation evidence: Orchestrator reviewed the plan decision log and resolved open questions.
  - Notes: No public trait or model-default changes are in scope.

- 2026-05-04 00:00 Wave 2 completed: [Task_2]
  - Summary: Replaced the serial provider bulk loop with batched array requests, response parsing by index, documented input-count chunking, and provider-local validation.
  - Validation evidence: `cargo test openai_embedding_provider --lib` passed; `cargo test internal::repositories::remember_pipeline --lib` passed.
  - Notes: Token-limit preflight and adaptive retry/backoff remain deferred by decision.

- 2026-05-04 00:00 Wave 3 completed: [Task_3]
  - Summary: Added service-free provider tests for payload shape, blank input validation, index-ordered parsing, count mismatch, dimension mismatch, duplicate index, and nonnumeric values.
  - Validation evidence: `cargo test openai_embedding_provider --lib` passed.
  - Notes: Removed reliance on dummy-key live-network tests.

- 2026-05-04 00:00 Review loop completed: [Task_4]
  - Summary: First reviewer pass found missing request-count coverage for the actual bulk path; implementation added a private transport seam and bulk-method tests. Second reviewer pass reported no findings and approved the branch.
  - Validation evidence: `cargo fmt --check` passed; `cargo check` passed; `cargo test --no-run` passed; `cargo test openai_embedding_provider --lib` passed; `cargo test internal::repositories::remember_pipeline --lib` passed; `cargo test internal::repositories::correction_forget_pipeline --lib` passed.
  - Notes: Live OpenAI integration remains out of scope; token-limit preflight remains intentionally deferred.

- 2026-05-04 00:00 Plan drafted on `feature-2026-05-04-openai-embedding-batching`.
  - Summary: Created execution plan for true OpenAI embedding batching.
  - Validation evidence: Planning artifact only; implementation validation pending.
  - Notes: Budget sweep and retrieval default tuning are out of scope.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-04 00:00 Decision:
  - Trigger / new insight: LongMemEval live ingestion showed the provider bulk path is serial.
  - Plan delta (what changed): Created a dedicated implementation scope for provider batching.
  - Tradeoffs considered: Keep batching provider-local rather than changing public traits or eval configs.
  - User approval: yes; user requested branches and committed plans for ready work scopes.

- 2026-05-04 00:00 Decision:
  - Trigger / new insight: OpenAI documents request-level token limits, but future-proof token handling across embedding provider/model configurations needs a broader abstraction than this provider-local batching fix.
  - Plan delta (what changed): Token estimation and token-limit preflight guards are explicitly deferred; the implementation should rely on API errors for token-limit failures in this scope.
  - Tradeoffs considered: Avoid adding OpenAI-specific token counting now, while still permitting splitting by documented input-array count limits if needed.
  - User approval: yes; user requested this decision be recorded where it is not easily missed.

- 2026-05-04 00:00 Decision:
  - Trigger / new insight: Remaining provider batching open questions were resolved before implementation.
  - Plan delta (what changed): Use one request for the full bulk input when within documented input-array count constraints; split automatically only when the documented input-array count limit is exceeded; restore response order by embedding `index`; fail the whole batch on API, parse, token-limit, or rate-limit errors; defer adaptive retry/backoff.
  - Tradeoffs considered: Preserve the existing all-or-nothing bulk trait behavior and avoid introducing retry/order complexity before the basic batch path is stable.
  - User approval: yes; user accepted these recommendations.

## Notes
- Risks:
  - OpenAI response parsing must preserve input-to-embedding correspondence.
  - Request-size limits may require internal chunking.
- Edge cases:
  - Empty batches.
  - Blank text entries.
  - Mismatched response counts.
  - Embedding dimensions that do not match the configured model.
