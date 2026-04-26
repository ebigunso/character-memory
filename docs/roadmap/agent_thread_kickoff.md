# Agent Thread Kickoff (Temporary)

This document is a **shared implementation brief** for running multiple coding-agent threads, each producing **one focused PR**.

## Global constraints (apply to every PR)

- Do **not** implement Oxigraph yet (roadmap goal remains, but current PRs focus on sorting current mismatches).
- Do **not** add new UX, CLI, examples, or extra features outside PR scope.
- Keep changes minimal and cohesive; avoid refactors unrelated to the PR.
- The `tests/` directory is for **integration tests**. Unit tests may live inline in `src/**` as `#[cfg(test)]`.
- Breaking changes are acceptable (pre-alpha), but each PR must clearly state API breaks.

## Suggested PR sequence

Some PRs are independent, but this order reduces merge conflicts:

1. PR-001: Qdrant full-text indexes + `matches_text` filters
2. PR-002: Stop storing embeddings in Qdrant payload
3. PR-003: Preserve vector search scores in API
4. PR-004: Public embedding injection (pluggable embeddings) + deterministic unit tests
5. PR-005: Public/internal module layout refactor (`api/` vs `internal/`)
6. PR-006: Naming cleanup / roadmap naming alignment (optional)
7. PR-007: Hybrid-first API + retrieval trace (recommended)

If you plan to run PRs in parallel, avoid overlapping files where possible.

---

# PR-001 — Full-text filtering for `participants` + `location_text`

## Goal
Enable **word-level partial matching** for:
- `participants` (array of strings)
- `location_text` (string)

by switching from exact-match filters to **full-text filters** and creating **full-text indexes** on those payload fields.

## Scope
- Replace `Condition::matches` usage with `matches_text` (SDK-equivalent) for `participants` and `location_text`.
- Extend collection initialization to create field indexes using:
  - `TokenizerType::Multilingual`
  - `min_token_len = 2`, `max_token_len = 10`, `lowercase = true`
- Ensure behavior is **idempotent**: safe if collection already exists and safe if index already exists.

## Non-goals
- Within-word substring matching (e.g., `Ali` matching `Alice`) is not required for this PR.
- No schema changes beyond indexing.

## Files likely touched
- `src/infrastructures/external_services/qdrant_vector_memory_repository.rs`

## Acceptance criteria
- Searching with `MemoryFilters { participants: ["Alice"], .. }` matches memories whose participants contain `"Alice"` even when participant strings include additional tokens (e.g., "Alice Johnson") if tokenization allows.
- Searching with `MemoryFilters { location_text: Some("New York"), .. }` matches appropriate memories.
- `init_storage()` works on a fresh collection and on an existing collection.

## Test guidance
- Add an integration test that demonstrates **token-level** matching behavior for both fields.
  - Avoid asserting within-word substring matches.

## Risk notes
- Confirm Qdrant full-text indexing works on an array payload field (`participants`). If not, record the limitation in the PR and propose a follow-up approach (derived `participants_text` field).

---

# PR-002 — Remove embedding duplication from Qdrant payload

## Goal
Ensure Qdrant payload stores only **canonical metadata** (id/type/content/episodic fields) and does **not** store `embedding` inside payload.

## Scope
- Introduce a dedicated payload struct/DTO for Qdrant upserts (or build payload directly from `VectorMetadata`).
- Update single insert and bulk insert to use the canonical payload instead of `serde_json::to_value(MemoryEntry)`.

## Non-goals
- No changes to vector contents (the Qdrant vector still stores the embedding).
- No new database schema beyond payload fields.

## Files likely touched
- `src/infrastructures/external_services/qdrant_vector_memory_repository.rs`
- Possibly `src/models/vector/vector_metadata.rs` (if adding serialization helpers)

## Acceptance criteria
- Qdrant payload for a point does not contain an `embedding` field.
- Existing read-path (`point_data_to_memory_entry`) continues to work.

## Test guidance
- Prefer a unit test for payload-building logic (no external calls).
- Optionally add an integration test that retrieves the raw payload from Qdrant and asserts `embedding` is absent.

---

# PR-003 — Preserve search scores (for future `RetrievalBundle`)

## Goal
Stop discarding Qdrant similarity scores, so later hybrid retrieval can rank/merge results and optionally expose scores in retrieval traces.

## Scope
- Introduce a scored result type (e.g., `ScoredMemory` or `MemoryCandidate`) containing `{ memory, score }` or `{ id, score, memory? }`.
- Plumb score from Qdrant `ScoredPoint.score` through the repository and public API.

## Non-goals
- No graph expansion yet.
- No `RetrievalBundle` yet (unless you choose to add a small intermediate type).

## Files likely touched
- `src/infrastructures/external_services/qdrant_vector_memory_repository.rs`
- `src/repositories/vector_memory_repository.rs`
- `src/internal/repositories/memory_repository.rs`
- `src/lib.rs`
- Potentially `src/models/**` for the new public type

## Acceptance criteria
- Search results include scores (or scores are preserved internally such that later hybrid retrieval and tracing can use them).
- Existing callers can be updated in a single PR (breaking change acceptable).

## Test guidance
- Integration test: create a few memories, search, assert results include scores and are ordered by score.

---

# PR-004 — Public embedding injection (pluggable embeddings)

## Goal
Provide a public construction path where callers can inject embedding generation (or a provider), enabling deterministic tests and keeping the roadmap’s “caller-provided or pluggable provider” promise.

