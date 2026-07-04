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

## 2026-07-04 - Evidence Greps Must Match The Syntactic Class Being Ruled Out  [tags: validation, grep-evidence]

Context:
- Plan: lint-suppression cleanup
- Task/Wave: Task_1 acceptance check
- Roles involved: Worker | Orchestrator

Symptom:
- Both the Orchestrator's inventory grep and the Worker's done-evidence grep for module-wide allows matched only single-lint forms (`allow(dead_code)`), missing two combined-form sites (`#![allow(dead_code, unused_imports)]`) — the task was reported done with its acceptance criterion unmet.

Root cause:
- The regex encoded the expected spelling of the attribute, not the syntactic class being ruled out.

Fix applied:
- Addendum cleaned the two missed files; completion evidence switched to `rg '^#!\[allow' src -n`, which matches every module-wide allow regardless of lint list.

Prevention:
- When acceptance is "no X remains", the evidence grep must match the broadest syntactic form of X (then narrower greps may summarize permitted exceptions). Encode the class, not the expected spelling.

Evidence:
- Task_1 report grep showed clean; Orchestrator's independent broad grep found embedded.rs/shared.rs; addendum report + corrected grep confirm only the two permitted files remain.

## 2026-07-04 - Verify Skip-Gating Predicate Branches Against What The Producer Actually Emits  [tags: validation, review]

Context:
- Plan: responsibility-boundary module reorg
- Task/Wave: Wave 6 Task_7 addendum / Task_9 review remediation
- Roles involved: Reviewer | Orchestrator | Worker

Symptom:
- The migrated Qdrant-unavailability test helper compared the neutral error payload's status to "unavailable", but the producer stores tonic's Code Display, which for Code::Unavailable is the sentence "The service is currently unavailable" — the branch was dead code, silently changing skip-vs-fail semantics while every locally runnable check stayed green.

Root cause:
- The only code path exercising the predicate (integration tests with the service in specific failure states) was under an environmental waiver, and the branch assumed Code::to_string() yields the enum name rather than the description sentence.

Fix applied:
- Predicate changed to a case-insensitive substring match (catches both the sentence and the bare enum name); verified end-to-end by stopping Qdrant and observing producer output, and by rerunning the healthy path after restart.

Prevention:
- When changing test skip-gating predicates, statically trace each compared value from the emitting site to the matcher, or run once with the service deliberately down; waived/skipped suites do not exercise these branches. Promoted to a worker repo rule.

Evidence:
- Task_9 Reviewer MAJOR finding (source-level confirmation via tonic 0.12.3 status.rs); Orchestrator service-down run showing producer output; healthy-path rerun green.

## 2026-07-04 - Verify Codex Delivery Path And Worker Capacity Before agmsg Dispatch  [tags: delegation, tooling, workflow]

Context:
- Plan: responsibility-boundary module reorg
- Task/Wave: Wave 5 dispatch
- Roles involved: Orchestrator | Worker (codex)

Symptom:
- Task dispatches sent via agmsg to the codex worker sat unnoticed in its inbox; earlier waves only worked because the user manually prompted the codex terminal. Separately, the plan's parallel wave (Task_6 ∥ Task_7) had only one worker available.

Root cause:
- Codex has no Monitor; automatic delivery needs the agmsg codex bridge or codex-side monitor mode. The bridge was dead: it resolves identities using the Windows-form project path while registrations store the MSYS form, so it found no identity (also, two codex-type roles were registered). The spawned worker's boot prompt (`/agmsg actas worker`) had additionally been mangled by MSYS path conversion into a filesystem path.

Fix applied:
- User activates monitor mode from inside the codex terminal (identity resolves correctly from that side). Orchestrator dispatches remain via agmsg; a pending dispatch is picked up once delivery is live or after one manual nudge.

Prevention:
- Before dispatching work to a codex agmsg member, confirm its delivery path is live (codex-side monitor mode / bridge running), or explicitly ask the user to nudge the terminal; never assume a send equals delivery.
- Before dispatching a parallel wave, count live workers and spawn the additional ones first (spawn.sh with explicit --team; beware MSYS mangling of slash-prefixed boot prompts on Windows).

