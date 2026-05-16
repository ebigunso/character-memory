# Character Memory Development Roadmap

## One-line thesis

```text
Character Memory is an episode-backed continuity substrate for persistent AI characters, assistants, companions, simulations, and research systems.
```

The roadmap should build the system in layers. The starter must be useful for chat-native memory without pretending to support every future modality or every epistemic feature. Later phases should add continuity, factual rigor, observability, association, and multimodal expansion without breaking the episode-backed core.

---

# 1. Design north star

A persona can be assigned in a prompt. Character is accumulated through remembered experience.

Therefore the system should optimize for:

```text
temporal continuity
entity continuity
relationship and scope continuity
project/thread continuity
correction and revision
retrieval rationale
provenance from derived memory back to episodes
bounded retrieval that remains useful over long timescales
```

The design should not be evaluated only by top-k retrieval quality. It should be evaluated by whether a persistent character can behave as the same continuing participant over time while remaining correctable, inspectable, and scoped to grounded memory.

---

# 2. Cross-version invariants

These should remain stable even as the library evolves.

## 2.1 Episodes are primary

Every behavior-influencing derived memory should trace back to at least one episode or observation.

```text
DerivedMemory → Episode / Observation
```

## 2.2 Stable IDs are mandatory

Every durable memory object has a stable ID and graph URI.

```text
object_id
object_type
graph_uri
schema_version
```

Vector points must reference graph objects; graph objects must be retrievable by ID.

## 2.3 Threads are soft overlays

A `MemoryThread` is a continuity pattern, not a chat container.

```text
Episode may belong to zero, one, or many threads.
Thread membership has confidence and rationale.
Thread assignment can be revised.
```

## 2.4 Current usable context is derived

Do not confuse raw historical memory with current context.

Current views may include:

```text
active threads
current scoped preferences
active commitments
active open loops
current character signals
current relationship state
current factual beliefs, once the later belief layer exists
```

## 2.5 Correction should usually supersede, not erase

Most corrections should create new memory and links:

```text
new memory supersedes old memory
correction episode explains why
old memory remains historical unless suppressed or a later explicit destructive policy is implemented
```

## 2.6 Retrieval should be explainable

The system should expose why a memory was retrieved:

```text
semantic similarity
same thread
same entity with high selectivity
same entity with low selectivity but additional support
recent event
open commitment
preference relevance
correction relevance
high salience
explicit retrieval scope
```

## 2.7 Entity-neutral retrieval policy

The core library must not hard-code special retrieval behavior for user, assistant, player, protagonist, NPC, or any application-specific entity role.

Entity-based retrieval policy should depend on:

```text
observed graph structure
relation type
object type
lifecycle/currentness
time
salience
retrieval scope
supporting evidence
```

The core library may expose hooks for applications to provide scope, actor identity, or domain-specific policy, but the base schema and retrieval policy should remain use-case agnostic.

## 2.8 Derived stats are not graph truth

Retrieval statistics may guide fanout policy, but they are not authoritative for:

```text
memory existence
relationships
provenance
lifecycle
currentness
final context inclusion
```

The authority split remains:

```text
Qdrant   suggests vector candidates.
Stats    guide fanout policy.
Oxigraph decides graph truth and final inclusion.
```

Stats must remain rebuildable from graph authority.

## 2.9 Recurring entities are anchors, not traversal invitations

Entities are central to continuity, but a recurring entity with many incident memories should not trigger unbounded expansion.

The broader an entity's graph footprint becomes under a relation, the more retrieval should require additional narrowing evidence.

High degree affects expansion policy. It does not mean the entity is unimportant.

## 2.10 Low-information co-occurrence is not enough for durable links

Durable pairwise memory links should not be created solely because two memories share a low-selectivity entity or broad relation.

Durable association requires stronger evidence, rationale, or explicit application intent.

## 2.11 Weak associations are recall evidence before durable relation truth

The system should support serendipitous recall, but weak co-occurrence should not be promoted directly into ordinary durable pairwise memory links.

The library should distinguish:

```text
entity incidence
query-time activation
association candidate evidence
active associative unit
strong durable relation
```

Intent:

Preserve human-like "this reminds me of that" recall without letting recurring entities create noisy graph cliques or false continuity.

v0.5 implements this through controlled associative recall, graph-internal associative units, member-level lifecycle, association support evidence, and bounded expansion.

## 2.12 Generated and manual writes should share one safe path

Manual caller-provided writes and future generated memory candidates should pass through the same validation and commit machinery.

The library should not grow a separate unsafe path where generated memory candidates can bypass provenance, lifecycle, retention, currentness, graph-authority validation, or idempotency checks.

```text
manual input
  -> MemoryCandidate / RememberWritePlan
  -> validation
  -> commit

future generated input
  -> MemoryCandidate / RememberWritePlan
  -> validation
  -> commit
```

Future assisted generation should improve usability without weakening Character Memory invariants.

