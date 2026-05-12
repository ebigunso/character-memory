# Plan: v0.1.2 Low-Information Link Guard And Entity-Neutral Hardening

- status: completed
- generated: 2026-05-09
- last_updated: 2026-05-10
- work_type: code

## Goal
- Prevent durable pairwise link creation from weak low-selectivity co-occurrence while preserving explicit or stronger-evidence links, and close v0.1.2 with entity-neutral acceptance checks across retrieval, linking, diagnostics, and milestone validation.

## Definition of Done
- Link admission can reject low-information co-occurrence when the only evidence is a broad/low-selectivity shared entity or relation.
- Explicit application-created links and stronger evidence paths remain accepted.
- Rejected link candidates are counted or exposed through report-only diagnostics.
- Entity-neutral tests prove the guard does not special-case users, assistants, players, NPCs, protagonists, or other roles.
- Full v0.1.2 validation passes after stats foundation and retrieval fanout plans are complete.
- Reviewer approves the final v0.1.2 authority, scope, and YAGNI boundaries.

## Scope / Non-goals
- Scope:
  - Durable link admission guard.
  - Diagnostics/counting for rejected low-information candidates.
  - Link/retrieval acceptance tests using heterogeneous high-degree fixtures.
  - Final cross-plan validation for v0.1.2.
- Non-goals:
  - Implementing v0.5 controlled associative recall and clustering.
  - Implementing `AssociativeUnit`, `AssociativeMembership`, `AssociationSupport`, query-time activation, promotion/decay policy, cluster summaries, or bounded cluster expansion.
  - Creating a separate weak associative hint store.
  - Disabling all co-occurrence links.
  - Learned association policy.
  - Public admin dashboard.
  - Entity identity or application-role special-casing.
  - Changing graph authority or Qdrant candidate semantics.

## Context (workspace)
- Related files/areas:
  - `docs/design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`
  - `docs/decisions/design/ADR-D-0009-entity-neutral-retrieval-policy.md`
  - `docs/decisions/design/ADR-D-0010-recurring-entities-are-anchors-not-traversal-invitations.md`
  - `docs/decisions/design/ADR-D-0013-controlled-serendipitous-recall-without-weak-pairwise-links.md`
  - `docs/decisions/design/ADR-D-0014-represent-associative-membership-lifecycle-separately.md`
  - `docs/decisions/implementation/ADR-I-0011-guard-against-low-information-co-occurrence-links.md`
  - `docs/decisions/implementation/ADR-I-0014-use-graph-internal-associative-units.md`
  - `docs/design/database/graph_schema_design.md`
  - `docs/design/database/schema_cheat_sheet.md`
  - `src/internal/repositories/link_pipeline.rs`
  - `src/internal/repositories/retrieve_pipeline.rs`
  - `src/internal/repositories/test_support.rs`
  - `src/api/types/retrieval.rs`
- Existing patterns or references:
  - Durable relationships are written through graph-authoritative repository paths.
  - Current public `link()` calls may represent explicit caller intent and should not be silently treated as weak automatic co-occurrence without a decision.
  - v0.5 owns controlled associative recall and clustering; v0.1.2 should only add guardrails.
  - Latest main distinguishes weak association evidence from ordinary durable relationship truth and reserves future serendipitous recall for graph-internal `AssociativeUnit`/`AssociativeMembership`/`AssociationSupport` structures.
- Repo reference docs consulted:
  - `docs/coding-agent/rules/common.md`
  - `docs/coding-agent/rules/orchestrator.md`
  - `docs/coding-agent/lessons.md`

## Open Questions (max 3)
- None.

## Resolved Decisions
- Apply the low-information guard to automatic or inferred association candidates, and to weak `AssociatedWith`-style links when the only evidence is low-selectivity co-occurrence. Do not block explicit caller-authored public `link()` calls by default.
- Represent link admission evidence with an internal evidence model such as `ExplicitCallerIntent`, `SameThread`, `Correction`, `Temporal`, `SemanticSupport`, `SelectiveEntity`, and `LowSelectivityCoOccurrenceOnly`. Explicit caller intent may bypass low-information rejection but still must pass graph/lifecycle validation.
- Count/store rejected low-information link diagnostics internally in stats diagnostics first. Surface aggregate counts in retrieval telemetry only when they explain a retrieval result; do not add a broad public diagnostics API in this phase.
- Do not persist reusable weak associative hints, associative units, associative memberships, association support records, query-time activation state, or cluster membership lifecycle in v0.1.2. Weak co-occurrence can be counted or diagnosed as rejected evidence, but it must not become ordinary pairwise relationship truth or a separate memory truth surface.

## Assumptions
- A1: Stats foundation and selectivity fanout plans are complete before this plan starts.
- A2: Explicit application-authored links are stronger evidence than automatic raw co-occurrence.
- A3: The initial guard should be conservative and narrow so it does not preempt v0.5 association design.
- A4: Diagnostics are report-only and do not repair graph/vector/stats stores.
- A5: Public explicit link calls still pass existing graph/lifecycle validation; explicit intent only bypasses low-information co-occurrence rejection.
- A6: Future graph-internal associative structures remain under Oxigraph authority and are outside this v0.1.2 plan.

