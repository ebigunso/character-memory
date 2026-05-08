# v0.4 Design Draft: Retrieval Observability and Governance

## Version intent

v0.4 makes retrieval decisions inspectable and validates graph-health invariants before the roadmap adds richer association and clustering.

This version is about:

```text
retrieval traces
context subgraphs
validation rules
graph health reports
policy diagnostics
retention assessment
```

The phase deliberately comes before advanced associative recall because association features create new edges and should be built after the system can explain and validate retrieval behavior.

---

# 1. Why v0.4 exists

By v0.3, Character Memory has:

```text
episodes
observations
entities
threads
derived memories
scoped continuity
factual claims and beliefs
retrieval stats and selectivity guardrails
```

That is enough structure for retrieval to become difficult to debug unless the system records what happened during retrieval.

v0.4 adds observability and governance so developers can answer:

```text
Why was this memory included?
Which path brought it into context?
Which candidates were rejected?
Was expansion bounded correctly?
Did a broad entity cause too much fanout?
Are low-information links being admitted?
Are retention and currentness policies working?
```

---

# 2. New concepts

## 2.1 RetrievalTrace

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
    "entity_selectivity": 0.81,
    "salience": 0.90,
    "recency": 0.80
  },
  "excluded_ids": [
    {"id": "dm_old", "reason": "superseded"},
    {"id": "ep_broad", "reason": "low_selectivity_only"}
  ]
}
```

RetrievalTrace should record semantic, temporal, entity, thread, salience, lifecycle, selectivity, and expansion contributions.

## 2.2 ContextSubgraph

A query-time compact graph of relevant memory context.

For Character Memory this is broader than factual evidence:

```text
active scope
active thread
recent episodes
salient derived memories
open loops
commitments
character signals
current beliefs, if relevant
retrieval rationale
```

## 2.3 ValidationRules

Lightweight rules that detect invariant violations.

Examples:

```text
DerivedMemory must have provenance.
Current CharacterSignal must be derived from episodes/reflections.
Suppressed/deleted memories must not appear in default retrieval.
Superseded memories must not appear in current views.
Thread membership should have confidence.
Low-selectivity co-occurrence alone should not create durable links.
Expansion attempts must respect relation/object fanout policies.
```

## 2.4 GraphHealthReport

A report of graph and retrieval-policy health.

It should identify:

```text
high-degree entities
high-fanout relation types
lowest selectivity entity/relation/object combinations
broad entities frequently used without supporting evidence
links created from weak co-occurrence
records missing required provenance
stale lifecycle hints in Qdrant
stats health issues
```

## 2.5 RetentionAssessment

A lifecycle object controlling whether memories are active, archived, suppressed, redacted, or deleted.

```json
{
  "id": "ret_...",
  "object_type": "retention_assessment",
  "target_id": "dm_...",
  "state": "suppressed",
  "reason": "user_request",
  "assessed_at": "2026-04-26T12:00:00+09:00",
  "rationale": "The source asked not to remember this detail."
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

## 2.6 PolicyDiagnostics

A diagnostic summary of policy behavior over time.

Examples:

```text
selectivity policy too conservative for scoped broad entities
fanout policy too permissive for participantEntity episode expansion
low-information co-occurrence guard rejected N candidate links
retention policy archived N low-salience stale memories
```

---

# 3. Retrieval observability

v0.4 retrieval pipeline:

```text
classify retrieval need
retrieve candidates from multiple routes
look up selectivity and policy inputs
expand by entity/thread/provenance/belief paths under bounds
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
scope lookup
recent episode lookup
open-loop/commitment lookup
current belief lookup
provenance expansion
```

Association expansion is intentionally deferred to v0.5.

---

# 4. Governance improvements

## 4.1 Retention policy

Implement policy hooks:

```text
low salience → archive/downrank
superseded → exclude from current views
explicit correction → supersede or suppress
explicit deletion → delete/redact according to policy
sensitive → access-control or redact
poisoning risk → quarantine
```

Principle:

```text
Forgetting for retrieval is not the same as erasure from provenance.
```

## 4.2 Validation rules

Start with lightweight validation, not heavy OWL reasoning.

Rules may be implemented as:

```text
Rust validation checks
JSON Schema
SHACL, if the graph layer benefits from it later
```

Do not make heavy ontology reasoning a prerequisite for v0.4.

---

# 5. Public API additions

Illustrative shape:

```rust
fn explain_retrieval(&self, trace_id: &str) -> Result<RetrievalTrace, MemoryError>;

fn get_context_subgraph(
    &self,
    context: RetrievalContext,
) -> Result<ContextSubgraph, MemoryError>;

fn validate(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<ValidationReport, MemoryError>;

fn graph_health_report(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<GraphHealthReport, MemoryError>;

fn policy_diagnostics(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<PolicyDiagnostics, MemoryError>;

fn apply_retention_policy(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<RetentionResult, MemoryError>;
```

---

# 6. Acceptance criteria

```text
RetrievalTrace records semantic, temporal, entity, thread, salience, lifecycle, selectivity, and expansion contributions.
ContextSubgraph shows the bounded graph neighborhood used for retrieval.
ValidationRules detect unbounded expansion attempts.
ValidationRules detect links created only from low-selectivity co-occurrence.
GraphHealthReport identifies high-degree entities and high-fanout relation types.
RetentionAssessment can identify old, low-salience, rarely retrieved memories without automatically deleting them.
Policy diagnostics can show when selectivity/fanout settings are too aggressive or too permissive.
Suppressed/deleted memories do not appear in default retrieval.
Current views exclude superseded memories.
```

---

# 7. Revisit guidance

Revisit if advanced association work cannot proceed safely without some v0.4 observability features. In that case, pull only the necessary trace/validation subset earlier, not the whole governance layer.
