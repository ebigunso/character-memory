---
status: proposed
adr_type: design
date: 2026-04-26
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0007: Start chat-native and transcript-compatible, not multimodal-native

## Context and Problem Statement

The long-term project should leave room for voice, visual, and embodied memories. However, the first implementation should not try to solve multimodal event segmentation, raw sensor storage, robotics context, or embodied routines. Most current voice assistants can still be modeled as transcript-style conversational interactions.

## Decision Drivers

- Keep v0.1 small and implementable.
- Support the most likely initial use cases: chat and transcript-like voice interactions.
- Avoid blocking future multimodal expansion.
- Avoid introducing lab-only embodied assumptions into the starter schema.

## Decision

v0.1 is chat-native and transcript-compatible.

It supports:

```text
chat conversations
voice conversations represented as transcripts
conversation/session-level episodes
salient message or transcript observations
```

It does not implement first-class support for:

```text
raw audio/video memory
robotics/embodied situation frames
spatial/object memory
continuous sensor event segmentation
```

The schema should still include extensibility points such as `modality` and `raw_ref`.

## Character Memory Relevance

This preserves YAGNI without closing the door on future modalities. The starter system should prove the core continuity model before expanding into embodied memory.

## Implementation Impact

- `Episode.modality` and `Observation.modality` should exist but can initially use values such as `chat` and `voice_transcript`.
- `raw_ref` may point to chat logs or transcript sources.
- Do not design the starter API around robotics, scenes, frames, object affordances, or sensor streams.

## Considered Options

1. Implement multimodal and embodied memory from the beginning.
2. Support only text chat with no modality fields.
3. Start chat-native and transcript-compatible, with reserved extensibility points.

## Decision Outcome

Chosen option: **3. Start chat-native and transcript-compatible, with reserved extensibility points**.

This satisfies current needs while reducing future breaking changes.

## Consequences

### Positive

- Keeps v0.1 focused.
- Allows voice transcript use without a separate architecture.
- Leaves room for future modality-specific observation types.

### Negative / Tradeoffs

- Does not solve raw audio/video or embodied memory yet.
- Some future multimodal needs may still require schema extensions.

## Validation

- Tests should include at least one `chat` episode and optionally one `voice_transcript` episode.
- No v0.1 API should require embodied context fields.
- Modality fields should not be hard-coded to only one possible value.

## Revisit When

Revisit when a concrete application needs raw audio/video references, visual observations, robotics actions, or non-transcript interaction flows.
