# v0.1.2 Design Draft: Continuous Entity Selectivity and Retrieval Guardrails

## Version intent

v0.1 and v0.1.1 make the starter memory substrate feature-complete and persistence-safe. v0.1.2 hardens retrieval against a long-running graph problem: recurring entities can become so connected that naive expansion through them produces weak relevance, context pollution, latency spikes, or accidental pairwise edge growth.

This phase does **not** add new memory concepts. It adds use-case-agnostic retrieval guardrails that let entity continuity remain valuable without turning high-degree entities into traversal invitations.

The key design principle is:

```text
All entities start equal.
Retrieval adapts to observed graph structure.
High degree affects expansion policy, not entity importance.
```

A high-degree entity may still be central and highly relevant. It should not be globally penalized as unimportant. Instead, low selectivity means:

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

---

# 1. Why v0.1.2 exists

Character Memory is not only for personal assistants. It is also intended for persistent companions, game or simulation characters, research systems, and developer tools for temporal, entity-based, and relational retrieval.

That means the core library must not special-case roles such as:

```text
user
assistant
player
protagonist
NPC
home
owner
party member
main character
```

Any entity can become broad over time:

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

The library should treat entities equally at the schema level, then let accumulated graph structure influence retrieval behavior.

The existing authority split remains:

```text
Qdrant:
  vector candidate recall and coarse payload hints

Oxigraph:
  authoritative memory graph, relationships, provenance, lifecycle, currentness, expansion context

RetrievalStatsStore:
  derived counters and selectivity inputs only
```

Qdrant relationship and lifecycle fields are hints only. Oxigraph decides graph truth and final context inclusion. The stats store guides fanout policy but must not become a third source of truth.

---

# 2. Goals

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

---

# 3. Non-goals

Do not implement in v0.1.2:

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

There is no production data to migrate for this phase. However, the stats design should remain rebuildable from graph authority later because stats are derived policy metadata.

Revisit these goals if the stats store starts becoming a general analytics system or if retrieval correctness starts depending on stats. Stats must remain derived policy metadata, not memory truth.

---

# 4. New internal component: RetrievalStatsStore

Add an internal persistence component:

```text
RetrievalStatsStore
```

Recommended default implementation:

```text
SqliteRetrievalStatsStore
```

Recommended test implementation:

```text
InMemoryRetrievalStatsStore
```

Possible future implementations:

```text
PostgresRetrievalStatsStore
RedbRetrievalStatsStore
```

## Intent

The stats store persists derived counters needed for selectivity scoring and fanout policy. These counters must survive app restarts, but they must not become graph authority.

Responsibility boundary:

```text
Qdrant:
  semantic candidate recall and coarse payload hints

Oxigraph:
  memory truth, relationships, provenance, lifecycle, currentness, expansion context

SQLite stats store:
  derived counters for selectivity scoring and retrieval fanout policy
```

## Why SQLite as default

Use SQLite as the default because the stats data is structured and relational:

```text
entity_id
relation_kind
object_type
active_count
current_count
global_count
updated_at
```

The system needs indexed lookups, composite keys, transactions, top-N diagnostics, and simple aggregate reporting. This is a better fit for embedded SQL than for a NoSQL service.

The stats store should live with the main app process:

```text
native app:
  local app data directory

single app container:
  mounted persistent volume

multiple app replicas:
  use a future Postgres adapter instead of sharing SQLite over network storage
```

Revisit SQLite as the default if one of these becomes true:

```text
multiple app instances need concurrent shared writes to the same stats store
deployment policy forbids SQLite/C dependencies
stats workload becomes analytics-heavy rather than counter/update-heavy
the application already requires Postgres for other state
```

Recommended next steps if revisited:

```text
multi-replica shared stats:
  PostgresRetrievalStatsStore

strict pure-Rust embedded storage:
  RedbRetrievalStatsStore

large analytical reporting:
  consider a separate analytics path, not normal retrieval dependency
```

---

# 5. Retrieval stats schema

The SQLite implementation should add these internal tables.

## 5.1 `entity_edge_index`

```sql
CREATE TABLE entity_edge_index (
  edge_key TEXT PRIMARY KEY,
  entity_id TEXT NOT NULL,
  relation_kind TEXT NOT NULL,
  object_id TEXT NOT NULL,
  object_type TEXT NOT NULL,
  retention_state TEXT NOT NULL,
  is_current INTEGER NOT NULL,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL
);

CREATE INDEX idx_entity_edge_entity_relation
ON entity_edge_index(entity_id, relation_kind, object_type);

CREATE INDEX idx_entity_edge_object
ON entity_edge_index(object_id, object_type);
```

