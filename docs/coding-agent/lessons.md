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

## 2026-09-02 - Rebuilt-Machine Qdrant Teardown Verification Retired The Waiver  [tags: validation, qdrant, teardown]

Context:
- Plan: qdrant-teardown-hardening; rebuilt Windows host with Qdrant 1.19.0.

Symptom:
- The July teardown catalog included delete-response loss, idle-mutation stalls, IPv6-localhost stalls, service wedges under concurrent suites, and a 353-second write-planning suite, requiring a temporary cleanup waiver.

Root cause:
- Rebuilt-machine verification did not reproduce any cataloged failure across seven service-up runs; the 353-second figure was degraded-host suite duration rather than a code timeout, and the surviving Qdrant volume required the 1.19.0 server line.

Fix applied:
- Retired the waiver, pinned qdrant-client and the Qdrant image to 1.19.0, added an explicit 30-second cleanup-client deadline, normalized the example endpoint to IPv4 loopback, and made a failed cleanup warn when the collection still exists.

Prevention:
- Keep service-up compatibility and leak censuses in dependency-bump validation. No REST delete verification, retry loop, pre-run sweeper, timeout calibration, or production-adapter behavior change was added because the verified failure modes no longer reproduced and a broad sweeper could delete concurrent runs.

## 2026-09-02 - Review The Full Result Cross-Product Of An Operation And Its Verification  [tags: review, validation, teardown]

Context:
- Plan: qdrant-teardown-hardening; Task_1 cleanup warning, cm-reviewer approval, Copilot finding on CM PR #70.

Symptom:
- The best-effort cleanup warned only when the existence probe returned `Ok(true)`; when the delete AND the probe both failed (the transport-failure case the hardening targets) it stayed silent, and the review approved it.

Root cause:
- Reviewer and worker checked the primary operation's outcomes but not the verification step's own failure branch; an unknown postcondition was treated as confirmed absence.

Fix applied:
- Match on the probe result explicitly: `Ok(false)` silent, `Ok(true)` warn, `Err(probe)` warn with both errors (a2e1fcc).

Prevention:
- When a change verifies a postcondition after a failed operation, enumerate the full cross-product of (operation outcome × verification outcome) and state what each cell emits; "could not verify" is observable degradation, never silence.

## 2026-09-02 - Rebuilt-Machine Environment Regressions Surface As Tooling Failures  [tags: environment, tooling, agmsg]

Context:
- Plan: qdrant-teardown-hardening; first session after the Aug 2026 hardware/OS rebuild.

Symptom:
- (1) Every codex→orchestrator agmsg body arrived truncated at its first space (PowerShell here-string → `bash.exe -lc` → `send.sh`), and one codex sandbox failed every process launch with `CryptUnprotectData error 2148073483`. (2) The generated pre-commit hook pointed at a removed Python 3.10, then at a Python 3.12 whose USER site-packages the codex sandbox interpreter ignores. (3) libclang was absent, so `oxrocksdb-sys` bindgen failed in both repos.

Root cause:
- Machine-state assumptions (interpreter paths, user-site visibility, installed toolchains) silently invalidated by the rebuild; none were repo defects. For (1), the user observed on the codex threads that the same commands succeed when escalated outside the Codex sandbox in the same thread, so the fault sits in the Codex Windows sandbox launcher (DPAPI decryption inside the sandbox), not in agmsg or shell quoting.

Fix applied:
- LLVM installed by the user; pre-commit installed into Python312's own site-packages (`python -m pip install pre-commit`, not `--user`) and the hook regenerated; agmsg reports switched to files under `.agent-work/<role>/` with single-token notifications until the delivery layer is repaired (open investigation).

Prevention:
- After any machine rebuild, before dispatching: run the ignored Qdrant canary, `cargo test --no-run` in both repos, `python -m pre_commit --version` with `PYTHONNOUSERSITE=1`, and an end-to-end agmsg round-trip that contains spaces. Treat one-word inbound bodies as a delivery fault, not an agent-formatting fault.

## Purge note (2026-07-23)

Eleven entries purged per the user-directed low-value/invalid sweep (Codex purge map, agmsg 2026-07-23T12:28Z): ten PURGE-LOW-VALUE (restatements of now-mandatory harness/rule content — plan-format task records, PR monitoring, canonical-byte verification, compatibility policy, module layout, evidenced-scope rulebook default, parallel dispatch — plus two cheaply rediscovered one-off quirks and one unstructured batch-notes bundle) and one PURGE-INVALID (the phase-bounded v0.1 compatibility ruling, superseded by the repo-wide Compatibility Policy). Full entries recoverable from git history at 4997bdc.