v0.6 generated processors plug into the existing write-plan path rather than inventing a parallel persistence pipeline.

## 2.13 Core stores curated memory and opaque source provenance, not raw logs

Character Memory core stores curated memory objects and provenance handles.

Core memory objects include:

```text
Episode
Observation
Entity
MemoryThread
DerivedMemory
MemoryLink
ContinuityContextPack inputs
currentness/lifecycle state
provenance links
source references and source spans
```

Core memory storage does not include:

```text
raw conversation-log storage
raw transcript storage
verbose tool-output storage
raw file/blob storage
raw image/audio/video storage
raw sensor-log storage
raw-log search
public raw-reference resolution
```

`raw_ref` and source-span fields are opaque provenance handles. They identify caller-managed source material but are not themselves raw source storage.

Assisted remember workflows may accept raw or semi-raw input as transient processing input. They produce validated candidates and write plans; they do not persist the raw input.

---

# 3. Version overview

| Version | Theme | Outcome |
|---|---|---|
| v0.1 | Starter episodic memory | Finished. Public graph-authoritative memory substrate with episodes, observations, entities, soft threads, derived memories, lifecycle facades, and continuity retrieval. |
| v0.1 backend | Storage contracts | Finished. Qdrant candidate recall, Oxigraph graph authority, stable IDs, vector metadata hints, graph triples, schema versions, bounded expansion support, and tests. |
| v0.1.1 | Persistent graph authority | Finished. Durable Oxigraph-backed graph authority, restart-safe retrieval, Qdrant/Oxigraph reconciliation, and persistence validation. |
| v0.1.2 | Continuous entity selectivity and retrieval guardrails | New. Use-case-agnostic guardrails for high-degree or low-selectivity entities, persistent retrieval statistics, continuous selectivity scoring, relation-specific fanout control, low-information co-occurrence prevention, and diagnostics. |
| v0.1.3 | Remember intake interfaces and deterministic write planning | Generation-ready write path with `RememberWritePlan`, memory candidates, validation, deterministic helpers, draft/validate/commit flow, and shared manual/future-generated commit machinery. |
| v0.2 | Scoped continuity and reflection | `ContinuityScope`, scoped reflection, relationship state between arbitrary entities, character signals for continuing entities, open-loop/commitment lifecycle, and current continuity views. |
| v0.3 | Factual rigor, temporal validity, and entity evolution | Assertions, claims, evidence links, belief assessments, source assessment, temporal validity, entity drift handling, and current-belief views. |
| v0.4 | Retrieval observability and governance | Retrieval traces, context subgraphs, validation rules, graph health reports, policy diagnostics, rejected expansion traces, cluster/activation diagnostics, and retention assessment. |
| v0.5 | Controlled associative recall and clustering | Query-time associative activation, graph-internal AssociativeUnit structures, member-level AssociativeMembership lifecycle, AssociationSupport evidence, cluster summaries, promotion/decay policy, and bounded cluster expansion for serendipitous recall without broad pairwise edge pollution. |
| v0.6 | Assisted remember workflow and memory candidate generation | Model/rule-assisted generation of memory candidates from caller-provided transient conversation, transcript-like, or structured interaction input, using the v0.1.3 write-plan path and later retrieval/governance safeguards. Raw input is not persisted by Character Memory core. |
| v1.0+ | Multimodal and embodied expansion | Voice beyond transcript, multimodal observations, situation frames, object/place/action memory, and embodied context through symbolic memory objects and opaque external source references. Raw media and sensor logs are not stored by Character Memory core. |

Revisit the split if v0.4 becomes too small after implementation planning, or if advanced association work becomes necessary before full governance. Default preference should remain: observability/governance before advanced association, because association features can create new edges and should be built after the system can explain and validate retrieval behavior.

---

# 4. Phase 0: repository and architecture foundation

## Intent

Set up the project so v0.1 can remain lean but future versions do not require breaking the memory structure.

## Deliverables

```text
Core model package
Storage interfaces
Default Qdrant candidate-recall adapter
Default Oxigraph graph-authority adapter
Opaque source-reference and source-span utilities
Schema/versioning utilities
Stable ID/IRI utilities
Test fixtures
Migration hooks
```

Phase 0 does not implement raw-log storage, raw-log search, or public raw-reference resolution. Source-reference utilities represent caller-managed source material for provenance.

## Implemented module layout

