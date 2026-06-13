---
status: accepted
adr_type: design
date: 2026-06-12
deciders: ["ebigunso"]
consulted: ["Claude Fable 5"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0017: Keep the memory record append-only, with erasure as an out-of-band operational action

## Context and Problem Statement

ADR-D-0006 made supersession and suppression the default correction/forgetting mechanisms, but deferred destructive deletion "until explicit production policy support exists." The project philosophy and roadmap carried matching hedges ("deletion" listed as a forgetting mechanism; "unless a later explicit destructive policy is implemented"). This left the permanence question open.

Deleting memory alters the perceived history of any character built on the memory base, disrupting character continuity. Deleting an episode that grounds derived memories or character signals leaves behavior-influencing memory with no remembered basis — exactly the false-continuity failure the philosophy exists to prevent. Surgical memory deletion is also non-human-like: humans cannot target a memory and excise it; they forget through decay, interference, and suppression.

At the same time, some erasure pressure is real and external to memory semantics: legal compliance obligations (personal-data erasure rights), security remediation of adversarially contaminated records, and deliberate operator-directed behavioral alteration.

## Decision Drivers

- Character continuity depends on a stable, append-only remembered history.
- Episode-backed provenance must never dangle silently.
- Surgical deletion is not a human-like forgetting mechanism; decay and suppression are.
- Integrators of companion/assistant applications may face legal erasure obligations that suppression cannot satisfy.
- Adversarially injected ("poisoned") records are contamination of the record, not remembered experience.
- The vector index is rebuildable and non-authoritative, so index hygiene must not be conflated with memory erasure.

## Decision

The memory record is append-only. Forgetting is suppression, archival, and decay — never erasure. Destructive deletion is not a memory operation and is not part of the memory model, the `forget` semantics, or any character-facing behavior.

Erasure may exist only as an **out-of-band operational purge**: an administrative tool outside memory semantics, analogous to surgery rather than forgetting. Its documented legitimate uses are:

```text
compliance erasure of personal data
security remediation of adversarially contaminated records
explicit operator-directed behavioral alteration
```

An operational purge:

```text
is never invoked by memory operations, retrieval, lifecycle policy, or character behavior
makes no pretense of preserving continuity
must tombstone dangling provenance targets rather than leave silent gaps
is owned by the operator/application, with policy outside Character Memory core
```

### Poisoned records: purge is justified by illegitimate origin, not undesirable content

A poisoned record is content written into the memory store by a third party or compromised path and presented as remembered experience that the character never legitimately had: prompt-injected memory, forged episodes, or records minted through a compromised write path. Append-only protection applies to **remembered experience**; an injected record fails that test. Removing it does not rewrite the character's history — it removes a forgery from the record.

The remediation policy is two-tier:

```text
1. suppress first
   immediate, reversible, auditable
   preserves forensic evidence of how the contamination entered

2. purge as preferred decontamination once illegitimate origin is confirmed
   removes the record so no future memory operation, un-suppression,
   or lifecycle change can ever accidentally restore the contamination
```

The purge trigger is the record's **origin** (injected, forged, compromised write path), never merely its **content** or behavioral effect. Organic bad memories — genuine interactions that led somewhere unhealthy — are remembered experience and remain governed by suppression, supersession, and decay; purging them is history-rewriting and stays outside normal remediation.

For genuine experiences, suppression in this system is stronger than human suppression: suppressed content is guaranteed never to reach retrieval, so the character keeps the continuity benefits of having a past without involuntary intrusion. This is why organic bad memories do not need the purge path.

Permanence applies to the **record**, not to **influence** or to **derived indexes**:

- Influence is lifecycle-managed through suppression, supersession, currentness, and (future) salience decay.
- Vector de-indexing (removing Qdrant points) is permissible hygiene because the vector store is rebuildable and non-authoritative — provided de-indexing does not prematurely remove access to otherwise-current, behavior-influencing memory.

The compliance posture is documented explicitly: personal-data erasure policy is the application's responsibility; Character Memory core provides non-destructive forgetting as the memory model and acknowledges the operational purge path for applications that need it.

## Character Memory Relevance

A character's perceived history is its retrievable memory. Keeping the record append-only guarantees that history can always be audited, corrected by supersession, and un-suppressed — the substrate never silently rewrites what was experienced. Placing erasure outside memory semantics keeps the philosophy honest: the character never "chooses" to delete, and forgetting remains human-like (decay and suppression), while operators retain a documented escape hatch for obligations the memory model cannot and should not absorb.

## Implementation Impact

- No `forget` mode performs destructive deletion; existing suppress/archive semantics are unchanged.
- A future operational purge tool (unscheduled; designed when concretely needed) must tombstone provenance targets and span all stores: graph records, vector points, retrieval-stats counters.
- Vector de-indexing policies (e.g., future RetentionAssessment outcomes in v0.4) may remove or downrank Qdrant points but must never be the sole record of a memory's existence — Oxigraph remains authoritative.
- README/positioning documents the compliance posture.
- Suppression robustness becomes more important: suppressed content must never leak into any retrieval path, since suppression is the primary remedy for unwanted content.

## Considered Options

1. Strict permanence: no erasure path exists at all, anywhere.
2. Append-only memory semantics with an out-of-band operational purge for compliance, security, and operator-directed alteration.
3. Destructive deletion as a first-class memory operation (e.g., a `forget` mode).

## Decision Outcome

Chosen option: **2. Append-only memory semantics with an out-of-band operational purge**.

Option 1 leaves integrators with no lawful-compliance path short of destroying the entire store and pretends security contamination cannot happen. Option 3 makes deletion part of character-facing memory semantics, inviting false continuity, dangling provenance, and non-human-like surgical forgetting. Option 2 preserves the philosophy completely — the memory model never deletes — while acknowledging that operators occasionally must.

## Consequences

### Positive

- Character-perceived history is never silently rewritten by memory operations.
- Provenance chains and audit/correction paths remain intact by default.
- Integrators have a documented, bounded answer to erasure obligations.
- Poisoned-record remediation has a principled escalation path: suppress for forensics, then purge confirmed-illegitimate records so they can never be accidentally un-suppressed.

### Negative / Tradeoffs

- Storage growth is unbounded by design; mitigation is de-indexing and downranking, never record deletion.
- A permanent store of intimate interaction history raises breach stakes; encryption at rest and access control matter earlier.
- Purged records create tombstones, which are a visible scar in provenance rather than seamless history.
- Suppression must be robust enough to be the sole influence-removal mechanism for genuine remembered experience.
- Origin classification carries misjudgment risk: purging a record wrongly judged illegitimate destroys genuine experience irreversibly, so origin confirmation should precede purge and suppression covers the interim.

## Validation

- Lifecycle tests continue to verify suppressed/superseded records are excluded from all default retrieval paths.
- The v0.1.4 evaluation harness correction-safety metric asserts zero suppressed/superseded admissions into context packs.
- When an operational purge tool is built: tests must verify tombstoned provenance targets, cross-store removal, and that no memory operation can trigger a purge.
- When an operational purge tool is built: tests must verify a purged record cannot be restored by any un-suppression or lifecycle operation.
- De-indexing tests (when implemented) must verify graph-authoritative records survive vector point removal.

## Revisit When

- An operational purge tool is concretely needed and its design must be specified.
- Origin classification proves unreliable in practice (would tighten purge preconditions, not reopen append-only semantics).
- Legal requirements emerge that tombstoning cannot satisfy.