## 2026-09-03 — Reconcile The Companion Pin Before Filing Cross-Repository Breakage  [tags: review, scope, cross-repo, assumptions]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_3 / Wave 2
- Roles involved: Reviewer | Orchestrator

Symptom:
- The reviewer filed a preliminary HIGH because a new variant in the closed error vocabulary would not compile in the evaluation repository's exhaustive match once that repository re-pins.

Root cause:
- The finding treated the companion repository's current main as if it were already pinned to the change under review, although the companion stays pinned at an older library commit until its own migration plan runs; ADR-I-0023's impact section names that conversion as the companion's re-pin prerequisite.

Fix applied:
- The orchestrator ruled the item out of scope; the reviewer recorded it as the already-tracked re-pin obligation and completed the review on the in-PR acceptance bullets.

Prevention:
- Before filing cross-repository breakage, reconcile the companion's pin and the plan that owns its migration; when the companion intentionally remains pinned until its own migration, record an obligation, not an in-PR defect.

Evidence:
- Reviewer messages of 2026-09-03 (preliminary HIGH, ruling acknowledgement, final REVIEW3_DONE) on Task_3 at 773d65e.

## 2026-09-04 — PR Watchers Must Not Depend On Tools Absent From The Monitor Shell  [tags: workflow, monitoring, tooling, orchestrator]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Wave 1–2 PR monitoring
- Roles involved: Orchestrator

Symptom:
- Copilot reviews on the wave PRs and on the evaluation repository's PR arrived without any watcher notification; the decider noticed the review before the orchestrator did.

Root cause:
- The watcher scripts piped GitHub API output through `jq`, which is not on the PATH of the background monitor shell; every poll failed on stderr, which the monitor does not surface, so the watchers stayed silent while appearing armed.

Fix applied:
- Watchers re-armed using only `gh api --jq` (no external `jq`), one loop covering the whole stack, printing on any change in review count, review-comment count, or merge state.

Prevention:
- A watcher script uses only tools proven available in the monitor shell (`gh --jq`, POSIX sh); before trusting a new watcher, read its output file once to confirm it produced a first sample rather than errors.

Evidence:
- Monitor output file for the PR #77 watcher on 2026-09-04: eighteen consecutive `jq: command not found` lines and no events, while Copilot's "approval recommended" review was already posted.

## 2026-09-04 — Edit Only Through Absolute Worktree Paths, And Run Broad Suites Only After The Live Window Is Granted  [tags: workflow, worktrees, live-mutex, worker]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_4 / Wave 3
- Roles involved: Worker | Orchestrator

Symptom:
- A relative-path patch briefly modified files in the shared main checkout instead of the task worktree before being reverted; and a full test suite started before the exclusive service window was confirmed, so service-backed tests could have collided with another agent's run.

Root cause:
- Relative paths resolve against whatever the current directory happens to be, and several checkouts of the same repository share identical relative paths; the live-window protocol grants exclusivity only on the explicit `WINDOW_YOURS` reply, not on sending `LIVE_START`.

Fix applied:
- The stray edits were reverted and verified clean; the suite was re-run inside the granted window.

Prevention:
- Every edit and every git command names an absolute path inside the task worktree, and the current directory is checked before each edit; a suite that may reach a shared service starts only after `WINDOW_YOURS` is observed, otherwise run the service-free command.

Evidence:
- Worker Task_4 report of 2026-09-03 (commit d232830) and the orchestrator's clean `git status` check on the main checkout.

## 2026-09-04 — Include Unit Assertions In Deleted-Helper Call-Site Censuses  [tags: validation, refactor, tests, worker]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5
- Roles involved: Worker

Symptom:
- The first focused compile after consolidating Qdrant payload reads failed because service-adapter unit assertions still called the deleted local payload helper and relied on its removed surface-field import.

Root cause:
- The pre-edit trace found the production callers but did not census the helper's unit-test callers before deleting it.

Fix applied:
- Route the assertions through a test-only raw string accessor and import the surface field in the test module, then rerun the focused compile.

Prevention:
- Before deleting or moving a shared helper, run an exact-symbol census across the whole repository, including inline unit-test modules, and resolve every hit in the same patch.
- Residual risk / waiver: none.

Evidence:
- `cargo test adapters::qdrant::payload::tests --lib` reported all remaining `payload_string` and `SURFACE_FIELD` test references before the correction.

## 2026-09-04 — Compare Recall Results At The Same Contract Boundary  [tags: validation, tests, vector-recall, worker]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5
- Roles involved: Worker

Symptom:
- The first library-suite run failed because the new indexed-exact canary compared 200 raw backend rows with the port's canonical 20-candidate result.

Root cause:
- The assertion crossed the backend-fetch and port-result boundaries without applying the port's canonicalization and requested limit.

