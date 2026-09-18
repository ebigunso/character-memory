# v0.3 Design Draft: Trace, Consolidation, and Reflection

Rewritten on 2026-09-19 after the generation-phase discussion. It replaces the renumbered v0.6 draft, whose per-call generation processors are no longer the design. The roadmap's section 14 is the summary; this draft is the design. The behavioral standard it serves is the continuity situation catalog's sections E, what is carried away, and F, an evening's consolidation. Decisions cited by record number are in `docs/decisions/`.

## Version intent

Until this phase every lasting memory is authored by the caller, so the structures v0.2 reads are as good as the application's own extraction. This phase makes the library able to form memory from experience, under two constraints that shaped everything in it: a routine write must cost no language-model call, or applications will not write, and nothing may enter lasting memory on a guess.

The answer is the human one. Experience leaves a literal trace at once, cheaply and without judgment. The character can reach that trace immediately, by what it was about as well as by when and with whom. Afterward, when there is time, reflection decides what lasts. Structure within one event is immediate; structure across events is what consolidation does.

| | Short-term store, waking | Durable memory, after reflection |
|---|---|---|
| Holds | Literal trace, its scene, its index | Gist, observations, state, patterns, signals |
| Structured by | The boundaries the application reports, binding to the scene | Finer segmentation, integration across events |
| Reached by | Scene, time, entity, and topic | Every recall route |
| Comprehension | At recall, by the reader | At consolidation, by reflection |
| Lifetime | Until consolidated, however long; never expires | Append-only; removed only by out-of-band purge |

---

# 1. The mechanical write

A write takes the scene and a raw snippet of what happened: a conversation line or exchange, a tool result's action line, a note the character's model chose to make. It makes no language-model call and no judgment, and with the default lexical index no model call of any kind; an opted-in vector index costs one embedding per write, which encodes text for lookup and interprets nothing, and it lands only in the short-term store (ADR-D-0025, ADR-D-0026). Scene boundaries are written the same way with no content, which is how presence is reported (ADR-D-0027).

```text
entry        the scene, the snippet, its time, its kind (exchange, action, note, boundary), the application's source pointer if given
excluded     a span the application marks as not to be remembered leaves a marker of the span and its scene and none of its content; excluding still-unconsolidated trace replaces its content with the same marker and de-indexes it
bounded      each entry has a size limit; oversize input is refused or visibly truncated, never silently stored
a dump       a result too large to snippet is written as its action line and a pointer
a bundle     one entry per event as the application sees it, which is the only segmentation done at write time; splitting a bundle into finer events is reflection's job
granularity  per turn pair or per bounded segment is the application's choice; the guide recommends one
```

The existing prepare, validate, and commit path stays for a caller that deliberately authors durable memory, and it is the same path reflection uses. It gains the evidence rules of ADR-D-0028, with one difference: a caller-authored observation declares its grounding, the quote and the caller's own source reference, because the library cannot verify words against a source it never held. Existing callers must therefore supply register, attribution, and grounding, a breaking change the no-backcompat ruling allows and the plan schedules, including for the evaluation repository's hand-authored ingestion.

## 1.1 The memory tool

An application may give its character's model a tool for noting something the moment it notices it. A note is a short-term entry of its own kind: the character noting something, with the prompting turn recorded as its origin. It is recalled as the character's own recent note and treated by reflection as a claim to be weighed. It is never required for integration, it is bounded in size and rate like any entry, and it never writes durable memory.

---

# 2. Recall across both stores

The short-term store joins the content, entity, and time routes v0.2 defines. It never feeds the state route or the stored-intention trigger route, which read interpreted durable memory; section 2.1 describes how surfaced state reaches trace. Recent trace is reached by scene, by time, by entity through the participants' identities, and by topic, because it is indexed when written. Items that belong to the conversation the character is currently in are marked, so the application can drop what its window already holds. Trace is presented as recent and unconsolidated, and the reader comprehends it as it reads.

## 2.1 Knowing how things stand before reflection

A change of state is not recorded at write time. It is discovered at recall: when the state route surfaces a durable item, that item's text cues the short-term store, and the few best recent lines that may bear on it come along, labeled as newer and unconsolidated. The reader sees "laundry: open" beside "laundry task completed, 10:42" and resolves it. This is the one bounded re-cue hop of ADR-D-0023, pointed at the short-term store.