Intent:

```text
Make stats updates idempotent.
Prevent duplicate counter increments when the same graph edge is processed more than once.
```

`edge_key` should be deterministic, for example:

```text
hash(object_id + relation_kind + entity_id)
```

This table mirrors only the edge facts needed for derived stats. It is not graph authority.

Revisit if graph mutation events already provide perfectly idempotent deltas and the ledger proves unnecessary. Default should be to keep the ledger because silent counter drift over long timescales would be difficult to debug.

## 5.2 `entity_relation_counts`

```sql
CREATE TABLE entity_relation_counts (
  entity_id TEXT NOT NULL,
  relation_kind TEXT NOT NULL,
  object_type TEXT NOT NULL,
  total_count INTEGER NOT NULL DEFAULT 0,
  active_count INTEGER NOT NULL DEFAULT 0,
  current_count INTEGER NOT NULL DEFAULT 0,
  last_seen_at TEXT,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (entity_id, relation_kind, object_type)
);
```

Intent:

```text
Store per-entity counters by relation and object type.
```

This supports questions such as:

```text
How many active derived memories are about entity E?
How many active episodes include entity E as a participant?
How many current derived memories point to entity E?
```

Revisit if retrieval policy needs more dimensions, such as source conversation, modality, salience bucket, or time bucket. Do not add those dimensions until diagnostics show the current counters are insufficient.

## 5.3 `global_relation_counts`

```sql
CREATE TABLE global_relation_counts (
  relation_kind TEXT NOT NULL,
  object_type TEXT NOT NULL,
  total_count INTEGER NOT NULL DEFAULT 0,
  active_count INTEGER NOT NULL DEFAULT 0,
  current_count INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (relation_kind, object_type)
);
```

Intent:

```text
Store global denominators for selectivity scoring.
```

A per-entity count is not meaningful alone. An entity connected to 100 memories means something different in a corpus of 200 memories than in a corpus of 2,000,000 memories.

Revisit when corpus size grows by orders of magnitude or when relation distributions become highly skewed. At that point, add histograms or percentile summaries in v0.4, not in v0.1.2.

## 5.4 Optional `stats_meta`

```sql
CREATE TABLE stats_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

Intent:

```text
Track operational metadata.
```

Examples:

```text
stats schema version
policy version
last clean shutdown
stats health state
```

Revisit if the metadata starts duplicating application configuration. Keep it limited to stats-store operational state.

---

# 6. Stats update behavior

Update stats from the same application write pipeline that mutates graph relationships and lifecycle state.

Examples:

```text
new DerivedMemory aboutEntity E:
  insert edge ledger row
  increment entity_relation_counts[E, aboutEntity, derived_memory]
  increment global_relation_counts[aboutEntity, derived_memory]

new Episode participantEntity E:
  insert edge ledger row
  increment entity_relation_counts[E, participantEntity, episode]
  increment global_relation_counts[participantEntity, episode]

memory becomes suppressed:
  update ledger lifecycle
  decrement active_count and current_count where applicable

memory becomes non-current:
  update ledger currentness
  decrement current_count

replacement memory supersedes old memory:
  old memory current_count decreases
  replacement memory current_count increases
```

Write ordering should preserve the existing authority model:

```text
1. Apply graph-authoritative mutation to Oxigraph.
2. Apply vector maintenance to Qdrant.
3. Apply stats delta to RetrievalStatsStore.
```

There is no distributed transaction across Oxigraph, Qdrant, and the stats store. This is acceptable because Qdrant is non-authoritative, and stats must also be non-authoritative.

If stats update fails, retrieval should fall back to conservative fanout and diagnostics should report unhealthy stats.

Revisit write ordering if inconsistent stats become frequent in tests or logs. Do not solve this with distributed transactions in v0.1.2. Prefer idempotent events, retry, health flags, and conservative fallback.

---

# 7. Continuous selectivity scoring

Use continuous scores from the beginning instead of persisted categories.

Do not persist:

```text
entity.selectivity_class = "broad"
```

Persist counts, then compute scores at retrieval time.

For an entity `e`, relation `r`, object type `o`, and lifecycle scope `s`:

```text
n = count(e, r, o, s)
N = global_count(r, o, s)

raw_selectivity =
  ln((N + α) / (n + α)) / ln(N + α)
