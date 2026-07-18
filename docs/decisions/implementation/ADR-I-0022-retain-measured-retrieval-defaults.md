---
status: accepted
adr_type: implementation
date: 2026-07-18
deciders: ["ebigunso"]
consulted: ["Claude Fable 5"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0022: Retain the v0.1.2 retrieval defaults, recording the measured tuning basis and the selectivity scope boundary

## Context and Problem Statement

The v0.1.2 selectivity and fanout defaults were set without workload data: selectivity smoothing alpha 1.0, fanout shaping gamma 1.0, relation/object fanout budgets 0/20 (about-entity derived memory), 0/5 (participant-entity episode), and 0/15 (thread derived memory), plus the per-call candidate limits of 48 vector candidates and 12 graph roots.
The v0.1.5 closeout phase owned measuring them and either tuning them or confirming them with evidence.
The same phase owned revisiting the v0.1.2 decision to apply selectivity only when the vector root candidate is an entity.

Two generations of parameter sweeps were run in the public companion evaluation repository (a development aid, not core library functionality) against the deterministic continuity scenario library.
The first generation ran on the original nine-scenario fixture and found every swept parameter inert, but conservative-fallback dominance and structurally redundant fixture roots made that inertness unmeasurable in principle.
A binding-scale fixture generation followed: a hub entity with 48 incidents across four salience levels and multiple embedding clusters, sufficient write volume for selectivity statistics to score rather than fall back, and a relevance-labeled probe memory reachable only through graph structure.

## Decision Drivers

- Defaults change only on measured benefit; keeping a default is a decision requiring evidence, not a non-decision.
- The tuning constraints of the phase: relation caps remain hard upper bounds, conservative fallback is not weakened, entity-neutrality is not weakened, no per-entity tuning.
- Measurements are conditional on the synthetic evaluation corpus and must say so.

## Decision

Retain every shipped retrieval default: selectivity smoothing alpha 1.0, fanout shaping gamma 1.0, the three relation/object fanout budgets (0/20, 0/5, 0/15), and the default candidate limits (48 vector candidates, 12 graph roots).

Retain the v0.1.2 selectivity scope boundary (selectivity applies to entity root candidates only), and defer its widening to the scoped-continuity phase, because widening requires retrieval statistics keyed by something other than entities — a new retrieval signal outside this phase's scope.

Record the measurement basis and its conditionality as part of this decision rather than in code.

## Measurement Basis

On the binding-scale corpus with scored (non-fallback) selectivity decisions present:

- Alpha at 0.5, 1.0, and 2.0, gamma at 0.5, 1.0, and 2.0, and each fanout budget cap tightened in isolation produced returned sets byte-identical to the default configuration across the full scenario suite.
- Raising the graph-root limit from 12 to 24 and 48 recovered no additional relevant memory, changed no recall or pollution metric, and monotonically worsened context size.
- Root saturation (all 48 candidates selected, zero omissions) still failed to admit the graph-only probe memory, isolating the probe loss to pack admission ranking rather than to any tunable limit; that gap is recorded as a deferred finding for the scoped-continuity phase, since ranking credit for graph-only evidence is a new retrieval signal.
- The selectivity scope question was answered with evidence: episode roots do expand hub-incident edges outside selectivity control (hub context share 1.0 in the hub scenarios), so the boundary is retained as a documented, measured limitation rather than re-affirmed as harmless, and the widening work is deferred with the evidence attached.
- The sweeps also exposed a nondeterministic equal-score admission boundary at the vector store cutoff, fixed within this phase by tie-cohort closure and canonical ordering at the adapter boundary.

Conditionality: all measurements are from one deterministic synthetic corpus with a controllable-similarity embedding provider; values require revalidation on materially different corpora, and alpha/gamma conclusions apply to the scored-decision regime measured there.

## Considered Options

1. Retain all defaults with the measured basis recorded.
2. Tighten fanout caps (for example 10/5/8) on a worst-case-exposure argument despite byte-identical measured outputs.
3. Raise the default graph-root limit toward the vector-candidate limit.

## Decision Outcome

Chosen option: **Option 1**.

Option 2 changes shipped behavior bounds without any measured benefit; the phase's own rule is no change for change's sake, and the caps remain hard bounds either way.

Option 3 is contradicted by the measurements: larger root limits bought context-size cost and nothing else, even at full saturation.

## Consequences

### Positive

- The defaults carry a recorded, reproducible measurement basis for the first time.
- The two real quality gaps found by measurement are precisely scoped and routed: admission ranking for graph-only evidence (deferred, scoped-continuity phase) and deterministic admission (fixed this phase).

### Negative / Tradeoffs

- The values remain tuned to nothing beyond the synthetic corpus; production workloads may motivate retuning once representative corpora exist.
- The selectivity boundary remains bypassable through non-entity roots until the deferred widening work lands.

## Validation

- The findings register in the public companion evaluation repository records both sweep generations, per-configuration artifact hashes, and the preservation audits.
- Library tests continue to pin the default values; no default-value source changed in this phase.

## Revisit When

- A representative production-like corpus exists, or the scoped-continuity phase changes retrieval behavior — rerun the sweep methodology (single-factor configurations, reproducibility pairs, preservation constraints) before and after.
- The deferred admission-ranking work lands; pack admission changes invalidate the pollution and context-size baselines recorded here.
- Warm-statistics regimes become the operational norm; alpha/gamma conclusions were measured at the boundary between fallback and scored decisions.

## More Information

- ADR-I-0010 (continuous selectivity and smooth fanout; the mechanism whose defaults this ADR confirms).
- ADR-I-0016 and the v0.2 roadmap phase (the deferred admission/ranking and selectivity-widening work).
- The v0.1.5 closeout report (findings dispositions and deferred-findings table).
