# Character Memory

## Implementation Philosophy and Design Brief

**Memory as the substrate of persistent AI character**

### One sentence thesis

Character Memory is not a generic memory store. It is an episodic memory substrate that lets an LLM-based assistant, companion, simulation character, or other persistent AI interface preserve continuity, form stable character through remembered experience, and behave less like a new instance every session.

---

## 1. Executive Summary

Character Memory is a library for giving LLM-based assistants and companions long-term episodic memory. Its purpose is not just to store facts or retrieve similar past messages. Its purpose is to support **character continuity**: the sense that the assistant or persistent AI character is the same continuing entity across interactions.

The central idea is that memory and character should reinforce each other. What an assistant remembers influences how it behaves. How it behaves influences what becomes salient and worth remembering. Over time, this feedback loop produces a more stable and recognizable character.

This library should therefore be designed as a memory layer for persistent AI systems, not as a task-agent framework, a vector database wrapper, or a prompt-based persona manager.

---

## 2. Core Philosophy

### Core distinction

**A persona is assigned. A character is accumulated.**

A prompt can assign a persona, but it cannot by itself create durable character. Character emerges from continuity: repeated interactions, remembered events, reinforced preferences, relationship history, unresolved tensions, fulfilled promises, mistakes, corrections, and reflections over time.

Character Memory exists to provide the memory substrate for that continuity. It should help an AI system remember past experience in ways that influence future interpretation and behavior.

### 2.1 Memory should be lived, not merely logged

The system should treat memories as episodes from a continuing relationship, character arc, project, or interaction history, not as inert records. A chat log records that something happened. Character Memory should help the assistant remember why it matters, who or what was involved, what it relates to, and how it should affect future interactions.

### 2.2 Character continuity is the product goal

The implementation should optimize for continuity of behavior, not just recall accuracy. A good system does not merely surface old facts. It helps the assistant or character act as though past interactions have shaped it.

### 2.3 Avoid overclaiming consciousness

The library should not claim to create inner life, sentience, or genuine subjective experience. The practical design goal is observable character continuity: behavior, recall, and relationship context that remain coherent over time.

### 2.4 Entity-neutrality is part of the core design

Character Memory should not assume that memory is always centered on a single user-assistant relationship. The core library should be usable for personal assistants, companions, game/simulation characters, research systems, and developer tools for temporal, entity-based, and relational retrieval.

Therefore the schema should treat entities equally. A user, assistant, player, protagonist, NPC, place, project, object, faction, organization, or topic can all be continuity anchors. Application-specific layers may interpret domain roles, but the core memory system should not hard-code them.

Memory is first-person. The remembering character is itself an entity in its own graph. Its actions are episodes it participated in, its promises are its commitments, and its history persists between interactions with anyone. What the character did yesterday has an answer regardless of who asks. Other people are entities in the character's life, not owners of its memory.

---

## 3. Problem Being Solved

Modern LLM agents are usually optimized for task execution. They can plan, call tools, and complete workflows, but they often lack continuity of experience. This makes them useful but psychologically shallow: each session can feel like a new instance with access to notes.

Common memory approaches are insufficient for persistent companions and assistants:

- **Flat chat history** preserves text but not meaning, salience, or relation structure.
- **Simple vector search** retrieves semantic similarity but often misses temporal order, recurring entities, causality, and relationship history.
- **User profile stores** capture facts about the user but not shared episodes or the assistant's evolving relationship to them.
- **Prompted personas** define surface behavior but do not accumulate history.
- **Task-agent memory** helps complete workflows but does not necessarily support persistent character.

Character Memory should solve the gap between useful task execution and persistent character continuity.

---

## 4. Intended Use Cases

The primary users are builders of AI systems where past interaction should shape future behavior. Examples include:

- LLM-based personal assistants that remember long-running projects, preferences, and relationship history.
- AI companions that should feel continuous across days, months, or years.
- Persistent game or simulation characters whose behavior should be shaped by prior encounters.
- Research systems studying memory, reflection, and long-term behavioral continuity in LLM applications.
- Developer tools for experimenting with temporal, entity-based, and relational retrieval.

The library should still be usable in ordinary assistant products, but its philosophy is strongest when the assistant or character is expected to be more than a stateless task executor.

---

## 5. System Mental Model

