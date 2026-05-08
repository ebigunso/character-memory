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

---

# 3. Version overview

| Version | Theme | Outcome |
|---|---|---|
| v0.1 | Starter episodic memory | Finished. Public graph-authoritative memory substrate with episodes, observations, entities, soft threads, derived memories, lifecycle facades, and continuity retrieval. |
| v0.1 backend | Storage contracts | Finished. Qdrant candidate recall, Oxigraph graph authority, stable IDs, vector metadata hints, graph triples, schema versions, bounded expansion support, and tests. |
| v0.1.1 | Persistent graph authority | Finished. Durable Oxigraph-backed graph authority, restart-safe retrieval, Qdrant/Oxigraph reconciliation, and persistence validation. |
| v0.1.2 | Continuous entity selectivity and retrieval guardrails | New. Use-case-agnostic guardrails for high-degree or low-selectivity entities, persistent retrieval statistics, continuous selectivity scoring, relation-specific fanout control, low-information co-occurrence prevention, and diagnostics. |
| v0.2 | Scoped continuity and reflection | `ContinuityScope`, scoped reflection, relationship state between arbitrary entities, character signals for continuing entities, open-loop/commitment lifecycle, and current continuity views. |
| v0.3 | Factual rigor, temporal validity, and entity evolution | Assertions, claims, evidence links, belief assessments, source assessment, temporal validity, entity drift handling, and current-belief views. |
| v0.4 | Retrieval observability and governance | Retrieval traces, context subgraphs, validation rules, graph health reports, policy diagnostics, and retention assessment. |
| v0.5 | Advanced associative recall and clustering | Associations, episode clusters, cluster summaries, and selectivity-aware association admission. |
| v1.0+ | Multimodal and embodied expansion | Voice beyond transcript, multimodal observations, situation frames, object/place/action memory. |

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
      vector_candidate_store.rs
      retrieval_stats_store.rs
    remember_pipeline.rs
    link_pipeline.rs
    retrieve_pipeline.rs
    correction_forget_pipeline.rs
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
    stats/
      mod.rs
      sqlite_retrieval_stats_store.rs
      in_memory_retrieval_stats_store.rs
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

# 9. v0.2: scoped continuity and reflection

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

# 10. v0.3: factual rigor, temporal validity, and entity evolution

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

# 11. v0.4: retrieval observability and governance

Detailed draft: [`v0_4_retrieval_observability_governance.md`](../design/roadmap-phases/v0_4_retrieval_observability_governance.md)

## New concepts

```text
RetrievalTrace
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
```

---

# 12. v0.5: advanced associative recall and clustering

Detailed draft: [`v0_5_advanced_associative_recall_clustering.md`](../design/roadmap-phases/v0_5_advanced_associative_recall_clustering.md)

## New concepts

```text
Association
EpisodeCluster
ClusterSummary
AssociationAdmissionPolicy
```

## Goals

```text
improve associative recall
compress repeated patterns across many episodes
support cluster-level retrieval
avoid pairwise clique growth
preserve provenance from summaries/clusters to source memories
```

---

# 13. v1.0+: multimodal and embodied expansion

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

# 14. Public API evolution

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
let validation = memory.validate(scope).await?;
let context_subgraph = memory.get_context_subgraph(context).await?;

let retention_scope: Option<ContinuityScope> = None;
let retention_result = memory.apply_retention_policy(retention_scope).await?;
```

## v0.5 API additions

```rust
memory.associate(from_id, to_id, association_type).await?;
let cluster_result = memory.cluster_episodes(scope).await?;
let cluster_context = memory.retrieve_cluster_context(cluster_id).await?;
```

---

# 15. YAGNI rules

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
