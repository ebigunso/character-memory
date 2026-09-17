---
status: proposed   # proposed until a human accepts the record on its own; then accepted; superseded on retirement
adr_type: {design | implementation}
date: YYYY-MM-DD
deciders: []
consulted: []   # durable identities: full model names or person names, never roles or platforms, e.g. "Claude Fable 5.1", "GPT-6 Astra"
informed: []
supersedes: []        # current relative paths of records this one replaces in full; update when an archive move renames the target file
superseded_by: null   # null while the record governs; the retired record carries the current relative path of its replacement
# Optional key, include only when applicable — depends_on: [] (records this decision builds on, as current relative paths)
---

# ADR-{D|I}-XXXX: {The decision, stated as a sentence a reader could act on}

<!-- Two tracks (design D / implementation I) with per-track numbering are the default; a repository may collapse to a single track. Merged IDs are never reused. One decision per record; see references/adr.md. -->

## Context and Problem Statement
{The fork in one paragraph: what pressure made this a decision point. No history of the investigation; see references/adr.md.}

## Decision
{The constraint on future work, stated directly. No implementation wording, exact strings, or thresholds; see references/adr.md.}

## Why
{One or two sentences of prose, present tense. No tables, commit hashes, or evidence sections; see references/adr.md.}

## Rejected Alternatives
{One line per alternative: why it lost, and the condition that would legitimately reopen it, or "rejected outright". Omit only when nothing needs explaining.}

## Decision Boundary
Invariant: {what changing requires retiring this record and writing a new one}.

Not covered: {the calibrated surfaces that may change through skill text, configuration, or a plan record}.

## Validation
{How implementation or review shows the decision is followed: a check, a test, a review question.}

## Revisit When
{The premise whose expiry reopens the decision. When the premise is model behavior, name the models and the date they were checked. No time-relative wording; see references/adr.md.}

## More Information
{Optional: related records by ID, and the location of records or experiments, as a pointer only.}
