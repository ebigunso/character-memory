# v0.2 Design Draft: Scoped Continuity and Retrieval Quality

Rewritten on 2026-09-17 from the earlier "scoped continuity and reflection" draft after the v0.2 planning discussion. The roadmap's section 13 is the summary; this draft is the design. Decisions cited by record number are in `docs/decisions/`.

## Version intent

v0.1 stores episodes, observations, entities, soft threads, and derived memories, retrieves a continuity context pack through hybrid recall with bounded expansion, and runs with no external service. v0.2 makes that memory usable as continuity by a builder on day one: a session-open primer for a scope, one character across settings, promises that can be followed up, admission that lets what was said compete with what was summarized, and time as a retrieval signal.

The phase is judged by what a builder observes, not by structural completeness. Its character-layer value stays bounded until the generation phase (v0.3) produces derived memories, because until then every derived memory is authored by the caller. The phase therefore favors the items every builder feels, retrieval quality and the current-state read, over structures that only a producer can fill.

## Inherited from the v0.1.5 closeout

- Admission gating and ranking credit for graph-only evidence, so memories reachable only through graph structure are not starved by vector-scored items at pack admission. The 2026-09-16 re-baseline in the evaluation repository is the planning input: with dataset summaries ingested as derived memories, session-level recall rose and dialog-level recall fell because derived memories take pack slots evidence turns held. Invariant 2.16 (interpreting neighbors travel with the memory they interpret) is this item's strongest case, since a handling request almost never matches a query vector. Pack-admission changes invalidate the pollution and context-size baselines of ADR-I-0022 and re-measure them once.
- Selectivity widening beyond entity roots, which needs retrieval statistics keyed by something other than entities. This phase designs that signal or declines it with recorded evidence.
- Scoped and person-keyed evaluation scenarios (catalog B1 to B3) before any scope implementation. Under ADR-D-0019 their meaning changed: B1 and B2 measure that the frame is present and correct on recall and, at the behavioral tier, that the character does not disclose across it. They never measure that a memory failed to surface.
- The concurrency question. With no background derivation inside the library, the reflection-scheduling form dissolves; what remains is the concurrent-facade-call question the library already has, and the census answers it.

Carried from earlier audits into the planning-time value audit: the parallel stats-projection clones in the correction path (2026-07-22 verdict OVERSIZED); the idempotency-ledger question; the lifecycle DTO modes the implementation rejects, which now includes the archived and deleted retention states.

Caveat: the benchmark gap-bucket baselines rest on small per-bucket samples and are directional input, not thresholds.

---

# 1. Concepts

## 1.1 ContinuityScope is a value

A scope names the context a piece of continuity state belongs to. It is a key, not a stored object:

```text
entity            one continuing entity
entity pair       a relationship between two entities
thread            a soft continuity pattern
source conversation
custom            an application-supplied identifier for a scope model the domain already has
```

Derived memories carry scope keys. The current-state read and the pack sections filter and rank by them. A stored scope object with a label or enumeration enters only when a consumer needs something a graph query over scope keys cannot answer; the value audit names that consumer or the object stays out.

Scope is a continuity focus. It is not a privacy partition; see 1.2.

## 1.2 The frame, and partitions as explicit policy

Every memory carries its frame: who was present, who said it, whether it was firsthand or told, and in which setting. Episodes already carry participants and a source conversation; this phase makes the frame reportable on every admitted memory and defines the retrieval-level shape. Attribution on observations and derived memories (who asserted, firsthand or told) completes the frame in v0.4.

Recall is never gated by the frame by default (ADR-D-0019). An application that must enforce a boundary passes a partition as a query-time option over the frame; the trace records it as an applied policy. The B1 and B2 scenarios exercise both the default and the policy.

## 1.3 The current-state read

`RetrievalIntent::CurrentState` (ADR-I-0016, pulled forward with `Continuity`) reads current continuity state for a scope without vector recall: active threads, current derived state by section (preferences, relationship notes, open loops, commitments, character signals), and recent high-salience episodes, all filtered by currency and scope. It takes a reference time and reports elapsed time since the scope was last touched, so a character returning after months knows it.

There is no separate view type. The read returns the pack, and the pack renderer turns it into prompt text.

## 1.4 Reference time and the temporal signal

Retrieval takes a reference time. Elapsed time since a memory, and relations such as recency, sequence, and anniversary, are ranking signals with their own rationale category, which retrieval does not produce today. The signal is designed against the temporal-patterns and departure scenarios and earns its weight by measurement like the other retrieval defaults (ADR-I-0022). Nothing about it changes eligibility (ADR-D-0018).

## 1.5 Open loops and commitments: status and direction

The derived subtypes stay (ADR-D-0005). What is added is what a prompt renderer must be able to trust: a status (active or resolved) expressed through currency and the existing resolves and fulfills links, and a direction, an actor and a counterpart, so the character can owe and be owed (ADR-D-0020). Resolution goes through the existing link and correct paths; a facade method for it enters only if the scenarios show callers get the recipe wrong.

## 1.6 Interpreting neighbors travel with the memory

