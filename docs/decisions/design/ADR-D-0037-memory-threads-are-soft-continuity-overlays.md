---
status: accepted
adr_type: design
date: 2026-09-21
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-D-0003-soft-memory-threads--superseded-by-ADR-D-0037.md]
superseded_by: null
depends_on: [ADR-D-0030-interpreted-memory-carries-no-confidence-score.md]
---

# ADR-D-0037: A memory thread is a soft continuity overlay, and membership carries no confidence score

## Context and Problem Statement

A memory thread represents an ongoing project, a recurring topic, a relationship arc, an unresolved tension. Modeled as a hard container, the way a chat session is, it makes memory chat-shaped and brittle, because real continuity is fuzzy and many-to-many. The record this one replaces settled that, and also had membership carry a confidence score. Nothing ever read that score, and ADR-D-0030 has since rejected a stored confidence on interpreted memory for reasons that apply to a membership link unchanged: a number supplied at write time is not evidence, and a number computed at write time is stale at the next piece of evidence.

## Decision

A memory thread is a soft continuity overlay. Membership in a thread is optional, many-to-many, and revisable, and it is represented as a link between the memory and the thread. An episode is never required to belong to exactly one thread, and retrieval works when no thread exists.

Membership carries no confidence score. How firmly something belongs to a thread is read, when it matters, from what is recorded: the link's rationale and the evidence around it.

## Why

Continuity emerges from repeated cues, projects, tensions, corrections, and commitments, and one episode can contribute to several of them. A soft overlay keeps threads a strong anchor for recall without forcing every episode into one clean container. A score on the link adds nothing the record does not already hold, and a field nothing reads invites being trusted.

## Rejected Alternatives

- Every episode belongs to exactly one thread: rejected outright; it forces artificial structure and breaks on any episode that serves two continuities.
- A thread is an external chat or session identifier: rejected outright; it ties memory to a user-interface structure and fails for modalities that have no sessions.
- Confidence-scored membership, as the replaced record had it: rejected because the score had no reader and carries the faults ADR-D-0030 names; reopen if a recall behavior is shown to need a graded membership that the recorded evidence cannot supply.

## Decision Boundary

Invariant: thread membership is optional, many-to-many, revisable, and held as a link; no memory requires a thread; no membership or other link stores a confidence score.

Not covered: how threads are formed, summarized, or closed; how recall orders a thread's members; the rationale a link may carry.

## Validation

- Tests cover episodes with no thread, one thread, and several threads.
- Retrieval tests confirm a matching active thread helps recall and is never required.
- Schema review rejects a confidence field on a link.

## Revisit When

Most applications turn out to use only explicit session threads, so that soft membership is unused overhead; or a recall behavior needs graded membership that evidence cannot supply.

## More Information

- Replaces ADR-D-0003 in full. The decision about threads is unchanged; the confidence score on membership is removed.
- ADR-D-0030 left the confidence on thread membership and other links to a later examination; this record settles it for links, following the decider's ruling of 2026-09-20.
- ADR-I-0004 lists confidence among the things a link may include; it permits and does not require it, so it is not contradicted.