Character Memory should sit around the LLM interaction loop. It participates both before generation and after interaction.

1. Observe the current interaction.
2. Recall what the present moment calls for, from the situation itself and from what is being said.
3. Provide memory context to the LLM in a concise, grounded form.
4. Generate the assistant response.
5. Keep a trace of what happened, as it happened.
6. Know at once how things now stand, before there has been time to reflect.
7. Reflect afterward to decide what lasts: what the experience meant, what changed, what repeats, and what to carry forward.
8. Use what lasts to reinforce character continuity over future interactions.

**Key implication:** memory retrieval should not be treated as a final answer. It is context for situated behavior.

---

## 6. Core Concepts

| Concept | Meaning | Implementation implication |
|---|---|---|
| Episode | A remembered event or interaction. | Store event content, time, participants, context, and provenance. Preserve enough episode detail and source provenance for later correction, inspection, and reflection. |
| Trace | What happened, kept as it happened, before it has been reflected on. | Recent experience is held literally and briefly. Consolidation turns it into lasting memory and lets the transient trace go; lasting memory itself is never let go. |
| Entity | A person, project, place, object, topic, character, organization, or recurring concept. | Extract and link entities so memories can be retrieved through relationships, not only similar wording. Treat entity roles as application-level interpretation, not core schema truth. |
| Temporal context | When something happened and how events relate over time. | Support recency, sequence, duration, intervals, anniversaries, change over time, and elapsed time relative to now. |
| Relation | A typed or inferred connection between memories and entities. | Represent relationships such as involved in, caused by, follows from, contradicts, resolved by, or similar to. |
| Salience | Why a memory matters. | Judge importance when the experience is consolidated, using behavioral, emotional, practical, or relational weight. At recall, weigh it against evidence and elapsed time rather than against a stored score that drifts. |
| Reflection | A higher-level interpretation derived from multiple memories. | Generate summaries, patterns, and stable observations with links back to source episodes. |
| Character signal | A stable tendency or preference inferred from memory. | Do not overwrite personality arbitrarily. Derive character signals from remembered evidence and attach them to scope. |
| Continuity | The assistant or character behaves as the same persistent entity over time. | The system should optimize for coherent behavior across sessions, not only recall of isolated facts. |
| Scene | The circumstances a memory was formed in, and the circumstances the character is in now: who is present, where and when, what is going on. | A memory is remembered together with its scene, and the present scene is what brings memories to mind. The character knows whether it was there when something happened or was told about it later. |
| Continuity scope | The scope in which continuity state is meaningful. | The scope a memory belongs to follows from the scene it was formed in; the application should not have to name it. |

---

## 7. Retrieval Philosophy

Retrieval should model recollection, not search alone. Human-like recall is associative: a person, character, place, time, recurring theme, object, project, or recent emotional tone can bring back relevant memories even when the wording is different.

Retrieved memory is material the character reconstructs a response from, not a record it recites. History should bend the response rather than appear in it as citations. The best continuity is invisible until it is tested.

Recall is situated before it is cued. A person walks into a room already carrying what the room calls for: who is here, what was last said with them, what is owed in either direction, what is in progress, what fell due today, what happened this morning. None of that is brought up by the topic, because there is no topic yet. The character should arrive the same way, and what the conversation then turns to should add to that, never replace it.

Purpose is not something the moment hands to the character. What it is trying to do comes up from what it remembers: an unfinished matter, a promise, a project in progress, a settled tendency. A purpose supplied from outside is a persona assigned for one turn. A purpose that surfaces from memory is accumulated character.

Character Memory should therefore combine multiple retrieval signals:

- **Semantic retrieval** for meaning similarity.
- **Temporal retrieval** for recency, order, duration, and time-based relevance.
- **Entity-based retrieval** for recurring people, projects, places, characters, objects, and concepts.
- **Relational retrieval** for connected memories and graph traversal.
- **Salience-aware retrieval** for memories that matter beyond textual similarity.
- **Scope-aware retrieval** for continuity contexts that should not be treated as global.
- **Scene-aware recall** for what the present moment brings up before anything is said: the people here, the place, the time and date, what is in progress, and anything the character was waiting for exactly this moment to do.

A retrieved memory should ideally include why it was retrieved: similar meaning, same thread, same entity, recent event, unresolved thread, repeated pattern, contradiction, relationship relevance, high selectivity, or explicit scope. This makes the system debuggable and reduces arbitrary memory injection.

