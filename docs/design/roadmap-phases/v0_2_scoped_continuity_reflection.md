# v0.2 Design Draft: Scoped Continuity and Reflection

## Version intent

v0.1 stores episodes and simple derived memories. v0.1.2 hardens retrieval so broad entities do not flood context. v0.2 then makes those memories actively support continuity through explicit scopes, reflections, relationship states, character signals, open loops, and commitments.

The key refinement in this version is scope:

```text
Continuity is generated for a scope, not assumed to be centered on one user-assistant relationship.
```

A scope may represent an entity, entity pair, thread, project, place, source conversation, character, or application-provided custom context.

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
character behavior adjustments
scope-specific continuity views
```

v0.2 adds these structures without yet implementing full factual belief tracking.

The design must remain use-case agnostic. The core library should not assume that the most important continuity relationship is `user ↔ assistant`. That may be true for personal assistants, but not for games, simulations, companions, research environments, or tools built around arbitrary entities.

---

# 2. New concepts

## 2.1 ContinuityScope

A `ContinuityScope` identifies the context for reflection, continuity state, open loops, commitments, and current views.

Possible scope kinds:

```text
entity
entity_pair
thread
project
place
source_conversation
character
application_custom
```

Example:

```json
{
  "id": "scope_character_memory_design",
  "object_type": "continuity_scope",
  "scope_type": "thread",
  "scope_id": "thread_character_memory_design",
  "label": "Character Memory design thread"
}
```

Applications may supply custom scope IDs when their domain already has a meaningful scope model.

## 2.2 ReflectionJob

A scheduled or manual process that reviews one or more episodes and updates derived memories within a scope.

```json
{
  "id": "reflect_01h...",
  "object_type": "reflection_job",
  "scope_id": "scope_character_memory_design",
  "input_episode_ids": ["ep_1", "ep_2"],
  "status": "completed",
  "created_memory_ids": ["dm_10", "signal_2"],
  "rationale": "The episodes repeatedly emphasized YAGNI and character-continuity framing."
}
```

## 2.3 RelationshipState

A current summary of a relationship between arbitrary entities or within a relationship-like scope.

```json
{
  "id": "relstate_project_collaboration",
  "object_type": "relationship_state",
  "scope_id": "scope_character_memory_design",
  "summary": "This design collaboration favors direct technical critique, explicit tradeoffs, and preservation of the Character Memory philosophy.",
  "preferred_interaction_style": "direct, technical, willing to revise",
  "confidence": 0.82,
  "derived_from_episode_ids": ["ep_..."],
  "is_current": true
}
```

A relationship state is not necessarily a personal relationship. It may describe:

```text
entity ↔ entity
character ↔ faction
project ↔ contributor
agent ↔ place
simulation character ↔ recurring scene
```

## 2.4 CharacterSignal

A behavior-shaping signal derived from memory for a continuing entity or scope.

```json
{
  "id": "signal_technical_design_partner",
  "object_type": "character_signal",
  "scope_id": "scope_character_memory_design",
  "signal_text": "In this project, act as a direct technical design partner rather than a generic RAG-system advisor.",
  "behavioral_implication": "Prioritize episodic continuity, provenance, correction, entity-neutrality, and YAGNI in design recommendations.",
  "stability": "medium",
  "evidence_count": 3,
  "derived_from_episode_ids": ["ep_..."],
  "is_current": true
}
```

A character signal should attach to a scope, not silently become global personality.

## 2.5 OpenLoop

An unresolved question, task, tension, promise, design issue, or pending matter within a scope.

```json
{
  "id": "loop_roadmap_selectivity",
  "object_type": "open_loop",
  "text": "Implement continuous entity selectivity and retrieval guardrails without special-casing user or assistant roles.",
  "status": "active",
  "priority": 0.86,
  "scope_id": "scope_character_memory_design",
  "created_in_episode_id": "ep_...",
  "thread_ids": ["thread_character_memory_design"]
}
```

## 2.6 Commitment

Something attributed to an entity or relationship scope as an obligation, intent, or promised follow-through.

```json
{
  "id": "commit_...",
  "object_type": "commitment",
  "actor_entity_id": "ent_assistant_self",
  "scope_id": "scope_character_memory_design",
  "text": "Produce revised roadmap and ADR documents for v0.1.2.",
  "status": "active",
  "created_in_episode_id": "ep_...",
  "due_at": null,
  "resolved_in_episode_id": null
}
```

## 2.7 CurrentContinuityView

A derived current view for a scope.

It may include:

```text
current relationship state
current character signals
active open loops
active commitments
current scoped preferences
active threads
recent high-salience episodes
relevant current beliefs once v0.3 exists
```

It is not raw history. It is usable current continuity context.

---

# 3. Reflection policy

v0.2 should not run expensive reflection on every turn.

Recommended triggers:

```text
session end
high-salience episode
explicit correction
explicit preference
new commitment/open loop
thread reaches N new episodes
scope receives N new high-salience memories
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

