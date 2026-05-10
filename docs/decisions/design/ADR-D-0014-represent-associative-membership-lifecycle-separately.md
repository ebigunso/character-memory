---
status: accepted
adr_type: design
date: 2026-05-10
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-D-0014: Represent associative membership lifecycle separately from associative unit lifecycle

## Context and Problem Statement

An associative cluster or cue bundle can become active while new memories are still only tentative members of that structure.

If the cluster has only one lifecycle status, every member appears equally established once the cluster is active. That harms retrieval quality because weak or newly added members may be retrieved as if they were core memories.

## Decision Drivers

- Keep associative retrieval nuanced enough for long-term continuity.
- Allow tentative members without overtrusting them.
- Make cluster expansion explainable at member level.

## Decision

Character Memory will represent associative recall with two lifecycle levels:

```text
AssociativeUnit:
  lifecycle of the associative structure itself

AssociativeMembership:
  lifecycle, role, strength, and rationale of each memory's participation in that structure
```

An active `AssociativeUnit` may contain memberships with candidate, active, retired, or rejected status and core, exemplar, peripheral, bridge, or outlier role.

## Character Memory Relevance

Long-term recall quality depends on nuanced membership. A new memory can be potentially related to an established pattern without being treated as equally important.

This supports serendipitous recall while avoiding false continuity.

## Considered Options

1. Use only unit-level lifecycle.
2. Store weak members outside the graph.
3. Represent unit lifecycle and membership lifecycle separately.

## Decision Outcome

Chosen option: **Represent unit lifecycle and membership lifecycle separately**.

This preserves retrieval quality by letting an active associative structure contain members with different confidence, roles, strengths, and retrieval eligibility.

## Consequences

### Positive

- Improves retrieval quality.
- Allows new memories to be considered without overtrusting them.
- Supports promotion, demotion, decay, and rejection at member level.
- Makes cluster expansion more explainable.

### Negative / Tradeoffs

- Adds schema complexity.
- Requires membership-level retrieval logic.
- Requires maintenance policies for promotion, demotion, and decay.

## Validation

- Tests should show that an active unit can contain candidate memberships.
- Ordinary retrieval should prefer core-role, exemplar-role, and strongly supported active-status members over candidate-status or peripheral-role members.
- Candidate members should require direct query support or exploratory mode.
- Cluster lifecycle must not override underlying memory lifecycle/currentness/suppression.

## Revisit When

Revisit during controlled associative recall implementation if member-level lifecycle proves too heavy. Simplify optional fields before removing member-level status.
