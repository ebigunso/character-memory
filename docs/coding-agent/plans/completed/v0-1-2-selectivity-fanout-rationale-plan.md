# Plan: v0.1.2 Selectivity Fanout And Retrieval Rationale

- status: completed
- generated: 2026-05-09
- last_updated: 2026-05-10
- work_type: code

## Goal
- Implement continuous relation/object-specific selectivity scoring, smooth stats-guided graph expansion fanout, conservative missing-stats behavior, and retrieval rationale/trace diagnostics while preserving Qdrant-as-candidate and Oxigraph-as-authority retrieval semantics.

## Definition of Done
- Selectivity scores are computed at retrieval time from stats counters using ADR-I-0010's formula.
- Fanout budgets are smooth, relation/object-specific, capped, and influenced by supporting evidence and explicit scope.
- Missing or unhealthy stats produce conservative fanout without disabling retrieval.
- Retrieval rationale and trace distinguish high-selectivity, low-selectivity-supported, low-selectivity-rejected, explicit-scope, and fallback decisions.
- Final context inclusion still requires Oxigraph validation for existence, lifecycle/currentness, suppression, supersession, and provenance.
- Entity-neutral high-degree retrieval tests pass and Reviewer approves the authority split.

## Scope / Non-goals
- Scope:
  - Selectivity scoring and fanout policy modules.
  - Retrieval pipeline integration.
  - Retrieval rationale/trace/telemetry additions.
  - High-degree retrieval fixtures and tests.
  - Conservative fallback for missing/unhealthy stats.
- Non-goals:
  - Stats store persistence foundation; depends on the stats foundation plan.
  - Durable link co-occurrence admission policy.
  - Full v0.4 retrieval trace or admin dashboard.
  - Future v0.4/v0.5 observability surfaces such as `ActivationTrace`, `RejectedExpansionTrace`, `ClusterExpansionTrace`, `MembershipDecisionTrace`, `AssociationCandidateDiagnostic`, or `CoactivationDiagnostic`.
  - AssociativeUnit, AssociativeMembership, AssociationSupport, query-time activation, cluster expansion, or associative membership lifecycle.
  - Graph centrality, PageRank, learned retrieval policy, or persisted selectivity categories.
  - Entity identity or application-role special-casing.

## Context (workspace)
- Related files/areas:
  - `docs/design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`
  - `docs/decisions/design/ADR-D-0009-entity-neutral-retrieval-policy.md`
  - `docs/decisions/design/ADR-D-0010-recurring-entities-are-anchors-not-traversal-invitations.md`
  - `docs/decisions/implementation/ADR-I-0008-retrieval-stats-are-derived-policy-metadata.md`
  - `docs/decisions/implementation/ADR-I-0010-use-continuous-selectivity-and-smooth-fanout.md`
  - `src/api/types/retrieval.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/internal/repositories/graph_authority_store.rs`
  - `src/internal/repositories/test_support.rs`
- Existing patterns or references:
  - `RetrievalGraphLimits` currently has static `max_fanout_per_node` and `max_hub_edges`.
  - `RetrievalRationale`, `RetrievalTelemetry`, and `RetrievalTrace` already use serde defaults for backwards-compatible additions.
  - Graph expansion queries must remain backend-neutral.
  - Qdrant candidates whose graph objects are missing are omitted from normal retrieval.
  - Latest main keeps v0.1.2 diagnostics lightweight and reserves activation, rejected-expansion, cluster-expansion, membership-decision, association-candidate, and coactivation diagnostics for v0.4/v0.5.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`

## Open Questions (max 3)
- None.

## Resolved Decisions
- Expose v0.1.2 retrieval diagnostics through existing `RetrievalRationale`, `RetrievalTelemetry`, and `RetrievalTrace` using serde-defaulted additions; do not add a new public diagnostic API in this phase.
- Do not introduce `ActivationTrace`, `RejectedExpansionTrace`, `ClusterExpansionTrace`, `MembershipDecisionTrace`, `AssociationCandidateDiagnostic`, or `CoactivationDiagnostic` in this phase; v0.1.2 may only add small serde-defaulted fields to existing retrieval rationale/telemetry/trace surfaces.
- Use conservative initial fanout defaults from the roadmap examples: `aboutEntity` to `derived_memory` has `min = 0`, `max = 20`; `participantEntity` to `episode` has `min = 0`, `max = 5`; `partOfThread` to `derived_memory` has `min = 0`, `max = 15`. Additional relation/object pairs default to conservative bounded settings and must preserve existing static graph caps as hard upper bounds.
- Do not introduce the full v0.2 `ContinuityScope` model in this phase. Use a narrow internal policy hook or optional retrieval-context support only if current types provide a clean fit; leave full scope modeling to v0.2.

## Assumptions
- A1: The stats foundation plan is completed first and provides `RetrievalStatsStore` plus policy config.
- A2: Rationale/trace fields may be extended if serde defaults preserve older payload compatibility.
- A3: Normal retrieval must not scan or hydrate the whole graph to classify selectivity.
- A4: Broad entities remain potentially important; selectivity restricts expansion rather than globally lowering relevance.
- A5: Explicit scope support in this plan is bounded policy input only, not the full future continuity-scope model.

## Tasks

### Task_1: Add Selectivity Scoring And Fanout Policy
- type: impl
- owns:
  - `src/internal/repositories/retrieval_stats_store.rs`
  - `src/internal/repositories/retrieval_selectivity.rs`
  - `src/config/settings/app_settings.rs`
  - `src/internal/config/settings.rs`
- depends_on: []
- description: |
  Implement continuous selectivity scoring from stats counts and relation/object fanout policy with smoothing, gamma, support factors, and conservative fallback.
- acceptance:
  - Formula matches ADR-I-0010 and clamps scores to `0.0..1.0`.
  - Initial policy defaults include `aboutEntity` to `derived_memory` with max 20, `participantEntity` to `episode` with max 5, and `partOfThread` to `derived_memory` with max 15, all with min 0 unless a graph-authoritative provenance/currentness lookup requires a narrower nonzero internal floor.
  - Increasing entity count while holding global count constant does not increase selectivity.
  - Increasing supporting evidence may increase fanout but never above relation/object caps.
  - Missing or unhealthy stats yields conservative fanout and diagnostic metadata.
  - Diagnostic labels are derived only and are not persisted as entity state.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test selectivity fanout --no-fail-fast"
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check"

### Task_2: Integrate Stats-Guided Fanout Into Retrieval
- type: impl
- owns:
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/internal/repositories/graph_authority_store.rs`
  - `src/internal/repositories/test_support.rs`
