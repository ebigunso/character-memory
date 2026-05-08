---
status: accepted
adr_type: implementation
date: 2026-05-08
deciders: ["ebigunso"]
consulted: ["GPT-5.5 Pro"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0012: Use prepare / validate / commit for the write workflow

## Context and Problem Statement

The remember intake interfaces and deterministic write planning phase introduces an inspectable write path that can be used by manual caller-provided writes and, later, assisted remember workflows that generate memory candidates.

That path needs to support draft inspection, validation without persistence, application-owned review, and final persistence without making each workflow variation a separate commit primitive.

The core design pressure is to keep planning, validation, review, and persistence explicit enough to test and reason about while still preserving `remember()` as the simple convenience path.

## Decision Drivers

- Keep planning, validation, review, and persistence operations distinct.
- Avoid a bloated commit-mode enum.
- Preserve application-owned approval workflows without making them core commit primitives.

## Decision

Character Memory will use a small write workflow:

```text
prepare()
validate()
commit()
remember()
```

`remember()` is a convenience wrapper around:

```text
prepare
  -> validate
  -> commit
```

The only true commit operation is `commit(plan)`.

## Implementation Impact

This keeps the remember intake interfaces and deterministic write planning API streamlined while preserving review and draft workflows.

Applications that need review can own the approval flow by preparing a plan, editing or filtering it, then committing the approved plan.

## Considered Options

1. Add many first-class commit modes.
2. Hide draft and validation behavior inside `remember()`.
3. Use explicit `prepare()`, `validate()`, and `commit()` operations with `remember()` as convenience.

Examples of first-class modes that were considered but rejected include draft-only, validate-only, require-approval, application review callback, and generated-candidate auto-commit variants. These names mix planning, validation, review, persistence, and admission policy concerns, so they are better represented by workflow composition.

## Decision Outcome

Chosen option: **3. Use explicit prepare, validate, and commit operations with remember as convenience**.

This keeps API semantics direct and avoids turning application review policy into core library persistence modes.

## Consequences

### Positive

- Avoids a bloated `CommitMode` enum.
- Keeps planning, validation, review, and persistence distinct.
- Makes future assisted generation easier to integrate.
- Allows application-specific approval workflows without adding core complexity.

### Negative / Tradeoffs

- Applications must compose review workflows explicitly.
- There is no built-in application review callback in the remember intake interfaces and deterministic write planning phase.
- Future generated-candidate admission policies may require additional APIs.

## Validation

- `prepare()` must not persist memory.
- `validate()` must not persist memory.
- `commit()` must persist only a valid plan.
- `remember()` should be tested as equivalent to prepare plus validate plus commit for simple cases.
- Approval-style workflows should be possible by editing or filtering a prepared plan before commit.

## Revisit When

Revisit during the assisted remember workflow and memory candidate generation phase if generated candidates require first-class admission policies such as `CommitAcceptedCandidates`.

## More Information

- Related roadmap phase: remember intake interfaces and deterministic write planning.
