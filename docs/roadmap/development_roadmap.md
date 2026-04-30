# Character Memory Development Roadmap

## One-line thesis

```text
Character Memory is an episode-backed continuity substrate for persistent AI assistants.
```

The roadmap should build the system in layers. The starter must be useful for chat-native memory without pretending to support every future modality or every epistemic feature.

---

# 1. Design north star

A persona can be assigned in a prompt. Character is accumulated through remembered experience.

Therefore the system should optimize for:

```text
temporal continuity
relationship continuity
project/thread continuity
correction and revision
retrieval rationale
provenance from derived memory back to episodes
```

The design should not be evaluated only by top-k retrieval quality. It should be evaluated by whether the assistant can behave as the same continuing participant over time.

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
current preferences
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
same entity
recent event
open commitment
preference relevance
correction relevance
high salience
```

---

# 3. Version overview

| Version | Theme | Outcome |
|---|---|---|
| v0.1 | Starter episodic memory | Public graph-authoritative memory substrate with episodes, observations, entities, soft threads, derived memories, lifecycle facades, and continuity retrieval. |
| v0.1 backend | Storage contracts | Qdrant candidate recall, Oxigraph graph authority, stable IDs, vector metadata hints, graph triples, schema versions, and tests. |
| v0.1.1 | Persistent graph authority | Durable Oxigraph-backed graph authority, restart-safe retrieval, Qdrant/Oxigraph reconciliation, and persistence validation before adding richer continuity/reflection features. |
| v0.2 | Continuity and reflection | Relationship state, character signals, open-loop/commitment lifecycle, scheduled reflection, current continuity views. |
| v0.3 | Factual rigor | Assertions, claims, evidence links, belief assessments, source assessment, temporal validity, current-belief view. |
| v0.4 | Advanced recall and governance | Associations, episode clusters, retention governance, retrieval traces, validation, context subgraph construction. |
| v1.0+ | Multimodal and embodied expansion | Voice beyond transcript, multimodal observations, situation frames, object/place/action memory. |

---

# 4. Phase 0: repository and architecture foundation

## Intent

Set up the project so v0.1 can remain lean but future versions do not require breaking the memory structure.

## Deliverables

```text
Core model package
Storage interfaces
Default Qdrant candidate-recall adapter
Default embedded Oxigraph graph-authority adapter
Raw store interface
Schema/versioning utilities
Stable ID/IRI utilities
Test fixtures
Migration hooks
```

## Suggested modules

```text
src/
  lib.rs
  api/
    mod.rs
    embedding.rs
    types/
      domain.rs
      draft.rs
      lifecycle.rs
      retrieval.rs
  internal/
    models/
      vector/
        mod.rs
        candidate_record.rs
        embedding_model.rs
        embedding_surface.rs
        record.rs
    repositories/
      graph_authority_store.rs
      remember_pipeline.rs
      link_pipeline.rs
      retrieve_pipeline.rs
      correction_forget_pipeline.rs
      vector_candidate_store.rs
      embedder.rs
      raw_reference_resolver.rs
      mod.rs
    infrastructures/
      external_services/
        mod.rs
        qdrant_payload.rs
        qdrant_vector_candidate_store.rs
        openai_embedding_provider.rs
      graph/
        mod.rs
        rdf_mapping.rs
        vocabulary.rs
        oxigraph_authority_store.rs
      mod.rs
  repositories.rs
  models.rs
  config/
    settings.rs
    settings/
      app_settings.rs
  errors.rs
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

Detailed draft: [`v0_1_1_persistent_graph_authority`](../design/roadmap-phases/v0_1_1_persistent_graph_authority.md)

## Intent

Make the v0.1 graph-authoritative architecture durable across process restarts before adding richer continuity and reflection features.

This phase closes the gap where Qdrant candidates may survive restart while the Oxigraph authority required to validate provenance, lifecycle, currentness, supersession, and links may not.

## Goals

