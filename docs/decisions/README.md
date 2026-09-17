# Decision Records

This directory contains decision records (ADRs), split into two tracks so high-level design decisions do not mix with implementation choices. New records follow the harness template `durable-docs-authoring/references/adr-template.md`, copied here as `template.md` when the repository keeps its own; the admission test, form, lifecycle, and acceptance rules are in the harness reference `durable-docs-authoring/references/adr.md`.

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

Separate numbering per track; a number is taken at merge, merged IDs are never reused, and a gap in an active directory means a retirement.
- `ADR-D-NNNN` — design track: use when overlooking the decision would risk violating the core Character Memory philosophy: episode-backed continuity, provenance, correction, reflection, scoped continuity, complete recall, or entity-neutral recall.
- `ADR-I-NNNN` — implementation track: use when the decision is primarily about how the system is built (storage, APIs, schemas, tooling, operations).

## Lifecycle and the superseded/ archive
- Active track directories list only governing records, one decision each, each complete on its own.
- A record is immutable once merged; a changed decision is a new record, and the old one is retired.
- On retirement the record gets a header line under its title ("Retired on DATE. Replaced by ADR-X." or "Retired on DATE; nothing in it still binds.") and moves to `superseded/`, a single flat folder where the track prefix in the filename preserves identity, renamed with `--superseded-by-ADR-X-NNNN` or `--retired`. Retired records are frozen.

## Status values
`proposed` (a draft on a branch, before a human has accepted it on its own), `accepted` (accepted explicitly; immutable once merged), `superseded` (retired). Acceptance and immutability are separate: acceptance is the human's yes, immutability is the merge.