Invariant 2.16, stated structurally: when a memory item is admitted, the current memory items linked to it item-to-item (the other end is an episode, observation, or derived memory, not an entity or thread) are admitted with it, and under pack pressure the pair is dropped rather than the neighbor. The relations are the existing ones: about, supersedes, contradicts, resolves, supports. Entity-mediated neighbors stay under selectivity and fanout. Expected pair sizes are small; the plan measures them, and an episode with many derived children is the case to check.

## 1.7 Renderer and example loop

A canonical rendering of a pack or current-state read, favoring gist and stance over quotation and separating what the character may say from what merely shapes it (philosophy 9.3). A minimal retrieve, respond, remember loop in the library's own examples, so the write-path friction is felt by a consumer before scope work bakes more of it in.

## 1.8 Reflection in this phase

Only a signal: a scope has accumulated enough since its last reflection to be worth one. Generation, the reflection record, and the trigger vocabulary belong to v0.3.

---

# 2. Retrieval changes

```text
RetrievalContext gains a scope hint, an intent (Continuity default, CurrentState), a reference time, and an optional partition policy over the frame
admission credits graph-only evidence and admits interpreting neighbors with their target; section budgets account for the pair
selectivity applies beyond entity roots with a new statistics key, or the declination is recorded with evidence
the temporal rationale category is produced
the trace records the applied intent, the applied partition policy, elapsed time, and every omission with its lifecycle or currency reason
```

The seven retrieval modes of the earlier draft are gone; intent and scope hint replace them.

---

# 3. Evaluation first

The evaluation-repository plan comes before the library plan, against the maintained smoke config and the library version pin:

```text
B1 person-keyed separation: frame present and correct on recall; non-disclosure at the behavioral tier; partition policy omits across the frame and the trace says so
B2 group versus one-on-one frames: same topic, different frames, both recalled, disclosure follows the frame
B3 differential relationship states: scope-keyed relationship notes retrieved per scope, current only
temporal patterns and C6 departure: the temporal signal and elapsed-time awareness at session open
graph-only probe: the permanent regression fixture for admission
tasks and favors: instructions recalled across long gaps (ADR-D-0018)
interpreting neighbor: a request not to raise a topic arrives with the topic, and pack pressure never separates them
```

Behavioral-tier judging of discretion is scheduled with the generation phase; this phase absorbs every retrieval-tier property first.

---

# 4. Public API additions

Illustrative shape:

```rust
let context = RetrievalContext::new("what were we working on?")
    .with_scope(ContinuityScope::Thread(thread_id))
    .with_intent(RetrievalIntent::CurrentState)
    .with_reference_time(now);
let outcome = memory.retrieve(context).await?;
let prompt_text = outcome.pack.render(RenderStyle::default());
```

No reflect, reinforce, or resolve methods. Open loops and commitments are read from the pack by section and resolved through link and correct.

---

# 5. Acceptance criteria

```text
A current-state read for a scope returns current derived state and active threads without vector recall, with elapsed time since the scope was last touched.
A memory learned in one setting is admitted when retrieved for another, with its frame reported; a partition applied as a query option omits across the frame and the trace records the applied policy.
Open loops and commitments can be retrieved by scope and by direction without assuming who the main actor is; resolution goes through the existing link and correct paths.
An interpreting neighbor is admitted with the memory it interprets, and pack pressure drops the pair, never the neighbor alone.
A memory reachable only through graph structure can be admitted over a vector-scored item, and the ADR-I-0022 baselines are re-measured once.
Retrieval produces the temporal rationale category, and the temporal-patterns and departure scenarios pass.
Selectivity beyond entity roots is either applied with its new signal or declined with recorded evidence.
The pack renderer and the example loop exist, and the README describes what ships.
No memory's eligibility changes with time, and every omission from a current view names a currency or lifecycle decision.
```

---

# 6. Not in v0.2

```text
reflection that generates text, the reflection record, and trigger vocabulary (v0.3)
first-class OpenLoop, Commitment, CharacterSignal, RelationshipState object types
a CurrentContinuityView type
a stored ContinuityScope object without a named consumer
reinforce, decay, archival
attribution fields on observations and derived memories (v0.4)
```

---

# 7. Questions for planning, not for the decider

```text
the canonical form of a scope key and how custom scopes are namespaced
the retrieval-level shape of the frame before v0.4 adds attribution
where direction lives on the open-loop and commitment subtypes
the interpreting-relation set and how section budgets account for a pair
the temporal signal's form and its measurement design
the renderer's style options and its treatment of the frame
the concurrent-facade-call census result and whether anything in the write path needs a guard
```

Implementation records expected with the plan, each written when its contract is set: the item-to-item co-retrieval guarantee (in the pattern of ADR-I-0006), elapsed time as a measured ranking signal (ADR-I-0022), familiarity and stability derived from evidence (ADR-I-0017), and the frame's retrieval-level shape.

---

# 8. Library boundary

Continuity structures are memory state, not agent orchestration. The library reports the frame and never decides disclosure; the application chooses partitions; the model reads the neighborhood and interprets. If scope modeling grows past a value key, the value audit names the consumer before any object is added.