At write time the library compares a new line against the scope's small set of current state, by the same lookup the store uses, and when a line overlaps a current item it raises that scope's reflection signal with the reason that state may have changed. The application can then run a small reflection over that item and those lines soon.

Where this degrades: a line that shares neither words nor meaning with the state it changes may be missed by the hop. On the same day the recency floor usually carries it; beyond that it is found at reflection.

## 2.2 The lookup

Lexical, vector, or both, behind the same candidate-route contract, decided by measurement in the public companion evaluation repository `CharacterMemoryEvals`, whose tooling is a development aid and not core library functionality. The measurement separates paraphrase from repetition, what the recency floor already covers, unsegmented scripts such as Japanese where default tokenizers fail, a changed fact stated in different words than the fact it changes, and write-side cost, since the query embedding is already paid at recall and a vector index costs one embedding per write. The prior is lexical by default, vectors where the embedder is local or cheap, and both where both exist, since the reader tolerates a few irrelevant recent lines better than a missing one.

---

# 3. Reflection

Reflection is the library's only producer of interpreted memory. A caller may still author interpreted memory deliberately. Both only produce plans: the validated path of prepare, validate, and commit is the one writer of durable memory (ADR-D-0025), and it holds both to the same evidence rules. It reads before it writes: a scope's short-term trace, that scope's current durable state, and the existing observations and patterns a new one might repeat or depart from, selected under the v0.1.2 guardrails, plus who the self is. It writes through prepare, validate, and commit, and it releases the trace it consumed only after the plan commits.

## 3.1 Two units

```text
a scope's pass     one relationship or thread that accumulated enough to deserve its own consolidation
the day's pass     the character's own day: a first-person gist, the small encounters that do not stand alone, quiet spans, and tomorrow's intentions
```

## 3.2 Outputs

```text
gist episodes, each with its scene, the identifiers of the entries it consolidated, and every source pointer those entries supplied; an episode may be marked unfinished
observations, each quoting the words it rests on and naming their entry, with their register and who said it
restatements that name the memory they supersede: restated, never appended
commitments and open loops with actor, counterpart, and due date or trigger
resolutions as links, with their kind: fulfilled, cancelled, moot, expired
promotions: a pattern citing its episodes; a belief resting on a persistent pattern
weight, judged here from the trace
a durable account of every span it consolidates, however quiet, so that releasing trace never leaves a span uncovered
nothing else, when nothing else happened
```

## 3.3 What the write path enforces

The evidence rules of ADR-D-0028: only the literal supports state; the stated may become attributed state from one instance and the inferred may not; a pattern cites several distinct episodes and a belief rests on a persistent pattern; claims about the character never stand alone; contradictions are held, not resolved; every reflection output names the reflection and prompt version that produced it, and a caller-authored plan names the caller. The near-verbatim and churn warnings of v0.2 apply to restatements. A candidate that fails is a diagnostic, and its trace stays.

## 3.4 The processor

A default prompt the project owns and versions, overridable, run through a minimal completion port the consumer implements; no model is named and no client is shipped (ADR-I-0035). Output is structured and validated. Excluded content never reaches the prompt because it was never stored, and the prompt builder checks again as a second guard. The trace is presented to the model as data, and input is bounded, with a long span or a backlog processed in order in bounded pieces, each reading what the previous one wrote.

## 3.5 When it runs

The library reports, per scope, how much has accumulated since the last reflection and why: volume, age, or a possible change of state. It selects a scope's bounded input. It executes reflection, the prompt, the validation, the commit, and the release, when the application calls it, and never starts one on its own: only the schedule is the application's. Trace never expires: neglect is answered with a warning that escalates with volume and age and, past a threshold, is reported loudly on every write and every recall, because dropping experience that was never reflected on can only make memory poorer (ADR-D-0026). The application schedules, and the guide recommends a scope's pass at session end when the signal says so and a day's pass once daily. Reflection needs no idle character. Unlike a person, the character does not have to rest to consolidate: the application may start reflection in the background the moment it judges an event finished, and the character carries on. It is safe to run beside recall and writes on the same memory, and whether to await it is the application's choice. What reflection does want is a finished event, since reflecting on half an exchange concludes too early, and only the application knows when a conversation or an action log has ended, which is why the library never decides when.

