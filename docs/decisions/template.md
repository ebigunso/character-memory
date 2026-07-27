---
status: accepted
adr_type: {design | implementation}
date: YYYY-MM-DD
deciders: []
consulted: []   # durable identities: full model names or person names, never roles or platforms, e.g. "Claude Fable 5", "GPT-5.6 Sol"
informed: []
warrant:                          # self-contained phrases, one topic per key — no shorthand codes. Include at least one cost key, and only the ones that apply: their presence names the mishandling modes. Every included cost must be costly to detect or undo, not normal-review/cheap-refactor catchable — that severity floor governs all three cost keys alike
  warranted_by: ""                # the falsifiable expectation this record prevents: "without this record, future work would likely ..."
  detected_signals: ""            # spelled out; vocabulary: cross-boundary contract/authority/evidence-ownership shape; rejected alternative likely to be re-proposed; costly migration/reversal; cross-repository obligation; a decider's ruling setting a durable governance default; premises likely to expire; deliberately bounded scope
  cost_of_violation: ""           # what breaks if future work goes against the decision while its premises hold; omit when not applicable
  cost_of_wrong_preservation: ""  # what it costs if the decision outlives its expired premises; omit when not applicable
  cost_of_over_extension: ""      # what it costs if the decision is applied beyond its deliberately bounded scope; omit when not applicable
supersedes: []        # current relative paths; update when an archive move renames the target file
superseded_by: null   # current relative path; update when an archive move renames the target file
supersession_scope: null   # full | partial; set on both sides of a supersession
# Optional keys, include only when applicable — depends_on: [] (ADRs this decision builds on); implements: [] (ADRs this decision implements); both carry current relative paths
---

# ADR-{D|I}-XXXX: {Decision title}

<!-- Two tracks (design D / implementation I) with per-track numbering are the default; a repository may collapse to a single track. IDs are never reused. -->

## Context and Problem Statement
{What problem, risk, or design pressure makes this decision necessary? Keep this concrete.}

## Decision Drivers
- {driver 1}
- {driver 2}

## Decision
{State the decision directly.}

## Character Memory Relevance
{Optional. Use when the decision protects the Character Memory philosophy: episodic continuity, provenance, correction, reflection, or character formation. Omit or shorten for purely implementation-level ADRs.}

## Implementation Impact
{Optional. Use when the decision affects API shape, storage, tests, migrations, performance, or operational behavior. Omit or shorten for high-level design ADRs.}

## Considered Options
1. {Option A}
2. {Option B — a clean one-line description; rejection reasoning goes below, not inline.}

## Decision Outcome
Chosen option: **{Option X}**. {Explain why this option best satisfies the decision drivers.}

### Rejected Alternatives
{One paragraph per rejected option: why it was rejected. When the option is likely to be re-proposed, also state the evidence or condition that would legitimately reopen it; when it is rejected outright, say so. Omit the section only when no rejection needs explaining.}

## Consequences
- Positive: {positive consequence}
- Negative / tradeoffs: {tradeoff}

## Decision Boundary
{Optional. Format the parts visually separately — one line or paragraph each, sized to their content.}

Invariant: {what changing requires a superseding ADR; include a deliberately bounded scope and its rationale here — the guard against wrongly extending the decision.}

Not covered: {calibrated defaults and free surfaces that may change through measured configuration or a plan record.}

## Measurement Basis
{Optional. For empirically grounded decisions: the corpus, configuration, and provenance behind the numbers; scope limits; a reproducibility pointer. Evidence alone does not warrant an ADR.}

## Validation
{How will implementation or review prove this decision is being followed? Examples: schema checks, compile-time types, integration tests, migration tests, documentation review.}

## Revisit When
{State the premise whose expiry reopens the decision — this section is what makes legitimate reversal safe instead of wrongly preserving the decision.}

## Consultation impact
{Optional, one line: question asked, ruling adopted or rejected, unresolved dissent.}

## More Information
{Optional links to related ADRs, issues, experiments, or evidence.}