```text
src/
  lib.rs
  api.rs
  api/
    embedding.rs
    types.rs
    types/
      domain.rs
      draft.rs
      lifecycle.rs
      retrieval.rs
  config.rs
  config/
    settings.rs
    settings/
      app_settings.rs
  errors.rs
  errors/
    custom.rs
  internal.rs
  internal/
    config.rs
    config/
      settings.rs
      settings/
        embedding_provider_settings.rs
    infrastructures.rs
    infrastructures/
      external_services.rs
      external_services/
        openai_embedding_provider.rs
        qdrant_payload.rs
        qdrant_vector_candidate_store.rs
      graph.rs
      graph/
        oxigraph_authority_store.rs
        rdf_mapping.rs
        sparql_selectors.rs
        vocabulary.rs
    models.rs
    models/
      vector.rs
      vector/
        candidate_record.rs
        embedding_model.rs
        embedding_surface.rs
        record.rs
    repositories.rs
    repositories/
      correction_forget_pipeline.rs
      embedder.rs
      graph_authority_store.rs
      link_pipeline.rs
      source_reference.rs
      reconciliation.rs
      remember_pipeline.rs
      retrieve_pipeline.rs
      test_support.rs
      vector_candidate_store.rs
    schema.rs
tests/
```

## Design boundary

The core library should not be a full agent framework.

But it may define processor interfaces for:

```text
entity extraction
salience scoring
summarization
reflection
thread linking
correction detection
```

Concrete LLM providers should be adapters or examples, not hard dependencies.

---

# 5. v0.1: starter episodic memory

Detailed draft: [`v0_1_starter_episodic_memory.md`](../design/roadmap-phases/v0_1_starter_episodic_memory.md)

## Core concepts

```text
Episode
Observation
Entity
MemoryThread
DerivedMemory
MemoryLink
ContinuityContextPack
```

## Goals

```text
remember chat sessions or meaningful segments
extract salient observations
link entities and soft threads
store derived memories with provenance
retrieve continuity context instead of generic top-k snippets
support basic correction, supersession, and suppression
```

## Acceptance criteria

```text
Episodes can be stored and retrieved by ID.
Derived memories trace back to source episodes/observations.
Thread membership is optional and confidence-scored.
Retrieval returns a ContinuityContextPack with rationale.
Suppressed memories are not used for generation.
Corrections can supersede older derived memories.
```

---

# 6. v0.1 backend contracts

Detailed draft: [`v0_1_storage_and_backend_contracts.md`](../design/roadmap-phases/v0_1_storage_and_backend_contracts.md)

## Goals

Backend storage work should preserve the engineering discipline that matters for a durable memory substrate:

```text
shared IDs
Qdrant payload/index conventions
Oxigraph IRI/triple conventions
schema versioning
regression tests
bounded graph expansion
backend abstraction
```

## Acceptance criteria

```text
same object can be joined across source references, vector candidates, and graph authority
Qdrant filters work for object_type/record_type, entity_ids, thread_ids, time, currentness, retention
Oxigraph graph queries return episode/entity/thread/provenance context
retrieval behavior is deterministic under fixed fixtures
```

---

# 7. v0.1.1: persistent graph authority

Detailed draft: [`v0_1_1_persistent_graph_authority.md`](../design/roadmap-phases/v0_1_1_persistent_graph_authority.md)

## Intent

Make the v0.1 graph-authoritative architecture durable across process restarts before adding richer continuity and reflection features.

This phase closes the gap where Qdrant candidates may survive restart while the Oxigraph authority required to validate provenance, lifecycle, currentness, supersession, and links may not.

## Goals

```text
support Docker-backed Oxigraph service configuration and embedded persistent storage configuration
preserve graph-authoritative state across process restarts
keep in-memory graph mode available for deterministic tests
validate restart-safe retrieval
detect Qdrant/Oxigraph drift
prevent vector-only candidates from becoming behavior-influencing memory
document persistence configuration and operational expectations
```

Oxigraph service mode is the application default.

Embedded persistent graph mode is explicit through `GRAPH_STORE_MODE=persistent`; in-memory graph mode is reserved for tests and explicit fixture runs through `GRAPH_STORE_MODE=in_memory`.

## Non-goals

```text
new memory object types
relationship-state model
character-signal reinforcement
reflection scheduler
separate Assertion / Claim / EvidenceLink / BeliefAssessment classes
advanced association graph
multimodal observation model
distributed transactions across Qdrant and Oxigraph
```

## Deliverables

```text
configurable Oxigraph graph store mode
service-backed and embedded persistent Oxigraph graph authority implementation
restart-safe graph authority tests
retrieval behavior tests after graph restart
Qdrant/Oxigraph reconciliation diagnostics
partial-persistence visibility gates
documentation for persistent graph setup
```

## Acceptance criteria

```text
Persistent graph mode can be configured.
In-memory graph mode remains available.
Objects, links, provenance, suppression, supersession, and currentness survive graph store restart.
Currentness filtering works after restart.
Retrieval after restart excludes suppressed, deleted, non-current, and superseded records by default.
Qdrant candidates whose graph objects are missing are rejected from normal retrieval.
Reconciliation diagnostics can report vector-only and graph-only drift.
Stable object ID to graph IRI mapping remains unchanged.
Existing v0.1 public APIs continue to work.
```

---

# 8. v0.1.2: continuous entity selectivity and retrieval guardrails

Detailed draft: [`v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`](../design/roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md)

