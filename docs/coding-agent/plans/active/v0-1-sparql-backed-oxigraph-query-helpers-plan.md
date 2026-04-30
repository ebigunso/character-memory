# Plan: v0.1 SPARQL-Backed Oxigraph Query Helpers

- status: draft
- generated: 2026-05-01
- last_updated: 2026-05-01
- work_type: code

## Goal
- Make the embedded Oxigraph graph authority prove its v0.1 graph-query behavior through RDF/SPARQL-backed selectors and regression tests, while keeping public/domain contracts backend-free.

## Definition of Done
- Internal SPARQL helpers execute against Oxigraph named graphs and return stable object refs or IDs.
- Oxigraph graph-authority query methods use SPARQL-backed selection for the v0.1 query intents instead of relying only on sidecar maps for candidate selection.
- Regression tests cover object lookup, provenance/thread/entity context, current-only filtering, suppressed/archived/superseded omission, deterministic ordering, and named-graph predicate regressions.
- Existing public facade behavior remains unchanged.

## Scope / Non-goals
- Scope:
  - Add internal SPARQL query helpers under graph infrastructure.
  - Route Oxigraph query/expansion selection through those helpers where practical for v0.1 contracts.
  - Keep domain object hydration from the existing canonical in-memory object cache unless a narrow helper needs only IDs.
- Non-goals:
  - Persistent Oxigraph storage configuration.
  - Qdrant/Oxigraph reconciliation diagnostics.
  - Public SPARQL APIs or backend-specific public types.
  - Full RDF-to-domain object hydration.

## Context (workspace)
- Related files/areas:
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `src/internal/infrastructures/graph/rdf_mapping.rs`
  - `src/internal/infrastructures/graph/vocabulary.rs`
  - `src/internal/repositories/graph_authority_store.rs`
  - `src/internal/repositories/test_support.rs`
- Existing patterns or references:
  - Current Oxigraph adapter materializes RDF quads but query methods hydrate from sidecar domain-object/link maps.
  - RDF triples are inserted into named graphs owned by canonical graph URIs.
  - Existing helper algorithms in `graph_authority_store.rs` provide deterministic ordering and lifecycle semantics.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md`

## Open Questions (max 3)
- Q1: Should SPARQL helpers only return IDs/object refs and let the cache hydrate canonical objects for v0.1?
- Q2: Should any literal values be upgraded to typed RDF literals now, or should SPARQL account for current simple-string literals?

## Assumptions
- A1: SPARQL/Oxigraph types remain internal to `src/internal/infrastructures/graph/**`.
- A2: Persistent graph authority and drift diagnostics stay deferred to v0.1.1.
- A3: Existing deterministic ordering semantics should be preserved even if SPARQL result ordering differs.

## Tasks

### Task_1: Add SPARQL Selector Layer
- type: impl
- owns:
  - `src/internal/infrastructures/graph/**`
- depends_on: []
- description: |
  Add internal helpers that execute SPARQL against the embedded Oxigraph `Store` and return backend-neutral IDs/object refs for v0.1 graph query intents. Keep helper signatures private/internal and avoid leaking Oxigraph types outside graph infrastructure.
- acceptance:
  - Helpers can select objects by ID/type from RDF triples across named graphs.
  - Helpers can select derived memories by provenance, thread, and entity predicates.
  - Helpers can identify lifecycle/currentness predicates needed for default filtering.
  - Helpers account for named graphs via `GRAPH ?g { ... }` or equivalent.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::graph::oxigraph_authority_store --lib"

### Task_2: Route Oxigraph Query Methods Through SPARQL Selectors
- type: impl
- owns:
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
- depends_on: [Task_1]
- description: |
  Update `GraphAuthorityStore` methods in the Oxigraph adapter to use SPARQL-backed selector results for candidate selection, then hydrate canonical objects/links from the existing in-memory cache. Preserve existing errors and deterministic sorting.
- acceptance:
  - `query_objects` uses SPARQL-selected refs before hydrating results.
  - `query_derived_memories_by_provenance` and `query_derived_memories_by_thread` use SPARQL candidate selection and existing lifecycle semantics.
  - `expand_bounded` uses SPARQL-visible graph refs/relations where practical while preserving bounded expansion behavior.
  - Missing graph roots still produce the existing `GraphExpansionRootNotFound` behavior.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::graph::oxigraph_authority_store --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::repositories::retrieve_pipeline --lib"

### Task_3: Add SPARQL Regression Coverage
- type: test
- owns:
  - `src/internal/infrastructures/graph/oxigraph_authority_store.rs`
  - `src/internal/infrastructures/graph/rdf_mapping.rs`
  - `src/internal/infrastructures/graph/vocabulary.rs`
- depends_on: [Task_2]
- description: |
  Add focused regression tests that fail when RDF predicate names, named graph handling, lifecycle predicates, or provenance/thread selectors drift from the v0.1 contract.
- acceptance:
  - Tests prove RDF/SPARQL selection works for representative fixtures.
  - Tests cover suppression, archive, non-current, and supersession filtering through graph-authority behavior.
  - Tests cover deterministic retrieval behavior after SPARQL-backed graph verification.
  - Tests document that memory links remain graph-only and are not vector-indexed records.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::graph --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"

### Task_4: Reviewer Gate
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Review the SPARQL-backed implementation against the storage contract and repository boundaries.
- acceptance:
  - Reviewer status is APPROVED.
  - Review confirms backend-specific types did not leak into public/domain/repository contracts.
  - Review confirms persistent Oxigraph and reconciliation were not accidentally included.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review against plan acceptance and v0.1 storage/backend contract"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. Rust library/storage behavior only.

## Rollback / Safety
- Keep the sidecar object/link cache as the hydration source during this plan so SPARQL selector regressions can be reverted without redesigning domain serialization.
- If SPARQL selector behavior proves too broad for one implementation pass, keep cache-backed behavior and land selector regression tests first, then replan routing.

## Progress Log (append-only)

- 2026-05-01 00:00 Draft created.
  - Summary: Split SPARQL-backed Oxigraph query helpers into its own v0.1 backend-contract plan.
  - Validation evidence: Not run; plan only.
  - Notes: Awaiting user approval before implementation.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-01 00:00 Decision:
  - Trigger / new insight: Prior repository assessment found RDF materialization exists, but query behavior still relies on sidecar maps rather than SPARQL regression coverage.
  - Plan delta (what changed): Created a standalone plan for SPARQL-backed query helpers and regression tests.
  - Tradeoffs considered: Full RDF-to-domain hydration is deferred to avoid expanding the feature beyond v0.1 query contracts.
  - User approval: no

## Notes
- Risks:
  - Oxigraph named graph syntax and simple-string literals may make SPARQL filters more brittle than existing Rust helper filters.
  - The plan should preserve deterministic ordering in Rust even if SPARQL result ordering is not stable.
- Edge cases:
  - Missing graph root.
  - Duplicate relation quads owned by different links.
  - Superseded derived memories where supersession evidence comes from relation links.