```text
support persistent Oxigraph storage configuration
preserve graph-authoritative state across process restarts
keep in-memory graph mode available for deterministic tests
validate restart-safe retrieval
detect Qdrant/Oxigraph drift
prevent vector-only candidates from becoming behavior-influencing memory
document persistence configuration and operational expectations
```

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
persistent Oxigraph graph authority implementation
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

# 8. v0.2: continuity and reflection

Detailed draft: [`v0_2_continuity_reflection.md`](../design/roadmap-phases/v0_2_continuity_reflection.md)

## New concepts

```text
RelationshipState
CharacterSignal
OpenLoop
Commitment
ReflectionJob
CurrentContinuityView
```

## Goals

```text
make memory shape future behavior more explicitly
track active commitments and unresolved threads
derive relationship/project-specific character signals
separate current continuity context from raw historical memories
```

---

# 9. v0.3: factual rigor and belief tracking

Detailed draft: [`v0_3_factual_rigor_belief_tracking.md`](../design/roadmap-phases/v0_3_factual_rigor_belief_tracking.md)

## New concepts

```text
Assertion
Claim
EvidenceLink
BeliefAssessment
SourceAssessment
TemporalValidity
CurrentBeliefView
```

## Goals

```text
distinguish source reports from truth
support contradictions and updates
track temporal validity and volatility
show why factual beliefs are accepted or rejected
```

This is important, but it should not block the starter because Character Memory's first value is continuity, not full truth maintenance.

---

# 10. v0.4: advanced recall and governance

Detailed draft: [`v0_4_advanced_recall_governance.md`](../design/roadmap-phases/v0_4_advanced_recall_governance.md)

## New concepts

```text
Association
EpisodeCluster
RetentionAssessment
RetrievalTrace
ContextSubgraph
ValidationRules
```

## Goals

```text
improve associative recall
support retention/downranking/deletion policies
explain retrieval decisions
bound graph expansion
validate invariants
```

---

# 11. v1.0+: multimodal and embodied expansion

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

# 12. Public API evolution

## v0.1 API

```rust
let memory = CharacterMemory::new(settings, collection_name).await?;

let stored = memory.remember(remember_draft).await?;
let context = memory.retrieve(retrieval_context).await?;
let correction = memory.correct(correct_memory_draft).await?;
let forget = memory.forget(forget_memory_draft).await?;
let link = memory.link(memory_link_draft).await?;
```

## v0.2 API additions

```rust
let reflection_scope: Option<ReflectionScope> = None;
memory.reflect(reflection_scope).await?;

let signal: Option<CharacterSignal> = None;
memory.reinforce(target_id, signal).await?;

let continuity_scope: Option<ContinuityScope> = None;
let open_loops = memory.get_open_loops(continuity_scope).await?;

let evidence: Option<EvidenceLink> = None;
memory
  .resolve_commitment(commitment_id, evidence)
  .await?;
```

## v0.3 API additions

```rust
let assessment = memory.assess_claim(claim_id).await?;

let belief_scope: Option<BeliefScope> = None;
let beliefs = memory
  .get_current_beliefs(belief_scope)
  .await?;

let reviewed = memory.review_stale_beliefs().await?;
```

## v0.4 API additions

```rust
let explanation = memory.explain_retrieval(trace_id).await?;
memory.associate(from_id, to_id, association_type).await?;

let retention_scope: Option<RetentionScope> = None;
let retention_result = memory
  .apply_retention_policy(retention_scope)
  .await?;
```

---

# 13. YAGNI rules

Do not implement in v0.1:

```text
true hypergraphs
full OWL reasoning
continuous multimodal segmentation
robotic situation frames
full evidence-backed belief subsystem
normalized belief ontology
source reliability scoring
learned admission control
complex spreading activation
reflection scheduler
raw transcript storage in graph/vector stores
physical redaction/delete as the default lifecycle path
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
```

This keeps the starter small while avoiding structural dead ends.
