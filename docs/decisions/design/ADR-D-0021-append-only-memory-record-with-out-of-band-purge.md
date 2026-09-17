---
status: accepted
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-D-0017-append-only-memory-record-with-out-of-band-purge--superseded-by-ADR-D-0021.md]
superseded_by: null
---

# ADR-D-0021: The memory record is append-only, and erasure exists only as an out-of-band operational purge

## Context and Problem Statement

Deleting memory alters the perceived history of any character built on it, and deleting an episode that grounds derived memories leaves behavior-influencing memory with no remembered basis. At the same time, some erasure pressure is real and external to memory semantics: personal-data erasure obligations, security remediation of records injected through a compromised path, and deliberate operator-directed alteration. The fork is whether erasure is a memory operation at all, and if not, where it lives.

This record restates the decision of ADR-D-0017 without naming decay as a form of forgetting; the forms of forgetting are decided by ADR-D-0018.

## Decision

The memory record is append-only. Forgetting changes influence, never history, and destructive deletion is not a memory operation, not part of the forget semantics, and not a character-facing behavior.

Erasure may exist only as an out-of-band operational purge: an administrative action outside memory semantics, owned by the operator or application, never invoked by memory operations, retrieval, lifecycle policy, or character behavior. Its legitimate uses are compliance erasure of personal data, security remediation of records with illegitimate origin, and explicit operator-directed alteration. A purge makes no pretense of preserving continuity and must tombstone dangling provenance targets rather than leave silent gaps.

A record written by a third party or a compromised path and presented as remembered experience the character never had is a forgery, not a memory. Its purge is justified by its origin, never by its content or behavioral effect. Remediation suppresses first, which is immediate, reversible, and preserves forensic evidence, and purges once illegitimate origin is confirmed, so that no later un-suppression can restore the contamination. Genuine experiences that led somewhere unhealthy remain governed by suppression and supersession.

Permanence applies to the record, not to influence and not to derived indexes. Removing vector points is permissible hygiene because the vector store is rebuildable and non-authoritative, provided it never becomes the sole record of a memory's existence. Personal-data erasure policy is the application's responsibility.

## Why

A character's perceived history is its retrievable memory, and keeping the record append-only guarantees that history can always be audited, corrected by supersession, and un-suppressed. Placing erasure outside memory semantics keeps the model honest: the character never chooses to delete, and operators keep a documented escape hatch for obligations the memory model cannot absorb.

## Rejected Alternatives

- Strict permanence with no erasure path anywhere: rejected outright; it leaves integrators no lawful compliance path short of destroying the store and pretends contamination cannot happen.
- Destructive deletion as a first-class memory operation: rejected outright; it invites false continuity and dangling provenance.
- Purging genuine memories judged undesirable by content: rejected outright; that is history-rewriting, and suppression covers it.

## Decision Boundary

Invariant: no memory operation, retrieval path, lifecycle policy, or character behavior deletes a record; a purge is invoked only out of band and tombstones dangling provenance targets; a purge of a record is justified by illegitimate origin, never by content.

Not covered: the design of the purge tool, which is unscheduled until concretely needed; vector de-indexing policy; the application's compliance procedures.

## Validation

- Lifecycle tests verify suppressed and superseded records are excluded from all default retrieval paths.
- The evaluation harness correction-safety metric asserts zero suppressed or superseded admissions into context packs.
- When a purge tool exists, tests verify tombstoned provenance targets, cross-store removal, that no memory operation can trigger a purge, and that a purged record cannot be restored by un-suppression or any lifecycle operation.
- De-indexing tests verify graph-authoritative records survive vector point removal.

## Revisit When

A purge tool is concretely needed and its design must be specified; origin classification proves unreliable in practice, which tightens purge preconditions rather than reopening append-only semantics; or a legal requirement emerges that tombstoning cannot satisfy.

## More Information

- ADR-D-0018 decides the forms of forgetting.
- ADR-D-0019 keeps sensitivity out of recall eligibility, so erasure obligations do not become retrieval gates.
- ADR-D-0017 in `superseded/` carries the original reasoning of 2026-06-12.
