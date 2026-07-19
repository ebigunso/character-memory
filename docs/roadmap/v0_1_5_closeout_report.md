# v0.1.5 Closeout Report: Eval-Driven v0.1 Family Closeout

Date: 2026-07-19.
Status: v0.1 family CLOSED; v0.2 entry confirmed.
Evidence of record: the findings register in the public companion `CharacterMemoryEvals` repository (`reports/v0-1-5-findings-register.md`), which carries per-finding dispositions, run configurations, artifact hashes, and reproduction provenance for every claim summarized here.

## What this phase did

v0.1.5 ran the v0.1.4 continuity evaluation harness across the full v0.1 family surface, recorded findings with severity and disposition, fixed accepted findings with regression coverage, confirmed the never-measured v0.1.2 defaults with binding measurements, resolved the selectivity scope boundary with evidence, and substantially expanded the evaluation suite along the way.
The phase closed with a repeated-run confirmation over a 33-scenario suite (15 canonical scenarios and 18 benchmark-adapted scenarios) with byte-identical run pairs, zero invariant violations, and no critical findings.

## Findings and dispositions

Eleven findings were recorded; none remain open and none are critical.

| Finding | Severity | Layer | Disposition |
|---|---|---|---|
| F-BASE-1: correction retrieval returned a stale pre-correction observation | critical (draft) → reclassified | fixture/harness | Fixed in the harness: diagnosis proved the library upheld its explicit-target and provenance-cascade contracts; the scenario's forget event was corrected. Post-fix: replacement recall 1.0, pollution 0. |
| F-BASE-2: high pollution and context packs exceeding full history | major | retrieval / fixture semantics | Fixed in part (fixture label semantics, event-level pollution metric, write-path echo warnings); residual admission gap deferred to v0.2. |
| F-BASE-3: hub expansion through non-entity roots bypasses selectivity | major | selectivity/fanout | Deferred to v0.2: widening selectivity requires non-entity-keyed statistics, a new retrieval signal outside this phase's scope. Evidence recorded. |
| F-BASE-4: conservative fallback dominates cold-statistics selectivity | minor | selectivity/fanout | Accepted as designed, with a warm/cold measurement-stratification requirement applied to all subsequent sweeps. |
| F-SEED-1: hub entity roots truncated at the default graph-root limit | major | retrieval | Accepted as designed: measured harmless at full root saturation on a binding-scale fixture; grounded in the philosophy's bounded-expansion intent; scale conditionality recorded. |
| F-SEED-2: nondeterministic pack composition under equal-score ties | major | retrieval (vector admission) | Fixed in the library: tie-cohort closure and canonical ordering at the vector-store boundary; repeated-run byte-identity verified end-to-end. |
| F-SEED-3: graph-only relevant memories lose pack admission to vector-scored items | major | retrieval | Deferred to v0.2, joined with the F-BASE-2 residual as one design item: admission gating and ranking credit for graph-only evidence. The graph-only probe scenario is its permanent regression fixture. |
| F-HARNESS-1: root-selection counters not projected into reports | major | harness | Fixed: counters projected; the tuning observation is now derived from measured telemetry. |
| F-HARNESS-2: reproducibility hash recipe under-specified | minor | harness | Fixed: canonicalization recipe fully pinned. |
| F-FIXTURE-1: pollution labels penalized behavior-shaping temporal and recurrence context; echo surfaces masked per-surface value | major (metric validity) | fixture | Fixed: labels corrected, a distinct-surface scenario added, event-level pollution introduced. |
| F-HARNESS-3: benchmark frozen-store runtime-surface mismatch | major | harness | Fixed: embedding manifests enumerate runtime-normalized lookup surfaces in strict bijection with the store, guarded by a live cross-repository drift regression. |

## Library changes shipped by this phase