- depends_on: [Task_1]
- description: |
  Replace static per-root expansion where needed with relation/object-specific budgets derived from stats and support signals, while preserving graph-authoritative final filtering.
- acceptance:
  - Retrieval fetches only the stats needed for current candidate roots/context, not graph-wide classification scans.
  - Low-selectivity entity evidence alone cannot flood the context pack.
  - High-selectivity entity evidence contributes meaningfully to retrieval.
  - Broad entities can contribute when supported by semantic, thread, temporal, salience, currentness, correction, or explicit scope evidence.
  - Suppressed, deleted, non-current, and superseded memories remain excluded by graph verification.
  - Qdrant hints and stats cannot force final inclusion when Oxigraph disagrees.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test retrieve_pipeline selectivity --no-fail-fast"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review retrieval flow for Qdrant/stat non-authority and Oxigraph final inclusion"

### Task_3: Extend Retrieval Rationale, Trace, And Diagnostics
- type: impl
- owns:
  - `src/api/types/retrieval.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/internal/repositories/test_support.rs`
- depends_on: [Task_1, Task_2]
- description: |
  Add report-only rationale/telemetry/trace fields for selectivity inputs, support signals, chosen fanout, expanded/included/rejected counts, fallback health, and broad-entity decisions.
- acceptance:
  - Rationale distinguishes high-selectivity same-entity matches from low-selectivity matches.
  - Rationale/trace can report low-selectivity allowed by supporting evidence and low-selectivity rejected as insufficient evidence.
  - Explicit-scope bounded expansion is represented only through a narrow policy hook or existing retrieval-context fit; full `ContinuityScope` remains deferred to v0.2.
  - Conservative fallback is visible when it occurs.
  - Public serde additions preserve backwards compatibility for older trace/rationale payloads.
  - Diagnostics stay within existing rationale/telemetry/trace surfaces, are report-only, and do not repair or override stores.
  - No future v0.4/v0.5 trace, activation, cluster, membership, association-candidate, or coactivation diagnostic type is introduced.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test retrieval_trace retrieval_telemetry retrieve_pipeline --no-fail-fast"
  - kind: review
    required: true
    owner: reviewer
    detail: "Check diagnostic fields are derived/report-only and public DTO compatibility is preserved"

### Task_4: Add Entity-Neutral Retrieval Acceptance Coverage
- type: test
- owns:
  - `src/internal/repositories/test_support.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/api/types/retrieval.rs`
- depends_on: [Task_2, Task_3]
- description: |
  Add synthetic high-degree retrieval fixtures across multiple domains and acceptance tests for entity-neutral selectivity behavior.
- acceptance:
  - Fixtures cover person, place, project, topic/concept, object/tool/document, and arbitrary custom entities.
  - Tests prove no retrieval rule depends on entity name, canonical key, or application role.
  - Broad non-user/non-assistant entities are bounded the same way as broad assistant-domain entities.
  - Stats missing/unhealthy produces conservative fanout.
  - Normal retrieval does not scan or hydrate the whole graph to classify selectivity.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test entity_neutral retrieve_pipeline selectivity --no-fail-fast"
  - kind: search
    required: true
    owner: reviewer
    detail: "Search retrieval/policy code for identity-specific checks against user/assistant/player/NPC/protagonist/main-character style roles"

