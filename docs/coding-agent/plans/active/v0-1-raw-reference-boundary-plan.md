# Plan: v0.1 Raw Reference Boundary Hardening

- status: draft
- generated: 2026-05-01
- last_updated: 2026-05-01
- work_type: mixed

## Goal
- Harden and document the v0.1 raw-reference boundary: raw source material stays external, `raw_ref` pointers are preserved, unresolved refs are representable, and graph/vector stores do not become raw transcript stores.

## Definition of Done
- Tests prove domain, RDF, Qdrant payload, and retrieval surfaces preserve `raw_ref` pointers without storing raw transcript fields.
- `RawReferenceResolver` behavior clearly distinguishes unavailable references from resolver errors.
- Documentation states that production raw storage is caller-owned/deferred and that v0.1 stores stable source pointers only.
- No production raw transcript store is added.

## Scope / Non-goals
- Scope:
  - Raw-reference preservation tests.
  - Optional resolver unavailable/error behavior tests.
  - Documentation alignment if wording implies graph/vector raw transcript storage.
- Non-goals:
  - Implementing a production raw store.
  - Adding public raw-resolution APIs.
  - Changing remember/retrieve public DTO shapes unless tests expose a concrete mismatch.
  - Persistent Oxigraph or Qdrant/Oxigraph reconciliation.

## Context (workspace)
- Related files/areas:
  - `src/api/types/domain.rs`
  - `src/api/types/draft.rs`
  - `src/api/types/retrieval.rs`
  - `src/internal/repositories/raw_reference_resolver.rs`
  - `src/internal/repositories/test_support.rs`
  - `src/internal/infrastructures/graph/rdf_mapping.rs`
  - `src/internal/infrastructures/external_services/qdrant_payload.rs`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/design/database/vector_payload_design.md`
  - `docs/design/database/graph_schema_design.md`
  - `README.md`
- Existing patterns or references:
  - `raw_ref` exists on source domain objects and is preserved in RDF/Qdrant payloads.
  - Existing resolver is an internal trait with test fixtures.
  - v0.1 contract says graph/vector layers store summaries, excerpts, derived memories, and stable pointers, not raw transcripts.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/index.md`
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`
  - `docs/decisions/design/ADR-D-0008-preserve-source-references.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`

## Open Questions (max 3)
- Q1: Should the internal `RawReferenceResolver` remain test/support-only for now, or should it become an injectable internal dependency in a later plan?

## Assumptions
- A1: This plan hardens the boundary; it does not add production raw transcript storage.
- A2: Unavailable raw refs should be represented as `Ok(None)`, while resolver failures remain `Err`.
- A3: Public APIs should not be expanded unless existing v0.1 acceptance cannot be met without doing so.

## Tasks

### Task_1: Audit And Test Raw Reference Preservation
- type: test
- owns:
  - `src/api/types/domain/tests.rs`
  - `src/api/types/retrieval.rs`
  - `src/internal/infrastructures/graph/rdf_mapping.rs`
  - `src/internal/infrastructures/external_services/qdrant_payload.rs`
- depends_on: []
- description: |
  Add or tighten tests proving `raw_ref` pointers are preserved where intended and raw transcript content does not appear as graph/vector payload fields.
- acceptance:
  - Domain serialization preserves source `raw_ref` values.
  - RDF mapping includes source pointers without full raw transcript content.
  - Qdrant payload includes `raw_ref` and excludes raw transcript fields.
  - Retrieval context pack preserves source refs needed for provenance display.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test api::types::domain --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::external_services::qdrant_payload --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::infrastructures::graph::rdf_mapping --lib"

### Task_2: Harden RawReferenceResolver Unavailable Behavior
- type: test
- owns:
  - `src/internal/repositories/raw_reference_resolver.rs`
  - `src/internal/repositories/test_support.rs`
- depends_on: []
- description: |
  Ensure the internal raw-reference resolver boundary distinguishes unavailable references from resolver errors and has deterministic test fixture behavior.
- acceptance:
  - Resolver tests cover successful resolution.
  - Resolver tests cover unavailable reference as `Ok(None)`.
  - Resolver behavior remains internal and does not imply production raw storage.
  - Fixture resolver remains deterministic and file-system independent unless a fixture explicitly models a file reference.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::repositories::raw_reference_resolver --lib"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test internal::repositories::test_support --lib"

### Task_3: Align Raw Reference Documentation
- type: docs
- owns:
  - `README.md`
  - `docs/design/roadmap-phases/v0_1_storage_and_backend_contracts.md`
  - `docs/design/database/vector_payload_design.md`
  - `docs/design/database/graph_schema_design.md`
- depends_on: [Task_1, Task_2]
- description: |
  Review and update docs only where wording implies that graph/vector stores own raw transcript storage. Keep docs aligned with the v0.1 boundary: pointers are stored, raw source material is external.
- acceptance:
  - Docs explicitly state that production raw storage is caller-owned/deferred.
  - Docs distinguish `raw_ref` source pointers from raw transcript content.
  - Docs do not promise public raw resolution unless implemented.
  - Persistent graph authority and reconciliation remain assigned to v0.1.1 docs.
- validation:
  - kind: review
    required: true
    owner: worker
    detail: "Manual doc diff review for raw storage wording"
  - kind: command
    required: true
    owner: worker
    detail: "cargo test --no-run"

### Task_4: Reviewer Gate
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3]
- description: |
  Review the raw-reference boundary for scope control and storage-contract accuracy.
- acceptance:
  - Reviewer status is APPROVED.
  - Review confirms no production raw transcript store was added.
  - Review confirms tests and docs distinguish raw pointers from raw content.
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review against ADR-D-0008 and v0.1 storage/backend contract"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1, Task_2]
- Wave 2 (parallel): [Task_3]
- Wave 3 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. Rust library/storage behavior only.

## Rollback / Safety
- Treat any public raw-resolution API pressure as a replan trigger.
- Documentation updates should be limited to boundary clarification, not roadmap expansion.

## Progress Log (append-only)

- 2026-05-01 00:00 Draft created.
  - Summary: Split raw-reference boundary hardening into its own v0.1 backend-contract plan.
  - Validation evidence: Not run; plan only.
  - Notes: Awaiting user approval before implementation.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-01 00:00 Decision:
  - Trigger / new insight: Prior repository assessment found raw refs are mostly present, but production raw-store boundaries and unavailable-resolution behavior should be hardened separately from graph/query work.
  - Plan delta (what changed): Created a standalone raw-reference boundary plan.
  - Tradeoffs considered: A production raw transcript store is excluded because it conflicts with the v0.1 contract boundary and would be a larger API/storage design.
  - User approval: no

## Notes
- Risks:
  - The phrase "raw store" can be misread as a request to implement transcript persistence; this plan intentionally avoids that.
  - Docs may already be accurate, in which case Task_3 should record no-op evidence rather than churn.
- Edge cases:
  - `raw_ref` on episode vs observation.
  - Resolver returns unavailable vs error.
  - Payload fields named `raw_transcript` or `transcript` accidentally appearing in vector payloads.