## Scope
- Make the embedding provider interface public (or expose a public builder).
- Add a constructor like:
  - `CharacterMemory::new_with_repositories(embed_repo, vector_repo)` OR
  - `CharacterMemoryBuilder` with `.embedding_provider(...)` / `.vector_store(...)`.
- Keep the existing convenience path (`CharacterMemory::new(settings, collection_name)`) as a default.

## Non-goals
- No change in default behavior for existing users unless explicitly desired.

## Files likely touched
- `src/api/embedding.rs`
- `src/internal/repositories/memory_repository.rs`
- `src/lib.rs`

## Acceptance criteria
- A test embedding provider can be used without network calls.
- Unit tests do not require OpenAI or Qdrant.

## Test guidance
- Add unit tests using a fake embedding provider (inline tests under `src/**` is fine).
- Keep `tests/**` as integration tests.

---

# PR-005 — Public/internal module layout refactor (`api/` vs `internal/`)

## Goal
Make it **obvious from the directory structure** which types/traits are part of the public API vs internal implementation details, while keeping the externally visible API as stable as practical (pre-alpha breaking changes acceptable, but prefer re-exports).

## Context / rationale
- Rust "public" is determined by module reachability; today, public and internal code live side-by-side in `src/models/**` and `src/repositories/**`, which makes it hard to see what is intended for caller use.
- We intentionally kept vector-store injection internal-only (to avoid forcing internal structs like `MemoryEntry` into the public contract). This refactor formalizes that boundary.

## Scope
- Introduce two top-level modules:
  - `src/api/**`: public contract surface (DTOs, public traits, public constructors)
  - `src/internal/**`: implementation details (repositories, internal domain structs, infrastructure adapters)
- Move (or wrap) current public DTOs under `src/api/types/**` (e.g., `Memory`, `MemoryInput`, `MemoryFilters`, `ScoredMemory`).
- Place the public embedding extension point under `src/api/**` (e.g., `EmbeddingProvider`).
- Keep internal-only structs and traits under `src/internal/**` (e.g., `MemoryEntry`, `ScoredMemoryEntry`, internal vector repository trait, Qdrant/OpenAI adapters).
- Update all `use` paths and module declarations accordingly.
- Keep crate-root exports stable where possible by re-exporting from `api` in `src/lib.rs`:
  - `pub use crate::api::types::{...}`
  - `pub use crate::api::embedding::{...}`

## Non-goals
- No behavior changes to memory creation/search/update/delete.
- No renaming of public types (that is PR-006).
- No new public vector-store injection surface.

## Files likely touched
- `src/lib.rs` (module declarations + public re-exports)
- New: `src/api/**`, `src/internal/**`
- Move/adjust existing modules under `src/models/**`, `src/repositories/**`, `src/infrastructures/**`
- Widespread `use` path updates

## Acceptance criteria
- A new contributor can identify public contract code by looking only at `src/api/**`.
- `cargo fmt`, `cargo check --all-targets`, `cargo test`, and `cargo clippy --all-targets -- -D warnings` all pass.
- Crate-root public exports remain stable (or any intentional break is clearly documented in the PR).

## Test guidance
- No new integration tests required.
- Ensure unit tests continue to pass (especially the deterministic embedding-injection tests).

## Risk notes
- This is a large mechanical move; keep it as a single cohesive refactor and avoid mixing in naming/API changes.
- Watch for accidental "API leaks" (e.g., making an internal module `pub` or re-exporting internal types from `lib.rs`).

---

# PR-006 — Naming cleanup (roadmap alignment)

## Goal
Align public names with roadmap intent while pre-alpha breaking changes are acceptable.

## Options
- Rename public `Memory` to `MemoryRecord` (or introduce `MemoryRecord` and deprecate `Memory`).
- Introduce `RetrievalBundle` placeholder only if needed by scoring work.

## Non-goals
- No Oxigraph code yet.

## Files likely touched
- `src/models/**`
- `src/lib.rs`
- `README.md` (if public API changes)
- `docs/roadmap/development_roadmap.md` (to reflect the updated contract)

## Acceptance criteria
- Names are consistent and reduce ambiguity between input/output/internal representations.

---
# PR-007 — Hybrid-first public API + RetrievalTrace

## Goal
Make `hybrid_search` the **only recommended retrieval entrypoint** and separate traceability from usability by adding an optional retrieval trace.

## Scope
- Define `RetrievalBundle` to contain a final `results[]` list suitable for direct caller use.
- Add an optional `RetrievalTrace`/`trace` field (opt-in) that records:
  - vector stage: candidate ids + scores + applied filters
  - graph stage: expansion policy + summarized edges/nodes added
  - merge stage: ordering rationale / dedupe decisions
- Ensure docs and README clearly state: callers should use `hybrid_search` and not compose vector-only/graph-only retrieval.

## Non-goals
- No promise that vector-only or graph-only retrieval APIs are stable or recommended.
- No Oxigraph implementation in this PR.

## Files likely touched
- `src/models/**`
- `src/lib.rs`
- `docs/roadmap/development_roadmap.md`
- `README.md`

## Acceptance criteria
- Callers can retrieve high-quality results via one API (`hybrid_search`) without implementing selection logic.
- When tracing is enabled, internal stage outputs are available for debugging/provenance.

## Test guidance
- Unit tests for trace struct serialization and deterministic trace content construction.

---

## PR template (recommended)

Each PR description should include:

- **Goal** (1–2 sentences)
- **Scope / Non-goals**
- **API changes** (breaking? yes/no)
- **Testing** (unit vs integration)
- **Follow-ups** (if limitations discovered)