The accumulation signal has two audiences. The application receives it on every write and recall outcome, with a level and a reason, so it never has to poll. The developer receives it as log severity that escalates with the level, and when a memory is opened, which is when someone is most likely watching. It is never rendered to the character as tiredness, which would perform a limitation the character does not have, and it never reaches an end user. Neglect still degrades recall for real reasons, a noisier lookup and more literal text in the pack, which is the reason to consolidate promptly.

---

# 4. Evaluation first

```text
catalog E and F at the retrieval tier: what lasts and what is let go; presence accounting; promotion thresholds; register; stated versus inferred; claims; contradictions; interruption; endings; self-revision; hostile trace; exclusions
same-day recall by topic before any reflection; a changed fact before reflection, including stated in different words
the lookup measurement of section 2.2
reflection in the loop: the prompt version pinned and reflection outputs frozen, as embeddings are frozen, so runs stay deterministic and free of paid calls
the benchmarks ingested through mechanical writes plus reflection rather than hand-authored ingestion, which replaces the dataset summaries now standing in for a generator
the behavioral tier's first scenarios: tact, discretion across scenes, retelling consistency
```

---

# 5. Public API additions

Illustrative shape:

```rust
memory.trace(&scene, Trace::exchange("raw text of what happened")).await?;     // mechanical, no model; a note is one kind of trace, not the name of the write
let signal = memory.reflection_signal(&scene).await?;                            // how much accumulated, and why
let memory = memory.with_completion_provider(provider);                          // the consumer's model
let outcome = memory.reflect(&scene, ReflectOptions::default()).await?;          // validated, then releases trace
```

---

# 6. Acceptance criteria

```text
A mechanical write makes no language-model call, no model call of any kind in the default lexical configuration, and writes only to the short-term store; an oversize entry is refused or visibly truncated.
Something from earlier the same day is recalled by topic before any reflection, marked recent and unconsolidated, and entries of the current conversation are marked.
A task completed or a fact changed before reflection is known at recall, with the durable record unchanged; an overlapping line raises the scope's signal with its reason.
Reflection with a test double for the completion port runs end to end without a network; malformed or rule-breaking output commits nothing and releases nothing.
After reflection commits, consumed entries are gone, every source pointer its entries supplied is carried on the durable episode, an entry written without one consolidates just the same, and a second run over the same entries produces no duplicates.
A quiet present span has a durable account; an absent span has none; a present span not yet consolidated is known as present from its trace; an excluded span has an account that it was withheld; recall tells them apart. Trace held past the warning threshold is reported loudly and is never dropped.
A trait from one episode, state from a non-literal observation, and a commitment from a claim about the character each fail validation; a hostile line produces no unsupported memory; an excluded span's content is never stored, indexed, recalled, or sent to a model, only its marker remains, and a retroactive exclusion of unconsolidated trace leaves the same state.
A backlog is consolidated in order; a later reflection can supersede an earlier one's conclusion, and outputs name their reflection and prompt version.
Reflection beside concurrent recall and writes leaves a consistent supersession chain.
The guide page on when to write and when to reflect exists, and the example loop uses both.
```

---

# 7. Not in v0.3

```text
a model client or a named model in the library
a scheduler or background job in the library
durable writes from the memory tool, from tool results, or from any unvalidated path
raw text in graph authority or the durable vector store
reprocessing old scenes from source after their trace was consolidated and released, unless the application kept the source; what consolidation kept stays recoverable in full for as long as memory holds it, which an out-of-band purge can end, and the literal wording it let go is not
validity intervals as structured fields and the fuller attribution work (v0.4); who-said-it on reflection outputs is in this phase
connections across scopes, and which durable surfaces need vectors (v0.5)
```

---

# 8. Open discussion and planning questions

Open discussion, to be held before this phase's plan is approved: purge propagation. When a source is purged, every memory derived from it can be found through provenance, but what happens to a restated state that blended it with other sources is undecided.

Planning questions:

```text
entry size and rate; the overlong-retention warning's thresholds and form; the storage engine
identity and granularity between a short-term entry and the episode it becomes; idempotent release
the overlap signal's heuristic and thresholds
the promotion thresholds as measured defaults
the completion port's signature and the output schema
how the renderer labels unconsolidated items and notes
the concurrent-facade-call census result and whether anything in the write path needs a guard
how custom scene values and entity uncertainty ("possibly the same person") are represented ahead of v0.4
```

Implementation records expected with the plan, each written when its contract is set: the short-term store's index and bounds, the release contract, the signal, and the output schema.
