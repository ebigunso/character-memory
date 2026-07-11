# Harness Migration Candidates

Repo-local staging for cross-repo harness improvements, per the improvement-loop skill. Entries here are candidates for promotion into first-party harness skills/references; they are not repo rules.

## 2026-07-11 — Admission-signal truth-table testing for diagnostic attribution [from PR #59 rationale-category defect chain]

- Symptom: three successive review rounds found provenance/attribution defects in diagnostic telemetry (categories seeded from destination metadata, relation endpoints conflated with admission causes, order-dependent propagation) that the first fix's tests missed.
- Root cause: tests asserted expected positive categories only — no systematic forbidden-category assertions and no permutation tests where input order is semantically neutral.
- Candidate guidance (for a validation/testing reference in the harness): when implementing or reviewing diagnostic attribution (rationale categories, provenance labels, cause tagging), require (1) a truth table covering each admission signal in isolation and in combination, with BOTH positive and forbidden-category assertions per case; (2) permutation-invariance tests wherever processing order is semantically neutral (e.g. same-depth graph edges ordered by ID); (3) an explicit check that no structural metadata (destination section, endpoint membership) doubles as a causal signal.
- Provenance: character-memory PR #59, commits 279891f → 2e4a7fc; findings by Copilot review and Tier D codex review.