Evidence:
- codex-bridge log: repeated "no matching codex identity"; identities.sh returns rows for `/c/Users/...` but none for `C:\Users\...`; codex process cmdline shows the mangled boot prompt.

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

## 2026-07-03 - Skip-On-Stall Integration Tests Can Mask Unexecuted Bodies In Green Runs  [tags: validation, ci]

Context:
- Plan: v0.1.3 remember intake write planning
- Task/Wave: Task_7 review / Task_4b re-review
- Roles involved: Reviewer | Orchestrator

Symptom:
- Integration tests that self-skip on the documented local Qdrant idle-stall report `ok`, so a green local target run can include tests whose bodies never executed.

Root cause:
- Skip-on-infra-stall is implemented as an early return inside the test body, invisible to the test harness result.

Fix applied:
- Reviewer performed targeted reruns of the specific tests under review and confirmed absence of skip messages; Linux CI remains the authoritative full-suite arbiter.

Prevention:
- When local evidence matters for a specific test, rerun it in a targeted invocation and confirm no skip message was printed for it. Treat aggregate green local runs with skip messages as incomplete evidence.

Evidence:
- Task_4b re-review confirmed full-body execution of all three new tests via targeted reruns; PR #55 CI green.

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

## 2026-07-02 - Append To Append-Only Logs, Never Replace Entries  [tags: planning, workflow]

Context:
- Plan: stabilize v0.1.2 retrieval guardrails
- Task/Wave: Decision Log maintenance during replans
- Roles involved: Orchestrator

Symptom:
- Twice in one session, a string-replacement edit targeted the latest Decision Log entry as its anchor and replaced it with the new entry, destroying history in an append-only log.

Root cause:
- Using the previous entry's text as the replacement anchor conflates "insertion point" with "content to replace".

Fix applied:
- Restored the overwritten entries immediately after each incident.

Prevention:
- When appending to append-only sections, anchor on the previous entry and produce previous entry + new entry in the replacement, or anchor on the section's tail marker. Verify the log length grew after the edit.

Evidence:
- Both overwritten Decision Log entries in the stabilization plan were restored in follow-up edits.
## 2026-07-02 - Use 127.0.0.1 Instead Of localhost For Local Qdrant gRPC  [tags: tooling, validation]

Context:
- Plan: v0.1.3 remember intake write planning
- Task/Wave: Task_1 required validation
- Roles involved: Orchestrator | Worker

Symptom:
- Qdrant-backed integration tests hung to gRPC timeouts ("The operation was cancelled Timeout expired") or intermittent ConnectionRefused, while REST on 6333 responded in <100ms. One run crashed the Qdrant container (exit 255).

Root cause:
- `QDRANT_CONNECTION_STRING=http://localhost:6334` resolves to IPv6 `::1` inside tonic; the Docker Desktop port proxy accepts the TCP connect on `::1` but does not reliably forward HTTP/2 traffic, so gRPC calls stall until client timeout. `127.0.0.1` works instantly.

Fix applied:
- Pinned `QDRANT_CONNECTION_STRING=http://127.0.0.1:6334` in the local gitignored .env.

Prevention:
- Prefer `127.0.0.1` over `localhost` in local service connection strings for gRPC clients targeting Docker Desktop published ports on Windows.
- When gRPC times out but REST on the sibling port is fast, test IPv4 vs IPv6 connect paths before suspecting the service.

Evidence:
- initialization_tests: 30s timeout failure with localhost vs 10s pass with 127.0.0.1 override; unit suite unchanged.

## 2026-07-02 - Verify Failures Against Baseline HEAD Before Blaming New Code  [tags: validation, workflow]

Context:
- Plan: v0.1.3 remember intake write planning
- Task/Wave: Task_1 required validation
- Roles involved: Orchestrator