### 7.1 Recurring entities are continuity anchors, not traversal invitations

Entity continuity is central to recall, but the system should not treat every memory connected to a recurring entity as relevant. Any entity can become high-degree over years of memory accumulation.

A broad entity may still be important. However, retrieval should adapt to entity specificity, relation type, temporal context, salience, lifecycle/currentness, and explicit scope. The system should require additional narrowing evidence before expanding broadly through low-selectivity entity links.

This protects continuity without turning recurring entities into context pollution.

### 7.2 Serendipitous recall should be supported without false continuity

Human-like recall includes weak associative moments: one memory can remind the assistant of another through partial cues, repeated coactivation, or a shared context that is not strong enough to be a formal relationship.

Character Memory should eventually support this kind of serendipitous recall.

However, weak broad co-occurrence should not be treated as durable relationship truth. A recurring entity is a continuity anchor, but not every memory involving that entity should become directly associated with every other memory involving it.

The system should distinguish:

```text
entity incidence
query-time associative activation
association candidate evidence
active AssociativeUnit
strong durable relation
```

This preserves human-like recall while reducing false continuity and graph pollution over long time horizons.

---

## 8. Design Principles for Implementation

- **Design for character continuity first.** Backend choices, vector indexes, and graph storage are implementation details. The user-visible outcome is an assistant or character that remembers and behaves consistently over time.
- **Prefer episodes over isolated facts.** Facts are useful, but character is shaped by remembered events. Preserve the context in which facts emerged.
- **Preserve provenance.** Derived reflections and character signals should link back to source memories. This keeps the system auditable and correctable.
- **Treat raw source material as evidence, not the memory substrate.** Character Memory should model remembered experience through episodes, observations, entities, links, reflections, and current continuity views. Source material should ground those memories through provenance, but the core memory model should not be shaped around raw archival logs.
- **Make time a first-class dimension.** A timestamp is not enough. The system should understand before and after, recent and old, repeated and one-off, ongoing and resolved, and how long it has been. Recall happens at a moment, and the gap since a memory is part of its meaning: a promise from last week and one from last year are not the same memory, and a person who stopped appearing months ago is remembered differently from one seen yesterday.
- **Make entities first-class.** People, projects, places, characters, objects, and recurring concepts are anchors for recall. Entity continuity is central to relationship and scope continuity.
- **Keep entity policy use-case agnostic.** The core library should not special-case user, assistant, player, NPC, protagonist, or other application roles. Application-specific layers can provide scope and role hints.
- **Bound entity expansion.** Recurring entities should support recall without causing unbounded graph traversal or accidental pairwise link growth.
- **Do not collapse memory into unsupported summaries only.** Summaries are useful for compression, but behavior-influencing memory should remain grounded in episodes, observations, provenance, and correction paths.
- **Let memory influence behavior, not dictate it.** The LLM should receive memory as grounded context. It should not be forced into brittle rules from stale memories.
- **Recall is complete.** The character can find any memory it holds. No memory becomes less reachable because time passed, and no stored measure of importance drifts on its own. What changes with time is rank and expression: recent and important material is offered in detail, old and minor material as gist, and the whole is available when it matters. A person's memory of an old event thins with the years, and this character's does not: whatever consolidation kept stays recoverable in full, and that is an advantage to keep, not a trait to imitate away. What consolidation let go, the literal wording of an ordinary exchange, is recoverable only while its trace is still held, or from the source if the application kept it.
- **Forgetting is always someone's decision.** The record is append-only, and forgetting changes influence, not history. It takes exactly two forms, each intentional, reversible, and inspectable: suppression removes a memory's influence, and supersession replaces it with a corrected memory while keeping the old one as history. A memory that is no longer current, such as a finished project or a relationship that ended, is not forgotten: it leaves current views through a change of currency and stays fully recallable. Destructive deletion is not a memory operation; erasure exists only as an out-of-band operational action for compliance, security remediation, or explicit operator-directed alteration.
- **Discretion is disclosure, not recall.** A person told something in confidence still knows it in the next room. They choose not to say it, and knowing it still shapes how they act. Recall is therefore never gated by privacy or sensitivity by default. Every memory carries its scene: who was present, who said it, whether the character was there when it happened, and in which setting. Recall reports that scene with the memory so the character can be consistent and careful at once. Where an application must enforce a boundary, it does so as an explicit query-time policy over the scene, never as a property stored on the memory.
- **A memory's meaning is other memories.** How a memory should be treated comes from what is linked to it: a request not to raise it, a correction, a resolution, a framing, a note about the source. Each of these is itself a remembered event with provenance. The library does not sort memories into treatments or annotate them with instructions, because an experienced fact can mean many things and the boundary between an instruction and an experience is not one the library can draw. When how a memory should be treated changes, the character's memory of it changes with it, and what it remembers afterward reads as one memory, not as a history of edits.
- **Everything experienced leaves a trace; what lasts is decided afterward.** A person cannot know at the moment which remark will matter in a year, and the character should not have to either. What happened is kept as it happened, cheaply and without judgment. Deciding what it meant, what changed, what repeats, and what to carry forward happens later, when there is time to reflect and more is known. Nothing is lost for having seemed unimportant at the time.
- **The character knows how things stand before it has reflected.** In the afternoon it still remembers the morning: what it just did, what it was told an hour ago, what changed since yesterday. Recent experience is as reachable by what it was about as old experience is, and more literal.
- **The character knows when it was there.** A quiet afternoon is part of its life and is remembered as quiet. A stretch it was absent for is known as absence. It never confuses nothing happening with not having been there.
- **What lasts must have earned it.** One remark is an observation, not a trait. A tendency is believed only after it repeats, and is held as a tendency. What someone states plainly about themselves can be taken at their word; what is inferred about them cannot, from one instance. What was said in jest, in play, or as a supposition is remembered as that and never as fact. What others say about the character's own past is a claim, weighed against what it remembers.
- **Expose retrieval rationale.** Implementation designers and application developers need to understand why a memory was retrieved.
- **Stay backend-agnostic where practical.** The default stack can use OpenAI and Qdrant, but the philosophy should not depend on either vendor.

