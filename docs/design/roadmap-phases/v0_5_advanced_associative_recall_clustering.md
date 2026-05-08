# v0.5 Design Draft: Advanced Associative Recall and Clustering

## Version intent

v0.5 builds richer associative memory after selectivity guardrails and retrieval observability exist.

This phase adds:

```text
Associations
EpisodeCluster
ClusterSummary
AssociationAdmissionPolicy
```

The goal is to improve cue-based recall and compress repeated patterns without creating unbounded pairwise links around broad entities.

---

# 1. Why this comes after v0.4

Associative recall can make memory feel more human-like, but it can also create noisy graph growth.

Without v0.1.2 selectivity guardrails and v0.4 retrieval observability, association features can accidentally create:

```text
pairwise cliques around recurring entities
large fanout through common places or topics
opaque context pollution
unexplained retrieval behavior
false reinforcement from repeated low-information co-occurrence
```

v0.5 should therefore build associations only after the system can explain and validate retrieval behavior.

---

# 2. New concepts

## 2.1 Association

A soft relation used for cue-based recall. Associations are not provenance.

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

Association types may include:

```text
co_mentioned
co_activated
same_thread
same_selective_entity
temporal_neighbor
same_project
same_preference_pattern
correction_related
commitment_related
```

Avoid generic `same_entity` associations unless the entity is selective or there is additional evidence.

## 2.2 EpisodeCluster

A provisional grouping of related episodes.

```json
{
  "id": "cluster_...",
  "object_type": "episode_cluster",
  "title": "Embedding surface design discussions",
  "summary": "Episodes where the embedding policy was refined.",
  "member_episode_ids": ["ep_1", "ep_2", "ep_3"],
  "confidence": 0.78,
  "promoted_to_thread_id": null
}
```

Clusters can later become MemoryThreads if they become stable continuity structures.

## 2.3 ClusterSummary

A derived summary of a cluster that preserves provenance to source episodes or derived memories.

```json
{
  "id": "clustersum_...",
  "object_type": "cluster_summary",
  "cluster_id": "cluster_...",
  "summary": "The embedding policy converged on natural-language semantic surfaces with structured metadata kept in payload filters.",
  "derived_from_episode_ids": ["ep_1", "ep_2", "ep_3"],
  "is_current": true
}
```

## 2.4 AssociationAdmissionPolicy

A policy for deciding whether an association should become durable.

Inputs may include:

```text
selectivity score
evidence strength
semantic similarity
same active thread
temporal proximity
causal relation
correction/supersession relation
salience
explicit application-created link
reflection-derived rationale
```

---

# 3. Goals

```text
improve associative recall
compress repeated patterns across many episodes
support cluster-level retrieval
avoid pairwise clique growth
preserve provenance from summaries/clusters to source memories
use selectivity scores and evidence strength for association admission
keep associations inspectable through retrieval traces and validation reports
```

---

# 4. Non-goals

Do not implement in v0.5:

```text
unbounded spreading activation
opaque learned association policy
association links created solely from low-selectivity co-occurrence
replacement of provenance links with associations
full graph analytics dashboard
multimodal clustering unless v1.0+ has started
```

---

# 5. Acceptance criteria

```text
Associations are not created solely from low-selectivity co-occurrence.
Association admission uses selectivity score and evidence strength.
EpisodeCluster can summarize broad recurring entity history without expanding every incident edge.
ClusterSummary preserves provenance to source episodes or derived memories.
Cluster retrieval does not bypass Oxigraph lifecycle/currentness verification.
Advanced associations remain inspectable through v0.4 retrieval traces/validation tools.
Associations improve recall without replacing provenance.
```

---

# 6. Revisit guidance

Revisit if actual usage shows that cluster summaries are needed earlier for performance.

If pulled earlier, implement cluster summaries as compression aids only, not as a broad association graph.