Symptom:
- Full `cargo test` failed during Task_1 validation in v0.1.2 integration tests, initially indistinguishable from a Task_1 regression.

Root cause:
- Environmental degradation (see IPv6/localhost entry) unrelated to the change under validation.

Fix applied:
- Stashed the task's src/ changes, reran the failing target on baseline HEAD, observed identical failures, restored the stash.

Prevention:
- When required validation fails in tests untouched by the current task, run the same target against baseline HEAD (stash/worktree) to classify the failure as pre-existing vs regression before remediation.

Evidence:
- Baseline HEAD produced the identical 1-pass/2-fail result on tests/retrieval_guardrails_tests.rs.

## 2026-06-14 - Prefer Root-Cause Fixes Over Symptom Patches  [tags: maintainability, review, validation]

Context:
- Plan: v0.1.2 closeout divergence fixes
- Task/Wave: PR review and CI remediation follow-up
- Roles involved: Orchestrator

Symptom:
- A warning-deny CI failure was initially approached with a broad suppression attribute before stepping back to consider whether the test helper structure itself was the root cause.

Root cause:
- Treated the immediate failing warning as the problem, instead of first asking what design or ownership boundary made the warning appear.

Fix applied:
- Reworked the integration test support layout so each test target imports only the helpers it needs, eliminating the warning naturally.

Prevention:
- When fixing a defect, first identify the smallest root cause that explains the symptom. Prefer structural or semantic fixes over suppressions, wrappers, retries, or local patches that merely hide the failure.
- Use symptom patches only when the root cause is out of scope, externally owned, or explicitly accepted as a tradeoff; document that decision.

Evidence:
- The test support split removed the need for `dead_code` allowances and passed Clippy/test-target validation with warnings denied.

## 2026-06-14 - Prefer Targeted Test Support Modules Over Broad `dead_code` Allows  [tags: validation, ci, maintainability]

Context:
- Plan: v0.1.2 closeout divergence fixes
- Task/Wave: PR #53 review and CI remediation
- Roles involved: Orchestrator

Symptom:
- Initial CI remediation used a broad `#![allow(dead_code)]` in the shared integration test helper module to silence warnings denied by Clippy and test target compilation.

Root cause:
- `tests/test_utils.rs` was compiled both as a standalone integration test target and as a helper module in each integration test. Different targets used different helper subsets, causing warning-deny failures. The first fix treated the warning as noise instead of the helper layout as the design problem.

Fix applied:
- Replaced the catch-all integration helper target with targeted modules under `tests/support/`, so each integration test imports only the helper set it needs.

Prevention:
- When CI fails under `-D warnings`, first ask whether the code layout can avoid the warning naturally. Use `allow` attributes only when unused code is intentional and no cleaner structure exists.

Evidence:
- `cargo clippy --all-targets -- -D warnings`, `cargo test --no-run`, and `cargo test` pass after splitting test support modules without any dead-code allowance.

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

## 2026-05-01 - Verify Copilot Re-Review Requests Against Repo Rule Before Trusting Them  [tags: tooling, review, git, workflow]

Context:
- Plan: PR #38 and PR #39 Copilot review-fix loop
- Task/Wave: post-fix re-review request
- Roles involved: Orchestrator

Symptom:
- Initially re-requested Copilot review with `gh pr edit --add-reviewer '@copilot'` and treated the successful exit as meaningful.
- Then retried GraphQL `requestReviewsByLogin` but omitted the repo-recorded `union: true` input before checking the existing lesson.

Root cause:
- Did not consult the repo lessons/orchestrator guidance before performing a known-special Copilot re-review workflow.
- Verified too late that `reviewRequests` and `latestReviews` had not changed for the current PR heads.

Fix applied:
- Switched to GitHub GraphQL `requestReviewsByLogin` with `userLogins: ["copilot-pull-request-reviewer"]` and retried with the repo-recorded `union: true` input.
- Verified `reviewRequests`, `latestReviews`, and review-thread state through GraphQL instead of relying on `gh pr edit` exit code.