```

Clamp the result to:

```text
0.0..1.0
```

Use smoothing:

```text
α = 1.0 initially
```

Interpretation:

```text
near 1.0:
  highly selective entity/relation/object combination

near 0.0:
  broad or low-selectivity entity/relation/object combination
```

Selectivity is relation-specific. Do not classify an entity globally as broad or selective. An entity may be broad under `participantEntity` but selective under `aboutEntity`, `partOfThread`, or another relation.

Diagnostic labels may still be useful:

```text
highly selective
selective
broad
very broad
```

But labels are only for:

```text
retrieval rationale
diagnostics
debugging
tests
graph health reports
```

They must not be the core fanout mechanism.

Revisit the formula if diagnostics show poor correlation between selectivity score and retrieval quality, or if score changes produce unstable context packs. The next step should be adjusting formula/configuration, not persisting categories.

---

# 8. Smooth fanout policy

Fanout should be a smooth function of:

```text
selectivity score
relation kind
object type
supporting evidence
explicit retrieval scope
```

Do not use hard category cliffs such as:

```text
if category == broad:
  fanout = 3
```

Recommended shape:

```text
base_budget = relation_policy.max_fanout
specificity_factor = raw_selectivity ^ gamma
support_factor =
  1.0
  + semantic_support
  + thread_support
  + temporal_support
  + salience_support
  + currentness_support
  + correction_support
  + explicit_scope_support

fanout_budget =
  clamp(
    floor(base_budget * specificity_factor * support_factor),
    relation_policy.min_fanout,
    relation_policy.max_fanout
  )
```

Relation policies should still impose hard upper bounds:

```text
aboutEntity:
  moderate max fanout

participantEntity:
  smaller max fanout

partOfThread:
  bounded thread-context fanout

derivedFromEpisode / derivedFromObservation:
  directional provenance lookup

supersedes:
  currentness/correction lookup, not broad association expansion