### Task_5: Retrieval Fanout Closeout Review
- type: review
- owns: []
- depends_on: [Task_1, Task_2, Task_3, Task_4]
- description: |
  Validate the retrieval fanout and rationale implementation against v0.1.2 docs, ADR-D-0009/D-0010, and ADR-I-0010.
- acceptance:
  - All required worker validation evidence is present.
  - Reviewer approves entity-neutral behavior and authority boundaries.
  - No persisted categories, learned policy, centrality algorithm, or identity-specific special case is introduced.
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
    detail: "Final review of selectivity/fanout/rationale implementation"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (parallel): [Task_4]
- Wave 5 (parallel): [Task_5]

## E2E / Visual Validation Spec

- Not applicable. No UI/user-flow changes.

## Rollback / Safety
- Keep static retrieval limits as a fallback path until stats-guided policy passes validation.
- New public DTO fields must use serde defaults so old serialized traces/rationales continue to decode.

## Progress Log (append-only)

- 2026-05-09 Draft created:
  - Summary: Drafted the selectivity/fanout/rationale plan as the second v0.1.2 feature plan.
  - Validation evidence: Researcher mapped existing retrieval types and pipeline integration points; no implementation dispatched.
  - Notes: Depends on completion of the stats foundation plan.
- 2026-05-10 Open questions resolved:
  - Summary: Recorded approved recommendations for diagnostic surface, initial fanout budgets, and bounded explicit-scope handling.
  - Validation evidence: Plan-only update; `git diff --check -- docs/coding-agent/plans/active` pending.
  - Notes: No implementation dispatched.
- 2026-05-10 Plan approved for execution:
  - Summary: User requested each plan be committed on its own implementation branch and readied for execution.
  - Validation evidence: Plan status updated to approved; implementation remains pending.
  - Notes: No Worker tasks dispatched yet.
- 2026-05-10 Main roadmap refresh reviewed:
  - Summary: Updated plan boundary after latest main added controlled associative recall and expanded future observability docs.
  - Validation evidence: Plan-only update; branch rebased onto `origin/main`.
  - Notes: v0.1.2 remains limited to lightweight additions on existing retrieval rationale/telemetry/trace surfaces.
- 2026-05-13 Completed:
  - Summary: PR #49 merged after conflict resolution against PR #48 and plan-intent review confirmed the selectivity fanout implementation remained within the approved v0.1.2 scope.
  - Validation evidence: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo check --lib`; focused selectivity and retrieve pipeline tests before merge.
  - Notes: Plan moved from active to completed as part of final v0.1.2 cleanup.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-09 Decision:
  - Trigger / new insight: Current retrieval expansion uses static graph limits while v0.1.2 requires relation/object-specific dynamic fanout.
  - Plan delta (what changed): Created a dedicated retrieval policy plan after stats foundation so fanout work can depend on concrete stats APIs.
  - Tradeoffs considered: Combining this with stats persistence would blur storage validation with retrieval behavior validation.
  - User approval: no
- 2026-05-10 Decision:
  - Trigger / new insight: User approved the recommended answers to all selectivity fanout/rationale open questions.
  - Plan delta (what changed): Resolved diagnostics to existing rationale/telemetry/trace surfaces, recorded initial conservative fanout budgets, and deferred full `ContinuityScope` modeling to v0.2.
  - Tradeoffs considered: Existing DTO surfaces avoid premature public diagnostic API growth; roadmap budget examples give a conservative starting point; narrow scope hooks avoid pulling v0.2 concepts into v0.1.2.
  - User approval: yes
- 2026-05-10 Decision:
  - Trigger / new insight: Latest main adds future observability concepts for rejected expansion, activation, cluster expansion, membership decisions, association candidates, and coactivation.
  - Plan delta (what changed): Added explicit non-goals and acceptance coverage to keep those future trace/diagnostic types out of the v0.1.2 selectivity/fanout implementation.
  - Tradeoffs considered: Keeping v0.1.2 lightweight preserves the retrieval guardrail scope while leaving full observability and associative activation diagnostics to v0.4/v0.5.
  - User approval: yes

## Notes
- Risks:
  - Dynamic budgets may require changing graph expansion query shape.
  - Supporting-evidence signals may not all exist yet; missing signals should default to neutral rather than invented semantics.
  - Public trace additions must preserve serde compatibility.
- Edge cases:
  - `N` near zero in selectivity formula.
  - Low-selectivity but explicitly scoped retrieval.
  - High Qdrant score with missing/suppressed/non-current graph object.
  - Broad entities that are central but should remain bounded.
