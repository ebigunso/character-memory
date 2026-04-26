# v0.2 Design Draft: Continuity and Reflection Layer

## Version intent

v0.1 stores episodes and simple derived memories. v0.2 makes those memories actively support character continuity.

The system should begin to answer:

```text
What ongoing relationship or project context matters here?
What has the assistant committed to?
What unresolved thread should be carried forward?
What stable behavior has been reinforced by memory?
```

---

# 1. Why v0.2 exists

A pile of episodes is not yet character continuity.

Character continuity requires higher-level state derived from repeated episodes:

```text
relationship patterns
interaction style
project history
open loops
commitments
corrections
stable preferences
assistant behavior adjustments
```

v0.2 adds these structures without yet implementing full factual belief tracking.

---

# 2. New concepts

## 2.1 ReflectionJob

A scheduled or manual process that reviews one or more episodes and updates derived memories.

```json
{
  "id": "reflect_01h...",
  "object_type": "reflection_job",
  "scope_type": "thread",
  "scope_id": "thread_character_memory_design",
  "input_episode_ids": ["ep_1", "ep_2"],
  "status": "completed",
  "created_memory_ids": ["dm_10", "signal_2"],
  "rationale": "The user repeatedly emphasized YAGNI and character-continuity framing."
}
```

## 2.2 RelationshipState

A current summary of the assistant-user relationship or another scoped relationship.

```json
{
  "id": "relstate_primary_user",
  "object_type": "relationship_state",
  "scope_id": "relationship_assistant_user",
  "summary": "The user prefers direct, research-colleague style technical discussion and values architectural precision.",
  "preferred_interaction_style": "concise, direct, technical, willing to revise",
  "confidence": 0.82,
  "derived_from_episode_ids": ["ep_..."],
  "is_current": true
}
```

## 2.3 CharacterSignal

A behavior-shaping signal derived from memory.

```json
{
  "id": "signal_technical_design_partner",
  "object_type": "character_signal",
  "scope": "project",
  "scope_id": "thread_character_memory_design",
  "signal_text": "In this project, act as a direct technical design partner rather than a generic RAG-system advisor.",
  "behavioral_implication": "Prioritize episodic continuity, provenance, correction, and YAGNI in design recommendations.",
  "stability": "medium",
  "evidence_count": 3,
  "derived_from_episode_ids": ["ep_..."],
  "is_current": true
}
```

## 2.4 OpenLoop

An unresolved question, task, tension, or pending decision.

```json
{
  "id": "loop_roadmap_revision",
  "object_type": "open_loop",
  "text": "Revise the roadmap by incorporating useful backend discipline from the old roadmap while preserving the Character Memory philosophy.",
  "status": "active",
  "priority": 0.86,
  "created_in_episode_id": "ep_...",
  "thread_ids": ["thread_character_memory_design"]
}
```

## 2.5 Commitment

Something the assistant or user agreed to do or preserve.

```json
{
  "id": "commit_...",
  "object_type": "commitment",
  "actor_entity_id": "ent_assistant_self",
  "text": "Produce revised roadmap documents and explain what was incorporated or dropped.",
  "status": "active",
  "created_in_episode_id": "ep_...",
  "due_at": null,
  "resolved_in_episode_id": null
}
```

---

# 3. Reflection policy

v0.2 should not run expensive reflection on every turn.

Recommended triggers:

```text
session end
high-salience episode
explicit user correction
explicit preference
new commitment/open loop
thread reaches N new episodes
idle/background job
manual reflect(scope) call
```

Reflection outputs may include:

```text
new DerivedMemory
updated thread summary
new/updated CharacterSignal
new/updated RelationshipState
new/updated OpenLoop
new/updated Commitment
supersession/correction links
```

---

# 4. Current continuity views

v0.2 should create derived current views.

```text
current-character-signals
current-relationship-state
active-open-loops
active-commitments
active-memory-threads
current-preferences
```

These views should be queryable without scanning all history.

Rules:

```text
Only current, non-suppressed records appear.
Superseded records are excluded unless explicitly requested.
Derived records retain provenance to episodes.
```

---

# 5. Retrieval changes

v0.2 retrieval should route by need.

Common retrieval modes:

```text
ongoing_project
relationship_context
preference_sensitive
commitment_followup
correction_context
general_recall
```

Example behavior:

```text
If current query continues an active thread, retrieve thread summary, recent high-salience episodes, open loops, and relevant character signals.
If user asks for a correction, retrieve the target memory, provenance, and supersession history.
If user asks “what were we working on?”, retrieve active threads and open loops before random semantically similar snippets.
```

---

# 6. Public API additions

```rust
fn reflect(&self, scope: Option<&MemoryScope>) -> Result<ReflectionSummary, MemoryError>;
fn reinforce(&self, target_id: &str, signal: Option<&ReinforcementSignal>) -> Result<(), MemoryError>;
fn get_open_loops(&self, scope: Option<&MemoryScope>) -> Result<Vec<OpenLoop>, MemoryError>;
fn get_commitments(&self, scope: Option<&MemoryScope>) -> Result<Vec<Commitment>, MemoryError>;
fn resolve_open_loop(&self, loop_id: &str, evidence: Option<&EvidenceInput>) -> Result<(), MemoryError>;
fn resolve_commitment(&self, commitment_id: &str, evidence: Option<&EvidenceInput>) -> Result<(), MemoryError>;
fn get_current_context(&self, scope: Option<&MemoryScope>) -> Result<ContinuityContextPack, MemoryError>;
```

---

# 7. Acceptance criteria

```text
Reflection can update thread summaries.
Open loops and commitments can be created, retrieved, and resolved.
Character signals retain provenance to episodes.
Relationship state can be updated without deleting history.
ContinuityContextPack includes active continuity structures when relevant.
Current views exclude superseded/suppressed records.
```

---

# 8. Relation to old roadmap

The old roadmap did not include a continuity layer. It treated application behavior as out-of-repository context.

The revised v0.2 keeps the library boundary but makes continuity structures first-class because they are the actual product goal.

This is not agent orchestration. It is memory state.
