# Plan: four logged residuals of the v0.2 recall slices are fixed, each with its measurement

- status: in-progress
- generated: 2026-10-15
- last_updated: 2026-10-15
- work_type: code

## Goal
- Close four residuals that the v0.2 slices logged and did not fix, chosen by the decider on 2026-10-15 (ruling 81): a character with a long history of beliefs about one person recalls them as fast as it recalls its promises; a character whose recall statistics once failed recovers without an operator; a second person present during an activity brings their own state as the first does; and what a busy occasion brings does not depend on how its identifiers happen to sort.

## Definition of Done
- The subject-state read selects exactly what it selects at the base, and at 2,000 beliefs about one person it takes no longer than the same order of time as the trigger and due reads at that size (about 0.15 s on this machine, against about 9 s at the base). The bound is ruling 67's.
- A recall-statistics store marked unhealthy becomes healthy again without operator action and without a new public call: the next time the store is opened, its statistics are rebuilt from the graph store and the marker clears. A rebuild that fails leaves the marker in place with its cause. What retrieval selects with healthy statistics is unchanged.
- With an activity and two people present, each person brings their own state memories by the same rule; the second person's count no longer drops from two to one at any activity floor, and nothing the first person or the activity brought is lost to make the room unless the floors say so.
- Busy-occasion selection gives the same result in both identifier orders.
- Every task is measured as its validation states, with before and after numbers, a paragraph on what they mean for the character and a verdict in this plan's Decision Log. The plan is not complete until they exist.

## Scope / Non-goals
- Scope: the four fixes, their tests and witnesses, their measurements, and this plan.
- Non-goals: the unlived-topic relevance floor (needs real embeddings; ADR-I-0036); the reserved root seat's subtree cost; the LoCoMo `question_id`; the ADR-I-0007 constants; any new public API; any change to what retrieval selects beyond the residual each task names.

## Design
- Each task first finds the root cause and states it in its report. If the fix needs a contract change, a new public surface, a stored-shape change or a ruling that is not in this plan, the worker stops and reports instead of working around it (workaround tripwire).
- Task_1 (subject-state read): the selector is `select_state` in `src/adapters/oxigraph/sparql_selectors.rs`. The trigger and due selectors beside it read the same scale in about 0.15 s, so the expected shape is the same bounded selection pushed into the query; the result must be identical.
- Task_2 (statistics recovery): the statistics are derived from the graph, so they can be recomputed from it. The chosen recovery is a rebuild at open when the stored marker is unhealthy, in the composition path that already chooses the in-memory unhealthy fallback (`src/composition.rs` near 309). No new public call, because no application can know when to call it and a character should not need an operator to recover.
- Task_3 (second person under activity pressure): the consolidation's principle 4 rounds among the scopes of one kind. The worker finds why the second person's state room shrinks when an activity is present (a shared budget, a round that counts the activity, or an order effect) and fixes the rule, not the count.
- Task_4 (busy-occasion order): the selection must break ties by a key that does not depend on identifier order where identifiers carry no meaning (recorded time, then salience, then identifier as the last resort), or the worker shows the dependence is in input order the caller controls.

## Integration
- Base: the renderer plan tip `ac41e5c0`, which carries the whole library stack through the prospective slice. The renderer's tasks touch no retrieval code, so the two lines run in parallel and are joined by plain merges before the closeout re-baseline.
- Tasks 3 and 4 both touch selection in graph expansion, so they run in sequence with one worker. Tasks 1 and 2 touch other files and run in parallel.
- After this plan, the evals Task_6 re-baseline runs once on the finished library.

## Tasks

### Task_1: A long history of beliefs about one person is read as fast as promises are
- type: impl
- owns: src/adapters/oxigraph/sparql_selectors.rs and its tests; any graph port file the selector's signature needs (report it)
- depends_on: []
- acceptance:
  - The root cause, stated with the query before and after.
  - At the base and the tip, the same store gives the same selected beliefs, in the same order, in both identifier orders.
  - A timing witness outside the permanent suite at 2,000 beliefs about one person, before and after, interleaved, three warm samples each.
  - Existing tests pass unchanged.
- validation:
  - kind: command; owner: worker; "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test; the equality and timing witnesses"
  - kind: review; owner: reviewer; "Tier D: the result is identical, the bound is in the query, the witness is not in the suite"
  - kind: command; owner: orchestrator; "Measurement: the timing witness before and after (instrument), plus the evals familiar-person and state families at the tip with native results identical to the base (control). Falsified if any selected belief differs or the after time at 2,000 beliefs is above 1 s."

### Task_2: A character whose recall statistics failed recovers on its own
- type: impl
- owns: src/composition.rs, src/adapters/stats/*, src/policy/retrieval_selectivity.rs (only if the marker path needs it), their tests
- depends_on: []
- acceptance:
  - A store marked unhealthy and reopened is healthy, with statistics equal to those a fresh build from the same graph gives.
  - A rebuild that fails leaves the marker and its cause, and retrieval behaves as it does with an unhealthy marker today.
  - No new public call or setting.
- validation:
  - kind: command; owner: worker; "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review; owner: reviewer; "Tier D: no new public surface, the rebuild is from the graph, a failed rebuild keeps the marker, healthy behavior unchanged"
  - kind: command; owner: orchestrator; "Measurement: an integration witness that marks a store unhealthy, reopens it and retrieves, before (selectivity stays degraded) and after (selectivity equals a healthy store's) as the instrument; the evals scene and keyless families with native results identical to the base as the control. Falsified if the reopened store's selected memories differ from a never-failed store's."

### Task_3: A second person present during an activity brings their own state
- type: impl
- owns: the graph expansion and selection files the cause lies in (report them); their tests
- depends_on: []
- acceptance:
  - The root cause and the rule changed.
  - With an activity and two people present, each brings two state memories at every activity floor in the evals activity-pressure story.
  - Existing tests pass, or each changed expectation is listed as an intended change with its reason.
- validation:
  - kind: command; owner: worker; "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review; owner: reviewer; "Tier D and Tier A: the rule, not the count, is fixed; principle 4 holds"
  - kind: command; owner: orchestrator; "Measurement: the evals activity-pressure family before and after, both id orders, twice. Falsified if the second person still brings fewer state memories than the first under the same floors, or the topic's on-topic count falls below the base at any floor."

### Task_4: What a busy occasion brings does not depend on identifier order
- type: impl
- owns: as Task_3
- depends_on: [Task_3]
- acceptance:
  - The root cause and the tie key.
  - The family that showed the dependence gives the same result in both identifier orders.
- validation:
  - kind: command; owner: worker; "cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review; owner: reviewer; "Tier D: the tie key carries meaning and identifier order is the last resort"
  - kind: command; owner: orchestrator; "Measurement: the family that showed the dependence, both id orders, twice, before and after. Falsified if the two orders still differ."

## Task Waves
- Wave 1: [Task_1, Task_2, Task_3]
- Wave 2: [Task_4]

## Progress Log (append-only)
- 2026-10-15: plan written; Wave 1 dispatched.

## Decision Log (append-only)
- 2026-10-15 Decision: the decider chose these four residuals for v0.2 (ruling 81) and asked for them to run while Codex capacity is available. One plan with four tasks, not four plans: each task is small, and they share one base and one measurement pattern.
  - User approval: the decider chose the scope; plan approval is waived under the standing authorization.
  - Record proposed: none.