```

Revisit if fanout budgets are consistently too conservative for central entities or too permissive for broad ones. The first fix should be relation-specific policy tuning and evidence weighting. Do not introduce entity identity special-casing.

---

# 9. Low-information co-occurrence guard

Prevent durable pairwise links from being created solely because two memories share a low-selectivity entity or broad relation.

Do not create pairwise durable links only because:

```text
two episodes mention the same frequent person
two episodes occur in the same common place
two memories involve the same recurring project
two derived memories share a broad topic
two observations share a low-selectivity participant
```

Durable association should require stronger evidence:

```text
semantic similarity
explicit application-created link
same active thread
causal relationship
temporal relationship
correction/supersession
shared selective entity
repeated pattern
high salience
reflection-derived rationale
```

This avoids O(N²) edge growth around broad recurring entities.

Revisit when v0.5 controlled associative recall and clustering work begins. At that point, association admission should use query-time activation, graph-internal associative units, member-level lifecycle, association support evidence, selectivity scores, and evidence strength, not raw co-occurrence.

## 9.1 Serendipitous recall tradeoff

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

## 9.2 Weak co-occurrence is not durable association

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

---

# 10. Retrieval rationale updates

The existing roadmap requires explainable retrieval. v0.1.2 refines the meaning of “same entity” so it does not hide broad-entity behavior.

Rationale should distinguish:

```text
same entity, high selectivity
same entity, low selectivity
same entity, low selectivity but supported by semantic similarity
same entity, low selectivity but supported by active thread
same entity, low selectivity and rejected as insufficient evidence
explicit scope allowed bounded expansion through broad entity
```

Do not allow rationale that merely says:

```text
same entity
```

when the entity match was broad and weak.

Revisit in v0.4 when full `RetrievalTrace` is added. At that point, rationale labels can become structured trace components with score contributions, expansion paths, and rejection reasons.

---

# 11. Diagnostics

Add lightweight report-only diagnostics. Do not build a public admin dashboard yet.

Diagnostics should expose:

```text
top entities by active relation degree
top entities by current relation degree
top relation/object combinations by edge count
lowest selectivity entity/relation/object combinations
highest observed expansion fanout
retrieval decisions rejected for low-selectivity-only evidence
pairwise links rejected for low-information co-occurrence
stats health state
stats update failures
qdrant candidates rejected by graph lifecycle/currentness
```

Per retrieval, log or make inspectable:

```text
entity_id
relation_kind
object_type
entity_count
global_count
share
selectivity_score
diagnostic label
supporting signals
chosen fanout budget
expanded count
included count
rejected count
```

Diagnostics should be report-only, matching the existing reconciliation philosophy: detect drift and visibility issues without making diagnostics repair or override stores.

Revisit in v0.4, where diagnostics should evolve into graph health reports, validation rules, context-subgraph inspection, and retrieval traces.

---

# 12. Tests and acceptance criteria

The tests must prove entity-neutral behavior and prevent future accidental personal-assistant assumptions.

Add synthetic fixtures for:

```text
high-degree person
high-degree place
high-degree project
high-degree topic
high-degree object
high-degree arbitrary custom entity
selective entity
ordinary entity
broad entity
very broad entity
```

The entity names should intentionally avoid only personal-assistant examples. Include fixtures where a central non-human or non-user entity becomes broad.

Required invariant tests:

```text
No normal retrieval scans or hydrates the whole graph to classify entity selectivity.
No retrieval rule checks hard-coded entity names, canonical keys, or application roles.
Increasing entity_count while holding global_count constant must not increase selectivity.
Increasing supporting evidence may increase fanout, but never above relation-specific caps.
Low-selectivity entity evidence alone cannot flood the context pack.
High-selectivity entity evidence can contribute meaningfully to retrieval.
Broad entities can still contribute when supported by semantic, thread, temporal, salience, currentness, correction, or explicit scope evidence.
Suppressed, deleted, non-current, and superseded memories remain excluded by graph verification.
Qdrant hints cannot force inclusion when Oxigraph disagrees.
Stats missing/unhealthy produces conservative fanout.
Stats survive app restart.
SQLite stats persistence works in native and single-container deployments.
```

Revisit test coverage when new relation types, object types, or association objects are added. New relation types should not bypass selectivity/fanout policy by default.

---

# 13. Configuration additions

Add settings for the stats store and selectivity policy.

Suggested environment-style names:

```text
RETRIEVAL_STATS_STORE=sqlite
RETRIEVAL_STATS_PATH=./data/character-memory/retrieval_stats.sqlite
RETRIEVAL_STATS_HEALTH_FAIL_MODE=conservative
SELECTIVITY_SMOOTHING_ALPHA=1.0
SELECTIVITY_GAMMA=1.0
```

Relation-specific budgets should live in structured app settings rather than many flat environment variables.

Example conceptual config:

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

Intent:

```text
Make policy tunable without hiding magic numbers in code.
```

Revisit after diagnostics show real retrieval distributions. Avoid adding too many knobs before there is evidence they are needed.

---

# 14. Implementation notes to preserve

## 14.1 No existing data migration

There is no existing production data, so do not spend v0.1.2 effort on migration/backfill.

However, the stats store should still be rebuildable later from Oxigraph because stats are derived.

## 14.2 Stats missing or unhealthy must degrade conservatively

If stats are missing, corrupt, or unhealthy:

```text
do not disable retrieval
do not expand broadly
use conservative fanout
require stronger supporting evidence
report diagnostics
```

## 14.3 High degree is not the same as low importance

A broad entity may be central to the application. Selectivity should limit unbounded expansion, not erase relevance.

## 14.4 Explicit scope can justify bounded broad-entity use

If retrieval is explicitly scoped to a broad entity, the policy may allow bounded expansion. The system should still respect relation caps, lifecycle, currentness, salience, and graph verification.

## 14.5 No persisted categories

Persist counters. Compute scores. Diagnostic labels are allowed, but categories should not be durable entity state.

## 14.6 No graph-wide scans during normal retrieval

Normal retrieval should fetch stats only for entities involved in the current context or candidate set. Diagnostics may run broader reports later, especially in v0.4.

## 14.7 Oxigraph final inclusion remains mandatory

Even if stats and Qdrant suggest relevance, Oxigraph still decides whether a memory exists, is current, is suppressed, is superseded, and belongs in final context.

---

# 15. Final implementation summary

v0.1.2 is not “add a graph analytics subsystem.” It is a small but important retrieval-hardening layer.

Implement:

```text
persistent derived counters
continuous selectivity score
relation-specific bounded fanout
low-information co-occurrence guard
entity-neutral tests
light diagnostics
```

Do not implement:

```text
special user/assistant handling
persisted entity categories
learned retrieval
centrality algorithms
association graph
admin dashboard
migration for nonexistent data
```

The architectural invariant is:

```text
Qdrant suggests.
Stats guide fanout.
Oxigraph decides.
```

The product invariant is:

```text
Entities should preserve continuity without turning recurring entities into unbounded traversal paths.
```
