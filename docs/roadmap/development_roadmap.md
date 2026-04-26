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
current factual beliefs
```

## 2.5 Correction should usually supersede, not erase

Most corrections should create new memory and links:

```text
new memory supersedes old memory
correction episode explains why
old memory remains historical unless suppressed/deleted
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
| v0.1 | Starter episodic memory | Chat-native memory substrate with episodes, observations, entities, soft threads, derived memories, and continuity retrieval. |
| v0.1 backend | Storage contracts | Qdrant/Oxigraph defaults, stable IDs, vector metadata, graph triples, migrations, and tests. |
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
Default Qdrant adapter
Default Oxigraph adapter
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
      memory.rs
      memory_input.rs
      memory_filters.rs
      memory_type.rs
      scored_memory.rs
  internal/
    models/
      memory/
        mod.rs
      vector/
        mod.rs
    repositories/
      memory_repository.rs
      vector_memory_repository.rs
      mod.rs
    infrastructures/
      external_services/
        mod.rs
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

Incorporate the useful engineering discipline from the old roadmap:

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
same object can be joined across raw store, vector store, and graph store
Qdrant filters work for record_type, entity_ids, thread_ids, time, currentness, retention
SPARQL queries return episode/entity/thread/provenance context
retrieval behavior is deterministic under fixed fixtures
```

---

# 7. v0.2: continuity and reflection

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

# 8. v0.3: factual rigor and belief tracking

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

# 9. v0.4: advanced recall and governance

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

# 10. v1.0+: multimodal and embodied expansion

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

# 11. Public API evolution

## v0.1 API

```rust
let memory = CharacterMemory::new(settings)?;

let stored = memory.remember(input).await?;
let context = memory.retrieve(filters).await?;
memory.correct(target_id, correction).await?;
memory.forget(target_id, ForgetMode::Suppress).await?;
memory.link(from_id, to_id, relation).await?;
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

# 12. YAGNI rules

Do not implement in v0.1:

```text
true hypergraphs
full OWL reasoning
continuous multimodal segmentation
robotic situation frames
full evidence-backed belief subsystem
source reliability scoring
learned admission control
complex spreading activation
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
