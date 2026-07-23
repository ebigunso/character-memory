# Lessons Log (Coding Agent)

Purpose:
- capture recurring mistakes and the prevention mechanism
- enable "read once, don't repeat" improvements

## How to use
- Append a new entry after any user correction or significant miss.
- Keep entries short and actionable.
- Promote repeated/high-severity lessons into repo rules, first-party skills/references, or troubleshooting knowledge.

## Tags (recommended)
- planning
- validation
- delegation
- review
- ui-e2e
- tooling
- ci
- scope-owns

## Entries

## 2026-07-04 - Route Worker Dispatches Through The agmsg Codex Worker  [tags: delegation, workflow, tooling]

Context:
- Plan: responsibility-boundary module reorg
- Task/Wave: Wave 2→3 transition
- Roles involved: Orchestrator | Worker

Symptom:
- Orchestrator dispatched Wave 1 and Wave 2 Worker tasks as directly spawned harness-worker subagents; user redirected mid-execution: Worker tasks should go to the spawned codex `worker` agent via agmsg instead.

Root cause:
- A codex worker agent had been spawned into the CharacterMemory agmsg team earlier in the session, but the Orchestrator defaulted to the harness's built-in subagent spawn path without considering the user's standing multi-agent setup.

Fix applied:
- Sent the codex worker a standing role instruction (assume harness-worker behavior: one Task_X per dispatch, owns-scope only, required validation with evidence, no git mutations, strict YAML report back via agmsg) and dispatched Task_3 through agmsg.

Prevention:
- In this workspace, when an agmsg team has a live `worker` agent, dispatch Worker-role tasks to it via agmsg (dispatch message names the plan file, Task_X, owns scope, validation commands, and the YAML report contract). Researcher/Reviewer/other subagents may still be spawned directly unless the user says otherwise.

Evidence:
- User instruction on 2026-07-04 during Wave 2; role instruction + Task_3 dispatch sent to `worker` in team CharacterMemory.

## 2026-07-04 - Bounded Expansion Has Two Semantically Distinct Flavors, Do Not Force-Merge  [tags: planning, architecture]

Context:
- Plan: responsibility-boundary module reorg
- Task/Wave: Task_2 (ports/policy extraction)
- Roles involved: Worker | Orchestrator

Symptom:
- The plan hoped to unify the bounded-expansion algorithm into one implementation, but inspection showed the adapter-side flavor is semantically distinct, not duplicated.

Root cause:
- `bounded_expansion` computes a complete plan over fully materialized objects+links (lifecycle filtering, deterministic ordering), while `bounded_incident_link_refs` is a per-node pre-hydration pruning pass over lightweight link refs that runs before objects exist to filter on.

Fix applied:
- Both flavors colocated in `src/policy/graph_expansion.rs` with a module comment stating the semantic difference; the genuinely duplicated primitives (relation/object-type filters, hub-limit handling, bounded-failure error construction) were unified.

Prevention:
- True unification would require reshaping the adapter BFS to feed the plan algorithm lazily — a behavior-adjacent redesign, not a mechanical move. Keep it out of refactor waves; treat as residual design debt if it ever matters.

Evidence:
- Task_2 Worker report deviation record; full validation suite green with unchanged test count (363 passed).

## 2026-07-03 - Local-Only gRPC Mutation Stall After Idle Is A Known Environment Constraint  [tags: tooling, validation, ci]

Context:
- Plan: stabilize v0.1.2 retrieval guardrails
- Task/Wave: Task_3b/3c/3d diagnosis chain
- Roles involved: Orchestrator | Worker