## Tasks

### Task_1: Record Link Guard Boundary And Admission Evidence
- type: design
- owns:
  - `docs/coding-agent/plans/active/v0-1-2-low-information-link-guard-plan.md`
- depends_on: []
- description: |
  Record and verify the approved initial guard boundary before code changes: which link creation paths are guarded, how explicit caller intent is represented, and where rejected-link diagnostics are reported.
- acceptance:
  - Plan records that automatic/inferred association candidates and weak `AssociatedWith`-style links are guarded when the only evidence is low-selectivity co-occurrence.
  - Plan records that explicit caller-authored public `link()` calls are not blocked by default, but still pass graph/lifecycle validation.
  - Plan records the internal admission evidence model and diagnostics location.
  - Decision preserves v0.5 controlled associative recall and clustering as future work.
  - Decision records that rejected weak co-occurrence diagnostics do not create reusable weak hint storage or graph-internal associative structures in this phase.
- validation:
  - kind: review
    required: true
    owner: orchestrator
    detail: "Decision Log updated with link guard boundary, tradeoffs, and impact on Task_2/Task_3 owns scopes"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review boundary against ADR-I-0011 before implementation dispatch"

### Task_2: Implement Low-Information Co-Occurrence Guard
- type: impl
- owns:
  - `src/internal/repositories/link_pipeline.rs`
  - `src/internal/repositories/retrieval_selectivity.rs`
  - `src/internal/repositories/retrieval_stats_store.rs`
  - `src/internal/repositories/test_support.rs`
- depends_on: [Task_1]
- description: |
  Add link admission logic that rejects durable pairwise links when the only evidence is shared low-selectivity entity/relation co-occurrence, while allowing explicit caller intent or stronger-evidence links.
- acceptance:
  - Low-selectivity shared entity/relation alone cannot create an automatic or weak `AssociatedWith`-style durable pairwise association.
  - Explicit application-created public links remain accepted by default when they pass graph/lifecycle validation.
  - Internal admission evidence distinguishes explicit caller intent, same thread, correction/supersession, temporal relation, semantic support, high salience where represented, selective shared entity, and low-selectivity co-occurrence only.
  - Stronger evidence paths remain allowed where represented.
  - Rejections are counted or reported without mutating graph authority.
  - Rejections do not persist weak associative hints, associative units, associative memberships, association support records, activation state, or cluster lifecycle state.
  - Guard behavior is relation/object-specific and does not check entity identity or app role.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test link_pipeline low_information --no-fail-fast"
  - kind: review
    required: true
    owner: reviewer
    detail: "Review guard behavior for explicit-intent preservation and no v0.5 association overreach"

### Task_3: Add Rejected-Link Diagnostics And Entity-Neutral Link Fixtures
- type: test
- owns:
  - `src/internal/repositories/link_pipeline.rs`
  - `src/internal/repositories/test_support.rs`
  - `src/api/types/retrieval.rs`
- depends_on: [Task_2]
- description: |
  Add diagnostics for rejected low-information link candidates and fixtures/tests showing the guard works across heterogeneous entity types.
- acceptance:
  - Internal stats diagnostics count rejected low-information co-occurrence candidates.
  - Retrieval telemetry includes aggregate rejected-link counts only when those counts explain a retrieval result.
  - Fixtures cover broad person, place, project, topic/concept, object/tool/document, and arbitrary custom entities.
  - Tests prove the guard treats broad non-user/non-assistant entities the same as broad assistant-domain entities.
  - Tests prove explicit or stronger-evidence links are accepted without identity-specific exceptions.
  - Diagnostics remain report-only and do not repair or override stores, introduce a broad public diagnostics API, or become reusable weak associative hint storage.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo test link_pipeline entity_neutral low_information --no-fail-fast"
  - kind: search
    required: true
    owner: reviewer
    detail: "Search link/retrieval/policy code for identity-specific checks against user/assistant/player/NPC/protagonist/main-character style roles"

### Task_4: Full v0.1.2 Closeout Validation
- type: review
- owns: []
- depends_on: [Task_2, Task_3]
- description: |
  Run final milestone validation across the three v0.1.2 feature plans after stats foundation, selectivity fanout, and link guard work all pass their task validations.
- acceptance:
  - Stats remain derived policy metadata and never decide graph truth.
  - Retrieval fanout is continuous, relation/object-specific, capped, and conservative on missing/unhealthy stats.
  - Durable link guard prevents weak broad-entity pairwise growth without blocking explicit/stronger-evidence links.
  - Entity-neutral acceptance tests cover retrieval and link behavior.
  - No persisted selectivity categories, graph centrality, learned policy, admin dashboard, or identity-specific special cases are introduced.
  - No v0.5 controlled associative recall structures or separate weak hint store are introduced.
  - Reviewer status is APPROVED.
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
  - kind: command
    required: true
    owner: orchestrator
    detail: "cargo test --lib"
  - kind: review
    required: true
    owner: reviewer
    detail: "Final v0.1.2 implementation review against roadmap and ADRs"

