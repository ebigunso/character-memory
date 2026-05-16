---
status: accepted
adr_type: design
date: 2026-05-16
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0015: Keep raw source storage outside Character Memory core

## Context and Problem Statement

Character Memory needs provenance back to source material so episodes, observations, derived memories, corrections, reflections, and current continuity views can be inspected and audited.

However, storing raw source material inside Character Memory core would change the library boundary. Raw source material can include turn-based chat logs, transcripts, verbose tool outputs, files, images, audio, video, screen recordings, sensor streams, embodied-agent telemetry, and arbitrary application logs.

Owning that material would require raw storage, retention policy, deletion policy, redaction, access control, export/import, raw search, raw-reference resolution, and possibly media-processing infrastructure.

Character Memory's core goal is episode-backed continuity memory, not raw archival storage.

## Decision Drivers

- Keep Character Memory focused on curated episodic continuity memory.
- Preserve provenance without becoming a raw-log archive.
- Avoid importing storage, privacy, export, deletion, redaction, and media-retention responsibilities into core.
- Keep v0.6 assisted remember useful without making raw input persistent.
- Maintain a clear distinction between source evidence and committed memory.

## Decision

Character Memory core does not store raw source material.

Character Memory core may store opaque provenance handles and source-location metadata, including:

```text
raw_ref
source_kind
source_span
message_id
turn_range
timestamp_range
modality
```

These values are source references. They do not imply that Character Memory can resolve, search, retain, export, delete, redact, encrypt, or otherwise manage the underlying source material.

Assisted remember workflows may receive raw or semi-raw input transiently for candidate generation, but the raw input is not persisted by Character Memory core.

## Character Memory Relevance

Character Memory stores memory objects that support character continuity: episodes, observations, entities, threads, derived memories, corrections, commitments, open loops, relationship state, character signals, and current continuity views.

Raw source material is evidence for memory construction and audit. It is not the core memory substrate.

## Implementation Impact

- Core memory objects may include `raw_ref` and source-span fields.
- `raw_ref` remains opaque to Character Memory core.
- No core raw-log storage API is introduced.
- No core raw-log search API is introduced.
- No public raw-reference resolution API is introduced.
- v0.6 processors may accept caller-provided input transiently.
- Generated candidates preserve caller-supplied source references.
- Raw source storage, if ever needed, requires a future ADR and should start outside core.

## Considered Options

1. Store raw source material directly in graph/vector storage.
2. Add a core raw transcript/log sidecar.
3. Keep raw source storage outside core and store only provenance handles.
4. Build a general raw-source archive/media/sensor store.

## Decision Outcome

Chosen option: **3. Keep raw source storage outside core and store only provenance handles**.

This preserves source traceability without expanding Character Memory into raw archive infrastructure.

## Consequences

### Positive

- Keeps the core library focused on curated memory and continuity retrieval.
- Avoids heavy storage, privacy, export, deletion, and redaction responsibilities.
- Prevents normal retrieval from being polluted by raw transcript or tool-output fragments.
- Keeps multimodal and embodied expansion symbolic and provenance-backed.
- Lets applications use domain-appropriate source storage.

### Negative / Tradeoffs

- Applications must manage raw source retention themselves.
- `raw_ref` values may become stale if upstream source material is deleted or moved.
- Character Memory cannot perform forensic raw recall by itself.
- Reprocessing old interactions requires the caller to provide the source material again.

## Validation

- README and roadmap state that raw source storage is outside Character Memory core.
- v0.6 tests prove raw input can generate candidates without being persisted.
- No core API exposes raw-log storage, raw-log search, or public raw-reference resolution.
- Memory fixtures may include `raw_ref`, but retrieval tests do not depend on resolving it.
- Generated candidates preserve caller-supplied source references.

## Revisit When

Revisit if real deployments repeatedly show that external source storage prevents required auditability, reprocessing, or portability, and if the need cannot be solved by application-level storage or adapter crates.

## More Information

- Related ADR: ADR-D-0008, Preserve source references because summaries are not source material.
- Related roadmap phase: v0.6 assisted remember workflow and memory candidate generation.
- Related future phase: v1.0+ multimodal and embodied expansion.
