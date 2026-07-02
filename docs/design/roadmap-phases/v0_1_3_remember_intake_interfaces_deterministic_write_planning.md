# v0.1.3 Design Draft: Remember Intake Interfaces and Deterministic Write Planning

## Version intent

Prepare Character Memory's write path for future assisted generation without implementing automatic semantic extraction yet.

The library should support a common write-planning flow that can be used by:

```text
manual caller-provided writes today
future generated memory candidates later
batch import tools
tests and fixtures
review/debug workflows
```

The common flow is:

```text
candidate objects
  -> validation
  -> write plan
  -> commit
```

This phase should make the write path generation-ready while remaining deterministic and YAGNI-compliant.

## Design principle

```text
Build the path that generated memories will travel later.
Do not build the generator yet.
```

v0.1.3 is not the assisted remember workflow. It does not infer high-level memory meaning from raw conversation logs.

## Relationship to future assisted generation

Future assisted generation should be able to produce the same `MemoryCandidate` and `RememberWritePlan` types introduced here.

That later workflow may use model-assisted processors for:

```text
summarization
observation extraction
entity extraction
entity resolution suggestions
thread/scope linking
preference extraction
commitment/open-loop extraction
correction detection
salience scoring
```

v0.1.3 does not implement those processors.

Instead, it prepares the safe validation and commit machinery they will use.

## Why not implement assisted generation now?

The generation workflow will be heavily influenced by what the library can store, how retrieval behaves, and which continuity structures prove useful.

Implementing model-assisted generation too early risks encoding unstable assumptions about:

```text
episode boundaries
observation granularity
derived memory types
scope and thread semantics
correction/currentness behavior
entity linking behavior
retrieval ranking and fanout policy
```

Bad generated memories are more harmful than missing generated memories. They can create false continuity, wrong preferences, unsupported character signals, and polluted entity graphs.

Therefore, full assisted generation should wait until retrieval quality, scoped continuity, factual rigor, observability, and association/clustering mechanisms are mature enough to evaluate generated candidates.

## Scope

v0.1.3 introduces:

```text
RememberInput
RememberWritePlan
MemoryCandidate
CandidateValidation
CandidateProvenance
RememberOutcome
RememberDiagnostics
prepare()
validate()
commit()
remember() convenience flow
deterministic write helpers
```

It does not introduce new memory object types beyond the existing v0.1/v0.1.2 model.

## New concepts

### RememberInput

A caller-provided input to the write-planning workflow.

`RememberInput` may contain already-structured memory objects or structured hints, such as:

```text
episode draft fields
observation draft fields
entity IDs or entity hints
thread IDs or thread hints
scope IDs
participants
timestamps
raw_ref
source spans
derived memory draft fields
memory link draft fields
```

The input should not require the library to infer semantic meaning from raw text.

### MemoryCandidate

A draft object that may become a persisted memory object or link.

Candidate examples:

```text
EpisodeCandidate
ObservationCandidate
EntityCandidate
MemoryThreadCandidate
DerivedMemoryCandidate
MemoryLinkCandidate
VectorIndexCandidate
StatsUpdateCandidate
```

A candidate should be inspectable before commit.

### CandidateProvenance

A structured explanation of where a candidate came from.

Possible provenance inputs:

```text
source conversation ID
message ID
turn range
character offset range
transcript segment ID
timestamp range
raw_ref pointer
episode ID
observation ID
```

Behavior-influencing `DerivedMemoryCandidate` values must carry provenance to an `Episode` or `Observation`.

`CandidateProvenance` also records narrow candidate-origin facts that cannot be reliably reconstructed after commit.

Planned fields:

```rust
enum CandidateProducerKind {
    Caller,
    DeterministicHelper,
    RuleProcessor,
    ModelProcessor,
    ImportTool,
    System,
    Unknown,
}

enum RationaleOrigin {
    ProvidedByCaller,
    ProvidedByProcessor,
    InferredByProcessor,
    Unavailable,
}
```

These fields answer:

```text
who or what proposed the candidate
whether rationale was caller-provided, processor-provided, inferred, or unavailable
```

They are intentionally narrow. v0.1.3 does not add a generic `MetaMemory` object, generic confidence field, parallel context-edge graph, durable retrieval reasons, or generic evidence-reference metadata that duplicates graph-authoritative provenance.

### RememberWritePlan

An inspectable plan describing what would be written if committed.

A plan may contain:

```text
operation ID
idempotency key
source input reference
episode candidates
observation candidates
entity candidates or references
memory thread candidates or references
derived memory candidates
memory link candidates
vector index candidates
retrieval stats update candidates
validation results
diagnostics
```

### RememberOutcome

The result of committing a plan.

It should include:

```text
committed object IDs
committed link IDs
vector indexing status
stats update status
repair-needed markers
diagnostics
```

### RememberDiagnostics

Diagnostics explaining what happened during preparation, validation, or commit.

Examples:

```text
candidate count by type
validation failures
missing link targets
provenance errors
duplicate idempotency detection
repairable vector indexing failure
repairable stats update failure
```

## Workflow

### prepare

`prepare()` converts caller-provided input into a `RememberWritePlan`.

It should not persist anything.

```rust
let plan = memory.prepare(input, prepare_options).await?;
```

### validate_plan

`validate_plan()` checks a plan without committing it.

```rust
let validation = memory.validate_plan(&plan).await?;
```

### commit

`commit()` persists a valid plan.

```rust
let outcome = memory.commit(plan, commit_options).await?;
```

`commit()` should always revalidate because graph state may have changed after `prepare()`.

### remember

`remember()` remains the common convenience method.

```rust
let outcome = memory.remember(input, remember_options).await?;
```

Conceptually:

```text
remember(input)
  = prepare(input)
  + validate_plan(plan)
  + commit(plan)
```

## Commit and review model

Do not model draft/review behavior as many commit modes.

Use workflow composition instead:

```text
DraftOnly                -> prepare()
ValidateOnly             -> validate(plan)
Commit                   -> commit(plan)
RequireApproval          -> prepare() + app-owned approval + commit(approved_plan)
ApplicationReviewCallback -> optional future adapter
AutoCommitSafeCandidates -> future admission/commit policy for generated candidates
```

The only true commit operation is `commit(plan)`.

Review and approval are application workflows, not primitive commit modes.

## Deterministic helpers

v0.1.3 may include helpers for:

```text
stable object ID generation
idempotency key generation
deterministic graph IRI generation
source reference construction
source span construction
one-input-one-episode episode candidate construction
caller-provided observation wrapping
caller-provided entity hint linking
caller-provided thread/scope hint linking
retention defaults
currentness defaults
schema version assignment
provenance link construction
embedding text fallback from caller-provided content text
write-plan validation
diagnostic reporting
```

These helpers should not infer high-level semantic meaning.

## Non-goals

v0.1.3 must not implement:

```text
LLM-based summarization
automatic observation extraction
automatic entity extraction from raw text
automatic entity resolution from natural language
automatic thread inference
automatic scope inference
automatic preference extraction
automatic commitment or open-loop detection
automatic correction detection
automatic character-signal generation
model-assisted salience scoring
model-assisted admission control
privacy classification using a model
raw audio/video processing
full assisted remember workflow
application review callback framework
learned write policy
generic MetaMemory object
generic durable rationale metadata on every memory object
parallel context-edge graph
raw-log storage
raw-log search
public raw-reference resolution
```

## Validation rules

The write-plan validator should check at least:

```text
stable IDs are present or can be assigned
object types are valid
schema version is present
MemoryLink targets exist or are part of the same write plan
behavior-influencing DerivedMemory has Episode or Observation provenance
suppressed memories are not current
superseded memories are not current unless explicitly historical
Qdrant vector candidates point to graph objects in the same write plan or existing graph authority
RetrievalStatsStore updates only reference accepted graph-authoritative relationships
source spans are structurally valid when provided
idempotency keys prevent duplicate retry writes
candidate producer kind is valid when present
rationale origin is explicit when rationale text is supplied
inferred rationale is not represented as caller-provided rationale
raw_ref values are treated as opaque source references, not resolved raw content
```

Invalid plans should not commit.

## Authority split

The existing authority split remains unchanged:

```text
Qdrant:
  vector candidate recall and coarse payload hints

Oxigraph:
  authoritative memory graph, relationships, provenance, lifecycle, currentness, expansion context

RetrievalStatsStore:
  derived counters and selectivity/fanout policy inputs only
```

v0.1.3 must not let write-plan helpers turn Qdrant or the stats store into memory authority.

## Persistence failure handling

Critical writes:

```text
Oxigraph object existence
provenance links
lifecycle/currentness state
supersession/suppression state
```

Repairable writes:

```text
Qdrant vector index
RetrievalStatsStore counters
diagnostics
optional secondary links
```

Partial persistence may create repairable degraded state. It must not create behavior-influencing ungrounded memory.

## Acceptance criteria

```text
A caller can prepare a RememberWritePlan without committing it.
A caller can validate a RememberWritePlan without committing it.
A caller can commit a validated RememberWritePlan.
remember() remains available as a convenience wrapper.
commit() revalidates before writing.
Invalid behavior-influencing DerivedMemory without provenance is rejected.
Missing MemoryLink targets are rejected or deferred according to explicit policy.
Idempotency keys prevent duplicate writes from retry.
Deterministic source references and source spans are preserved.
Manual writes and future generated writes can share the same commit path.
The write-plan flow works with in-memory and persistent graph modes.
Qdrant remains candidate recall only.
Oxigraph remains authoritative for object existence, links, provenance, lifecycle, currentness, and final inclusion.
RetrievalStatsStore remains derived policy metadata only.
No v0.1.3 helper infers preferences, commitments, corrections, character signals, thread membership, or entity identity from raw natural language.
CandidateProvenance records candidate producer kind and rationale origin.
Missing rationale can be represented explicitly as unavailable.
No v0.1.3 helper persists raw logs or resolves raw_ref values.
```

## v0.6 integration path

v0.6 generated memory processors produce `MemoryCandidate` and `RememberWritePlan` values rather than bypassing the validation and commit path.

The v0.6 work owns generated-candidate admission states such as:

```text
Accepted
Deferred
NeedsReview
Rejected
Invalid
```

v0.1.3 does not add those states.

## Implementation notes

- `RememberOutcome` was extended in place for commit diagnostics, vector indexing status, stats update status, and repair markers. No separate commit-outcome type was introduced.
- Missing `MemoryLink` targets are strictly rejected when the target is absent from both the plan and the graph. Deferring unresolved link candidates remains a future option, not a v0.1.3 behavior.
- `CandidateProducerKind` and `RationaleOrigin` are plan/diagnostics-time metadata. They do not add a durable metadata plane or require a schema-version bump.
- Idempotency uses deterministic UUIDv5 object/link IDs plus idempotent graph upsert keyed by the plan idempotency key. Exact retries are idempotent, same-key divergent plans are rejected with diagnostics, and there is no persisted operation ledger in v0.1.3.
- The shipped facade keeps `remember(RememberDraft)` source-compatible. `RememberInput` is used through `prepare(input, options)`; the earlier `remember(input, options)` shape in this draft is a suggested shape, not the implemented signature.
- "Superseded memories are not current unless explicitly historical" is implemented with `RetentionState::Archived` as the explicit historical state for superseded candidates.