## Intent

Harden retrieval against high-degree recurring entities without baking in assumptions about which entities matter in a particular application.

Any entity may become broad over time:

```text
person
character
place
project
topic
organization
object
faction
scene
conversation partner
domain-specific concept
```

The retrieval layer should adapt to the graph's accumulated structure instead of relying on hard-coded entity roles.

## Key design principle

```text
All entities start equal.
Retrieval adapts to observed graph structure.
High degree affects expansion policy, not entity importance.
```

A high-degree entity may still be central and highly relevant. It should not be globally penalized as unimportant. Instead, low selectivity should mean:

```text
Do not expand broadly from this entity unless additional retrieval evidence supports it.
```

Supporting evidence may include:

```text
semantic similarity
thread membership
temporal relevance
salience
currentness
correction/supersession relevance
explicit retrieval scope
application-provided scope
```

## Goals

```text
treat all entities equally at schema level
persist lightweight retrieval statistics across app restarts
compute continuous relation-specific selectivity scores from counters
use selectivity scores to control graph expansion fanout
prevent durable pairwise links from weak low-information co-occurrence
preserve Oxigraph graph authority for final inclusion
keep Qdrant relationship/lifecycle fields as hints only
add diagnostics showing selectivity inputs and fanout decisions
add tests proving no entity identity is special-cased
```

## Serendipitous recall tradeoff

v0.1.2 blocks durable pairwise links created only from weak low-selectivity co-occurrence. This protects the graph from hub-driven pairwise growth, false continuity, and context pollution.

This is an accepted temporary tradeoff, not a dismissal of human-like associative recall.

The system should preserve:

```text
entity incidence
semantic retrieval
temporal retrieval
thread retrieval
salience retrieval
explicit links
correction/supersession/provenance links
```

while preventing:

```text
Episode A --associated_with--> Episode B
```

when the only evidence is:

```text
both episodes share a broad low-selectivity entity or relation.
```

Later associative recall should reintroduce controlled serendipity through query-time activation, graph-internal associative units, member-level lifecycle, association support evidence, and cluster summaries.

The intended tradeoff is:

```text
Prefer missing weak serendipity temporarily
over creating durable false continuity permanently.
```

## Weak co-occurrence is not durable association

Weak co-occurrence may be recorded or diagnosed as retrieval evidence, but it should not be represented as an ordinary durable pairwise memory association.

The following must not create a durable pairwise association by itself:

```text
same broad entity
same common place
same high-degree project
same recurring participant
same broad topic
same low-selectivity relation
```

Durable association requires stronger evidence, such as:

```text
same active thread
explicit application-created link
semantic similarity
temporal pattern
causal relation
correction/supersession relation
commitment lifecycle relation
shared high-selectivity cue
repeated coactivation
reflection-derived rationale
high salience with topical support
```

## Non-goals

```text
hard-coded user/assistant/protagonist/player/NPC behavior
persisted selectivity categories
NoSQL service
mandatory Postgres service
graph centrality algorithms
PageRank-like memory importance
learned retrieval policy
full retrieval trace object
admin dashboard
episode clustering
advanced association graph
automatic retention optimization
migration/backfill for existing production data
```

## Acceptance criteria

```text
Stats survive app restart.
Normal retrieval does not scan the whole graph to classify entity selectivity.
Selectivity is computed continuously from counters.
Selectivity labels are diagnostic only.
Fanout budgets are smooth functions of selectivity, relation kind, object type, and supporting evidence.
No retrieval rule depends on entity name, canonical key, or application role.
High-degree entities require additional narrowing evidence for broad expansion.
High-degree entities can still contribute when supported by semantic, temporal, thread, salience, currentness, correction, or explicit scope evidence.
Durable pairwise links are not created solely from shared low-selectivity entity co-occurrence.
Qdrant relationship hints remain non-authoritative.
Oxigraph remains authoritative for graph truth, lifecycle, currentness, provenance, and expansion context.
Missing or unhealthy stats produce conservative fanout.
Synthetic high-degree fixtures cover people, places, projects, topics, objects, and arbitrary custom entities.
```

---

# 9. v0.1.3: remember intake interfaces and deterministic write planning

Detailed draft: [`v0_1_3_remember_intake_interfaces_deterministic_write_planning.md`](../design/roadmap-phases/v0_1_3_remember_intake_interfaces_deterministic_write_planning.md)

## Intent

Prepare the memory write path for future assisted generation without implementing model-assisted extraction yet.

The library should support a common flow:

```text
candidate objects
  -> validation
  -> write plan
  -> commit
```

This flow should be usable by manual caller-provided writes today and by future generated memory candidates later.

The phase should make the write path generation-ready, but it should not infer high-level memory meaning from raw natural language.

## Why this comes after v0.1.2

v0.1.2 adds selectivity and retrieval guardrails for high-degree or low-selectivity entities. That should come before easier intake APIs because better intake can increase memory volume.

