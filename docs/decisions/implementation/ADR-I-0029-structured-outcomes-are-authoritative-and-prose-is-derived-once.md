---
status: accepted
adr_type: implementation
date: 2026-09-14
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely add a new outcome, diagnostic, or error as a message string with the evidence interpolated into it, because that is the shortest change at the producing site"
  detected_signals: "cross-boundary contract shape (every public outcome, diagnostic, trace, and error payload); rejected alternative likely to be re-proposed; a decider's ruling setting a durable governance default"
  cost_of_violation: "a consumer that must parse prose to learn which object, surface, or cause an outcome names cannot be tested against structure, drifts silently when wording changes, and turns inspectable recall into unexplained recall in structured clothing"
  cost_of_over_extension: "applying the closed-vocabulary requirement to adapter-internal or transport-level detail that has no consumer forces a public enum for every backend quirk"
depends_on: [implementation/ADR-I-0012-use-prepare-validate-commit-write-workflow.md, implementation/ADR-I-0018-responsibility-boundary-modules-with-enforced-dependency-direction.md]
implements: []
supersedes: []
superseded_by: null
supersession_scope: null
---

# ADR-I-0029: Structured outcomes are authoritative and prose is derived once at the owning type

## Context and Problem Statement

Every public path of the library reports what it did through outcomes, diagnostics, retrieval traces, and errors. The state that the structured-verdict observability plan (`docs/coding-agent/plans/completed/structured-verdict-observability-plan.md`, July 2026) found was this: several of those reports carried evidence only in prose: a validation warning interpolated object identifiers into a sentence, a failed graph expansion carried a reason string and a location string, an error kind was a bare string, and a context-pack section assignment explained itself in a free-text reason. A consumer that needed the evidence had to parse the sentence, and tests asserted substrings, so a wording change could break consumers or hide a regression. The retrieval philosophy requires that results carry score components and rationale, and treats unexplained recall as a failure mode; a trace row that cannot name its evidence in structure is unexplained recall with a sentence attached.

## Decision Drivers

- Consumers, including the evaluation harness, need the evidence an outcome names, not a rendering of it.
- Tests must assert on variants and fields so that wording is free to change and regressions cannot hide behind message equality.
- The convenience write path and the explicit prepare, validate, commit path must expose identical structured outcomes (ADR-I-0012).
- Structured payloads must sit where the dependency direction allows every producer and the error type to reference them (ADR-I-0018).

## Decision

Structure is authoritative and prose is a projection derived exactly once, at the type that owns the structure.

- Every public outcome, trace element, and error payload carries its evidence as typed fields: identifiers as identifier types, object references as the shared object reference, vocabularies as enums, causes as the typed cause. Evidence means a fact a consumer acts on or asserts: which object, which surface, which vocabulary value, which cause, how many. An opaque detail that no consumer branches on (a backend driver message, a parser's own text, a file path from the environment) may be carried as a string field beside the typed fields. No producer interpolates evidence into a message, and no message is the only carrier of a structured fact. A diagnostic is the one bounded exception: it is a rendering (a severity, a code, and a message) of typed facts that travel on the same outcome, so it may carry prose, but only prose derived from those typed facts, and never a fact that exists nowhere else.
- Each such payload renders its message in one place, its own display implementation; the top-level error type carries every payload as a typed field a consumer can match on, whether wrapped transparently, attached as a source, or formatted into the variant's own message. A composite message, such as a diagnostic that names a cause, is assembled only from those displays and never from a payload's fields, and the typed cause travels on the outcome beside the diagnostic so the message is never the only carrier.
- Vocabularies that consumers must match exhaustively (validation issues, diagnostic codes, section-assignment reasons, graph failure modes, indexing causes) are closed enums that consumers can match completely; new meaning enters as a new variant with its fields, not as a new sentence.
- Payload types referenced by the error type live in the domain or error modules; outcome records that embed them live in the API layer and may reference domain and error types, following the dependency direction of ADR-I-0018.
- Tests assert variants and structured fields. String assertions are permitted on serialization tokens (the public wire contract of an enum or code) and on a projection's code or severity; a new or changed test never asserts message text.

## Character Memory Relevance

Provenance and inspectable recall are product goals: a character must be able to say why it remembered something and what it did with a correction. That explanation is trustworthy only when the structure carries it and the words are derived from the structure, never the other way round.

## Implementation Impact

- A new outcome, diagnostic, or error is designed as a typed payload first; its message is written once on that type.
- Adding evidence to an existing outcome adds a field or a variant, never a phrase.
- Consumers exhaustively match the closed vocabularies, so a new variant is a compile-time event for every consumer rather than a silent change in wording.
- Consumers, including the evaluation harness through its own conversion layer, take the structured fields and never parse prose.

## Considered Options

1. Typed payloads with one central message derivation per type.
2. Message strings with a documented format that consumers parse.
3. Typed payloads plus hand-written messages at each producing site.

## Decision Outcome

Chosen option: **Option 1**. It is the only option under which a consumer can be tested against the evidence, wording can change freely, and the two write paths cannot diverge in what they report.

### Rejected Alternatives

Option 2 makes every consumer a parser of an informal grammar and turns every wording change into a compatibility event; rejected outright.

Option 3 keeps the structure but lets the same fact render differently at different sites, which is how the two write paths had diverged in the state that the structured-verdict observability plan found; rejected outright.

## Consequences

- Positive: outcomes, traces, and errors are testable and consumable without parsing.
- Positive: messages can improve without breaking anything.
- Negative / tradeoffs: every new evidence-bearing case costs a variant and its fields rather than a sentence.

## Decision Boundary

Invariant: no public outcome, diagnostic, trace element, or error carries a fact only in prose, and no message is rendered in more than one place for the same payload. Changing this requires a superseding record.

Not covered: which enums beyond the exhaustively matched vocabularies stay open for extension (the top-level error type, backend mismatch and configuration reason enums, and telemetry structs may remain non-exhaustive); the exact field sets of any payload, which follow the producers and are recorded in code and tests; error variants whose whole content is opaque detail from a backend, parser, or the environment (for example a database driver failure or a graph selection detail), which carry that detail as a string because no consumer branches on it, and which become typed the moment a consumer needs a fact from them; the remember diagnostic (a severity, a code, and a rendered message) as a deliberately bounded projection whose typed facts travel elsewhere on the same outcome (the validation rows and the repair markers' typed causes), so the diagnostic is a rendering and never a fact's only carrier; the message-text assertions that predate this record, which are migrated as their sites are touched rather than in one sweep.

## Validation

- Compile-time: consumers match the closed vocabularies exhaustively; a new variant fails their build until handled.
- Review: a change that adds a message without a typed field, or renders a payload's message outside its display implementation, is rejected at review.
- Tests: a new or changed test asserts variants and fields, with string assertions only on serialization tokens, codes and severities; a message-text assertion is migrated whenever its site is touched.

## Revisit When

A consumer class appears that can only receive text (for example a transport that carries no structure) and needs a stable message grammar; that would call for a projection contract, not for prose becoming authoritative.

## More Information

- Design history: the structured-verdict observability phase (`docs/coding-agent/plans/completed/structured-verdict-observability-plan.md`, Decision Log and its amendments), where the typed vocabularies, the typed error story, and the trace identity fields were decided finding by finding; this record states the rule they share.
- ADR-I-0030 (ports own their postconditions) records the other durable ruling of the structured-verdict observability plan named above.
