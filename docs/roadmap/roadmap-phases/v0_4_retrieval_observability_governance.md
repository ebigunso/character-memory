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
rejected expansion traces
cluster and activation diagnostics
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
Why was broad-entity-only expansion blocked?
Which activation paths were considered?
Why was a possible cluster member included, excluded, or rejected?
```

---

# 2. New concepts

## 2.1 RetrievalTrace

A diagnostic object explaining retrieval.

Earlier v0.1-family phases expose only light per-retrieval rationale/telemetry and admin-facing reconciliation diagnostics. v0.4 is where durable first-class `RetrievalTrace` objects become part of retrieval observability.

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

## 2.7 Additional v0.4 concepts

```text
ActivationTrace
RejectedExpansionTrace
ClusterExpansionTrace
MembershipDecisionTrace
AssociationCandidateDiagnostic
CoactivationDiagnostic
```

These concepts are diagnostic and report-only in v0.4. They prepare the retrieval layer to explain controlled associative recall in v0.5 without implementing the associative cluster machinery in this phase.

## 2.8 RetrievalIntent

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

## 3.1 Additional goals

```text
make rejected low-information expansions inspectable
show why broad-entity-only expansion was blocked
show activation paths used during retrieval
show when weak coactivation was considered but not persisted
show cluster membership inclusion/exclusion rationale
diagnose candidate membership promotion, demotion, decay, or rejection
detect over-broad clusters and high-fanout cluster expansions
```

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

fn validate_graph(
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

## 6.1 Additional acceptance criteria

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

---

# 7. Additional non-goal

v0.4 should not implement the associative cluster machinery itself. It should make retrieval decisions and blocked expansions observable so v0.5 can safely add controlled associative recall.

---

# 8. Revisit guidance

Revisit if advanced association work cannot proceed safely without some v0.4 observability features. In that case, pull only the necessary trace/validation subset earlier, not the whole governance layer.