The safer sequence is:

```text
first: retrieval guardrails
then: easier intake/write planning
then: scoped continuity/reflection
```

v0.1.3 should therefore introduce a safer write-planning surface only after the retrieval layer has basic protection against fanout, weak co-occurrence, and context pollution.

## Why this comes before v0.2

v0.2 introduces stronger continuity concepts such as scoped reflection, relationship state, character signals, commitments, open loops, and current continuity views.

Those features should eventually be generated or updated through a safe write path. v0.1.3 establishes that path before the library starts creating richer continuity structures.

## Core distinction

This phase is not the full assisted generation workflow.

```text
v0.1.3:
  package, validate, and commit caller-provided or deterministic memory candidates

v0.6:
  generate memory candidates from raw conversation/transcript-like input
```

v0.1.3 should not infer:

```text
this is a user preference
this is a commitment
this is a correction
this is a character signal
this text mentions entity X
this episode belongs to thread Y
```

unless the caller supplied that information.

## New concepts

```text
RememberInput
RememberWritePlan
MemoryCandidate
CandidateValidation
CandidateProvenance
RememberOutcome
RememberDiagnostics
```

These concepts support future generation without requiring generation now.

## Goals

```text
introduce RememberWritePlan
introduce MemoryCandidate types for planned writes
support prepare / validate / commit workflow
keep remember() as a convenience wrapper
add deterministic helpers for stable IDs, graph IRIs, source references, source spans, lifecycle defaults, and provenance links
allow callers to provide structured hints such as entity IDs, thread IDs, scope IDs, participants, timestamps, raw references, and source spans
validate behavior-influencing DerivedMemory provenance before commit
validate MemoryLink targets before commit
make manual writes and future generated writes share the same validation and commit path
preserve Oxigraph as graph authority
preserve Qdrant as vector candidate recall only
preserve RetrievalStatsStore as derived selectivity/fanout metadata only
```

## Non-goals

Do not implement in v0.1.3:

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
```

This phase should remain deterministic and schema-oriented.

## Write workflow

The core workflow should be:

```text
prepare
  -> validate
    -> commit
```

`remember()` should remain available as a convenience wrapper around those steps.

```text
remember(input)
  = prepare(input)
  + validate_plan(plan)
  + commit(plan)
```

## API direction

Suggested public or semi-public API shape:

```rust
let plan = memory.prepare(input, prepare_options).await?;
let validation = memory.validate_plan(&plan).await?;
let outcome = memory.commit(plan, commit_options).await?;
```

Convenience path:

```rust
let outcome = memory.remember(input, remember_options).await?;
```

`commit()` should always revalidate, because graph state may have changed after `prepare()`.

## Commit and review model

Do not introduce many commit modes.

Avoid first-class modes such as:

```text
DraftOnly
ValidateOnly
RequireApproval
ApplicationReviewCallback
AutoCommitSafeCandidates
```

Instead, use explicit workflow operations:

```text
DraftOnly      = prepare()
ValidateOnly   = validate(plan)
Commit         = commit(plan)
RequireApproval = prepare() + app-owned approval + commit(approved_plan)
ApplicationReviewCallback = optional future adapter, not v0.1.3 core
AutoCommitSafeCandidates = future admission policy for generated candidates, not v0.1.3 core
```

The only true commit operation is `commit(plan)`.

Review is application workflow, not a primitive commit mode.

## Deterministic helpers

v0.1.3 may implement deterministic helpers for:

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

## RememberWritePlan contents

A `RememberWritePlan` should be explicit and inspectable.

It should be able to contain:

```text
operation ID
idempotency key
source input reference
episode candidates
observation candidates
entity candidates or entity references
memory thread references or candidates
derived memory candidates
memory link candidates
vector index candidates
retrieval stats update candidates
validation results
diagnostics
```

The plan should make it possible for an application or test to inspect what would be written before anything is persisted.

## Candidate provenance

Every candidate that could later influence behavior should carry provenance.

For v0.1.3, provenance may come from caller-provided source references or spans.

Examples:

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

Behavior-influencing `DerivedMemory` candidates must have provenance to an `Episode` or `Observation`.

## Candidate origin metadata

v0.1.3 adds narrow origin metadata to `CandidateProvenance` so future generated candidates can share the same write path as manual candidates without conflating caller-supplied rationale with processor-inferred rationale.

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

These fields are write-time provenance. They do not introduce a generic `MetaMemory` object and do not add generic confidence, generic assumptions, generic alternatives, generic context edges, or durable retrieval reasons.

## Validation rules

Validation should check at least:

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
```

Invalid plans should not commit.

## Persistence failure policy

v0.1.3 should continue the existing authority split:

```text
Qdrant suggests.
Stats guide fanout.
Oxigraph decides.
```

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

