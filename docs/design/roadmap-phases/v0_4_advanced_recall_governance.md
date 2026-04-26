# v0.4 Design Draft: Advanced Recall and Governance

## Version intent

v0.4 improves recall quality, safety, lifecycle control, and debuggability once the core memory system is already working.

This version is about:

```text
associative recall
retention governance
retrieval traces
validation
query-time context construction
schema hardening
```

---

# 1. New concepts

## 1.1 Association

A soft relation used for cue-based recall.

Associations are not provenance.

```json
{
  "id": "assoc_...",
  "object_type": "association",
  "from_id": "ep_...",
  "to_id": "dm_...",
  "association_type": "co_activated",
  "weight": 0.72,
  "activation_count": 5,
  "last_activated_at": "2026-04-26T12:00:00+09:00",
  "rationale": "These memories are often retrieved together in the Character Memory design thread."
}
```

Association types:

```text
co_mentioned
co_activated
same_thread
same_entity
temporal_neighbor
same_project
same_preference_pattern
correction_related
commitment_related
```

## 1.2 EpisodeCluster

A provisional grouping of related episodes.

```json
{
  "id": "cluster_...",
  "object_type": "episode_cluster",
  "title": "Embedding surface design discussions",
  "summary": "Episodes where the user and assistant refined the vector embedding policy.",
  "member_episode_ids": ["ep_1", "ep_2", "ep_3"],
  "confidence": 0.78,
  "promoted_to_thread_id": null
}
```

Clusters can later become MemoryThreads.

## 1.3 RetentionAssessment

A lifecycle object controlling whether memories are active, archived, suppressed, redacted, or deleted.

```json
{
  "id": "ret_...",
  "object_type": "retention_assessment",
  "target_id": "dm_...",
  "state": "suppressed",
  "reason": "user_request",
  "assessed_at": "2026-04-26T12:00:00+09:00",
  "rationale": "The user asked not to remember this detail."
}
```

Retention states:

```text
active
archived
suppressed
quarantined
redacted
deleted
```

## 1.4 RetrievalTrace

A diagnostic object explaining retrieval.

```json
{
  "id": "trace_...",
  "object_type": "retrieval_trace",
  "query_text": "What should we implement first?",
  "retrieved_ids": ["thread_...", "dm_...", "ep_..."],
  "ranking_features": {
    "semantic_similarity": 0.72,
    "thread_match": 0.95,
    "salience": 0.90,
    "recency": 0.80
  },
  "excluded_ids": [
    {"id": "dm_old", "reason": "superseded"}
  ]
}
```

## 1.5 ContextSubgraph

A query-time compact graph of relevant memory context.

For Character Memory this is broader than factual evidence:

```text
active thread
recent episodes
salient derived memories
open loops
commitments
character signals
beliefs, if relevant
retrieval rationale
```

---

# 2. Retrieval improvements

v0.4 retrieval pipeline:

```text
classify retrieval need
retrieve candidates from multiple routes
expand by entity/thread/provenance/association
construct compact context subgraph
filter by currentness and retention
compress/deduplicate
return ContinuityContextPack
record RetrievalTrace
```

Routes:

```text
semantic vector search
entity lookup
thread lookup
recent episode lookup
open-loop/commitment lookup
current belief lookup
association expansion
provenance expansion
```

---

# 3. Governance improvements

## 3.1 Retention policy

Implement policy hooks:

```text
low salience → archive/downrank
superseded → exclude from current views
user correction → supersede or suppress
user deletion → delete/redact according to policy
sensitive → access-control or redact
poisoning risk → quarantine
```

Principle:

```text
Forgetting for retrieval is not the same as erasure from provenance.
```

## 3.2 Validation rules

Start with lightweight validation, not heavy OWL reasoning.

Rules:

```text
DerivedMemory must have provenance.
Current CharacterSignal must be derived from episodes/reflections.
BeliefAssessment must assess a Claim.
Suppressed/deleted memories must not appear in default retrieval.
Thread membership must have confidence.
Superseded memories must not appear in current views.
```

These can be implemented as JSON Schema, application checks, or SHACL depending on the backend.

---

# 4. Public API additions

```rust
fn explain_retrieval(&self, trace_id: &str) -> Result<RetrievalTrace, MemoryError>;
fn associate(&self, from_id: &str, to_id: &str, association_type: AssociationType, weight: Option<f32>) -> Result<MemoryLink, MemoryError>;
fn apply_retention_policy(&self, scope: Option<&MemoryScope>) -> Result<RetentionReport, MemoryError>;
fn validate(&self, scope: Option<&MemoryScope>) -> Result<ValidationReport, MemoryError>;
fn get_context_subgraph(&self, context: RetrievalContext) -> Result<ContextSubgraph, MemoryError>;
fn cluster_episodes(&self, scope: Option<&MemoryScope>) -> Result<Vec<EpisodeCluster>, MemoryError>;
```

---

# 5. Acceptance criteria

```text
Associations can improve recall without replacing provenance.
Retrieval traces explain why memories were selected or excluded.
Retention state affects default retrieval.
Validation catches missing provenance and invalid current views.
Graph expansion remains bounded under hub entities.
ContextSubgraph can be built for a query and converted into ContinuityContextPack.
```

---

# 6. Relation to old roadmap

The old roadmap's performance hardening phase is incorporated here and partially moved earlier.

Kept:

```text
bounded graph expansion
hub-entity handling
caching hooks
regression tests
clear migration/versioning
```

Expanded:

```text
retrieval traces
retention governance
association graph
context subgraph construction
validation of Character Memory invariants
```
