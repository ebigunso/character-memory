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

## 2026-07-03 - Qdrant Builder .timeout() Is A Server-Side Operation Parameter, Not A Client Bound  [tags: tooling, validation]

Context:
- Plan: stabilize v0.1.2 retrieval guardrails
- Task/Wave: Task_3
- Roles involved: Worker | Orchestrator

Symptom:
- Qdrant mutating operations (upsert/delete/scroll) were several seconds slower than raw REST/gRPC probes of the same operations.

Root cause:
- UpsertPointsBuilder/DeletePointsBuilder/ScrollPointsBuilder `.timeout(secs)` sets the proto request field (a server-side operation timeout), not a client deadline. The client deadline is configured once via QdrantConfig::timeout. Setting the per-request field added measurable server-side wait overhead per call.

Fix applied:
- Removed per-request `.timeout()` from mutation/scroll builders; kept the 30s client-level QdrantConfig timeout (isolated upsert: 2.41s → 0.048s).

Prevention:
- Bound Qdrant client calls via QdrantConfig::timeout only; use per-request `.timeout()` solely when a specific server-side operation limit is intended and probe-verified.

Evidence:
- Live probe timings recorded in the stabilization plan Decision Log; config unit test pins client-level timeout.

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

## 2026-06-12 - Cast `usize` To `i64` For `config` Crate `set_override` In Test Settings  [tags: tooling, validation]

Context:
- Plan: v0.1.2 closeout divergence fixes
- Task/Wave: Task_3 facade integration tests
- Roles involved: Worker

Symptom:
- Integration test compilation failed when passing `usize` values to `config::ConfigBuilder::set_override`.

Root cause:
- The config crate's `Value` conversion supports signed integer types such as `i64` but not `usize`.

Fix applied:
- Cast fanout override values to `i64` before calling `set_override`.

Prevention:
- When constructing test `Settings` through `config::ConfigBuilder`, cast `usize` numeric overrides to `i64` or another supported config value type.

Evidence:
- tests/test_utils.rs helper compiles and full validation passes after the cast.

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

## 2026-05-10 - Dispatch Research Before Broad Discovery On Non-Trivial Work  [tags: workflow, planning, delegation]

Context:
- Plan: controlled associative recall docs integration
- Task/Wave: pre-plan repository discovery
- Roles involved: Orchestrator | Researcher

Symptom:
- Ran a broad `rg --files docs` discovery command before dispatching the required Researcher for a non-trivial documentation integration.

Root cause:
- Treated docs discovery as harmless setup after loading the harness, instead of applying the non-trivial Research Dispatch Gate immediately.

Fix applied:
- Dispatched a Researcher before further product-doc exploration and limited subsequent work to plan drafting pending user approval.

Prevention:
- For non-trivial requests, classify the request and dispatch at least one Researcher before any repo-wide discovery outside `docs/coding-agent/**`.
- Turn-closing guardrail: before ending a planning turn, confirm the Research Dispatch Gate was satisfied or explicitly waived as trivial.

Evidence:
- Researcher produced the local roadmap, phase-doc, database-doc, philosophy, and ADR-numbering map used by the active plan.

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

## 2026-05-03 - Keep Schema References Separate From Configuration Docs  [tags: documentation, scope-ownership, planning]

Context:
- Plan: v0.1.1 persistent graph authority
- Task/Wave: documentation follow-up after PR creation
- Roles involved: Orchestrator

Symptom:
- Added `GRAPH_STORE_MODE` and `OXIGRAPH_CONNECTION_STRING` explanations to `docs/design/database/schema_cheat_sheet.md`.
- User clarified that the schema cheat sheet should reference actual database designs, not general database-related configuration.

Root cause:
- Treated all database-adjacent setup information as acceptable for the schema reference instead of preserving the document's narrower database-design purpose.

Fix applied:
- Removed graph-store configuration settings from the schema cheat sheet and left configuration explanation in README/design/roadmap planning docs.

Prevention:
- Keep schema reference docs focused on stored fields, graph classes/predicates, join keys, authority boundaries, and retrieval rules.
- Put runtime service/configuration instructions in README, environment examples, operational docs, or phase plans.