`commit()` should distinguish critical failure from repairable degraded state. It should not allow behavior-influencing ungrounded memory.

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
```

## v0.6 integration path

v0.6 model-assisted processors produce `MemoryCandidate` and `RememberWritePlan` values rather than bypassing the validation and commit path.

The v0.6 work owns generated-candidate admission states such as:

```text
Accepted
Deferred
NeedsReview
Rejected
Invalid
```

v0.1.3 keeps candidate state simpler unless implementation clearly requires more.

---

# 10. v0.2: scoped continuity and reflection

Detailed draft: [`v0_2_scoped_continuity_reflection.md`](../design/roadmap-phases/v0_2_scoped_continuity_reflection.md)

## New concepts

```text
ContinuityScope
ReflectionJob
RelationshipState
CharacterSignal
OpenLoop
Commitment
CurrentContinuityView
```

## Goals

```text
make memory shape future behavior more explicitly
track active commitments and unresolved scoped matters
derive relationship/project/entity-specific character signals
separate current continuity context from raw historical memories
avoid assuming continuity is centered on one user-assistant relationship
```

## Acceptance criteria additions

```text
Reflection jobs require explicit or inferred ContinuityScope.
CurrentContinuityView is generated for a scope.
RelationshipState can describe arbitrary entity relationships.
CharacterSignal can attach to any continuing entity or scope.
Reflection avoids all-history scans through broad entities.
Open loops and commitments can be retrieved by scope without assuming who the main actor is.
```

---

# 11. v0.3: factual rigor, temporal validity, and entity evolution

Detailed draft: [`v0_3_factual_rigor_temporal_validity_entity_evolution.md`](../design/roadmap-phases/v0_3_factual_rigor_temporal_validity_entity_evolution.md)

## New concepts

```text
Assertion
Claim
EvidenceLink
BeliefAssessment
SourceAssessment
TemporalValidity
EntityStateHistory
CurrentBeliefView
```

## Goals

```text
distinguish source reports from truth
support contradictions and updates
track temporal validity and volatility
represent entity drift over time
show why factual beliefs are accepted or rejected
```

This is important, but it should not block the starter because Character Memory's first value is continuity, not full truth maintenance.

---

# 12. v0.4: retrieval observability and governance

Detailed draft: [`v0_4_retrieval_observability_governance.md`](../design/roadmap-phases/v0_4_retrieval_observability_governance.md)

## New concepts

```text
RetrievalTrace
ActivationTrace
RejectedExpansionTrace
ClusterExpansionTrace
MembershipDecisionTrace
AssociationCandidateDiagnostic
CoactivationDiagnostic
ContextSubgraph
ValidationRules
GraphHealthReport
RetentionAssessment
PolicyDiagnostics
```

## Goals

```text
make retrieval decisions inspectable
show bounded expansion paths
validate graph/retrieval invariants
detect high-fanout relation patterns
evaluate retention/downranking candidates
report policy behavior over time
make rejected low-information expansions inspectable
show why broad-entity-only expansion was blocked
show activation paths used during retrieval
show when weak coactivation was considered but not persisted
show cluster membership inclusion/exclusion rationale
diagnose candidate membership promotion, demotion, decay, or rejection
detect over-broad clusters and high-fanout cluster expansions
```

## Additional acceptance criteria

```text
RetrievalTrace can explain why broad entity expansion was limited.
RetrievalTrace can distinguish strong association, candidate association, and ordinary entity incidence.
ActivationTrace can show which cues activated which entities, concepts, scopes, threads, or associative units.
RejectedExpansionTrace records when a low-selectivity entity match was insufficient for expansion.
ClusterExpansionTrace records which AssociativeUnit was used and which memberships were included, excluded, or considered.
MembershipDecisionTrace records member status, role, strength, and rationale used during retrieval.
GraphHealthReport can identify clusters with excessive candidate members, stale memberships, or high expansion fanout.
Diagnostics remain report-only and do not override Oxigraph lifecycle/currentness/provenance authority.
```

## Additional non-goal

v0.4 should not implement the associative cluster machinery itself. It should make retrieval decisions and blocked expansions observable so v0.5 can safely add controlled associative recall.

## Retrieval intent

v0.4 adds query-time retrieval intent as part of retrieval governance.

Planned shape:

```rust
enum RetrievalIntent {
    Continuity,
    CurrentState,
    CorrectionReview,
    SourceAudit,
    AssociativeProbe,
}
```

`RetrievalIntent` is an input to retrieval policy. It is not persisted on memory objects.

The default intent is `Continuity`.

`SourceAudit` returns provenance paths and source-reference metadata. It does not resolve or search raw logs.

`AssociativeProbe` exposes weak activation and association diagnostics. It does not automatically promote weak associations to durable graph truth.

---

# 13. v0.5: controlled associative recall and clustering

Detailed draft: [`v0_5_controlled_associative_recall_clustering.md`](../design/roadmap-phases/v0_5_controlled_associative_recall_clustering.md)

## New concepts

```text
AssociativeUnit
AssociativeMembership
AssociationSupport
QueryTimeActivation
AssociationPromotionPolicy
AssociationDecayPolicy
ClusterSummary
```

## Goals

```text
support human-like serendipitous recall
avoid broad-entity clique growth
represent associative structures inside graph authority
track member-level status, role, strength, and rationale
support query-time activation before durable association
promote associations only with repeated or multi-signal support
use summaries and exemplars for retrieval quality
keep cluster expansion bounded and explainable
```

## Association support over durable association scores

v0.5 persists association structure and support evidence.

Persisted graph concepts:

```text
AssociativeUnit
AssociativeMembership
AssociativeMembership.status
AssociativeMembership.role, when needed
AssociationSupport
AssociationSupport.support_type
AssociationSupport.support_source_id
AssociationSupport.created_at
```

Derived or rebuildable values:

```text
membership_strength
membership_confidence
membership_salience
supporting_signal_count
last_reinforced_at
activation score
review priority
```

Durable graph truth is the associative unit, membership lifecycle, and support evidence. Retrieval-time and maintenance-time policy compute scores from that evidence.

---

# 14. v0.6: assisted remember workflow and memory candidate generation

Detailed draft: [`v0_6_assisted_remember_workflow_memory_candidate_generation.md`](../design/roadmap-phases/v0_6_assisted_remember_workflow_memory_candidate_generation.md)

## Intent

Let callers provide bounded raw, transcript-like, or structured interaction input transiently to `remember()`, while the library generates validated memory candidates and write plans.

The caller still decides:

```text
when to call remember()
what input to offer
what processing policy to use
whether generated candidates are committed, reviewed, or discarded
where source material is retained outside Character Memory, if retained
```

The library helps decide:

```text
how offered experience becomes memory candidates
how candidates are validated
how candidates preserve provenance
how accepted candidates are committed
```

The library does not persist the raw input.

## Why this comes later

Assisted generation should wait until the memory substrate has stronger retrieval quality, scope handling, factual rigor, observability, governance, and association/clustering behavior.

The generation workflow will be shaped by what the library can store and how retrieval behaves. Implementing it too early risks generating plausible-looking memory objects that degrade continuity.

## Dependency on v0.1.3

v0.6 should use the v0.1.3 write-plan path.

Generated processors should produce:

```text
MemoryCandidate
RememberWritePlan
CandidateProvenance
RememberDiagnostics
```

They should not bypass validation or commit directly to stores.

## Possible generated candidates

```text
Episode candidates
Observation candidates
Entity candidates
Thread/scope link candidates
DerivedMemory candidates
salience/admission candidates
natural embedding surfaces
memory link candidates
```

## Non-goals

Do not make v0.6 an autonomous memory agent that scans logs without caller intent.

The caller should still control:

```text
when raw input is offered
which raw input is offered
which processors are enabled
what privacy policy applies
whether candidates require review
```

## Acceptance criteria

```text
Caller can pass raw chat/transcript-like input and receive a RememberWritePlan.
Generated DerivedMemory candidates include provenance.
Explicit corrections generate correction candidates.
Explicit commitments generate commitment/open-loop candidates.
Entity candidates are resolved through graph authority rather than direct model-minted IDs.
Thread/scope links are optional and confidence-scored.
Embedding text is natural language, not metadata dumps.
Generation diagnostics expose accepted, rejected, and deferred candidates.
Privacy exclusions are applied before external processor calls.
Generated candidates use the same validation and commit path as manual candidates.
```

---

# 15. v1.0+: multimodal and embodied expansion

Detailed draft: [`v1_0_multimodal_embodied_expansion.md`](../design/roadmap-phases/v1_0_multimodal_embodied_expansion.md)

## New concepts

```text
SituationFrame
MultimodalObservation
ObjectMemory
PlaceMemory
ActionTrace
OutcomeObservation
```

## Goals

```text
support voice beyond transcripts
support image/video/screen observations
support object/place/action memory
support embodied context when practical
```

This is a future path, not starter scope.

---

# 16. Public API evolution

## v0.1 API

```rust
let memory = CharacterMemory::new(settings, collection_name).await?;
let stored = memory.remember(remember_draft).await?;
let context = memory.retrieve(retrieval_context).await?;
let correction = memory.correct(correct_memory_draft).await?;
let forget = memory.forget(forget_memory_draft).await?;
let link = memory.link(memory_link_draft).await?;
```

## v0.1.2 configuration / internal additions

v0.1.2 should not require a new public memory facade. It adds retrieval hardening through configuration and internal stores.

Conceptual configuration:

```toml
[retrieval.stats]
store = "sqlite"
path = "./data/character-memory/retrieval_stats.sqlite"
health_fail_mode = "conservative"