---

## 9. API and Product Implications

The API should reinforce the intended mental model. Names and workflows should make the library feel like a memory system, not just a database client.

### 9.1 Suggested lifecycle operations

- **remember** to store a new episode or memory candidate.
- **retrieve** to find relevant memory context for a current interaction.
- **reflect** to connect episodes into higher-level patterns.
- **link** to connect entities, memories, and relations.
- **correct** to update or revise memory when the user or system clarifies something.
- **forget** to suppress memories when appropriate. Forgetting is never destructive; the record remains for audit, correction, and un-suppression. A request to leave something alone is usually itself a memory to keep, not a suppression to perform.

### 9.2 Data model expectations

- Memory records should include content, timestamp, source interaction, involved entities, relations, salience, confidence, and provenance.
- Reflections should be stored separately from raw episodes and should reference their source memories.
- Entity nodes should be allowed to evolve over time as new memories add evidence or revise old assumptions.
- Contradictions should not simply overwrite old memories; they should be represented as changes, corrections, or conflicts.
- Retrieval results should include score components or rationale when possible.
- Retrieval should not depend on hard-coded entity roles in the core library.

### 9.3 Prompt integration expectations

The library should make it easy for an application to insert memory into the LLM context in a concise, grounded form. Memory context should distinguish between raw remembered episodes, derived reflections, and stable character signals, so the application can decide what the character says and what it is merely shaped by. Any rendering help the library offers follows that distinction and favors gist and stance over quotation.

Example memory context categories:

- Relevant episodes from past interactions.
- Active relationship, project, or continuity scopes.
- Preferences with provenance.
- Character-relevant reflections derived from repeated memories.
- Open loops, promises, unresolved questions, and recent commitments.
- Relevant current beliefs once the later belief layer exists.

---

## 10. What the System Should Avoid

