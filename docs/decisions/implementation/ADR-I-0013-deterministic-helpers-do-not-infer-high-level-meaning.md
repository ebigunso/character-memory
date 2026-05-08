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

# ADR-I-0013: Deterministic write-planning helpers do not infer high-level meaning

## Context and Problem Statement

The remember intake interfaces and deterministic write planning phase introduces helper APIs to prepare write plans and make the remember path ready for future assisted generation.

There is a risk that this phase could gradually turn into premature automatic extraction, such as inferring preferences, commitments, thread membership, or entity identity from raw text before the retrieval and governance layers are ready to evaluate generated memory quality.

## Decision Drivers

- Keep the remember intake interfaces and deterministic write planning phase deterministic and testable.
- Avoid false continuity from premature semantic inference.
- Preserve assisted generation for the later assisted remember workflow and memory candidate generation phase.

## Decision

Deterministic write-planning helpers may normalize, package, validate, link caller-provided IDs, assign stable IDs, preserve source spans, assign lifecycle defaults, and construct write plans.

They must not infer high-level memory meaning from raw natural language.

Specifically, these helpers must not automatically infer:

```text
preferences
commitments
open loops
corrections
character signals
relationship state
thread membership
scope membership
entity identity
salience from raw natural language
```

unless the caller supplied the relevant structured information.

## Implementation Impact

This keeps the remember intake interfaces and deterministic write planning phase focused on write-path safety rather than assisted memory generation.

Full model/rule-assisted generation remains a later roadmap phase.

## Considered Options

1. Let deterministic helpers infer obvious semantic facts from text.
2. Add model-assisted extraction in the remember intake interfaces and deterministic write planning phase.
3. Keep deterministic write-planning helpers deterministic and require caller-supplied structure for high-level meaning.

## Decision Outcome

Chosen option: **3. Keep deterministic write-planning helpers deterministic and require caller-supplied structure for high-level meaning**.

This prevents the write-planning phase from silently becoming the assisted remember workflow.

## Consequences

### Positive

- Prevents premature generation assumptions from shaping the memory model.
- Keeps the write-planning phase deterministic and testable.
- Avoids false continuity from weak inferred memories.
- Preserves room for assisted remember workflow and memory candidate generation to be designed after retrieval observability and governance mature.

### Negative / Tradeoffs

- Callers must still provide structured memory information in the write-planning phase.
- The library remains less convenient than the full assisted remember workflow and memory candidate generation phase.
- Some deterministic extraction opportunities may be deferred.

## Validation

- Tests for the write-planning phase should verify that raw text alone does not automatically create preferences, commitments, character signals, or entity resolutions.
- Helpers may wrap caller-provided observations or links, but should not infer them from raw text.
- Any future processor that performs inference should live in the later assisted generation phase or behind clearly named processor interfaces.

## Revisit When

Revisit during the assisted remember workflow and memory candidate generation phase, where model/rule-assisted processors may generate candidates but must still use the write-planning validation and commit path.

## More Information

- Related roadmap phase: remember intake interfaces and deterministic write planning.
- Future roadmap phase: assisted remember workflow and memory candidate generation.
