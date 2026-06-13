# v0.1.4 Design Draft: Continuity Evaluation Harness

## Version intent

Build a deterministic evaluation harness that measures whether the v0.1 family substrate actually serves character continuity, before scoped continuity features are built on top of it.

The v0.1 family has so far been validated through structural acceptance tests and report-only diagnostics:

```text
objects round-trip across stores
provenance links resolve
lifecycle exclusion works
selectivity math is monotonic
fanout stays within caps
```

Those checks prove the machinery works. They do not prove the machinery produces good continuity. The project philosophy's success criteria are behavioral and longitudinal:

```text
relevant recall after long gaps without exact wording
recurring entities anchor recall without flooding context
temporal retrieval beyond semantic similarity
stable behavior shaped by history while accepting correction
inspectable retrieval rationale
```

v0.1.4 builds the instrument that can measure these properties under long-horizon workloads. v0.1.5 then uses that instrument to close out the v0.1 family.

---

# 1. Why this comes after v0.1.3

v0.1.3 completes the generation-ready write path:

```text
prepare -> validate -> commit
RememberWritePlan
MemoryCandidate
CandidateProvenance
idempotent retry-safe writes
```

The harness should exercise the full write surface the library will carry forward, including the write-plan workflow. Building the harness before v0.1.3 would mean re-validating it immediately after the write path changes.

The harness also needs the v0.1.2 guardrails in place, because selectivity and fanout behavior under hub-entity stress is one of the primary things it must measure.

---

# 2. Why this comes before v0.2

v0.2 builds scoped continuity and reflection on top of retrieval behavior:

```text
ContinuityScope
ReflectionJob
RelationshipState
CharacterSignal
OpenLoop
Commitment
CurrentContinuityView
```

If retrieval quality, fanout discipline, or correction safety is weak, v0.2 features will inherit and amplify the weakness. Reflection over polluted context produces polluted derived memory. Character signals reinforced from poorly-retrieved evidence produce false continuity.

The safer sequence is:

```text
first: measure the substrate
then: fix what measurement reveals (v0.1.5)
then: build scoped continuity on a measured substrate (v0.2)
```

A secondary benefit: selectivity smoothing (`alpha`) and fanout shaping (`gamma`) defaults are currently unmeasured guesses. The harness produces the data needed to tune them in v0.1.5, and v0.4 observability work later gains a concrete consumer.

---

# 3. Scope

## 3.1 What the harness is

```text
a development and measurement tool inside the repository
deterministic synthetic long-horizon interaction fixtures
a minimal example assistant loop exercising the public facade
continuity-oriented retrieval-quality metrics
selectivity/fanout measurement under hub-entity stress
persistence and restart measurement under eval workloads
repeatable machine-readable eval reports
```

## 3.2 What the harness is not

```text
not a public benchmark
not a learned retrieval policy or training loop
not a CI-blocking quality gate
not a live-LLM evaluation
not a new public memory facade API
not a new memory object type
```

## 3.3 Non-goals

Do not implement in v0.1.4:

```text
learned retrieval policy
model-graded eval scoring
live LLM calls inside deterministic eval runs
CI-blocking quality gates
public benchmark publication or leaderboard
new memory object types
new public memory facade APIs
changes to retrieval behavior or defaults (that is v0.1.5)
v0.2 continuity concepts in fixtures beyond what v0.1 objects express
```

The harness observes and reports. It does not change library behavior.

---

# 4. Deliverables

```text
synthetic long-horizon fixture generator with fixed seeds
fixture scenario library covering continuity-relevant patterns
minimal example assistant loop using the public facade
metric definitions and metric computation
eval runner producing machine-readable reports
report format with per-query rationale samples
documentation for running and extending evals
```

---

# 5. Fixture design

## 5.1 Determinism requirements

```text
fixed seeds for any randomized generation
no external LLM calls during eval runs
deterministic or fixture-pinned embeddings
stable fixture IDs across runs
identical fixture + config => identical report
```

Embedding determinism options to decide during implementation:

```text
pinned pre-computed embedding fixtures
deterministic local embedding stub with controllable similarity structure
```

A deterministic embedding stub with controllable similarity is preferred, because it lets fixtures express "these episodes are semantically close" as test intent instead of depending on a live provider.

## 5.2 Scenario coverage

Fixtures should simulate months-scale interaction accumulation, compressed into deterministic event sequences:

