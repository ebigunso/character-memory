# v1.0+ Design Draft: Multimodal and Embodied Expansion

## Version intent

This is a future expansion path, not starter scope.

The v0.x library should support chat and voice transcripts. It should not attempt to fully support robots, continuous video, spatial memory, or embodied action from the beginning.

But the v0.x structure should avoid blockers.

---

# 1. Why this is delayed

Most practical assistant products remain chat-native or voice-transcript based.

Full embodied memory requires:

```text
continuous event segmentation
sensor logs
object tracking
spatial context
action/outcome traces
gesture interpretation
affordance memory
```

That is too much for the starter and risks distorting the core library around lab-like use cases.

---

# 2. Existing hooks from v0.x

v0.x already provides:

```text
Episode.modality
Observation.modality
raw_ref
Entity
MemoryLink
MemoryThread
DerivedMemory
```

These are enough to extend from chat to voice transcripts and later to other modalities.

Example now:

```json
{
  "modality": "voice_transcript",
  "raw_ref": "raw://voice/session_123/transcript"
}
```

Future:

```json
{
  "modality": "video",
  "raw_ref": "raw://video/segment_456"
}
```

The provenance pattern remains:

```text
Episode → Observation → DerivedMemory
```

---

# 3. Future concepts

## 3.1 SituationFrame

A short-lived current activity context.

```text
SituationFrame = current activity context
Episode = bounded remembered event
MemoryThread = persistent continuity pattern
```

Example:

```json
{
  "id": "situation_...",
  "object_type": "situation_frame",
  "title": "Helping user make tea",
  "started_at": "...",
  "ended_at": null,
  "participants": ["ent_user_primary", "ent_assistant_self"],
  "active_place_id": "ent_place_kitchen",
  "active_object_ids": ["ent_blue_mug", "ent_kettle"],
  "current_goal": "prepare tea",
  "status": "active"
}
```

## 3.2 MultimodalObservation

An observation whose primary source is not text.

```json
{
  "id": "obs_...",
  "object_type": "observation",
  "episode_id": "ep_...",
  "modality": "vision",
  "text": "The user pointed at the blue mug and corrected the assistant's previous choice.",
  "raw_ref": "raw://video/segment_...",
  "detected_entity_ids": ["ent_user_primary", "ent_blue_mug"],
  "confidence": 0.77,
  "observed_at": "..."
}
```

## 3.3 ObjectMemory

Memory about recurring physical or visual objects.

Examples:

```text
blue mug
kitchen table
user's laptop
front door
project whiteboard
```

## 3.4 PlaceMemory

Memory about recurring places or spatial contexts.

Examples:

```text
user's kitchen
office desk
shared workspace
VRChat event world
```

## 3.5 ActionTrace

A record of actions taken by the assistant or embodied agent.

```json
{
  "id": "action_...",
  "object_type": "action_trace",
  "episode_id": "ep_...",
  "actor_entity_id": "ent_assistant_self",
  "action_type": "suggested_design_change",
  "description": "Suggested using natural-language semantic surfaces instead of structured embedding templates.",
  "started_at": "...",
  "ended_at": "..."
}
```

## 3.6 OutcomeObservation

A result or consequence of an action.

```json
{
  "id": "outcome_...",
  "object_type": "outcome_observation",
  "episode_id": "ep_...",
  "related_action_id": "action_...",
  "text": "The user accepted the YAGNI starter architecture but asked for old roadmap reconciliation.",
  "outcome_type": "accepted_with_revision_request"
}
```

---

# 4. Multimodal storage policy

Do not store raw media directly in the graph.

Use:

```text
raw media archive → audio/video/sensor files or references
graph → symbolic observations, entities, actions, outcomes
vector store → natural-language surfaces over salient observations
```

This keeps the Character Memory graph tractable.

---

# 5. Future retrieval changes

Multimodal retrieval may add:

```text
current situation lookup
object/place memory lookup
action/outcome history
recent correction in same physical context
voice tone summaries
screen/document observation context
```

But it still returns a continuity context:

```text
What has happened before that should shape behavior now?
```

---

# 6. Relation to old roadmap

The old roadmap had multimodal hooks as a storage-level extension. That idea is retained but delayed.

v0.x keeps:

```text
modality
raw_ref
entity links
memory links
schema versioning
```

v1.0+ adds modality-specific interpretation only when there is a real use case.
