# Session Handoff — 2026-07-23 (post structured-verdict-observability closeout)

Audience: the next orchestrator session. Untracked working state; delete once absorbed. Committed records: the archived plan (`docs/coding-agent/plans/completed/structured-verdict-observability-plan.md`, full Decision Log), the design doc + 12 Amendments (`docs/design/structured_verdict_contract.md`), both repos' rule suites (heavily extended this phase), `FOLLOWUP-SEED.md` (next-work index), and auto-memory.

## Setup

- You are `orchestrator` in agmsg team `CharacterMemory`. Arm the inbox watcher per the SessionStart directive (30-60s attach; never double-invoke; on resume-restarts BOTH monitors die silently — re-arm the agmsg watcher yourself if no directive fires, and any PR monitor you need).
- Team (10): codex `worker`/`worker2`/`cm-reviewer`/`cm-researcher` rooted in CM; `evals-worker`/`evals-worker2`/`evals-reviewer`/`evals-researcher` rooted in CME; `ebigunso` via app. IMPORTANT — THE CODEX THREADS PERSIST AND ARE REUSED: only the orchestrator thread was retired. Every agent retains its full session context — standing role instructions, all protocol rules (tripwire, local-first commits, live-run mutex announce discipline, artifact placement, typed-from-introduction), the phase's rulings, and their own lesson history. Do NOT re-register roles or re-teach protocols; open with a brief 'new orchestrator session, context continues' note and dispatch normally. They read at turn boundaries; silence during gate runs is normal; they self-report strict YAML. The researchers are strictly read-only forensic (censuses with file:line, auditable methods, explicit zero-hits) for pre-ruling blast-radius coverage — both decision-grade.
- Where things stand: v0.1.5 + backcompat sweep + observability phase all MERGED (CM main 0408e71→b934d76 docs; CME main ea01f8e→038afdd docs). Everything green, all queues clear, zero unresolved anything.

## Next work (in order, user-confirmed)

1. Qdrant teardown hardening — first item; complete spec + failure catalog in FOLLOWUP-SEED.md. Dispatch shape: evals-worker (CME test support) + CM twin; retires the standing two-test teardown-transport waiver (scoped to observability-phase validation runs — a new phase touching those tests needs its own waiver decision until hardening lands).
2. v0.1.6 embedded vector-recall planning — draft branch `draft/v0-1-6-embedded-vector-recall` still exists; the phase doc does NOT exist yet. MUST absorb: the one-port design pass (R2-03 completeness envelope + CME vector_only capability + R2-05 hint semantics + R2-13 text columns), the Tier-A deferral-reconfirmation checklist (every consumer claim parked on v0.1.6 gets re-verified when the doc is authored), CanonicalCandidates survival expectation.
3. v0.2 (inherits R2-02/R2-04 lifecycle-mode work, R2-01 idempotency ledger, correction-path divergence-rejection residue).
4. Harness candidates: SIX staged in `skill-candidates.md` awaiting promotion (truth-tables era ×3 + workaround-tripwire/design_alerts, lossless-boundary, consolidation-completeness, negative-evidence, coordination/advice split, value-audit triggers).

## Process rules born this phase (all codified — enforce them; listed so you know they're new and why)

Workaround Tripwire (common.md both repos); push-after-internal-approval (orchestrator.md — pushes are the promotion step, workers commit local-only, reviewers pin from the LOCAL repo); Design-Consult Threshold (contract-shape rulings get a Claude design consult BEFORE ruling; skip only with recorded blast-radius); blast-radius rulings (you own everything the change affects — grep the sibling BEFORE ruling; two failures this phase prove why); Value-Audit Triggers (design review consumer-naming, 3rd-bounce-per-seam proportionality, pre-merge milestone gate, next-phase planning); Artifact Placement (.agent-work/<role>/, delete-or-promote stated in reports); typed-from-introduction (worker.md); reader-side admission strictness (CME common.md, SCOPED to hash-cited evidence readers); reviewer two-run trigger = byte-shape intent.

