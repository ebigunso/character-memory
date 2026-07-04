# Plan: Remove or justify blanket lint suppressions

- status: in_progress (scope pre-agreed as the deferred reorg follow-up; user directed execution 2026-07-04)
- generated: 2026-07-04
- last_updated: 2026-07-04
- work_type: code

## Goal

- Eliminate module-wide `#![allow(unused_imports)]` and `#![allow(dead_code)]` suppressions so the compiler can see rot again; every surviving allow is item-level with a rationale comment (per repo lessons "Explain Temporary Suppressions" and "Prefer Root-Cause Fixes Over Symptom Patches").

## Definition of Done

- No module-wide (`#![allow(...)]`) lint suppressions remain in `src/` except where a documented structural reason survives review (expected: `test_support.rs`, possibly `reconciliation.rs`).
- Every remaining `#[allow(dead_code)]`/`#[allow(unused_imports)]` is item-level and carries a rationale + removal-condition comment.
- Genuinely dead code is deleted; unused imports removed.
- `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --no-run`, `cargo test` all pass.

## Scope / Non-goals

- Scope: the 26 inventoried suppression sites (2 unused_imports module-wide, 12 dead_code module-wide, 12 item/statement-level) and the code they hide.
- Non-goals: behavior changes; API additions; restructuring (the reorg is done); deleting roadmap-intended API surface.

## Context (workspace)

- Research: Task_9 review + post-completion architecture audit flagged these as rot-masking debt (debt items #8, #11-adjacent in the completed reorg plan's register). Inventory grep run 2026-07-04 (26 sites). Research dispatch waived: inventory is mechanical and prior audit context is fresh.
- Key judgment rule: ports/, models/, api DTO families contain deliberately pre-built surface for roadmap phases — for items unused today but API-shaped (pub(crate) types, trait methods, builders), NARROW the allow to the item with a rationale, do not delete. Delete only provably dead internals (private helpers with no callers, stale imports).
- Known-special sites: `src/test_support.rs` (shared fakes: different tests use different subsets — module-wide allow is structurally justified, keep with comment); `src/usecases/reconciliation.rs` (internal seam pending a governance surface per its own comments — narrow or keep with explicit rationale); `src/policy/graph_expansion.rs:20` (audit explicitly wants this narrowed).

## Open Questions (max 3)

- None.

## Assumptions

- A1: The compiler/clippy under `-D warnings` is the arbiter for what each removal exposes; the worker iterates per file.
- A2: Local full `cargo test` is currently viable (machine stall condition cleared 2026-07-04).

## Tasks

### Task_1: Remove/narrow all blanket suppressions and clean exposed rot
- type: impl
- owns:
  - src/** (suppression attributes, unused imports, dead private code, rationale comments only — no logic changes)
- depends_on: []
- description: |
  For each inventoried site: remove the module-wide allow; run cargo clippy --all-targets -- -D warnings;
  triage each exposed warning: (a) delete unused imports; (b) delete provably dead PRIVATE code;
  (c) for API-shaped or roadmap-intended items, add item-level #[allow(dead_code)] with a one-line
  rationale + removal condition; (d) for test_support.rs (and reconciliation.rs if narrowing is
  impractical) keep a module-wide allow with an explanatory comment. Report per-site outcomes
  (removed / narrowed / kept-with-comment, and lines of dead code deleted).
- acceptance:
  - Zero undocumented module-wide allows in src/
  - All surviving allows are item-level (or documented module-level exceptions) with rationale comments
  - No logic/behavior changes; test counts unchanged or reduced only by deleted dead test helpers
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check && cargo check && cargo clippy --all-targets -- -D warnings && cargo test --no-run && cargo test"

### Task_2: Independent review
- type: review
- owns: []
- depends_on: [Task_1]
- description: |
  Reviewer verifies: no deleted item was roadmap-intended API surface (spot-check deletions against
  ports/models/api families); surviving allows have honest rationales; validation evidence complete.
- acceptance:
  - Reviewer status APPROVED
- validation:
  - kind: review
    required: true
    owner: reviewer
    detail: "Diff review vs acceptance criteria + deletion-safety spot check"

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]

## Rollback / Safety

- Single-purpose branch refactor/lint-suppression-cleanup off merged main; pure-cleanup commit, trivially revertible.

## Progress Log (append-only)

- 2026-07-04: Plan drafted from suppression inventory (26 sites) + reorg audit debt register.
- 2026-07-04 Wave 1 completed: [Task_1 + addendum] (codex worker via agmsg)
  - Summary: all module-wide suppressions removed except the two documented exceptions (test_support.rs shared fakes, reconciliation.rs dormant governance seam, both with rationale + removal conditions); API-shaped dormant seams narrowed to item-level allows with rationale/removal comments; unused imports cleaned; 19 lines of provably dead private helpers deleted (embedded.rs insert_quads/remove_quads, shared.rs graph_object_ref). Inventory gap: two combined-form allows (embedded.rs, shared.rs) were missed by both the plan inventory grep and the worker's done-evidence grep — caught by Orchestrator broad grep, fixed via addendum; lesson recorded.
  - Validation evidence: fmt --check / check / clippy --all-targets -D warnings / test --no-run / FULL cargo test all pass twice (main pass + addendum): 338 lib + 25 integration passed, 0 failed, 3 ignored live smokes. Orchestrator independent broad grep + clippy pass.
  - Notes: one attempted Qdrant constant deletion was self-reverted when all-target clippy showed test ownership — deletion-safety rule worked.

## Decision Log (append-only; re-plans and major discoveries)

- 2026-07-04 Decision: Deletion-safety rule — API-shaped/roadmap-intended items get narrowed allows with rationale, never deletion; only provably dead private internals are removed.

## Notes

- Risks: deleting something a service-gated integration path uses (mitigated: full cargo test now runs locally); allow removal cascading into large diffs in ports/models (mitigated: narrowing is always available).
