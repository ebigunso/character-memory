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

# ADR-D-0016: Do not add a generic MetaMemory plane to core

## Context and Problem Statement

Several roadmap features need explanations, provenance, rationale, and diagnostics. A tempting shortcut is to add one generic durable metadata plane that attaches rationale, confidence, intent, assumptions, alternatives, retrieval reasons, and evidence references to every memory object.

That would blur graph-authoritative provenance, retrieval diagnostics, reflection rationale, retention rationale, and association support into one cross-cutting object model.

## Decision Drivers

- Keep provenance in the structures that own the underlying claim or relationship.
- Avoid a parallel graph of generic context edges.
- Keep committed memory objects focused on continuity memory, not arbitrary metadata accumulation.
- Preserve narrow, inspectable explanations where they are semantically meaningful.

## Decision

Character Memory core will not add a generic durable MetaMemory object or cross-cutting MetaMemory plane.

The following are not added as generic metadata on every memory object:

```text
generic rationale summary
generic confidence
generic intent
generic assumptions
generic alternatives
generic decision context
generic evidence_refs duplicate
generic context_edges meta-graph
allowed retrieval profiles
review priority
retrieval reasons
```

Instead:

```text
source references and source spans live in provenance structures
derived memory provenance lives in graph-authoritative links
claim support lives in EvidenceLink
reflection explanation lives in ReflectionJob rationale
retention explanation lives in RetentionAssessment rationale
thread membership explanation lives in membership rationale
association evidence lives in AssociationSupport
retrieval explanation lives in RetrievalTrace
```

## Consequences

### Positive

- Keeps each explanation attached to the feature that can validate it.
- Avoids duplicate provenance paths competing with graph authority.
- Reduces schema sprawl and generic metadata drift.

### Negative / Tradeoffs

- Cross-cutting inspection must read feature-specific rationale and trace objects.
- New features need their own narrowly scoped explanation fields when needed.

## Validation

- v0.1.3 candidate-origin metadata stays narrow and write-time only.
- v0.4 retrieval explanations live in query-time retrieval traces and policy diagnostics.
- v0.5 association evidence lives in `AssociationSupport`, not a generic meta-edge graph.