Evidence:
- `docs/design/database/schema_cheat_sheet.md` no longer contains `GRAPH_STORE_MODE` or `OXIGRAPH_CONNECTION_STRING`.

## 2026-05-02 - Replan Before Implementation Direction Changes  [tags: workflow, planning, scope-ownership, validation]

Context:
- Plan: v0.1.1 persistent graph authority
- Task/Wave: follow-up change from embedded persistence default to Docker-backed Oxigraph service default
- Roles involved: Orchestrator

Symptom:
- Began changing code for Oxigraph service mode before updating the active execution plan.
- User corrected the workflow: "Apply the required adjustments to the plan as well. Don't veer off plan and be fine with it."

Root cause:
- Treated a follow-up implementation preference as a local adjustment instead of a plan-changing requirement under the active harness workflow.

Fix applied:
- Updated the active plan scope, resolved decisions, task acceptance criteria, validation expectations, progress log, and decision log to make Oxigraph service mode the default and embedded filesystem persistence explicit.

Prevention:
- When a user changes implementation direction under an active plan, stop implementation first and update the plan's decisions, owns scopes, acceptance criteria, and validation gates before further code edits.
- Do not treat passing local checks as sufficient if the plan no longer describes the current implementation direction.

Evidence:
- Active plan now records Docker-backed Oxigraph service mode, explicit embedded persistent mode, and prerequisite-gated live Oxigraph smoke validation.

## 2026-05-01 - Confirm Repository Branch Convention Before Branch Creation  [tags: git, tooling, workflow]

Context:
- Plan: persistent graph authority planning branch
- Task/Wave: branch creation before plan drafting
- Roles involved: Orchestrator

Symptom:
- Tried generic agent-style branch names before following the repository branch naming convention.
- User corrected the workflow: "Follow the repository branch name conventions."

Root cause:
- Used the desktop default branch prefix before consulting repo-local lessons and branch naming history.

Fix applied:
- Created the plan branch with the repository convention: `feature/2026-05-01/persistent-graph-authority-plan`.

Prevention:
- Before creating a branch, inspect repo-local rules, lessons, and visible branch naming patterns.
- Prefer the repository convention over generic agent defaults unless the user explicitly asks for a different branch name.

Evidence:
- Current branch: `feature/2026-05-01/persistent-graph-authority-plan`.

## 2026-05-01 - Follow Repository Branch Naming Over Generic Agent Prefix  [tags: git, tooling, assumptions]

Context:
- Plan: none
- Task/Wave: branch creation for separate plan commits
- Roles involved: Orchestrator

Symptom:
- Started creating a branch with a generic Codex-style name for plan commits.
- User corrected the workflow to follow the repository's branch naming conventions instead.

Root cause:
- Applied the desktop default branch prefix before checking the repo's visible branch naming pattern.
- The current branch already showed the local convention: `feature/YYYY-MM-DD/<slug>`.

Fix applied:
- Switched to repository-convention branch names for the remaining branch/commit work.
- Treat the temporary generic branch name as a misstep to rename or replace before committing.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: Before creating branches, inspect existing local branch naming patterns and follow the repository convention over generic tool defaults unless the user requests otherwise.
- Dispatch/plan guardrail:
  - For branch creation tasks, record the selected branch naming pattern before the first branch mutation.

Evidence:
- User correction on 2026-05-01: "Actually, follow the branch name conventions rather than using the codex name."

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

## 2026-04-28 - Parallelize Review Loops And Avoid Token-Burning Waits  [tags: delegation, review, tooling]

Context:
- Plan: PR #31 Copilot review remediation
- Task/Wave: PR comment triage and re-review loop
- Roles involved: Orchestrator

Symptom:
- The user clarified that review/remediation loops should use subagents as much as possible and should wait in ways that do not burn inference tokens.

Root cause:
- The main thread was carrying too much review/verification work directly and risked treating periodic Copilot polling as an active waiting loop.

