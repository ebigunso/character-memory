# Harness Migration Candidates

Repo-local staging for cross-repo harness improvements, per the improvement-loop skill. Entries here are candidates for promotion into first-party harness skills/references; they are not repo rules.

## 2026-07-11 — Admission-signal truth-table testing for diagnostic attribution [from PR #59 rationale-category defect chain]

- Symptom: three successive review rounds found provenance/attribution defects in diagnostic telemetry (categories seeded from destination metadata, relation endpoints conflated with admission causes, order-dependent propagation) that the first fix's tests missed.
- Root cause: tests asserted expected positive categories only — no systematic forbidden-category assertions and no permutation tests where input order is semantically neutral.
- Candidate guidance (for a validation/testing reference in the harness): when implementing or reviewing diagnostic attribution (rationale categories, provenance labels, cause tagging), require (1) a truth table covering each admission signal in isolation and in combination, with BOTH positive and forbidden-category assertions per case; (2) permutation-invariance tests wherever processing order is semantically neutral (e.g. same-depth graph edges ordered by ID); (3) an explicit check that no structural metadata (destination section, endpoint membership) doubles as a causal signal.
- Provenance: character-memory PR #59, commits 279891f → 2e4a7fc; findings by Copilot review and Tier D codex review.

## 2026-07-11 — Choose the data model AFTER deriving the attribution truth table [second lesson from the same defect chain]

- Symptom: even after adopting truth-table testing, two more defect rounds occurred because each implementation abstraction (score-presence, component closure) was chosen before the complete signal-by-path semantics were written down; the abstraction could not represent rows it was never designed for (side branches, relation-specific categories).
- Candidate guidance: for provenance/attribution features, derive the full test matrix FIRST — positive, forbidden, side-branch, fallback, union, root-exclusion, and permutation rows — then select a data model capable of representing every row (here: per-path signal tracking, not set closure). The truth table is a design input, not just a test artifact.
- Provenance: character-memory PR #59, commits 2e4a7fc -> 89108dd.

## 2026-07-11 — Producer-set/consumer-set reconciliation for pre-admission telemetry [reviewer-miss triage, PR #59 round 6]

- Symptom: Tier D review approved pre-hydration fanout telemetry although visibility-layer rows for lifecycle-suppressed intermediate nodes were copied wholesale into the final expansion, where the policy expansion never expanded those nodes.
- Root cause: review verified count timing/value correctness, adapter parity, and absence of double counting, but never reconciled the telemetry PRODUCER set (pre-hydration visibility frontier) against the final eligible CONSUMER set (lifecycle-admitted, actually-expanded nodes). Parity/high-fanout tests used only active nodes, so scope leakage was invisible.
- Candidate guidance (for harness review/diagnostics references): whenever diagnostics or telemetry are computed before hydration, filtering, admission, or dedupe and then attached to a final result, the reviewer must audit that producer-set == final-eligible-set (or that a subset relation is explicitly documented), and require at least one rejected/filtered-candidate negative regression.
- Provenance: character-memory PR #59 round 6; cm-reviewer self-triage after a Copilot catch.