Prevention:
- Before any Copilot re-review request, search `docs/coding-agent/lessons.md` and repo orchestrator rules for the current required command shape.
- Treat a Copilot review request as unverified until GraphQL shows either a queued `reviewRequests` entry or a new `latestReviews` entry on the current head SHA.
- If GraphQL returns success but neither signal appears after polling, report that no fresh Copilot review was observed instead of claiming one happened.

Evidence:
- PR #38 current head `87ced4dcf60f3fa572435b306660f5f7081ec4a9` had all Copilot threads resolved; GraphQL retries produced a Copilot internal-error review at `2026-05-01T07:37:52Z` rather than a usable review.
- PR #39 current head `704d39f04120d434e28a79713ea1c5090d73a38b` had all Copilot threads resolved; GraphQL retries produced a Copilot internal-error review at `2026-05-01T07:37:53Z` rather than a usable review.

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

## 2026-04-30 - Re-request Copilot PR Review Through GraphQL When Add-Reviewer Is A No-Op  [tags: tooling, review, git]

Context:
- Plan: PR #35 final Copilot re-review request
- Task/Wave: post-remediation review request
- Roles involved: Orchestrator

Symptom:
- `gh pr edit 35 --add-reviewer '@copilot'` returned success, but the PR page did not show Copilot reviewing and no new Copilot review appeared.
- Removing and re-adding `@copilot` through `gh pr edit` also returned success without starting a new review.

Root cause:
- Copilot PR review behaves like a special reviewer/re-review flow after it has already reviewed a PR.
- The normal `gh pr edit --add-reviewer '@copilot'` path can be a no-op for re-review even when it returns the PR URL.
- GitHub GraphQL exposes the working request as `requestReviewsByLogin` with `userLogins: ["copilot-pull-request-reviewer"]`; using `botLogins` caused a GitHub server-side error.

Fix applied:
- Requested review with GraphQL:
  - query PR id:
    `gh api graphql -f query='query { repository(owner: "OWNER", name: "REPO") { pullRequest(number: PR_NUMBER) { id } } }'`
  - request Copilot re-review:
    `gh api graphql -f query='mutation($pullRequestId: ID!) { requestReviewsByLogin(input: { pullRequestId: $pullRequestId, userLogins: ["copilot-pull-request-reviewer"], union: true }) { pullRequest { reviewRequests(first:20) { nodes { requestedReviewer { __typename ... on Bot { login } ... on User { login } } } } } } }' -f pullRequestId='PR_ID'`
- Verified that `reviewRequests` temporarily contained `copilot-pull-request-reviewer`, then that Copilot posted a new review with no new comments.

Prevention:
- In PowerShell, quote `@copilot` when using the ordinary path: `gh pr edit PR_NUMBER --add-reviewer '@copilot'`.
- If Copilot has already reviewed the PR and the ordinary path returns success but does not start a new review, use the GraphQL `requestReviewsByLogin` fallback with `userLogins: ["copilot-pull-request-reviewer"]`.
- After requesting, verify via GraphQL `reviewRequests` and `latestReviews`, not just by command exit code.

Evidence:
- PR #35 received a new Copilot review at `2026-04-30T11:42:14Z` after the GraphQL fallback: "Copilot reviewed 57 out of 58 changed files in this pull request and generated no new comments."

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

## 2026-04-29 - Scan Complete PR Thread Payload Before Closeout  [tags: review, tooling, validation]

Context:
- Plan: PR #33 retrieve/context-pack review remediation
- Task/Wave: repeated PR comment triage follow-up
- Roles involved: Orchestrator

Symptom:
- Reported that all active PR review threads were handled while one unresolved thread remained in the fetched payload.

Root cause:
- Relied on a partial read of a large saved GraphQL payload and did not run a final unresolved-thread extraction across the complete payload before closeout.

Fix applied:
- Re-fetched the full PR thread list, searched it for every `isResolved: false` entry, and addressed the missed fail-closed graph policy comment.