Fix applied:
- Canonicalize and truncate the indexed exact rows to the request limit before comparing them with the unindexed port result.

Prevention:
- When a test compares backend rows with a port result, explicitly apply the port's ordering, deduplication, and limit rules before asserting equality.
- Residual risk / waiver: none.

Evidence:
- `cargo test --lib` passed 393 tests before failing only `indexed_test_configuration_reports_boundary_and_matches_exact_recall` on the 200-row versus 20-row comparison.

## 2026-09-04 — Export The Endpoint For Ignored Qdrant Unit Tests  [tags: validation, qdrant, environment, worker]

Context:
- Plan: `docs/coding-agent/plans/active/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5
- Roles involved: Worker | Orchestrator

Symptom:
- The first ignored service-Qdrant run failed three tests with `QDRANT_CONNECTION_STRING is required`, while the idle-channel test passed through its separate setup path.

Root cause:
- The integration tests load `.env`, but the ignored service-adapter unit tests read `QDRANT_CONNECTION_STRING` directly; setting only `REQUIRE_QDRANT_TESTS=1` was insufficient.

Fix applied:
- Export the existing `.env` endpoint into the test process together with `REQUIRE_QDRANT_TESTS=1`; all four ignored service-Qdrant tests then passed.

Prevention:
- The live-gate command for ignored Qdrant unit tests must explicitly export both `QDRANT_CONNECTION_STRING` and `REQUIRE_QDRANT_TESTS=1`; do not assume unit tests load `.env`.
- Residual risk / waiver: none.

Evidence:
- Corrected ignored run: 4 passed, 0 failed, 396 filtered out; the exclusive Qdrant window was then released.

## 2026-09-04 — Classify Boundary Conversions By Operation, Not Nearby Helper  [tags: review, errors, conversion, worker]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5 review revision
- Roles involved: Worker | Reviewer

Symptom:
- The service adapter classified a `usize`-to-`u32` scroll-limit overflow as `PayloadDeserialization`.

Root cause:
- The closeout reused the nearby payload-error helper while replacing a stringly error, without checking the semantic kind already used for scope-count width conversion.

Fix applied:
- Construct the existing typed `Conversion` error directly and update the boundary test to require that kind.

Prevention:
- When replacing a stringly error, classify the failed operation first and compare sibling conversions before selecting an existing helper.
- Residual risk / waiver: none.

Evidence:
- Reviewer finding on Task_7 at `src/adapters/qdrant/store.rs`; the corrected overflow assertion requires `VectorDatabaseErrorKind::Conversion`.

## 2026-09-04 — Give Every Deletion Deliverable Its Own Exact Census  [tags: review, deletion, tests, worker]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5 review revision
- Roles involved: Worker | Reviewer

Symptom:
- The fake/type/helper census passed, but the separately required zero test-only payload-field constants still had a `SURFACE_FIELD` alias.

Root cause:
- The closeout census covered named fake artifacts but did not translate every independent deletion requirement into an exact symbol search.

Fix applied:
- Delete `SURFACE_FIELD` and have tests use `QdrantPayloadField::Surface.name()` directly.

Prevention:
- List each deletion deliverable separately and record an exact zero-hit census for each; do not treat one representative census as covering adjacent deletions.
- Residual risk / waiver: none.

Evidence:
- Reviewer finding on Task_7 and `rg -n "SURFACE_FIELD" src tests` after the correction.

## 2026-09-04 — Define Completeness By Proven Work, Not Index State  [tags: review, docs, vector-recall, worker]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5 review revision
- Roles involved: Worker | Reviewer | Orchestrator

Symptom:
- Public and decision-record wording said `Exhaustive` was available only on an unindexed shard, although the service zero-norm full-scope scroll correctly reports it too.

Root cause:
- Documentation generalized one implementation path into the verdict's semantic condition instead of checking every branch that produces the public enum.

Fix applied:
- State that exhaustive means every requested-scope record was scored through a known-exhaustive path, the cutoff cohort closed, and `scanned` came from the records that path actually scored; name unindexed scans and full-scope scrolls as examples.

Prevention:
- Define public verdicts from observable proof conditions and audit every constructor branch before documenting implementation examples.
- Residual risk / waiver: none.

Evidence:
- Orchestrator ruling during Task_7 review and the service zero-norm parity test that reports `Exhaustive` from a full-scope scroll.

## 2026-09-04 — Shut Down Background Owners Before Temporary Directories Drop  [tags: review, cleanup, windows, tests, worker]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5 review revision
- Roles involved: Worker | Reviewer

Symptom:
- The embedded test fixture could drop its temporary directory while the shard still held Windows file handles, and `TempDir` would ignore the cleanup error.

Root cause:
- The fixture first relied on a shutdown signal; the initial correction then relied on a reply that the owner sent after flush but before the `EdgeShard` destructor released its handles.

Fix applied:
- The owner now drops `EdgeShard` before sending the test-only close reply; the fixture waits for that reply on a helper thread, joins it, and only then lets the temporary directory drop.

Prevention:
- A shutdown acknowledgment must be emitted after the owned resource's destructor completes, not merely after its final flush; filesystem tests must also assert the directory is actually removed.
- Residual risk / waiver: none.

Evidence:
- Reviewer finding on Task_7 and `temporary_vector_store_removes_its_directory_on_drop`.

## 2026-09-04 — Explicitly Clean Integration Roots Around Signal-Only Production Drop  [tags: review, cleanup, windows, integration-tests, worker]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5 review revision
- Roles involved: Worker | Reviewer

Symptom:
- The restart contract test let a reopened `CharacterMemory` and its `TempDir` fall out of scope together, so the facade's intentionally signal-only production destructor could race directory deletion and leave one `.tmp*` root on Windows.

Root cause:
- The integration fixture did not own the shutdown-to-cleanup lifecycle explicitly, and sibling embedded cases used the same scope-drop pattern.

Fix applied:
- All embedded `TempDir` cases in the vector-port integration file now drop the facade, retry root removal with a ten-second bound while the owner releases its handles, and assert the root is gone.

Prevention:
- Integration tests around signal-only production destructors must keep the temporary path, perform bounded cleanup after dropping the facade, and include a before/after temporary-root census for the full suite.
- Residual risk / waiver: none.

Evidence:
- Reviewer finding on Task_7 and the full-suite `.tmp*` before/after census recorded in the completed plan.

## 2026-09-04 — Close Every Direct Background-Owner Test Fixture Explicitly  [tags: review, cleanup, windows, tests, worker]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5 review revision
- Roles involved: Worker | Reviewer

Symptom:
- The service-up full suite left one 112 MB `.tmp*` root named `indexed` after the indexed exact-recall adapter test.

Root cause:
- The earlier cleanup audit covered the shared fixture and integration facade cases but missed direct adapter stores whose background owners still held the temporary directory when it dropped.

Fix applied:
- The indexed exact-recall test now awaits both direct-store closes before explicitly closing and asserting removal of its temporary root; the remaining direct point-identity and zero-norm stores also close explicitly.

Prevention:
- Audit every `TempDir`/store pair, require each background owner to acknowledge close before its directory is removed, and compare exact temporary-root counts around both bare and live-switch full suites.
- Residual risk / waiver: none.

Evidence:
- Reviewer item 6; the bare full-suite census held at 149 before and after, and the `REQUIRE_QDRANT_TESTS=1` full-suite census held at 149 before and after.

## 2026-09-04 — Derive Exhaustive Counters From The Records Actually Scored  [tags: review, concurrency, telemetry, vector-recall, worker]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5 Copilot follow-up
- Roles involved: Worker | Reviewer | Orchestrator

Symptom:
- The service zero-norm path used a separate filtered count request for `scanned`, so a concurrent write between that request and the full-scope scroll could make the verdict report a population different from the records it scored.

Root cause:
- The counter was treated as a scope snapshot instead of evidence about the exhaustive operation that produced the candidates.

Fix applied:
- Derive `scanned` from the final closed scroll response, which is the exact scoped record set scored by the zero-norm path.

Prevention:
- A telemetry counter describing completed work must come from that operation's result, not a separate query that can observe different state.
- Residual risk / waiver: none.

Evidence:
- Copilot finding on PR #79 and the service zero-norm live regression.

## 2026-09-04 — Keep One Governing Claim Per Decision Record  [tags: review, adr, durable-docs, worker]

Context:
- Plan: `docs/coding-agent/plans/completed/v0-1-6-embedded-vector-recall-plan.md`
- Task/Wave: Task_7 / Wave 5 Copilot follow-up
- Roles involved: Worker | Reviewer | Orchestrator

Symptom:
- ADR-I-0024 combined recall-completeness semantics with vector-prefilter admission, and the decision set retained unanchored time-relative wording.

Root cause:
- Two nearby port concerns were recorded together without applying the one-decision warrant test or the durable-wording sweep independently to each claim.

Fix applied:
- Keep completeness in ADR-I-0024, move prefilter admission into ADR-I-0028 with its own warrant and revisit conditions, and remove unanchored time-relative wording from ADR-I-0023 through ADR-I-0028.

Prevention:
- Before finalising an ADR cluster, state one governing claim per record and run a time-relative-word census across every record in the cluster.
- Residual risk / waiver: none.

Evidence:
- Copilot findings on planning PR #72 and the post-split ADR census.