[retrieval.selectivity]
smoothing_alpha = 1.0
gamma = 1.0

[retrieval.fanout.about_entity.derived_memory]
min = 0
max = 20

[retrieval.fanout.participant_entity.episode]
min = 0
max = 5

[retrieval.fanout.part_of_thread.derived_memory]
min = 0
max = 15
```

## v0.1.3 API additions

v0.1.3 introduces an explicit write-planning workflow.

```rust
let plan = memory.prepare(input, prepare_options).await?;
let validation = memory.validate_plan(&plan).await?;
let outcome = memory.commit(plan, commit_options).await?;
```

The existing `remember()` API remains the convenience path:

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

The purpose is to let manual writes and future generated writes share the same validation and commit path.

Application-owned approval flows should compose these primitives:

```rust
let plan = memory.prepare(input, prepare_options).await?;

// Application reviews, edits, or filters the plan.
let approved_plan = app_review(plan).await?;

let outcome = memory.commit(approved_plan, commit_options).await?;
```

`RequireApproval` and `ApplicationReviewCallback` are not core v0.1.3 commit modes. They are application workflows or future adapters.

## v0.2 API additions

```rust
let reflection_scope: Option<ContinuityScope> = None;
memory.reflect(reflection_scope).await?;

let signal: Option<ReinforcementSignal> = None;
memory.reinforce(target_id, signal).await?;

