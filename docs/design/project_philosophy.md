# Character Memory

## Implementation Philosophy and Design Brief

**Memory as the substrate of persistent AI character**

### One sentence thesis

Character Memory is not a generic memory store. It is an episodic memory substrate that lets an LLM-based assistant preserve continuity, form stable character through remembered experience, and behave less like a new instance every session.

---

## 1. Executive Summary

Character Memory is a library for giving LLM-based assistants and companions long-term episodic memory. Its purpose is not just to store facts or retrieve similar past messages. Its purpose is to support **character continuity**: the sense that the assistant is the same continuing entity across interactions.

The central idea is that memory and character should reinforce each other. What an assistant remembers influences how it behaves. How it behaves influences what becomes salient and worth remembering. Over time, this feedback loop produces a more stable and recognizable character.

This library should therefore be designed as a memory layer for persistent AI systems, not as a task-agent framework, a vector database wrapper, or a prompt-based persona manager.

---

## 2. Core Philosophy

### Core distinction

**A persona is assigned. A character is accumulated.**

A prompt can assign a persona, but it cannot by itself create durable character. Character emerges from continuity: repeated interactions, remembered events, reinforced preferences, relationship history, unresolved tensions, fulfilled promises, mistakes, corrections, and reflections over time.

Character Memory exists to provide the memory substrate for that continuity. It should help an AI assistant remember past experience in ways that influence future interpretation and behavior.

### 2.1 Memory should be lived, not merely logged

The system should treat memories as episodes from the assistant-user relationship, not as inert records. A chat log records that something happened. Character Memory should help the assistant remember why it matters, who was involved, what it relates to, and how it should affect future interactions.

### 2.2 Character continuity is the product goal

The implementation should optimize for continuity of behavior, not just recall accuracy. A good system does not merely surface old facts. It helps the assistant act as though past interactions have shaped it.

### 2.3 Avoid overclaiming consciousness

The library should not claim to create inner life, sentience, or genuine subjective experience. The practical design goal is observable character continuity: behavior, recall, and relationship context that remain coherent over time.

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

The library should still be usable in ordinary assistant products, but its philosophy is strongest when the assistant is expected to be more than a stateless task executor.

---

## 5. System Mental Model

Character Memory should sit around the LLM interaction loop. It participates both before generation and after interaction.

1. Observe the current interaction.
2. Retrieve relevant memories using semantic, temporal, entity, and relational signals.
3. Provide memory context to the LLM in a concise, grounded form.
4. Generate the assistant response.
5. Decide what from the interaction is worth remembering.
6. Store new episodes with time, entities, relations, salience, and provenance.
7. Reflect periodically to connect episodes into higher-level patterns.
8. Use those patterns to reinforce character continuity over future interactions.

**Key implication:** memory retrieval should not be treated as a final answer. It is context for situated behavior.

---

## 6. Core Concepts

| Concept | Meaning | Implementation implication |
|---|---|---|
| Episode | A remembered event or interaction. | Store event content, time, participants, context, and provenance. Preserve raw episode data where possible. |
| Entity | A person, project, place, object, topic, or recurring concept. | Extract and link entities so memories can be retrieved through relationships, not only similar wording. |
| Temporal context | When something happened and how events relate over time. | Support recency, sequence, duration, intervals, anniversaries, and change over time. |
| Relation | A typed or inferred connection between memories and entities. | Represent relationships such as involved in, caused by, follows from, contradicts, resolved by, or similar to. |
| Salience | Why a memory matters. | Rank and store importance using behavioral, emotional, practical, or relational weight. Salience should evolve. |
| Reflection | A higher-level interpretation derived from multiple memories. | Generate summaries, patterns, and stable observations with links back to source episodes. |
| Character signal | A stable tendency or preference inferred from memory. | Do not overwrite personality arbitrarily. Derive character signals from remembered evidence. |
| Continuity | The assistant behaves as the same persistent character over time. | The system should optimize for coherent behavior across sessions, not only recall of isolated facts. |

---

## 7. Retrieval Philosophy

Retrieval should model recollection, not search alone. Human-like recall is associative: a person, a place, a time, a recurring theme, or a recent emotional tone can bring back relevant memories even when the wording is different.

Character Memory should therefore combine multiple retrieval signals:

- **Semantic retrieval** for meaning similarity.
- **Temporal retrieval** for recency, order, duration, and time-based relevance.
- **Entity-based retrieval** for recurring people, projects, places, and concepts.
- **Relational retrieval** for connected memories and graph traversal.
- **Salience-aware retrieval** for memories that matter beyond textual similarity.

A retrieved memory should ideally include why it was retrieved: similar meaning, same entity, recent event, unresolved thread, repeated pattern, contradiction, or relationship relevance. This makes the system debuggable and reduces arbitrary memory injection.

