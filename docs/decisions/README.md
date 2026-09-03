# Decision Records

This directory contains decision records (ADRs), split into two tracks so high-level design decisions do not mix with implementation choices. New records follow [template.md](template.md).

## Directory layout
```text
docs/decisions/
  README.md
  template.md
  design/
    ADR-D-0001-...
  implementation/
    ADR-I-0001-...
  superseded/
    ADR-D-0002-...--superseded-by-ADR-D-0009.md
    ADR-I-0003-...--retired.md
```

## Numbering and tracks

Separate numbering per track; IDs are never reused.
- `ADR-D-NNNN` — design track: use when overlooking the decision would risk violating the core Character Memory philosophy: episode-backed continuity, provenance, correction, reflection, scoped continuity, or entity-neutral recall.
- `ADR-I-NNNN` — implementation track: use when the decision is primarily about how the library is built: storage contracts, indexing, IDs, schema versions, retrieval bounds, fanout policy, derived stats, and integration behavior.

## Lifecycle and the superseded/ archive
- Active track directories list only governing decisions; numbering gaps signal archived history.
- On full supersession or retirement, the record moves to `superseded/` — a single flat folder where the track prefix in the filename preserves identity — renamed with a self-describing suffix: `--superseded-by-ADR-X-NNNN` or `--retired`.
- Partial supersession stays in place: the record remains authoritative for its surviving clauses, with `supersession_scope` and reciprocal frontmatter links recording the split.
- In records predating the current template, a blank warrant means it was not recorded at decision time, not an authoring omission. Fill blank newer frontmatter fields only when the record is substantively revisited.

## Authoring rules
- One decision per record. A choice of component and a choice of how that component is run or made durable are separate decisions with separate records.
- A record encodes a locked-in decision. A deciding factor is deferred only when it is out of reach because of external factors, never when the same change produces the evidence.
- The decision body pins the system's design and, at most, the behaviour of a narrow part of the implementation. Code shapes go in a non-binding appendix, never in the decision.
- Every protected clause is checked against the project philosophy: it stays only if a philosophy goal (continuity, provenance, inspectable recall, correction) is what it protects. Current state is recorded under "Not covered", not as an invariant.
- Records read the same at any time. No wording that hinges on when the record was written ("this phase", "later decision", "at decision time", "once", "already", change verbs such as "gains" or "moves"); history is anchored to named records, versions, and absolute dates, and Implementation Impact describes the resulting state.

## Status values
`accepted`, `rejected`, `superseded`, `deprecated`. Records capture decisions, not undecided proposals.