Fix applied:
- Delegated focused remediation review to Reviewer subagents, kept the main thread to orchestration and decisions, and avoided sleep/poll loops.

Prevention:
- For PR review remediation, split independent review aspects into Reviewer subagents and use main-thread checks only for state transitions, validation evidence, or user/terminal notifications.
- Do not run token-burning polling loops while waiting for external review; use non-interactive status checks only when prompted by a state change or after returning control.

Evidence:
- PR #31 Copilot remediation used focused Reviewer subagents for scoped patch review and validation confirmation.

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

## 2026-04-27 - Rust Module Layout And Unit Test Placement  [tags: planning, output-contract, validation]

Context:
- Plan: Rust module file layout migration
- Task/Wave: post-domain-foundation cleanup
- Roles involved: Orchestrator

Symptom:
- Newly added domain code used the then-current domain module path, and pure domain tests were added under `tests/` as integration-test targets.
- User clarified the repo should use direct Rust module filenames and reserve `tests/` for integration tests.

Root cause:
- Followed the existing mixed module layout and placed pure domain tests in integration-test files instead of applying the desired Rust 2018-style module and unit-test convention.

Fix applied:
- Migrated source modules away from `mod.rs` files and moved pure domain tests into the source-tree domain test module.

Prevention:
- Prefer direct module files such as `foo.rs` over `foo/mod.rs` for Rust modules.
- Put unit tests in the same source module tree as the production code they test; use `tests/` only for integration tests.

Evidence:
- Repo rules now record the module layout and test placement convention.

## 2026-04-27 - No Legacy Compatibility Goal For v0.1  [tags: planning, scope-owns, architecture]

Context:
- Plan: v0.1 starter episodic memory roadmap and store contracts planning
- Task/Wave: roadmap correction before next implementation chunk
- Roles involved: Orchestrator

Symptom:
- Roadmap and store-contracts planning still implied that old flat API compatibility or legacy repository paths might be preserved if cheap.
- User clarified that compatibility is not a concern and legacy implementations that do not contribute to the new architecture should be removed.

Root cause:
- Treated the old flat API as a temporary compatibility surface rather than as removable migration residue for the v0.1 rewrite.

Fix applied:
- Updated the roadmap and store-contracts plan context to make legacy compatibility a non-goal for v0.1 work.
- Removed the bounded v0.1 compatibility guidance from repo-wide common rules after user correction.

Prevention:
- Future v0.1 plans should identify legacy pieces that can be removed or replaced, not preserve them for compatibility alone.
- Do not add compatibility wrappers for old flat APIs unless they directly serve the new v0.1 architecture.

Evidence:
- Roadmap resolved decisions now state that legacy implementations which do not contribute to v0.1 should be removed as replacement chunks land.

## 2026-04-27 - Keep Bounded Guidance Out Of Common Rules  [tags: rulebook, scope-owns, planning]

Context:
- Plan: v0.1 roadmap and store-contracts planning correction
- Task/Wave: repo rule cleanup
- Roles involved: Orchestrator

Symptom:
- A v0.1-specific compatibility direction was added to `docs/coding-agent/rules/common.md`.
- User clarified common rules should contain repo-wide rules that always apply, not bounded task or phase guidance.

Root cause:
- Promoted a useful but phase-scoped planning constraint into the repo-wide rulebook instead of keeping it in the roadmap and relevant plans.

Fix applied:
- Removed the v0.1 compatibility bullet from `common.md`.
- Left the bounded guidance in the roadmap and store-contracts plan where it belongs.

Prevention:
- Before editing common rules, check whether the guidance is always repo-wide or only applies to a bounded plan, task, phase, or migration.
- Keep bounded guidance in plans/roadmaps/lessons unless it truly applies across the repository indefinitely.

Evidence:
- `common.md` now contains only repo-wide validation, naming, module-layout, and test-placement rules.

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

## 2026-07-18 - Sequence Shared Review-Sibling Provenance Flips With Active Reviews  [tags: review, tooling, workflow]

Context:
- Plan: v0.1.5 eval-driven closeout
- Task/Wave: Wave 4 Task_6/Task_13 reviews
- Roles involved: Orchestrator | Reviewer

