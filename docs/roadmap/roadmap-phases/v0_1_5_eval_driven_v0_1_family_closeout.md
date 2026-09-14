# v0.1.5 Design Draft: Eval-Driven v0.1 Family Closeout

## Version intent

Run the v0.1.4 continuity evaluation harness against the full v0.1 family surface, identify weaknesses, fix what should be fixed now, tune unmeasured defaults from measured data, and explicitly close the v0.1 family before v0.2 scoped continuity work begins.

This phase is the v0.1 family's exit gate:

```text
v0.1     starter episodic memory
v0.1 backend  storage contracts
v0.1.1   persistent graph authority
v0.1.2   selectivity and retrieval guardrails
v0.1.3   intake interfaces and write planning
v0.1.4   continuity evaluation harness
v0.1.5   eval-driven closeout  <- this phase
```

After v0.1.5, the substrate is considered measured, tuned, and stable enough to carry scoped continuity, reflection, and the later epistemic layers.

---

# 1. Why this phase exists

The v0.1 family was built against structural acceptance criteria. v0.1.4 builds the instrument that measures behavioral quality. Neither phase owns acting on what the instrument finds.

Without an explicit closeout phase, two failure modes are likely:

```text
findings accumulate as known issues while v0.2 starts on top of them
tuning of alpha/gamma/fanout defaults never happens because no phase owns it
```

v0.1.5 makes findings-to-fixes an owned, finite work item with an explicit end state.

---

# 2. Scope

## 2.1 In scope

```text
running the v0.1.4 harness across v0.1 through v0.1.4 behavior
recording findings as a structured eval report
classifying findings with explicit dispositions
fixing accepted findings in retrieval, guardrails, write path, and persistence
tuning selectivity and fanout defaults from measured data
regression fixtures for every fixed finding
re-running the harness to confirm fixes and tuned defaults
declaring v0.2 entry against the closed v0.1 family
```

## 2.2 Non-goals

Do not implement in v0.1.5:

```text
v0.2 continuity concepts
new memory object types
new public memory facade APIs
new retrieval signals beyond tuning what exists
harness feature growth beyond what findings require
learned retrieval policy
performance optimization beyond fixing measured defects
speculative refactoring
```

If a finding's correct fix requires a new concept or signal, the finding is deferred with a target phase, not fixed here.

---

# 3. Findings workflow

## 3.1 Finding record

Every finding from an eval run is recorded with:

```text
finding ID
scenario and metric that revealed it
observed vs expected behavior
severity: critical / major / minor
suspected layer: retrieval, selectivity/fanout, link guard, write path, persistence, fixture/harness defect
disposition: fix-now / defer / accept-as-designed
rationale for the disposition
target phase, when deferred
```

## 3.2 Disposition rules

```text
fix-now:
  behavior contradicts a v0.1 family acceptance criterion or philosophy invariant,
  and the fix does not require new concepts or signals

defer:
  the correct fix belongs to a later phase's concepts
  (for example: weak serendipity gaps belong to v0.5,
   richer traces belong to v0.4, scope-conditioned retrieval belongs to v0.2)

accept-as-designed:
  the behavior is an explicit documented tradeoff
  (for example: missing weak associative recall under the v0.1.2 link guard)
```

Harness/fixture defects are fixed in the harness and do not count against the library.

## 3.3 Severity guidance

```text
critical: correction safety violations, ungrounded behavior-influencing memory,
          lifecycle exclusion failures, fanout cap violations
major:    poor continuity recall, hub flooding, high pollution rates,
          missing rationale, persistence drift
minor:    suboptimal ranking, noisy diagnostics, rough report output
```

Critical findings cannot be dispositioned accept-as-designed.

---

# 4. Tuning workflow

The v0.1.2 defaults were set without workload data:

```text
selectivity smoothing alpha = 1.0
fanout shaping gamma = 1.0
relation/object fanout budgets (configurable since the v0.1.2 closeout)
conservative-fallback budget
```

v0.1.5 tunes them with measured evidence:

```text
sweep candidate values across the fixture scenario library
compare continuity recall, pollution rate, and fanout discipline metrics
prefer values that improve recall without raising pollution or breaching caps
record the chosen values together with the comparison data that justified them
update shipped defaults and document the tuning basis
```

Tuning constraints:

```text
relation caps remain hard upper bounds
conservative fallback on unhealthy stats is not weakened
entity-neutrality is not weakened
no per-entity or per-name tuning of any kind
```

## 4.1 Selectivity scope revisit

v0.1.2 applies selectivity only when the vector root candidate is an Entity. That boundary was documented as intentional during the v0.1.2 closeout, with this phase named as the revisit point.

v0.1.5 should answer with eval data:

```text
Does entity-root-only selectivity leave measurable hub flooding through non-entity roots?
If yes: widen selectivity application and measure again.
If no: re-affirm the boundary and record the evidence.
```

---

# 5. Fix workflow

Every fix-now finding follows:

```text
write or extend a regression fixture that reproduces the finding
fix the defect within existing concepts and signals
re-run the affected scenarios and the full structural test suite
confirm the metric moved and no other metric regressed
record before/after report references on the finding
```

Fixes must preserve all v0.1 family invariants:

```text
Oxigraph decides graph truth and final inclusion
Qdrant remains candidate recall only
stats remain derived, rebuildable policy metadata
behavior-influencing derived memory keeps episode/observation provenance
suppressed/superseded memories stay excluded by default
no entity identity or application role special-casing
```

---

# 6. Closeout declaration

The v0.1 family is closed when:

```text
all fix-now findings are resolved with regression coverage
all deferred findings carry rationale and a target phase
tuned defaults are shipped and documented with their measurement basis
the full harness re-run shows no critical findings
all v0.1 through v0.1.4 structural acceptance criteria still pass
the closeout report is recorded and linked from the roadmap or plans archive
```

v0.2 entry is then explicitly confirmed against the closeout report.

---

# 7. Acceptance criteria

```text
Eval findings are recorded with severity and disposition.
Critical findings are never dispositioned accept-as-designed.
Every fix-now finding is resolved and covered by a regression test or fixture.
Deferred findings carry rationale and a target phase.
Tuned defaults are documented together with the measurements that justified them.
The selectivity scope boundary is widened or re-affirmed with recorded evidence.
Fixes introduce no new memory object types, public facade APIs, or retrieval signals.
All v0.1 through v0.1.4 acceptance criteria still pass after fixes and tuning.
The final harness re-run shows no critical findings.
v0.2 entry is explicitly confirmed against the closed v0.1 family.
```

---

# 8. YAGNI rules

Do not implement in v0.1.5:

```text
new retrieval signals
new memory concepts
learned or adaptive tuning loops
continuous tuning infrastructure
automatic finding triage
performance benchmarking beyond defect confirmation
v0.2 scope inference or reflection previews
```

Do design for:

```text
findings and dispositions reusable as v0.2+ planning input
regression fixtures joining the permanent harness scenario library
tuning methodology repeatable when later phases change retrieval behavior
deferred findings feeding directly into v0.2/v0.4/v0.5 phase planning
```