Reflection jobs should use v0.1.2 selectivity guardrails. They should not discover a scope by walking the entire graph through a broad entity.

---

# 4. Current continuity views

v0.2 should create derived current views keyed by `ContinuityScope`.

Examples:

```text
current-character-signals(scope)
current-relationship-state(scope)
active-open-loops(scope)
active-commitments(scope)
active-memory-threads(scope)
current-preferences(scope)
```

Rules:

```text
Only current, non-suppressed records appear.
Superseded records are excluded unless explicitly requested.
Derived records retain provenance to episodes.
Scope is explicit or inferred and stored.
Current views should not require scanning all history through broad entities.
```

---

# 5. Retrieval changes

v0.2 retrieval should route by need and scope.

Common retrieval modes:

```text
ongoing_project
relationship_context
scope_context
preference_sensitive
commitment_followup
correction_context
general_recall
```

Example behavior:

```text
If current query continues an active scope, retrieve scope summary, recent high-salience episodes, open loops, and relevant character signals.

If a correction is requested, retrieve the target memory, provenance, and supersession history.

If a user or application asks “what were we working on?”, retrieve active scopes, active threads, and open loops before random semantically similar snippets.
```

v0.1.2 selectivity should remain active. Broad entities may help identify scope, but they should not automatically expand to all connected memories.

---

# 6. Public API additions

Illustrative shape:

```rust
fn reflect(&self, scope: Option<&ContinuityScope>) -> Result<ReflectionResult, MemoryError>;

fn reinforce(
    &self,
    target_id: &str,
    signal: Option<&ReinforcementSignal>,
) -> Result<(), MemoryError>;

fn get_open_loops(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<Vec<OpenLoop>, MemoryError>;

fn get_commitments(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<Vec<Commitment>, MemoryError>;

fn resolve_open_loop(
    &self,
    loop_id: &str,
    evidence: Option<&EvidenceInput>,
) -> Result<(), MemoryError>;

fn resolve_commitment(
    &self,
    commitment_id: &str,
    evidence: Option<&EvidenceInput>,
) -> Result<(), MemoryError>;

fn get_current_context(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<CurrentContinuityView, MemoryError>;
```

---

# 7. Acceptance criteria

```text
Reflection jobs require explicit or inferred ContinuityScope.
CurrentContinuityView is generated for a scope.
RelationshipState can describe arbitrary entity relationships.
CharacterSignal can attach to any continuing entity or scope.
Open loops and commitments can be retrieved by scope without assuming who the main actor is.
Reflection avoids all-history scans through broad entities.
Character signals retain provenance to episodes.
Relationship state can be updated without deleting history.
ContinuityContextPack includes active continuity structures when relevant.
Current views exclude superseded/suppressed records.
```

---

# 8. Library boundary

v0.2 keeps the library boundary but makes continuity structures first-class because they are the product goal.

This is not agent orchestration. It is memory state.

Revisit if scope modeling becomes too complex for v0.2. If necessary, keep `ContinuityScope` simple initially and let applications provide custom scope IDs. Do not revert to user/assistant assumptions.