Symptom:
- On this development machine, the first Qdrant gRPC MUTATION after ~10s of wall-clock idle stalls to the 30s client deadline (often 60s with the client's automatic retry) and fails with "operation was cancelled: Timeout expired". Reads after idle stay fast; Python gRPC and REST clients are immune; requests do reach the server.

Root cause:
- Not established in code. Experimentally excluded: Docker Desktop port proxy (fails with host networking and with a native Windows Qdrant binary), tokio runtime starvation (fails on multi-thread runtime), stale client/channel state (fresh client also fails), dependency and toolchain drift (June-green lockfile unchanged). Remaining suspects are in the machine's loopback stack interacting with tonic/h2 mutation traffic.

Fix applied:
- None in production code (correctly so). A live-gated #[ignore] canary test (qdrant_channel_survives_idle_gap_before_mutating_upsert) encodes the failure signature; CI on Linux is the authoritative green signal.

Prevention:
- If guardrail/facade integration tests fail locally at the remember stage with vector_indexing_failure timeouts, run the canary test first; if it fails, treat the machine as affected and rely on CI instead of re-diagnosing.
- Re-run the canary after Docker Desktop, Windows, or network-stack updates to detect recovery or regression.

Resolution (2026-07-03):
- A full OS reboot resolved the condition. Immediately after boot the canary showed a transitional mode (post-idle upsert succeeded on retry in ~40s); after the system settled, the canary passes (<1s post-idle upsert) and the full local cargo test suite is green (guardrail tests 4.6s for all three, previously 45–70s each). Root cause remains unpinned but is confirmed to live in transient host networking state that survives Docker restarts and daemon recreation but not a reboot. If symptoms recur: run the canary, and reboot before deeper diagnosis.

Evidence:
- Full falsification matrix in the stabilization plan Decision Log (entries 2–6).

## 2026-06-12 - Constrain Graph Roots When Asserting Entity-Root Fanout  [tags: planning, validation]

Context:
- Plan: v0.1.2 closeout divergence fixes
- Task/Wave: Task_3 facade integration tests
- Roles involved: Worker

Symptom:
- A fanout-override assertion failed because returned derived memories were also reachable through additional vector roots, masking the entity-root fanout constraint.

Root cause:
- The retrieval context allowed multiple graph roots while the test intended to isolate entity-root selectivity behavior.

Fix applied:
- Limited the test retrieval context to a single selected entity graph root.

Prevention:
- Facade tests for entity-root-only selectivity should constrain graph root selection enough to isolate the entity root under test.

Evidence:
- tests/retrieval_guardrails_tests.rs fanout scenario passes with traced fanout and result-count assertions.

## 2026-06-12 - Start Qdrant Before Full cargo test Validation  [tags: validation, tooling]

Context:
- Plan: v0.1.2 closeout divergence fixes
- Task/Wave: Task_1 required validation
- Roles involved: Worker

Symptom:
- `cargo test` failed in tests/initialization_tests.rs because Qdrant was configured but unreachable at localhost:6334; the failure surfaced as a wrapped Qdrant transport error instead of a clean skip.

Root cause:
- Live-gated integration tests can fail rather than skip when Qdrant configuration resolves but the service is down.

Fix applied:
- Started Qdrant with `docker compose -f docker-compose.qdrant.yml up -d` and reran the exact required validation command.

Prevention:
- Before full `cargo test` validation, verify local Qdrant is up (`docker compose -f docker-compose.qdrant.yml ps`) and start it if needed.

Evidence:
- Final validation runs in Task_1, Task_3, and Task_4 all passed with Qdrant running.

## 2026-05-09 - Triage Copilot Review Comments Against Current Diff  [tags: review, ci, assumptions]

Context:
- Plan: PR #46 Rust CI path filters
- Task/Wave: Copilot review comment remediation
- Roles involved: Orchestrator

Symptom:
- Treated sequential Copilot comments as potentially contradictory because a later comment asked to include `build.rs` after an earlier comment asked to remove it.

Root cause:
- Assumed Copilot review passes consider prior review context, instead of recognizing each pass reviews the current diff independently.

Fix applied:
- Chose the long-term CI-correct filter: include Rust build/config inputs such as `build.rs`, `.cargo/**`, `rustfmt.toml`, and `clippy.toml`, even if some do not exist yet.

Prevention:
- When Copilot comments appear to contradict earlier Copilot feedback, triage each comment against the current diff and the durable repo outcome, not against previous Copilot review history.

Evidence:
- PR #46 path filters now include future Rust build/lint/format configuration inputs.

## 2026-05-09 - Keep ADR Context Focused On The Decision  [tags: documentation, adr, assumptions, output-contract]

Context:
- Plan: v0.1.3 remember intake and assisted remember roadmap docs
- Task/Wave: ADR wording correction after documentation integration
- Roles involved: Orchestrator

Symptom:
- ADR-I-0012 opened with the exact rejected commit-mode names, making alternatives feel like the central context rather than supporting considered options.
- New ADRs also referred to roadmap phases primarily by version numbers, which made them less self-contained.

Root cause:
- Copied too much hand-off comparison language directly into ADR context and leaned on roadmap version labels instead of the phase names that explain the concepts.

Fix applied:
- Rewrote ADR-I-0012 context to focus on why prepare / validate / commit was chosen.
- Moved rejected workflow shapes into the considered-options discussion.
- Replaced version-number shorthand in the new ADRs with roadmap phase names.

Prevention:
- When adding ADRs from a hand-off, keep context centered on the decision pressure and put rejected alternatives under considered options.
- Prefer roadmap phase names over bare version numbers in ADR prose, especially in context, consequences, and revisit sections.

Evidence:
- New ADRs no longer contain `v0.1.3`, `v0.6`, or exact rejected commit-mode names.

## 2026-04-30 - Treat Cleanup Chunks As Completion Work When Roadmap Says Migration Cleanup  [tags: planning, scope-owns, assumptions]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-documentation-migration-cleanup-release-validation-plan.md`
- Task/Wave: pre-implementation plan review and replan
- Roles involved: Orchestrator | Researcher | Worker | Reviewer

Symptom:
- Initially interpreted the documentation/migration cleanup step as retaining the legacy public constructor/create/search/read path while only removing or isolating the hardest update/delete conflicts.
- User clarified that the step should leave the project fully migrated to the new architecture and that new implementation should be added if needed.

Root cause:
- Overweighted the current code shape and the active plan's transitional open questions instead of treating the roadmap phrase "migration cleanup" as a completion gate for the v0.1 public architecture.
- Did not immediately convert the user's "implement the step" request into a requirement that the public surface match the landed internal graph/vector/embedder architecture.

Fix applied:
- Replanned Task_3 to require public graph/vector/embedder constructor/facade wiring, removal of the old flat public facade, deletion of legacy repository modules and flat DTO re-exports, and replacement of legacy integration tests with public v0.1 facade tests.

Prevention:
- Before executing a cleanup/release-validation chunk, explicitly ask: "What must no longer exist after this step?" and compare that against the roadmap expected outcome.
- If the roadmap says old architecture concepts are retired or removed, do not preserve them as transitional unless the user explicitly accepts a deferred migration boundary.

Evidence:
- User correction on 2026-04-30 redirected the plan from transitional retention to full public migration, and the completed plan now records the scope correction.

## 2026-04-30 - Check Roadmap Functionality Before Narrowing Scope  [tags: planning, scope-owns, validation]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-correction-forget-lifecycle-plan.md`
- Task/Wave: pre-implementation plan review
- Roles involved: Orchestrator | Researcher | Reviewer

Symptom:
- Narrowed the lifecycle plan to derived-memory-only correction/forget behavior before fully reconciling the chunk with the development roadmap and v0.1 roadmap.
- The narrowed plan would have left episode/observation forget cascades and correction-origin provenance under-specified despite roadmap expectations for `correct`, `forget`, suppression, and correction provenance.

Root cause:
- Overweighted current implementation convenience and code-shape constraints before checking the intended functional acceptance for the roadmap chunk.
- Focused on which objects were easiest to mutate, not enough on whether forgotten source material could still influence generation through provenanced derived memories.

Fix applied:
- Rechecked the development roadmap, v0.1 design, backend-contract draft, ADR-D-0002, and ADR-D-0008.
- Broadened the plan to include episode/observation suppression with default provenance-based cascade, source-object correction of affected derived memories, memory-thread archival, and explicit correction-origin provenance.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: Before narrowing an implementation plan for feasibility, explicitly compare the narrowed scope against roadmap/design acceptance and record which intended features remain in scope, are deferred, or require user approval.
- Dispatch/plan guardrail:
  - For correction/forget plans, check both provenance chains before approval: original source provenance and correction-origin provenance.

Evidence:
- User correction on 2026-04-30 prompted roadmap recheck and plan revisions in `docs/coding-agent/plans/active/v0-1-correction-forget-lifecycle-plan.md`.

## 2026-04-28 - Distinguish Temporary And Durable Code Comments  [tags: code-quality, communication, architecture]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-remember-and-link-pipelines-plan.md`
- Task/Wave: plan decision refinement before implementation
- Roles involved: Orchestrator

Symptom:
- The user clarified that comments should communicate whether a structure is temporary migration scaffolding or durable production API/design surface.

Root cause:
- Planning could otherwise treat all comments as generic explanation, leaving future Workers/Reviewers unsure which code should be removed later and which code is intended to survive the complete v0.1 refactor.

Fix applied:
- Updated the remember/link plan to require removal-condition comments for temporary scaffolding and stable production-ready comments for durable injectable constructor/API structures.

Prevention:
- When adding comments during v0.1 refactor work, explicitly choose the comment category: temporary comments name when to remove/change the code; durable comments describe stable intent without implying future cleanup.
- Reviewers should flag transitional comments without removal conditions and durable API comments that read like temporary scaffolding.

Evidence:
- Active remember/link plan now includes resolved decision and Task_1/Task_5 acceptance coverage for temporary-vs-durable comment guidance.

## 2026-04-28 - Avoid Separate Skipped Checks For CI Rationale  [tags: ci, review, communication]

Context:
- Plan: PR #29 CI trust-gated integration test follow-up
- Task/Wave: PR review follow-up
- Roles involved: Orchestrator

Symptom:
- Added a separate `integration_tests_skipped` job to explain why live integration tests do not run for fork/Dependabot PRs.
- User clarified that surfacing the explanation as its own skipped check is confusing.

Root cause:
- Treated visible CI explanation as equivalent to a dedicated check, without considering how that extra check appears in the PR status UI.

Fix applied:
- Removed the separate skipped-check job and moved the rationale into comments on the actual live integration-test job.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: Prefer inline workflow comments or existing job/step logs for CI rationale; do not add separate skipped check jobs solely for explanation unless the user wants that PR checks UI.
- Dispatch/plan guardrail:
  - When adding skipped CI jobs, explicitly consider whether the extra check improves or clutters the PR status surface.

Evidence:
- PR #29 follow-up removed `integration_tests_skipped` and kept the trust-gating rationale near the `integration_tests` job condition.

## 2026-07-17 - Assess Memory-Type Contribution Before Tuning Away "Pollution"  [tags: planning, assumptions, validation]

Context:
- Plan: v0.1.5 eval-driven closeout
- Task/Wave: Task_4 disposition gate (F-BASE-2)
- Roles involved: Orchestrator

Symptom:
- Proposed disposition "fix pollution via parameter tuning" treated the eval pollution metric's relevance labels as ground truth and multiple same-event surfaces (episode + observation + derived memory) as duplicate noise to cut.

Root cause:
- Conflated metric-labeled noise with actual continuity noise. The product goal is character continuity — an observation surface can carry the character's inner reading of an event while the episode carries facts; dropping surfaces by knob-tuning before understanding per-type behavioral contribution optimizes the metric, not the product.

Fix:
- User redirected: before any pollution-targeted tuning, analyze which memory object types/surfaces genuinely shape current character behavior from past memories and which are noise; re-examine fixture relevance labels in the same light.

Prevention:
- Plan guardrail: retrieval-quality findings get a memory-type contribution analysis task (philosophy-grounded, trace-based) BEFORE any tuning task consumes them; tuning targets derive from that analysis, not raw metric deltas.
- When a metric disagrees with the product goal's framing, treat the metric's labels as a finding candidate too (fixture semantics), not only the system under test.

## 2026-07-17 - Route Memory-Quality Fixes To The Write Path, Not Retrieval  [tags: planning, assumptions]

Context:
- Plan: v0.1.5 eval-driven closeout
- Task/Wave: Task_12 review (F-BASE-2 fix shape)
- Roles involved: Orchestrator

Symptom:
- Recommended same-event echo dedup in pack assembly (retrieval-time collapsing of identical-text sibling surfaces) as the F-BASE-2 fix.

Root cause:
- Treated the symptom location (bloated packs) as the fix location. The project's append-only stance extends to retrieval fidelity: packs reflect what was committed; silently manipulating them post-write hides data problems from the caller who owns them.

Fix:
- User ruling: enforce at the write path — validation warns on known recall-harming failure modes (echo-duplicate surfaces; cascade-would-suppress-current-replacement), refusal reserved for very critical cases.

Prevention:
- Durable project principle recorded (auto-memory + this entry): retrieval-quality fix proposals route to write-plan validation diagnostics or lifecycle-mutation warnings, never to retrieval/pack post-processing.

## 2026-07-21 - Checked Incidental "Legacy" Phrasing, Not The Design Record  [tags: review, planning, delegation]

Context:
- Plan: backcompat-sweep-plan; item E (remember() facade)
- Roles involved: Orchestrator | Worker

Symptom:
- Orchestrator approved removing the public remember() facade because an inventory cited phase-doc phrasing calling it "legacy/source-compatible"; the user vetoed — remember() is the intended consumer convenience API wrapping prepare/validate/commit.

Root cause:
- The removal ruling was made from the forensic inventory's evidence alone without consulting the design-intent record (philosophy §9.1, ADR-I-0012, roadmap), which unambiguously specifies remember() as a first-class surface; "legacy" in the phase doc described the shipped internals/signature, not the surface.

Fix applied:
- E reclassified as rework: implement remember(RememberInput, RememberOptions) as the thin prepare→validate_plan→commit composition per ADR-I-0012; remove only the divergent pre-plan-era pipeline.

Prevention:
- Before ruling any public API surface removable, check it against philosophy/ADRs/roadmap intent, not just code-adjacent comments; forensic inventories (Codex) establish what exists, the design record (orchestrator altitude) decides what it means. Word-level markers like "legacy" in historical phase docs describe their moment, not current intent.

## 2026-07-21 - Typed Error Contracts Must Survive Producers, Serde, And Test Gates  [tags: review, validation, errors]

Context:
- Plan: structured-verdict-observability; PR #65 review fixes
- Roles involved: Worker | Reviewer

Symptom:
- Closed error vocabularies were flattened or made unserializable at three different boundaries: a public provider trait accepted broad `CustomError`, graph-mode validation passed through serde prose, and a primitive newtype variant could not serialize under an internal tag. Separately, the Qdrant skip gate recovered transport meaning by matching rendered error text.

Root cause:
- Type design was checked at enum declarations but not end to end through producer signatures, adapter normalization, serde representation, and control-flow consumers. Representative tests constructed only a subset of variants and the service-down control had not been run after removing prose matching.

Fix applied:
- Retyped the provider boundary to `EmbeddingError`, preserved graph-mode validation before serde flattening, made every internally tagged variant structurally serializable, normalized Qdrant connection failures inside the adapter, and made skip gating consume typed transport classifications only. Exhaustive per-variant serde coverage and both service-down/service-up controls now enforce the contract.

Prevention:
- For every closed error vocabulary, audit four surfaces together: producer return type, adapter conversion, serialization of every variant, and downstream branching. Coverage must be compiler-exhaustive, unknown fallbacks must use opaque markers or representation-frozen tokens rather than Debug output, and regression tests must traverse production wiring instead of testing only the extracted helper. Any skip/retry/fallback predicate must consume typed classification, and its verification must include both a forced-failure control and a successful exercised path.

## 2026-07-22 - Typed Observability Must Include Persistence And Failure Multiplicity  [tags: review, validation, errors, configuration]

Context:
- Plan: structured-verdict-observability; final thesis audit
- Roles involved: Worker | Reviewer

Symptom:
- Follow-up fixes left stats causes as prose at the graph and stats ports, persisted a rendered cause in health metadata, discarded a second simultaneous stats failure, classified one upstream-erased transport error through an undocumented prefix, and special-cased one configuration field with a pre-read before deserializing the full settings object again.

Root cause:
- The typed-contract audit stopped at the immediate public enum and did not trace the same information through producer signatures, durable state, multi-error aggregation, external dependency loss, and configuration admission.

Fix applied:
- Closed graph-query and stats-store error vocabularies now cross their ports, health metadata persists a typed operation and error, stats failures and repair markers retain every observed cause, the unavoidable qdrant-client 1.17.0 prefix dependency is ruled and pinned by a canary, and settings deserialize once into a raw representation before structured conversion and validation.

Prevention:
- For typed observability changes, review a cause matrix from producer to adapter, persistence, public DTO, serde, and every branching consumer; include simultaneous-failure tests wherever operations can continue after an earlier failure. Treat external prose coupling as an exception requiring an exact upstream citation, version marker, drift canary, and retirement condition. Configuration admission must deserialize once into raw data and perform semantic parsing in one typed conversion rather than pre-reading individual fields.

## 2026-07-23 - Observability Phase Closeout Batch  [tags: review, validation, delegation, errors]

Consolidated from sixteen worker/reviewer/audit lesson candidates accumulated across the phase (full bodies in agmsg history 2026-07-21/23):

- Typed-from-introduction (three enforced recurrences before it stuck; now a worker.md rule): a new validator classifies its failures with an owned structured error at introduction, tests asserting variants/fields — never anyhow prose retrofitted later.
- Enforcement claims need per-branch negative evidence: a five-branch validator with one tested branch is four untested claims; parametrized tamper coverage per branch (staged as a harness candidate earlier, confirmed by recurrence).
- Idempotency/retry regressions must reuse the same mutated store and exercise resolution-driven targets; fresh-store/direct-ID tests miss read-after-write identity drift. Convergence tests must separate graph authority from replayable derived-store work and inspect actual stale state after partial failure.
- Shared operation identity requires explicit attempt identity before dedup/counting; family-wide invariants (writer preflight, admission strictness) need a producer/reader/writer sibling census before closure claims.
- Rules promoted from single incidents get the evidenced scope, not the broadest phrasing (Tier A lesson, applied to the reader-strictness rule).
- Validation-table triggers should encode intent, not file paths: the two-run gate fires on changes that can alter successful artifact bytes (refined at closeout after a correct procedural hold on a failure-path-only change).
- Tooling: agmsg send.sh takes exactly four positionals; Windows Git-Bash invocations need /usr/bin:/bin prepended; zero-executed --exact filters remain the most-recurred evidence bug of the phase (rule already exists — count: 5).

## Promotion drain note (2026-07-23)

Drained after agent-harness v0.9.0 went live in this workspace (installed plugin + Codex profiles updated 2026-07-23); each prevention now exists verbatim-or-stronger in harness content: Dispatch Research Before Broad Discovery (orchestration-harness Research Dispatch Gate), Replan Before Implementation Direction Changes (Replan Triggers + lifecycle-gates), ADRs Are Orchestrator/Claude-Authored (subagent-strategy model-routing), Equivalence Tests Must Compare The Full Observable Contract (review-latent-risk-conservation + owning-surface assertion line), Constraint-Induced Workarounds Need A Tripwire (Drift Tripwires + dispatch escape hatch + Escalation Ruling).

## Repo-rule promotion drain note (2026-07-23)

Promoted into this repo's rule suite and removed from this log (per-lesson triage against harness promotion guidelines, agmsg 2026-07-23T12:17Z): Qdrant client-vs-server timeout (worker.md), branch-naming convention x2 (orchestrator.md), shared sibling-checkout serialization x2 (orchestrator.md), production-default constructor tracing (worker.md), pruning-closeout evidence set (reviewer.md).

## Purge note (2026-07-23)

Eleven entries purged per the user-directed low-value/invalid sweep (Codex purge map, agmsg 2026-07-23T12:28Z): ten PURGE-LOW-VALUE (restatements of now-mandatory harness/rule content — plan-format task records, PR monitoring, canonical-byte verification, compatibility policy, module layout, evidenced-scope rulebook default, parallel dispatch — plus two cheaply rediscovered one-off quirks and one unstructured batch-notes bundle) and one PURGE-INVALID (the phase-bounded v0.1 compatibility ruling, superseded by the repo-wide Compatibility Policy). Full entries recoverable from git history at 4997bdc.
