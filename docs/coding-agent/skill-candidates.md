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

## 2026-07-11 — Cost-gate table and staged-cardinality binding for optional diagnostics [reviewer-miss triage, PR #59 round 7]

- Symptom: review approved optional telemetry although (a) the disabled path still paid the full provenance-walk cost, and (b) a fanout omission metric consumed an already hub-truncated list, silently redefining what "omitted" measured.
- Root cause: review verified value semantics of enabled output and final row filtering/parity, but built neither an execution-cost gate table for the disabled path nor an ordered cardinality table across the chained limiters (eligible -> hub cap -> fanout cap).
- Candidate guidance (harness review references): for every optional diagnostic, review BOTH value semantics and disabled-path work (prove the computation itself is gated, not merely its output). For every chained limiter, enumerate producer cardinality at each stage and bind every emitted metric to exactly one named stage before approval, with boundary tests where stages interact.
- Provenance: character-memory PR #59 round 7; cm-reviewer self-triage.

## 2026-07-11 — Labels are not invariants: prove endpoint-type guarantees before semantic classification [reviewer-miss triage, PR #59 round 8]

- Symptom: review approved a relation-label => Entity mapping although the domain permits Mentions/Involves/About between non-Entity endpoints; entity-less paths were classified Entity.
- Root cause: review validated mapping exhaustiveness and propagation mechanics but accepted relation-name intuition without proving endpoint-type invariants from domain validation and production constructors.
- Candidate guidance (harness review references): whenever a semantic category is inferred from an enum label, require a truth table against all domain-permitted endpoint/state combinations, and cite the specific validation invariant that makes any label shortcut sound; if no invariant exists, classify from the actual node/state types instead.
- Provenance: character-memory PR #59 round 8; cm-reviewer self-triage after a Copilot catch.

## 2026-07-11 — Every emittable category needs a paired positive and zero/negative row at the consumer boundary [reviewer-miss triage, PR #59 round 9]

- Symptom: Salience attribution had absence-style coverage only; a threshold or producer regression could silently remove the category with tests staying green.
- Root cause: review required forbidden/spurious-category rows and broad truth tables, but not at least one production-reachable positive row for every category the classifier can emit, asserted at the final consumer boundary (not helper-level values).
- Candidate guidance (harness validation references): for every enum variant/category a classifier can emit, require one positive row (fixture strictly beyond the production threshold) and one zero/boundary row through the same path, both asserted on the final consumed output. Absence-only coverage is insufficient.
- Provenance: character-memory PR #59 round 9; cm-reviewer self-triage after a Copilot catch.