let continuity_scope: Option<ContinuityScope> = None;
let open_loops = memory.get_open_loops(continuity_scope).await?;
let commitments = memory.get_commitments(continuity_scope).await?;

let evidence: Option<EvidenceInput> = None;
memory.resolve_commitment(commitment_id, evidence).await?;
```

## v0.3 API additions

```rust
let assessment = memory.assess_claim(claim_id).await?;

let belief_scope: Option<ContinuityScope> = None;
let beliefs = memory.get_current_beliefs(belief_scope).await?;

let reviewed = memory.review_stale_beliefs().await?;
```

## v0.4 API additions

```rust
let explanation = memory.explain_retrieval(trace_id).await?;
let report = memory.graph_health_report(scope).await?;
let validation = memory.validate_graph(scope).await?;
let context_subgraph = memory.get_context_subgraph(context).await?;

let retention_scope: Option<ContinuityScope> = None;
let retention_result = memory.apply_retention_policy(retention_scope).await?;
```

## v0.5 API additions

```rust
let activation = memory.activate_associative_recall(query, activation_options).await?;
let unit = memory.get_associative_unit(unit_id).await?;
let cluster_context = memory.retrieve_associative_context(unit_id, retrieval_mode).await?;
```

## v0.6 API additions

```rust
let plan = memory.prepare(raw_interaction_input, generation_options).await?;
let validation = memory.validate_plan(&plan).await?;
let outcome = memory.commit(plan, commit_options).await?;
```

Generated processors should produce `MemoryCandidate` and `RememberWritePlan` values rather than bypassing validation or committing directly to stores.

---

# 17. YAGNI rules

Do not implement in v0.1 / v0.1.2:

```text
hard-coded entity role treatment
persisted selectivity categories
learned retrieval policy
graph centrality algorithms
true hypergraphs
full OWL reasoning
continuous multimodal segmentation
robotic situation frames
full evidence-backed belief subsystem
normalized belief ontology
source reliability scoring
complex spreading activation
reflection scheduler
raw transcript storage in graph/vector stores
physical redaction/delete as the default lifecycle path
admin dashboard
analytics-heavy stats system
migration/backfill for nonexistent production data
```

Do not implement before v0.5:

```text
ordinary low-value pairwise association edges
cluster-level status as a substitute for membership-level lifecycle
unbounded spreading activation
global graph centrality as memory importance
summary-only associative clusters by default
automatic clique creation around recurring entities
```

Do design for:

```text
controlled serendipitous recall
query-time activation
graph-internal associative units
member-level association lifecycle
association support evidence
bounded cluster expansion
promotion/decay of candidate memberships
```

## v0.1.3 YAGNI rules

Do not implement in v0.1.3:

```text
LLM-based summarization
automatic observation extraction
automatic entity extraction
automatic entity resolution from natural language
automatic thread or scope inference
automatic preference extraction
automatic commitment/open-loop detection
automatic correction detection
automatic character-signal generation
model-assisted salience scoring
learned admission policy
application review callback framework
full assisted remember workflow
raw audio/video processing
```

Do design for:

```text
RememberWritePlan
MemoryCandidate
CandidateProvenance
CandidateValidation
RememberDiagnostics
prepare / validate / commit workflow
manual and future-generated writes sharing the same commit path
deterministic source spans and source references
idempotent retry-safe writes
validation before behavior-influencing persistence
```

The principle is:

```text
Build the path that generated memories will travel later.
Do not build the generator yet.
```

Do design for:

```text
stable IDs
extensible object types
typed links
raw_ref pointers
schema versions
provenance links
modality fields
backend adapters
retrieval stats rebuildable from graph authority
entity-neutral retrieval policy
scope-aware future continuity
```

This keeps the starter small while avoiding structural dead ends.