```text
long-gap recall:
  an entity or topic goes dormant for a long simulated interval, then returns

recurring hub entity:
  one entity accumulates hundreds of incident memories across unrelated contexts

selective entity:
  an entity appears rarely but with high continuity significance

correction chains:
  derived memories superseded once, twice, and with suppression mixed in

thread drift:
  a soft thread accumulates members with declining confidence

temporal structure:
  ordered sequences, intervals, recurring-date patterns, one-off vs repeated events

mixed-salience accumulation:
  high- and low-salience memories competing for the same retrieval context

cross-store stress:
  restart between write and retrieve; stats reopen; persistent graph reopen
```

## 5.3 Entity-neutrality requirements

Fixture entities must be heterogeneous and role-free:

```text
people, places, projects, topics, objects, organizations, factions, custom domain concepts
no fixture or metric may special-case user/assistant/player/NPC roles
hub-entity scenarios must exist for at least three different entity kinds
```

---

# 6. Example assistant loop

A minimal example loop demonstrates the intended integration pattern and gives the harness a realistic call sequence:

```text
observe interaction event from fixture stream
retrieve continuity context for the event
record what was retrieved and why
decide what to remember (fixture-scripted, not model-driven)
write through prepare/validate/commit or remember()
periodically: correct, forget, link per fixture script
```

The loop exercises:

```text
remember()
prepare() / validate_plan() / commit()
retrieve()
correct()
forget()
link()
```

The loop is fixture-scripted and deterministic. It is an example and an eval driver, not a product feature.

---

# 7. Metrics

## 7.1 Continuity recall

After a long simulated gap, do retrievals for a returning entity/topic surface the relevant dormant episodes without exact wording overlap?

```text
inputs: long-gap fixtures with labeled expected-recall sets
measure: recall@k against labeled sets, by gap length
```

## 7.2 Entity continuity without flooding

Do recurring hub entities anchor recall while staying bounded?

```text
measure: share of context pack occupied by hub-incident memories
measure: labeled-relevant hits among hub expansions
measure: fanout budget utilization vs cap per relation/object pair
```

## 7.3 Temporal retrieval quality

```text
measure: retrieval correctness for recency-, order-, and interval-conditioned fixture queries
```

## 7.4 Correction safety

```text
measure: zero suppressed or superseded memories admitted into context packs
measure: superseding memory retrieved where its predecessor would have been
```

## 7.5 Rationale quality

```text
measure: share of context pack members carrying a retrieval rationale
measure: rationale category distribution (semantic, entity, thread, temporal, salience, scope)
```

## 7.6 Context pollution rate

```text
measure: share of context pack members not in the labeled-relevant set for the query
measure: pollution attributed by rationale category (which signal admitted the noise)
```

## 7.7 Fanout discipline

```text
measure: expansions exceeding budget (must be zero)
measure: conservative-fallback activations under induced stats failure
measure: selectivity score distributions across fixture entity kinds
```

Metric thresholds are not pass/fail gates in v0.1.4. The harness reports values; v0.1.5 decides dispositions.

---

# 8. Report format

Eval runs produce machine-readable reports plus a human-readable summary:

```text
run metadata: fixture set, seeds, config snapshot, schema versions, timestamp
metric values per scenario and aggregated
per-query samples: query, context pack contents, rationale, selectivity telemetry
fanout decisions: budgets, utilization, rejections
stats health events and fallback activations
restart/persistence observations
```

Reports should be diffable across runs so v0.1.5 can show before/after evidence for fixes and tuning.

---

# 9. Acceptance criteria

```text
Eval runs are deterministic and reproducible under fixed fixtures and seeds.
Eval runs require no external LLM or embedding-provider calls.
Fixtures include heterogeneous high-degree entities across at least three entity kinds.
No fixture, metric, or harness rule special-cases application roles or entity names.
The harness exercises remember/retrieve/correct/forget/link and prepare/validate/commit.
The harness measures behavior across restart for persistent graph and stats stores.
Eval reports include metric values and per-query retrieval rationale samples.
Reports are machine-readable and diffable across runs.
Selectivity and fanout measurements are recorded in a form usable for tuning defaults.
Running the harness does not modify library behavior or defaults.
```

---

# 10. YAGNI rules

Do not implement in v0.1.4:

```text
model-graded scoring
live-provider eval modes
statistical significance tooling beyond simple aggregation
dashboard or visualization UI
fixture DSL beyond what the scenario library needs
plugin system for third-party metrics
long-running soak/performance benchmarking infrastructure
```

Do design for:

```text
fixture reuse by v0.1.5 regression fixtures
report diffing across runs
new scenarios added cheaply as later phases land
v0.2+ scenario extension (scopes, commitments) without harness rework
v0.4 observability features consuming the same fixture base
```