Prevention:
- Before claiming all PR comments are resolved, run a complete-payload unresolved-thread extraction, not just a line-range read of the saved output.
- Treat a large `gh api graphql` result as incomplete until every unresolved thread ID has been enumerated and triaged.

Evidence:
- Missed thread `PRRT_kwDONxNRBs5-Z49b` was found by searching the complete saved PR thread payload for `"isResolved": false`.

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

## 2026-04-29 - Keep Roadmap Versions Out Of Durable Code  [tags: code-quality, architecture, communication]

Context:
- Plan: v0.1 remember/link implementation follow-up
- Task/Wave: post-implementation code review discussion
- Roles involved: Orchestrator

Symptom:
- Durable production comments and names used roadmap-version language such as `v0.1`, even when the concept should survive beyond the roadmap milestone.

Root cause:
- Treated roadmap phase labels as convenient implementation descriptors instead of limiting them to roadmap docs and clearly temporary migration artifacts.

Fix applied:
- Classified versioned code comments/names as cleanup targets when they describe durable structures such as composition boundaries, facades, or provider-neutral APIs.

Prevention:
- Do not use roadmap version numbers in long-lived production code comments, identifiers, or user-facing errors. Use stable domain language instead.
- Roadmap version labels are acceptable in roadmap/planning docs and temporary migration comments only when the cleanup/removal condition is explicit.
- Schema/data version identifiers must be treated as separate persisted contract decisions, not casual roadmap labels.

Evidence:
- `CharacterMemory::from_parts` comment and `v0_1_parts` naming were identified as durable concepts needing stable naming/comment cleanup.

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

## 2026-04-28 - Quote PR Bodies As Literal Input  [tags: tooling, git, output-contract]

Context:
- Plan: none
- Task/Wave: PR creation
- Roles involved: Orchestrator

Symptom:
- Initial `gh pr create --body "..."` attempt treated markdown backticks in the PR body as shell command substitutions.
- The shell tried to execute commands and markdown file paths before the PR was created.

Root cause:
- Passed a markdown PR body containing backticks through a double-quoted shell argument.

Fix applied:
- Retried PR creation with `gh pr create --body-file -` and a single-quoted heredoc delimiter so the body was passed literally.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: When creating or updating PR bodies from shell, pass markdown through a literal file/stdin path such as `--body-file - <<'EOF'` instead of a double-quoted `--body` argument.
- Dispatch/plan guardrail:
  - For PR bodies containing backticks, checkboxes, or command snippets, use literal stdin/file input before running `gh pr create` or `gh pr edit`.

Evidence:
- Retry created PR #29 successfully: https://github.com/ebigunso/CharacterMemory/pull/29

## 2026-04-28 - Explain Temporary Suppressions  [tags: review, code-quality, communication]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-store-contracts-test-harness-plan.md`
- Task/Wave: PR review follow-up
- Roles involved: Orchestrator

Symptom:
- Added `allow` suppressions for transitional v0.1 contract/test-support code without explaining why they exist or when they can be removed.
- User clarified that code should not just work without context when the safe revision point is not obvious.

Root cause:
- Treated the suppression as self-explanatory implementation scaffolding instead of documenting the temporary boundary it protects.

Fix applied:
- Added concise rationale and removal-condition comments for the transitional `dead_code` and `unused_imports` suppressions.

Prevention:
- Repo rule candidate:
  - audience: common
  - proposed rule: Temporary lint suppressions should explain why they exist and when they can be safely removed, unless the reason is obvious from nearby code.
- Dispatch/plan guardrail:
  - During review follow-up, inspect newly added `allow` attributes for rationale and removal conditions before marking comments resolved.

Evidence:
- Suppression comments added to v0.1 store contract and test-support modules.

## 2026-04-27 - Hidden PR Template Discovery  [tags: tooling, output-contract]

Context:
- Plan: none
- Task/Wave: PR creation
- Roles involved: Orchestrator