---

## 8. Design Principles for Implementation

- **Design for character continuity first.** Backend choices, vector indexes, and graph storage are implementation details. The user-visible outcome is an assistant that remembers and behaves consistently over time.
- **Prefer episodes over isolated facts.** Facts are useful, but character is shaped by remembered events. Preserve the context in which facts emerged.
- **Preserve provenance.** Derived reflections and character signals should link back to source memories. This keeps the system auditable and correctable.
- **Make time a first-class dimension.** A timestamp is not enough. The system should understand before and after, recent and old, repeated and one-off, ongoing and resolved.
- **Make entities first-class.** People, projects, places, and recurring concepts are anchors for recall. Entity continuity is central to relationship continuity.
- **Do not collapse memory into summaries only.** Summaries are useful for compression, but raw episodes or detailed episode records should remain available when possible.
- **Let memory influence behavior, not dictate it.** The LLM should receive memory as grounded context. It should not be forced into brittle rules from stale memories.
- **Support correction and forgetting.** Persistent memory must support updates, contradictions, deletion, decay, and user-controlled correction.
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
- **reinforce** to update salience or character signals based on repeated evidence.
- **correct** to update or revise memory when the user clarifies something.
- **forget** to delete, suppress, or decay memories when appropriate.

### 9.2 Data model expectations

- Memory records should include content, timestamp, source interaction, involved entities, relations, salience, confidence, and provenance.
- Reflections should be stored separately from raw episodes and should reference their source memories.
- Entity nodes should be allowed to evolve over time as new memories add evidence or revise old assumptions.
- Contradictions should not simply overwrite old memories; they should be represented as changes, corrections, or conflicts.
- Retrieval results should include score components or rationale when possible.

### 9.3 Prompt integration expectations

The library should make it easy for an application to insert memory into the LLM context in a concise, grounded form. Memory context should distinguish between raw remembered episodes, derived reflections, and stable character signals.

Example memory context categories:

- Relevant episodes from past interactions.
- Active relationship or project threads.
- User preferences with provenance.
- Character-relevant reflections derived from repeated memories.
- Open loops, promises, unresolved questions, and recent commitments.

---

## 10. What the System Should Avoid

- **Avoid looking like a generic RAG wrapper.** RAG retrieves information. Character Memory should preserve continuity of experience.
- **Avoid turning every interaction into a permanent fact.** Not everything is worth remembering. Memory write policy matters.
- **Avoid personality overwrites.** Character should be reinforced through memory, not replaced by arbitrary labels.
- **Avoid false intimacy.** The system should only use memories it actually has and should make corrections possible.
- **Avoid unexplained recall.** When a memory influences behavior, developers should be able to inspect why it was selected.
- **Avoid third-person archive framing.** This is not a world chronicle. It is memory for a continuing assistant-user relationship.
- **Avoid anthropomorphic claims in documentation.** Describe behavioral continuity and memory-shaped character without claiming consciousness.

---

## 11. Non-Goals

- Not a complete agent framework.
- Not a replacement for the LLM.
- Not merely a vector database abstraction.
- Not only a chat history store.
- Not only a user profile or preference database.
- Not a claim that the AI has sentience or subjective experience.
- Not a roleplay-only system, even though it can support persistent AI characters.

---

## 12. Success Criteria

The implementation should be considered successful if it enables these outcomes:

- After a long gap, the assistant can recall relevant past events without requiring exact wording from the user.
- The assistant can connect a current topic to earlier people, projects, commitments, or emotional context.
- The assistant can retrieve memories based on temporal relation, not only semantic similarity.
- The assistant can show stable behavior shaped by prior interactions while still accepting correction.
- The application developer can inspect why a memory was retrieved and where derived claims came from.
- The system can distinguish raw episodes, inferred reflections, and stable character signals.
- The assistant feels less like a new instance every session and more like a continuing participant in the user's life.

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

- What qualifies an interaction as worth remembering?
- How is salience calculated, updated, and decayed?
- How are memory contradictions represented?
- How does the user inspect, correct, or delete memories?
- How are reflections generated, scheduled, and validated?
- How are relationship-specific memories separated from global character signals?
- How should retrieval balance recency, similarity, entity relevance, and salience?
- How should private or sensitive memories be handled?
- What should the system do when memory evidence is weak or ambiguous?
- How can the library remain useful without overfitting to one LLM provider or vector backend?

---

## 15. Final Guidance

### Design north star

Build Character Memory as the layer that lets an assistant carry experience forward.

The implementation should always be evaluated against this question: **does this design help the assistant remain behaviorally continuous across time?**

When in doubt, favor designs that preserve temporal context, entity continuity, provenance, correction, and reflection. These are the ingredients that let memory shape character instead of merely filling context.