## Task Waves (explicit parallel dispatch sets)

- Wave 1 (parallel): [Task_1]
- Wave 2 (parallel): [Task_2]
- Wave 3 (parallel): [Task_3]
- Wave 4 (parallel): [Task_4]

## E2E / Visual Validation Spec

- Not applicable. No UI/user-flow changes.

## Rollback / Safety
- The guard should be implemented as a narrow admission check so it can be tuned or disabled by relation policy if validation shows it blocks explicit application intent.
- Rejected-link diagnostics are report-only and must not mutate graph/vector/stats stores.
- Weak co-occurrence diagnostics must remain counters/evidence for review, not durable relationship truth, reusable weak hint storage, or associative unit/membership/support state.

## Progress Log (append-only)

- 2026-05-09 Draft created:
  - Summary: Drafted the low-information link guard and final entity-neutral hardening plan as the third v0.1.2 feature plan.
  - Validation evidence: Researcher identified link guard scope ambiguity and affected files; no implementation dispatched.
  - Notes: This plan intentionally starts with a design boundary task before implementation.
- 2026-05-10 Open questions resolved:
  - Summary: Recorded approved recommendations for guarded link scope, explicit-intent evidence modeling, and rejected-link diagnostics location.
  - Validation evidence: Plan-only update; `git diff --check -- docs/coding-agent/plans/active` pending.
  - Notes: No implementation dispatched.
- 2026-05-10 Plan approved for execution:
  - Summary: User requested each plan be committed on its own implementation branch and readied for execution.
  - Validation evidence: Plan status updated to approved; implementation remains pending.
  - Notes: No Worker tasks dispatched yet.
- 2026-05-10 Main roadmap refresh reviewed:
  - Summary: Updated plan after latest main clarified controlled associative recall, graph-internal associative units, member lifecycle, and no weak pairwise durable links.
  - Validation evidence: Plan-only update; branch rebased onto `origin/main`.
  - Notes: v0.1.2 remains a guardrail phase and does not implement future associative recall structures.
- 2026-05-13 Completed:
  - Summary: PR #50 branch was updated onto merged PR #49, conflicts were resolved, and the low-information link guard was reviewed against the final v0.1.2 authority, scope, and entity-neutrality boundaries.
  - Validation evidence: `cargo fmt --check`; `cargo check --lib`; `cargo test low_information --no-fail-fast`; `cargo test link_pipeline --no-fail-fast`.
  - Notes: Plan moved from active to completed before PR #50 merge so no residual v0.1.2 active implementation plan remains.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-05-09 Decision:
  - Trigger / new insight: ADR-I-0011 requires guarding weak co-occurrence links, but current public `link()` may represent explicit caller intent.
  - Plan delta (what changed): Added a required boundary task before link guard implementation.
  - Tradeoffs considered: Implementing the guard immediately could surprise explicit callers or accidentally implement v0.5 association policy too early.
  - User approval: no
- 2026-05-10 Decision:
  - Trigger / new insight: User approved the recommended answers to all low-information link guard open questions.
  - Plan delta (what changed): Resolved guard scope to automatic/inferred associations and weak `AssociatedWith`-style low-selectivity-only links, preserved explicit caller-authored public links by default, added an internal admission evidence model, and kept rejected-link diagnostics internal-first.
  - Tradeoffs considered: This protects against broad-entity pairwise growth without surprising explicit public callers or prematurely implementing v0.5 association policy.
  - User approval: yes
- 2026-05-10 Decision:
  - Trigger / new insight: Latest main adds ADR-D-0013, ADR-D-0014, ADR-I-0014, and schema docs for controlled associative recall without weak pairwise durable links.
  - Plan delta (what changed): Updated v0.5 terminology, added ADR/schema references, and clarified that rejected weak co-occurrence diagnostics must not persist weak hints or future associative unit/membership/support structures in v0.1.2.
  - Tradeoffs considered: The guard should preserve graph quality now while leaving serendipitous recall to future graph-authoritative associative structures.
  - User approval: yes

## Notes
- Risks:
  - Guard scope is ambiguous until explicit caller intent is resolved.
  - Diagnostics could creep toward v0.4/v0.5 observability if not kept report-only.
  - Rejected weak co-occurrence counts could accidentally become a separate weak hint store if implementation stores reusable candidate associations instead of report-only diagnostics.
  - Tests must avoid proving only personal-assistant examples.
- Edge cases:
  - Two memories share a broad project but are also in the same active thread.
  - Two records share a selective entity but low semantic similarity.
  - Explicit caller-provided association through a broad entity.
  - Suppressed or non-current objects participating in a candidate link.