- Deterministic vector admission: equal-score cohorts at the vector-store cutoff are closed via bounded overfetch, canonically ordered, and truncated deterministically (fixes F-SEED-2; regression: repeated live all-tied searches and full-pack permutation equality). One documented residual: a pathological cohort larger than the overfetch bound can still vary in membership; realistic cohorts close well inside it.
- Write-path warning diagnostics: a lifecycle-mutation warning when a correction/forget cascade would suppress a currently-current supersession replacement, and a write-plan validation warning for echo surfaces (observation/derived content byte-identical to its source episode). Warn-only; no write behavior changed. These implement the principle that memory-quality enforcement belongs at the write path, never in post-hoc retrieval manipulation.
- Embedded persistent Oxigraph is the validated default graph store; the unvalidated HTTP service mode was removed (ADR-I-0021). Configuration rejects the removed mode with a migration hint.
- Retrieval defaults retained with a recorded measurement basis (ADR-I-0022): alpha 1.0, gamma 1.0, fanout budgets 0/20, 0/5, 0/15, candidate limits 48/12. Two sweep generations showed every swept parameter inert or harmful to change on the measured corpora; the basis and its conditionality are part of the decision record.
- Selectivity scope boundary retained as a documented, measured limitation: non-entity roots do expand hub-incident edges outside selectivity control; the widening work is deferred to v0.2 because it requires a new signal (ADR-I-0022 records the resolution).

## Evaluation capability shipped by this phase (companion repository)

- Run-config sweep plumbing for the library's selectivity/fanout settings.
- A binding-scale fixture generation whose statistics score rather than fall back and whose graph-only probe makes root-selection and admission behavior measurable.
- A frozen real-embedding store: vectors generated once offline from a real embedding model, persisted, and replayed deterministically; generation-time ranked-cosine validation enforces that authored texts actually carry their intended semantics; adapter-boundary provenance guards prevent live evidence on non-real vectors.
- Five new catalog-derived scenarios (graded similarity, combined life, temporal patterns, entrenched correction, autobiographical continuity), expanding the canonical suite to 15 scenarios and 23 queries with genuine authored texts.
- Eighteen benchmark-adapted scenarios converted from LongMemEval-S and LoCoMo with byte-exact source text, mechanical evidence-label conversion, an abstention scenario class (empty relevant sets with pollution-only scoring), and full license attribution. The converter regenerates all artifacts byte-identically from the official datasets.
- A continuity situation catalog (`docs/design/continuity_situation_catalog.md`): a durable, mechanism-independent behavioral specification of what human-comparable continuity requires across companion, small-circle, and independent-entity deployments, with a deterministic-versus-behavioral evaluation tier taxonomy.

## Confirmation evidence

- Canonical suite (15 scenarios), shipped defaults, run twice: byte-identical pairs; all ten pre-expansion scenario metrics reproduce the earlier confirmation exactly; zero invariant violations.
- Benchmark suite (18 scenarios), shipped defaults, run twice: byte-identical pairs; zero invariant violations; metric registries complete, with abstention fanout reported as unsupported rather than fabricated.
- First-live benchmark baselines (measurements, not thresholds): short-gap recall 0.521@5 / 0.865@10, medium-gap 0.333, long-gap 0.125@5 / 0.625@10, context pollution 0.451, event pollution 0.294, context reduction 0.772.
- These baselines on realistic conversational data are materially harder than the synthetic-scenario results and are the intended planning input for the v0.2 retrieval work (in particular the deferred admission/ranking design item).

## Deferred findings and forward roadmap notes

Deferred to v0.2 (scoped continuity):
- Admission gating and ranking credit for graph-only evidence (F-SEED-3 joined with the F-BASE-2 residual), with the graph-only probe and the benchmark baselines as its measured starting point.
- Selectivity widening beyond entity roots (F-BASE-3), requiring non-entity-keyed retrieval statistics.

Roadmap notes recorded by this phase:
- v0.1.6 (proposed next): embedded vector candidate recall — an embedded store mode behind the existing vector port (SQLite exact-scan first, embedded ANN as escalation), completing the zero-infrastructure local deployment story now that graph authority defaults to embedded persistent storage. ADR-I-0003's revisit clause is formally triggered.
- Remote graph authority remains a demand-conditional future phase (multi-replica deployments), per ADR-I-0021; the removed HTTP adapter is reference material, not a reinstatement path.
- A behavioral (model-judged) evaluation tier is the recorded future home for the situation catalog's expression-level qualities; the deterministic tier absorbed every retrieval-level property first.

## v0.2 entry confirmation

Against the phase's closeout criteria: all fix-now findings are resolved with regression coverage; all deferred findings carry rationale and a target phase; the retained defaults are documented with their measurement basis; the selectivity boundary is resolved with recorded evidence; the final harness re-run shows no critical findings; and the v0.1 through v0.1.4 structural acceptance criteria continue to pass.
The v0.1 family is closed, and v0.2 scoped-continuity work is confirmed to enter against this report.
