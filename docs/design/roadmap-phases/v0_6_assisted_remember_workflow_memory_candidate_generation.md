# v0.6 Design Draft: Assisted Remember Workflow and Memory Candidate Generation

## Version intent

Make Character Memory easier to use by allowing callers to pass bounded raw, transcript-like, or structured interaction input transiently into the assisted remember workflow while the library generates structured memory candidates.

This phase keeps caller control over memory creation.

The caller decides:

```text
when to remember
what input to offer
which processors may run
what privacy policy applies
whether candidates are committed, reviewed, deferred, or discarded
where source material is retained outside Character Memory, if retained
```

The library provides:

```text
candidate generation
candidate provenance
candidate validation
write-plan construction
diagnostics
commit through the v0.1.3 path
```

The library does not persist the raw input.

## Relationship to v0.1.3

v0.6 depends on v0.1.3.

Model/rule-assisted processors should produce:

```text
MemoryCandidate
RememberWritePlan
CandidateProvenance
RememberDiagnostics
```

They must not bypass:

```text
validation
provenance checks
graph-authority checks
retention/currentness checks
idempotency checks
commit pipeline
```

## Raw input boundary

Raw input passed to v0.6 processors is transient processing input.

Character Memory may use that input to produce:

```text
EpisodeCandidate
ObservationCandidate
EntityCandidate
Thread/scope link candidate
DerivedMemoryCandidate
MemoryLinkCandidate
salience/admission candidate
embedding surface candidate
CandidateProvenance
RememberDiagnostics
```

Character Memory does not persist the raw input itself.

When the caller supplies source references, generated candidates preserve:

```text
raw_ref
source_kind
source_span
message_id
turn_range
timestamp_range
modality
```

Those references remain opaque provenance handles. They do not require Character Memory core to resolve, retain, search, export, delete, redact, encrypt, or otherwise manage the underlying source material.

## Why this comes after retrieval and governance work

Generated memories are only useful if the system can evaluate whether they improve recall and continuity.

By v0.6, the roadmap should already have:

```text
entity selectivity and fanout guardrails
generation-ready write planning
scoped continuity and reflection
factual rigor and temporal validity
retrieval traces and governance
association/clustering behavior
```

Those layers make it possible to judge whether generated candidates help or pollute memory.

## Goals

```text
accept bounded caller-provided raw/transcript-like input as transient processor input
normalize supported input envelopes for candidate generation
optionally segment input into episode candidates
generate episode summaries
extract salient observation candidates
extract entity candidates
suggest entity resolution actions
suggest thread/scope links
generate DerivedMemory candidates
score salience/admission candidates
generate natural-language embedding surfaces
respect privacy/exclusion policy before processor calls
preserve caller-supplied source references and source spans
return inspectable RememberWritePlan
support draft/review/commit workflows through v0.1.3 primitives
```

## Non-goals

```text
autonomous background log scanning
raw conversation-log storage
raw transcript storage
verbose tool-output storage
raw file/blob storage
raw image/audio/video storage
raw sensor-log storage
raw-log search
public raw-reference resolution
raw audio transcription
raw image/video understanding
robotic sensor fusion
unreviewable autonomous memory persistence
hard-coded application roles
generated memory bypassing validation
direct model-minted final entity IDs
automatic stable character-signal consolidation from single episodes
learned memory policy as default
```

## Processor boundary

Processors should be pluggable.

Possible processors:

```text
EpisodeSegmenter
EpisodeSummarizer
ObservationExtractor
EntityMentionExtractor
EntityResolutionSuggester
ThreadScopeLinker
DerivedMemoryGenerator
SalienceAdmissionScorer
EmbeddingSurfaceGenerator
PrivacyExclusionFilter
```

The library may provide default processors, but the commit path remains provider-agnostic.

## Candidate states

v0.6 may introduce richer candidate/admission states:

```text
Proposed
Accepted
Deferred
NeedsReview
Rejected
Invalid
```

These states are not required in v0.1.3.

## Commit policy

v0.6 may add a convenience policy such as:

```text
CommitAcceptedCandidates
```

This should mean:

```text
commit candidates accepted by validation and admission policy
return deferred or review-needed candidates
reject invalid candidates
```

Avoid vague names such as:

```text
AutoCommitSafeCandidates
```

because "safe" is policy-dependent.

## Privacy and exclusion

Privacy policy must apply before model/external processor calls.

The workflow should support:

```text
excluded spans
do-not-remember spans
local-only processing
external processing disabled
draft-only generation
processor-level redaction
```

## Acceptance criteria

```text
Caller can pass raw chat/transcript-like input and receive a RememberWritePlan.
Raw/transcript-like input can generate candidates without being persisted by Character Memory.
Generated candidates preserve caller-supplied raw_ref/source-span provenance.
raw_ref remains opaque to Character Memory core.
No raw-log search API is added.
No public raw-reference resolution API is added.
Caller can request draft-only generation through prepare-like behavior.
Generated DerivedMemory candidates include provenance.
Explicit corrections generate correction candidates.
Explicit commitments generate commitment/open-loop candidates.
Entity candidates are resolved through graph authority rather than final IDs minted directly by a model.
Thread/scope links are optional and confidence-scored.
Embedding text is natural language, not metadata dumps.
Generation diagnostics expose accepted, rejected, deferred, and review-needed candidates.
Privacy exclusions are respected before external processor calls.
Generated candidates use the same validation and commit path as manual candidates.
```

## Revisit when

Revisit if assisted generation creates too many low-value candidates, false preferences, false commitments, or unsupported character signals.

The first fix should be processor policy, validation, admission, or review controls, not weakening provenance requirements.