- **Avoid looking like a generic RAG wrapper.** RAG retrieves information. Character Memory should preserve continuity of experience.
- **Avoid turning every remark into a lasting fact.** Everything leaves a trace, but few things deserve to become a belief, and the difference is decided with evidence and time.
- **Avoid personality overwrites.** Character should be reinforced through memory, not replaced by arbitrary labels.
- **Avoid false intimacy.** The system should only use memories it actually has and should make corrections possible.
- **Avoid unexplained recall.** When a memory influences behavior, developers should be able to inspect why it was selected.
- **Avoid silent forgetting.** If a memory stops influencing behavior, someone decided that, and the decision is visible. A memory that quietly faded is a failure the developer cannot diagnose and the character cannot explain.
- **Avoid third-person archive framing.** This is not a world chronicle. It is memory for a continuing assistant, character, or relationship context, held from that character's own point of view.
- **Avoid anthropomorphic claims in documentation.** Describe behavioral continuity and memory-shaped character without claiming consciousness.
- **Avoid hard-coded role assumptions in the core library.** A system that only understands user/assistant roles will not serve companions, simulations, games, research systems, or developer tools well.
- **Avoid unbounded traversal through recurring entities.** Entity continuity is valuable, but broad entities need scope, selectivity, and supporting evidence.

---

## 11. Non-Goals

- Not a complete agent framework.
- Not a replacement for the LLM.
- Not merely a vector database abstraction.
- Not only a chat history store.
- Not only a user profile or preference database.
- Not a claim that the AI has sentience or subjective experience.
- Not a roleplay-only system, even though it can support persistent AI characters.
- Not an application-specific entity-role engine in the core library.

---

## 12. Success Criteria

These outcomes summarize a fuller standard. The continuity situation catalog describes, from lived experience inward, how a character with human-comparable memory behaves: what it recalls unprompted and what it lets fade, how it treats the people it knows well and the ones it barely knows, how it handles being corrected, being told about itself, and the passage of time. The implementation is successful to the degree that a character behaves that way across those situations, whatever it perceives through and whatever mechanism stands behind it.

The implementation should be considered successful if it enables these outcomes:

- After a long gap, the assistant or character can recall relevant past events without requiring exact wording.
- The assistant can connect a current topic to earlier people, characters, projects, commitments, objects, places, or emotional context.
- The assistant can retrieve memories based on temporal relation, not only semantic similarity.
- The assistant can show stable behavior shaped by prior interactions while still accepting correction.
- The application developer can inspect why a memory was retrieved and where derived claims came from.
- The system can distinguish raw episodes, inferred reflections, and stable character signals.
- Broad recurring entities do not flood context merely because they are connected to many memories.
- The assistant feels less like a new instance every session and more like a continuing participant in the user's life or application world.
- Before anything is said, the character already carries what the moment calls for: who is here, what was last said with them, what is owed, what is in progress, and what fell due.
- The character can say what it did earlier today, and knows what changed an hour ago, without having had time to reflect.
- It can tell a quiet stretch from an absence.
- It does not mistake a joke for a fact, a single remark for a trait, or someone's claim about its past for its own memory.
- Surfaced detail matches relevance and importance. The character does not volunteer distant trivia, and it recalls the detail fully when asked.
- Memory failures have a human shape. Uncertainty surfaces as natural hedging with a stated basis, never as confident wrongness about the people and commitments the character knows best, and never as blankness toward something it plainly experienced.

---

## 13. Documentation Positioning

The public README should communicate the purpose before the backend. First-time viewers should understand the library before seeing Qdrant, OpenAI, Docker, or test setup details.

Recommended short description:

> Long-term episodic memory for persistent AI assistants.

Recommended explanatory paragraph:

> Character Memory helps an assistant remember what happened, when it happened, and who or what was involved, so future responses can be shaped by past experience instead of only the current prompt. The goal is character continuity: an assistant that remembers its past can behave more like the same continuing character over time.

The README should then explain hybrid retrieval, typical assistant loop, construction, backend setup, and tests in that order.

---

## 14. Open Design Questions

These are questions for the implementation designer to resolve or make explicit:

- How are memory contradictions represented?
- How does the user or application inspect, correct, or delete memories?
- How are relationship-specific or scope-specific memories separated from global character signals?
- How can the library remain useful without overfitting to one LLM provider, vector backend, or application role model?

---

## 15. Final Guidance

### Design north star

Build Character Memory as the layer that lets an assistant or character carry experience forward.

The implementation should always be evaluated against this question:

> Does this design help the assistant or character remain behaviorally continuous across time?

When in doubt, favor designs that preserve temporal context, entity continuity, provenance, complete recall, correction, bounded retrieval, and reflection. These are the ingredients that let memory shape character instead of merely filling context.