Symptom:
- Orchestrator re-pinned the shared .review-worktrees/CharacterMemory sibling clone to Task_13 provenance while the Task_6 review (pinned to different CM provenance) was still active; reviewer's pre-gate provenance check caught the mismatch.

Root cause:
- All CME review worktrees resolve the path dependency to ONE shared sibling clone; provisioning a queued review's provenance eagerly invalidated the active review's environment.

Fix:
- Restored the active review's pin; established a sequencing rule: the sibling clone serves one review at a time, flipped only between reviews on reviewer handshake.

Prevention:
- Provision the sibling-clone pin only when dispatching the review that will use it, never when queueing; reviewers confirm provenance before Cargo gates (already their rule — keep it).

## 2026-07-18 - Trace Compared Fields Through Default Constructors For Plan Diagnostics  [tags: validation, review]

Context:
- Plan: v0.1.5 eval-driven closeout
- Task/Wave: Task_5 phase 2 (F-BASE-5 echo warning) bounce
- Roles involved: Worker | Reviewer (lesson recorded by Orchestrator on Worker's behalf; lessons file was orchestrator-dirty at the time)

Symptom:
- Echo-surface warning compared VectorIndexCandidate.embedding_text, which on the default prepare path inherits RememberInput.content for every candidate — so normal plans with distinct surfaces warned falsely; the negative test masked it by disabling vector candidates.

Root cause:
- Ambiguous "content/embedding text" dispatch wording resolved without tracing each compared field through the default plan constructor; negative regression did not run on production-default candidate options.

Fix:
- Comparison scoped to draft content texts only; negative regression now runs with vector candidates enabled and distinct content.

Prevention:
- For diagnostics over generated plans: trace every compared field through the default constructor before choosing it, and keep at least one negative regression on production-default options.

## 2026-07-18 - Verify Supplied Evidence Claims Against The Canonical Artifact Before Committing  [tags: docs, validation]

Context:
- Plan: v0.1.5 eval-driven closeout
- Task/Wave: Task_13 bounce (F-FIXTURE-1 wording)
- Roles involved: Orchestrator | Worker | Reviewer

Symptom:
- A Task_12 analysis overclaim ("byte-identical" hub texts; actually ordinal-differing template instances) flowed through the orchestrator-supplied draft into the committed findings register; reviewer caught it.

Root cause:
- Supplied draft wording was treated as verified evidence; neither drafter nor committer re-checked each claimed equality/difference against the canonical fixture bytes.

Fix:
- Register wording corrected (template-aligned recurrence, ordinal+timestamp differences); finding conclusion unchanged.

Prevention:
- Before committing artifact-specific claims (equalities, counts, hashes), inspect the canonical artifact and verify every claimed equality and difference — regardless of who supplied the wording.

## 2026-07-18 - ADRs Are Orchestrator/Claude-Authored, Not Worker Tasks  [tags: delegation]

Context:
- Plan: v0.1.5 eval-driven closeout
- Task/Wave: Task_17 (ADR-I-0021) dispatch
- Roles involved: Orchestrator | Worker

Symptom:
- ADR-I-0021 authoring was bundled into a Codex worker dispatch alongside the code removal task; user corrected: the orchestrator holds the decision context and runs on the model suited to high-level decision records.

Root cause:
- Treated the ADR as a docs deliverable co-located with the code change instead of applying the existing model-strength routing (design/altitude work routes to Claude; the decision context lives with the orchestrator who ran the decision process).

Fix:
- Task_17 withdrawn from the worker; orchestrator drafted the ADR directly from the decision packet.

Prevention:
- Repo rule added (orchestrator.md Delegation Routing): ADRs and design-decision records are drafted by the Orchestrator or a Claude design agent; implementation workers may be asked to fact-check file:line claims, never to author the decision record.

## 2026-07-19 - Mid-Plan Task Additions Get Formal Task Records At Creation Time  [tags: planning, workflow]

Context:
- Plan: v0.1.5 eval-driven closeout
- Task/Wave: Tasks 14-24 and Task_9b (added across the measurement-hardening and scope-addition gates)
- Roles involved: Orchestrator

Symptom:
- Eleven tasks added during execution existed only as Decision Log narrative; the Tasks section and Task Waves stopped at the originally drafted set, so per-task acceptance/validation evidence was not trackable in the plan's structured form. User correction.

Root cause:
- Decision Log entries felt sufficient in the moment because they carried the ruling context; the plan-format requirement that every Task_X have type/owns/acceptance/validation was applied only at initial drafting, not treated as a standing invariant.

Fix:
- Retrospective Task_14..Task_24 + Task_9b records written with status and closure evidence; Task Waves extended with an executed-wave note.

Prevention:
- New default: any mid-plan task creation writes the formal Task_X record (type/owns/depends_on/acceptance/validation) into the Tasks section and updates Task Waves in the same edit as the Decision Log entry that creates it — the Decision Log records WHY, the task record tracks WHAT/EVIDENCE.

## 2026-07-19 - Arm A Review-Comment Monitor When Opening A PR  [tags: workflow, review]

Context:
- Plan: v0.1.5 closeout (schema-migration PR #62)
- Roles involved: Orchestrator

Symptom:
- PR #62 was opened without arming the review-comment/merge-state monitor; the user had to request it separately.

Root cause:
- PR creation was treated as complete at URL creation; the feedback channel (reviews, comments, merge state) was not treated as part of the same action even though acting on those events is the orchestrator's job.

Fix:
- Monitor armed retroactively (reviews, inline comments, issue comments, merged/closed terminal states).

Prevention:
- New default: opening or being handed a PR immediately arms a monitor covering new reviews, review comments, issue comments, and terminal merge/close state, in the same action as PR creation.

## 2026-07-21 - Backcompat Code Slipped Through All Review Rounds  [tags: review, validation, scope-owns]

Context:
- Plan: v0.1.5 closeout merge PRs (CM #63, CME #13); post-hoc sweep plan `backcompat-sweep-plan.md`
- Roles involved: Orchestrator | Worker | Reviewer

Symptom:
- The user found unnecessary backwards-compatibility code in both merge PRs after multiple Copilot rounds and Tier D reviews had passed them as clean.

Root cause:
- No rule declared that a pre-consumer library never needs backwards compatibility, so neither workers (who wrote shims defensively) nor reviewers (who check against rules and acceptance criteria) had any basis to flag compat surfaces as defects.

Fix applied:
- Forensic inventories dispatched to codex workers, cleanup commits on both PR branches; Compatibility Policy section added to both repos' `rules/common.md` (user-directed 2026-07-21).

Prevention:
- The Compatibility Policy rule makes compat surfaces a rule violation reviewers must flag; Tier D review prompts should name it explicitly for API-surface diffs until it becomes habitual.

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

## 2026-07-21 - Equivalence Tests Must Compare The Full Observable Contract  [tags: validation, review]

Context:
- Plan: backcompat-sweep-plan; ADR-I-0012 equivalence test (CM) and typed-plan migration test (CME)
- Roles involved: Worker | Reviewer

Symptom:
- Both repos' workers independently wrote equivalence/migration tests that compared cardinalities or partial fields (vector counts + a stats flag in CM; ID vectors + timestamps in CME), so a wrapper or migration persisting wrong content with matching counts would pass; both Tier D reviewers caught the same class independently.

Root cause:
- Equivalence was asserted on the easiest observable slice rather than the contract: different inputs per path, coarse assertions, no canonical-state comparison.

Fix applied:
- CM: identical deterministic inputs through wrapper and manual paths in isolated stores, comparing complete outcomes plus retrieved canonical graph state. CME: complete object/link MemoryCandidate value equality plus exact vector target/text pins.

Prevention:
- For API-path equivalence or migration regressions: identical deterministic inputs, isolated stores, compare the full observable contract and contract-relevant persisted/canonical state, normalizing only unavoidable generated metadata. Reviewers treat count/partial-field equivalence assertions as a standing Tier D check.
