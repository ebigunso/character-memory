---
status: accepted
adr_type: implementation
date: 2026-09-03
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely call the embedded engine directly from the async retrieval path (it is a plain synchronous API and the first implementation compiles), and would rely on the shard's final drop for persistence because the engine's write call returns success before anything is durable; both appeared in the implementation draft reviewed on 2026-09-03"
  detected_signals: "cross-boundary contract shape (an engine with synchronous, non-durable writes inside an async host); premises likely to expire (the engine is beta and its persistence model may change); costly to detect (lost writes surface as a character forgetting after a crash, long after the cause)"
  cost_of_violation: "an engine call on an executor thread stalls every other retrieval in the process for the duration of a scan, build, or flush; a write acknowledged before its flush is lost on any exit that skips the shard's drop, and the loss is silent — the store reopens cleanly and simply lacks the memories"
  cost_of_wrong_preservation: "if the engine starts replaying its log on load or persisting on write and this record is preserved, every write keeps paying a synchronous disk sync it no longer needs"
depends_on: [implementation/ADR-I-0023-embedded-qdrant-edge-vector-candidate-store.md]
implements: []
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-I-0027: The embedded vector engine runs on a dedicated blocking owner that flushes every write before acknowledging it

## Context and Problem Statement

The embedded vector engine (ADR-I-0023) exposes a synchronous API: opening a shard, creating payload indexes, upserting and deleting, searching, counting, building an index, and dropping a shard (which flushes) all block the calling thread.
The library's retrieval and write paths are async, and the composition entry point is itself async, so the natural first implementation calls the engine on an executor thread and stalls every other task in the process.
Separately, the engine's write call returns success before anything is durable.
Measured on the pinned version on 2026-09-02: a writer that skipped the shard's drop and a writer that exited the process after successful writes both reopened cleanly with zero of two hundred points, while the normal-drop control reopened with all of them; the engine persists only when its flush runs, and a load does not replay the write-ahead log.
An adapter that relies on the final drop for persistence therefore loses every unflushed memory on any exit that pre-empts that drop, and the loss is invisible at reopen.

## Decision Drivers

- No engine call may occupy an async executor thread; the library's other embedded stores hold this line.
- A write the library has acknowledged must survive a process exit that skips orderly shutdown; a character that forgets after a crash violates continuity silently.
- No port or facade method is added for shutdown; the existing facade drop remains the only close path.
- The rule must be pinned to measured engine behaviour so a change in the engine reopens it rather than silently voiding it.

## Decision

The adapter creates a dedicated blocking owner at construction: one blocking worker that opens the shard itself, holds it, and serialises every engine call — shard open and load including the lock backoff, payload index creation, upsert and delete, search, the filtered scope count behind the exhaustive verdict, index build, and the final drop.
The async composition entry point never touches the engine; it only hands work to the owner and awaits the result.
The owner acknowledges an upsert or delete only after the engine's flush has completed, so every acknowledged write is durable independently of the shard's final drop.
Dropping the adapter through the existing facade drop only signals the owner; the shard's final drop happens on the owner's thread, and a process exit that pre-empts it loses nothing acknowledged.
A shard directory stays locked while an owner holds it; a constructor that meets a locked directory waits with a bounded backoff for the previous owner to release it rather than failing or opening a second handle.
The contract canary (ADR-I-0023) additionally pins the three engine facts this record rests on: the engine does not persist a write until its flush runs, a load does not replay the log, and a shard directory held by one owner refuses a second open until it is released.

## Implementation Impact

- The adapter owns a blocking thread and a request channel; every port method becomes a message to the owner.
- Each write costs one synchronous disk sync on the owner's thread; the in-phase benchmark's write burst measures it.
- The close-then-reopen test, the hard-exit test, and the responsiveness benchmark (construction and reopen including a lock-backoff wait, a scan, a write burst, a build, and a close) are the phase's evidence.

## Considered Options

1. A dedicated blocking owner that flushes after every write and acknowledges only then; signal-only facade drop.
2. Call the engine directly from the async paths and rely on the shard's drop for persistence.
3. A blocking owner with a signal-only drop and no per-write flush, documenting a weaker crash guarantee.
4. An explicit awaited close on the port or facade that flushes before returning.

## Decision Outcome

Chosen option: **Option 1**.
It is the only option that keeps executor threads free, makes every acknowledged write durable, and adds no API surface.

### Rejected Alternatives

Option 2 stalls the process on every scan, build, and flush, and was measured to lose every unflushed write on exit; rejected outright.
Option 3 makes the library's write acknowledgement a lie under crash or exit, and a character's lost memories are the cost; rejected outright.
Option 4 adds a close method every consumer must remember to call and still loses writes on any exit that skips it; the per-write flush makes it unnecessary; it is reopened only if the write-burst measurement shows the per-write flush dominating ingestion cost, in which case batched flushes behind an explicit acknowledgement are the shape, not a weaker guarantee.

## Consequences

- Positive: executor responsiveness is independent of corpus size and engine activity; acknowledged writes survive crashes and hard exits; no new API.
- Negative / tradeoffs: a synchronous disk sync per write; a serialised engine (one call at a time per adapter), which the candidate-recall role tolerates.

## Decision Boundary

Invariant: every engine call runs on the adapter's blocking owner; a write is acknowledged only after it is durable; the facade drop stays signal-only; a constructor waits for a locked directory rather than opening a second handle; the three engine facts are pinned by the canary.

Not covered: the channel and thread mechanics, the backoff bound, and the batching of flushes behind an acknowledgement if measurement calls for it.

## Validation

- The hard-exit test writes, exits the process without dropping the shard, reopens the directory from a second process, and finds every acknowledged write.
- The close-then-reopen test drops the facade inside an async runtime, reopens the same directory immediately, and finds every write.
- The benchmark shows no engine call occupying an async executor thread and observes the shard's final drop on the owner's thread.
- The contract canary fails if the pinned engine starts replaying its log on load, persisting on write, or admitting a second open of a held shard directory.

## Revisit When

- The engine persists on write or replays its log on load (the canary fails in that direction) — the per-write flush becomes optional and this record is revised.
- The write-burst measurement shows the per-write flush dominating ingestion cost — batch flushes behind an explicit acknowledgement rather than weakening the durability rule.
- The engine's directory lock changes semantics (the canary fails in that direction) — the constructor's wait-for-release rule is re-derived before adopting a different engine pin.
- A multi-process deployment shape is designed — the single-owner lock discipline is reconsidered with the graph and statistics stores, never alone.

## Consultation impact

Question asked in review: whether the shard's drop-time flush is persistence or compaction; a process-level probe settled it as persistence, and the decider's rule "await the flush or document the weaker guarantee" was met by flushing per write.

## More Information

- ADR-I-0023 (the engine and the canary this record extends).
- The durability probe report of 2026-09-02 is a transient working artifact; the numbers above are its record.