Symptom:
- Created PR #22 with a generic Summary/Testing body after failing to find the repository PR template.
- User later provided `.github/pull_request_template.md`, which should have been used.

Root cause:
- The template search used `rg --files` without `--hidden`, so the hidden `.github/` directory was missed.

Fix applied:
- Read `.github/pull_request_template.md`, ran the template checklist commands, and updated the PR body to follow the template.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: When creating or updating a PR, search hidden template locations such as `.github/pull_request_template.md` before assuming no template exists.
- Dispatch/plan guardrail:
  - Include hidden `.github/**` in PR-template discovery commands.

Evidence:
- `cargo check --all-targets` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test --verbose` passed.

## 2026-04-27 - PR Description Final-State Framing  [tags: output-contract, communication]

Context:
- Plan: none
- Task/Wave: PR description update
- Roles involved: Orchestrator

Symptom:
- PR description described additions as a sequence of branch edits and follow-up metadata changes.
- This exposed intermediary states that reviewers do not see when evaluating the final PR diff.

Root cause:
- The PR body was written from the agent's work history rather than from the reviewer's final-state perspective.

Fix applied:
- Rewrote PR #25 description to describe the documentation set as it exists in the final branch state.

Prevention:
- Repo rule candidate:
  - audience: orchestrator
  - proposed rule: PR descriptions should describe the final branch state and reviewer-facing intent, not intermediary implementation states or correction history.
- Dispatch/plan guardrail:
  - Before updating a PR body, review it for phrases that imply hidden intermediate states when a final-state description would be clearer.

Evidence:
- PR #25 body now describes the philosophy doc, roadmap, roadmap phase docs, and ADR set as final additions.

## 2026-04-27 - Roadmap Granularity For Large Implementation Phases  [tags: planning, scope-owns, output-contract]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-starter-episodic-memory-roadmap-plan.md`
- Task/Wave: roadmap drafting
- Roles involved: Orchestrator

Symptom:
- Drafted concrete Task_X implementation plans for many downstream v0.1 chunks before the first chunk had resolved model, store, and validation shape.
- User clarified that the roadmap should contain high-level chunks for the whole implementation phase, with concrete plans drafted chunk-by-chunk as work reaches them.

Root cause:
- Treated the implementation-phase roadmap as a full execution plan for every future chunk, creating false precision for work that depends on earlier discoveries.

Fix applied:
- Reworked the roadmap into high-level implementation chunks and kept concrete Task_X detail only for Chunk 1.

Prevention:
- For large multi-phase implementation roadmaps, keep future phases/chunks at roadmap granularity unless the user asks for full execution detail.
- Add concrete Task_X plans only for the next executable chunk, then draft the next chunk plan from the code and decisions that actually landed.

Evidence:
- Updated active roadmap now separates `High-Level Implementation Roadmap` from `Concrete Plan For Chunk 1`.

## 2026-04-27 - Separate Roadmaps From Concrete Plan Lifecycle  [tags: planning, scope-owns, output-contract]

Context:
- Plan: v0.1 starter episodic memory implementation planning
- Task/Wave: roadmap and first concrete plan drafting
- Roles involved: Orchestrator

Symptom:
- The roadmap still embedded the concrete first-chunk execution plan, making independent plan approval/completion tracking awkward.
- User asked for the roadmap to be separate from concrete implementation plans and for plans to be named by achieved outcome rather than chunk number.

Root cause:
- Treated a roadmap file as a convenient container for the first executable plan instead of giving roadmap and plan artifacts separate lifecycles.

Fix applied:
- Split the combined artifact into `v0-1-starter-episodic-memory-roadmap.md` and `v0-1-domain-foundation-plan.md`.
- Named the concrete plan by the capability it establishes rather than by chunk ordinal.

Prevention:
- For multi-chunk work, keep phase roadmaps and concrete execution plans in separate files when plans need independent completion tracking.
- Name concrete plans by the outcome they achieve, not by sequence number alone.

Evidence:
- Active plans directory now contains a roadmap-only file plus a separate domain-foundation plan file.

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