NOT codified but proven practice (codify if it recurs):
- EXIT RUBRIC for terminal review loops (user-directed): in-PR fixes only for phase-delivered evidence-integrity defects or phase-introduced regressions; all else defers to seed with recorded disposition. Ended a very long Copilot tail; apply when a loop's marginal finding severity drops below cycle cost.
- LIVE-RUN MUTEX: shared Qdrant takes ONE live suite at a time; orchestrator schedules exclusive windows; agents announce START/END; prune orphan collections BEFORE granting (both prefixes: `test_collection_*` AND `cmem_eval_*` — sweeping only one was an actual mistake); readyz-probe at grant.
- LIGHT-DELTA path for small fixes: worker local commit → worker2 spot-check → reviewer offline formality → push, one relay each.
- Reviewer worktree provisioning is ORCHESTRATOR duty on handshake: `git worktree add .review-worktrees/<name> <sha> --detach`, remove the stale one; sibling-clone pin flips via stash-shim dance (stash Cargo.toml, fetch origin(=local CM repo), checkout --detach, stash pop — the uncommitted [workspace] shim is expected state).

## Environment facts (thread-only)

- Qdrant: docker container `charactermemory-qdrant-1`. gRPC path DEGRADES over uptime/load (delete-responses lost while REST stays 100ms-healthy; deterministic when degraded, on localhost AND VM-IP routes); remedy = `docker restart` then run promptly in the fresh window. Canonical: explicit `QDRANT_CONNECTION_STRING=http://127.0.0.1:6334` (never env-fallback → IPv6-localhost stall). Serial write_planning legitimately ~353s — set 600-900s caps. Prune recipe: REST DELETE per collection, both prefixes. Delete-timeout triage: REST-check the collection immediately — 404 means committed-but-response-lost (transport), present means server-side.
- Copilot auto-reviews every push; explicit re-request via REST fallback (`gh api .../requested_reviewers -f 'reviewers[]=copilot'`); resolve threads via GraphQL `resolveReviewThread`. It reviewed superbly this phase (~35 accepted findings) — treat its comments as signal, but through the exit rubric.
- CM main is push-protected (PRs only) — closeout docs went via PR #66/#16; plan any direct-to-main docs accordingly.
- agmsg send.sh = exactly `TEAM FROM TO MESSAGE` (extra positional silently eats the body); long reviewer messages sometimes truncate/timeout — ask for resend. The Bash safety classifier had one transient outage; queue sends and retry.
- The design-consult CLAUDE SUBAGENT (resident context: design doc + amendments + all identity rulings) dies with the retired orchestrator thread — unlike the codex agents, which persist. Its replacement: spawn fresh consults reading `structured_verdict_contract.md` + the archived plan's Decision Log — those two documents ARE the resident context, deliberately.

## Loose ends (small, none urgent)

- Branch/worktree cleanup DONE (user-authorized 2026-07-23): all merged branches deleted local+remote in both repos (lineage-verified before deletion — squash merges make commit-count checks useless; ancestor/content checks used instead); all review worktrees removed; empty .agent-work dirs removed; no stashes anywhere. KEPT deliberately: CM `draft/v0-1-6-embedded-vector-recall` (local+remote, needed for v0.1.6 planning), the CME sibling review clone (pinned CM main 0408e71, shim as expected), HANDOFF.md + FOLLOWUP-SEED.md + .claude/ untracked in CM.
- OVERSIZED trims seeded: MutationPlan stats-projection clones (next correct_forget touch); CME mirror vocabulary is the watch-item for dead weight.
- Exit-rubric deferrals live in the seed: bm25_only surface validation (typed-family fix shape recorded); input-side Value-parse sites (fixture/enrichment/loaders/predictions).
- `.agent-work/` gitignored dirs may exist per role — agents self-clean, but verify empty at phase ends.
- Sibling review clone (`CharacterMemoryEvals/.review-worktrees/CharacterMemory`) pinned at CM main 0408e71, shim preserved — re-pin per review needs.
