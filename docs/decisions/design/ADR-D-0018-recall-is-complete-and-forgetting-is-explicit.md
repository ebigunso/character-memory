---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "Without this record, future retrieval or maintenance work would likely add a time-based decay of salience or eligibility to imitate human forgetting, because the philosophy names human-comparable memory as the target and decay is the obvious mechanism."
  detected_signals: "A rejected alternative likely to be re-proposed; a decider's ruling setting a durable governance default; a cross-boundary contract between lifecycle operations and retrieval eligibility."
  cost_of_violation: "A memory that faded on its own cannot be found when a character acts on someone's behalf, and the failure cannot be diagnosed from any trace because no decision produced it; removing decay after memories have drifted requires rebuilding derived scores from evidence."
  cost_of_over_extension: "Reading this record as forbidding elapsed time as a query-time ranking signal, or as forbidding archival of consolidated periphery, would block bounded context packs over long timescales."
depends_on: [ADR-D-0006-supersession-and-suppression.md, ADR-D-0017-append-only-memory-record-with-out-of-band-purge.md]
supersedes: [ADR-D-0017-append-only-memory-record-with-out-of-band-purge.md]
superseded_by: null
supersession_scope: partial
---

# ADR-D-0018: Recall is complete, and forgetting is always an explicit operation

## Context and Problem Statement

The philosophy sets human-comparable memory as the target, and the continuity situation catalog describes human forgetting: peripheral detail fades, gist persists, importance resists fade. The direct way to build that is a decay mechanism, a stored importance that falls with time or a reachability that shrinks with age. ADR-D-0017 names "salience decay" as a future lifecycle mechanism and lists decay among the forms of forgetting.

A character built on this library is relied upon to act for people: it is asked favors, given instructions, and dispatched on tasks, and it usually runs in its default mode when it does. A person who forgets an instruction answers for it. Software answers through its operator, so the operator must be able to rely on complete recall. A memory that quietly became unreachable is a defect the developer cannot diagnose and the character cannot explain.

The question is whether any human-shaped forgetting belongs in the substrate, or whether it is entirely a property of what the character volunteers.

## Decision Drivers

- A character acting on someone's behalf must be able to find every memory it holds.
- Every change in what influences behavior must be inspectable, with a decision behind it.
- Derived policy values must not become durable graph state that drifts from its evidence.
- The catalog's human-shaped fading must still be achievable in observable behavior.

## Decision

Recall is complete. The character can find any memory it holds. No memory becomes less reachable because time passed, and no stored measure of importance changes on its own.

Forgetting takes exactly three forms, each intentional, reversible, and inspectable:

```text
suppression   removes a memory's influence; the record remains for audit and un-suppression
archival      moves a memory out of current context; it remains retrievable on request
supersession  replaces a memory with a corrected one; the old memory remains as history
```

Human-shaped fading is a property of ranking and expression, never of retention. Recent and important material is offered in detail, old and minor material as gist, and the whole remains available when it matters. Elapsed time since a memory may be a query-time ranking signal with its own rationale. Familiarity, stability, and the weight of repeated evidence are derived from provenance at query time, not stored as scores that move.

This record replaces the clauses of ADR-D-0017 that name decay as a form of forgetting or as a future lifecycle mechanism. Every other clause of ADR-D-0017 stands.

## Character Memory Relevance

A character that fades cannot defend its own history when told about itself, cannot keep a favor it was asked months ago, and cannot survive a model swap invisibly, because the memory is the character and the model is the actor. Complete recall is what makes each of those possible. A person cannot always recover the detail behind the gist; this character can, and that asymmetry is kept as an advantage rather than imitated away.

## Considered Options

1. Stored decay: a salience or eligibility that falls with time, imitating human forgetting in the substrate.
2. Complete recall with explicit lifecycle operations, and fading produced by ranking and expression.
3. Complete recall by default with a decaying retrieval mode selectable per query.

## Decision Outcome

Chosen option: **2. Complete recall with explicit lifecycle operations**. Every behavior the catalog attributes to human forgetting was traced to a mechanism that does not lose anything: write-time attention decides what becomes memory, query-time ranking weighs salience and elapsed time, expression offers gist or detail, and the three lifecycle operations cover every case where influence should stop. No situation was found in which a memory becoming unreachable served the ideal behavior.

### Rejected Alternatives

Stored decay is rejected outright. It makes the same query return different memories on different days for no reason a trace can show, it turns a derived policy value into durable graph truth, and it fails the roles that rely on the character most. No measurement reopens it, because the objection is to unexplainable loss, not to a threshold.

A selectable decaying mode is rejected because consumer applications run the default mode when interacting with the outside world, including when dispatching tasks, so the default would carry the failure. It may be reconsidered only if a deployment class emerges whose ideal behavior requires loss rather than discretion, and none has been identified.

## Consequences

- Positive: recall is diagnosable from traces; task-critical memories cannot silently vanish; the character can verify claims about its own past.
- Positive: no reinforce operation and no drifting scores; repeated evidence is counted from provenance.
- Negative / tradeoffs: bounded context over long timescales must come from ranking, consolidation into gist, and archival, all of which must earn their place through measurement.
- Negative / tradeoffs: what the character volunteers becomes an expression concern the application shares, since the library cannot make a memory less findable to make it less said.

## Decision Boundary

Invariant: no memory's retrieval eligibility changes except through suppression, archival, or supersession, each recorded as a decision with an actor. No stored importance, confidence, or stability changes without a write that carries provenance.

Not covered: whether elapsed time is a ranking signal and with what weight, which is a measured retrieval default; how consolidation summarizes periphery; how expression renders gist versus detail.

## Validation

- Lifecycle tests show that a memory's eligibility is unchanged by the passage of time alone and changes only through the three named operations.
- Retrieval traces name a lifecycle decision for every omitted eligible memory, never an age.
- Review rejects any stored score that a maintenance pass updates without provenance.
- Evaluation scenarios for tasks and favors assert recall of instructions across long gaps.

## Revisit When

A deployment class is identified whose ideal behavior requires a memory to become unreachable rather than undisclosed, and discretion at expression demonstrably cannot serve it.

## More Information

- ADR-D-0006 defines suppression and supersession.
- ADR-D-0017 defines the append-only record and the out-of-band purge; its decay clauses are replaced by this record.
- ADR-I-0008 and ADR-I-0017 establish that derived values are not graph truth and that evidence, not scores, is persisted.
