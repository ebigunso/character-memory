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

## The v1.0 release-ready state

v1.0 is the text-only release-ready state: a character whose input arrives as text, including transcripts, exercises recall and remembering that let it behave in a human-comparable way in how and what it remembers. The acceptance basis is the [continuity situation catalog](../design/continuity_situation_catalog.md): v1.0 is reached when the catalog's situations are met across the companion, small-circle, and independent-entity deployments, with every retrieval-level property passing deterministically and the behavioral qualities judged. Perception beyond text is an aspiration past v1.0, described in the unnumbered beyond-text horizon, and never a v1.0 gate.

---

# 2. Cross-version invariants

These should remain stable even as the library evolves. An invariant that cites a decision record still marked proposed is provisional until that record is accepted.

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

Current context is presented for shaping behavior, so its rendering favors gist and stance over verbatim episode text.

## 2.5 Correction supersedes; the record is append-only

Most corrections should create new memory and links:

```text
new memory supersedes old memory
correction episode explains why
old memory remains historical; suppression removes influence, never the record
```

Destructive deletion is not a memory operation. Erasure exists only as an out-of-band operational purge (compliance, security remediation, operator-directed alteration) that tombstones dangling provenance targets. See [ADR-D-0021](../decisions/design/ADR-D-0021-append-only-memory-record-with-out-of-band-purge.md).

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

The long-horizon phase implements this through query-time activation first, and through graph-internal associative units, member-level lifecycle, association support evidence, and bounded expansion only when measurement shows activation alone is not enough.

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

Reflection (section 14), the library's one producer of interpreted memory, produces plans for the existing write-plan path rather than inventing a parallel persistence pipeline.

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
source references
source spans once introduced by v0.1.3 write planning
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

Core memory never persists raw input. Recent raw snippets are held as trace in a short-term store beside core memory until consolidation turns them into curated memory and releases them (invariant 2.17). Raw source text reaches reflection only as trace, beside what durable memory already holds for the scope, because exclusion, presence accounting, and grounding are applied at the mechanical write. An application importing a history writes it as trace with its own times and reflects. The short-term store is not core memory storage, nothing in it enters graph authority or the durable vector store, and it is not a log to be searched or exported.

## 2.14 Memory is first-person

A memory store has one remembering subject, and that subject is an ordinary entity in its own graph. Its actions are episodes it participated in, its promises are its commitments, and its history persists between interactions with anyone.

```text
the self is an Entity, identified by the application at construction; a scope may name the same self again and never a different one
no object type, entity type, or retrieval path treats the self as a special role
entity types that encode application roles (user, assistant) are not core schema truth
```

Recall is also relative to a moment: retrieval takes a reference time, and elapsed time since a memory is a retrieval signal with its own rationale, not only an ordering key.

## 2.15 Recall is complete; forgetting is explicit

No memory becomes less reachable because time passed, and no stored measure of importance, confidence, or stability changes without a write that carries provenance. Eligibility changes only through suppression or supersession, each a recorded decision. What is no longer current leaves current views through a change of currency, not of eligibility. An item is current when it is the latest in its supersession chain, not resolved, and, if it carries a validity interval recorded at write time, within that interval; currency selects versions and never selects or removes items; staleness is the age since a memory's last evidence, reported and never enforced. An interval is a fact recorded when the memory was written, so expiry is a known event, not a decay. Human-shaped fading is produced by ranking and expression, never by retention. Familiarity and the weight of repeated evidence are derived from provenance at query time, so there is no reinforce operation. See [ADR-D-0018](../decisions/design/ADR-D-0018-recall-is-complete-and-forgetting-is-explicit.md).

```text
write-time attention decides what becomes memory
query-time ranking weighs salience and elapsed time
expression offers detail or gist
suppression and supersession are the only forgetting
currency, not eligibility, takes what is over out of current views
```

## 2.16 Recall is situated, and the scene travels with every memory

Every memory carries its scene: who was present, who said it, whether the character was there when it happened, and in which setting. Retrieval takes the present scene, who is present, where, when, and what is in progress, and reports each admitted memory's scene. The application gives the scene as the character perceives it, in descriptions, with keys where they exist, and only the time is required; it never resolves a reference ([ADR-D-0029](../decisions/design/ADR-D-0029-the-scene-is-given-as-perceived-and-the-application-never-resolves-it.md)). Who and what those references are is the character's own understanding: an entity records a notion the character holds, and its name, its kind, whether anything answers to it, and what it is the same as are beliefs that can be revised ([ADR-D-0034](../decisions/design/ADR-D-0034-an-entity-is-a-notion-the-character-holds.md)). Recall is never gated by privacy or sensitivity by default; an enforced boundary is an explicit query-time policy over the scene chosen by the application. See [ADR-D-0019](../decisions/design/ADR-D-0019-discretion-is-disclosure-not-recall.md).

Recall is activation by the cues the present scene supplies, with the topic of the current turn as one cue among them. Each cue kind has its own way of finding candidates (content through each store's content lookup, vectors in durable memory and a lexical or vector index in the short-term store of v0.3; entities, threads, and places through the graph in durable memory, and through the participants and setting recorded on each entry in the short-term store; time and dates through timestamps, the graph's and each entry's own; stored intentions through their trigger, in durable memory only) and its own admission floor, so no cue kind can starve another. See [ADR-D-0022](../decisions/design/ADR-D-0022-recall-is-activation-by-scene-cues.md). Purpose is never a supplied cue: it surfaces from memory as an open loop, a commitment, a thread, or a signal, and once surfaced it re-cues one bounded hop. See [ADR-D-0023](../decisions/design/ADR-D-0023-purpose-is-never-a-supplied-cue.md).

How a memory should be treated is a change of the character's state about it, carried by supersession with a restatement rather than an appended note; there is no treatment category and no annotation plane. The write path warns on a replacement that contains its predecessor nearly verbatim and on a chain that churns.

## 2.17 Experience enters durable memory by one path, and the moment lands as trace

Durable memory is written only through the validated write path: consolidation by reflection, or a caller that deliberately prepares, validates, and commits. Everything that happens in the moment, a conversation line, a tool result, a note the character's model chose to make, lands as trace in a short-term store beside core memory and nowhere else. Immediacy comes from recall over that trace, never from an early durable write. See [ADR-D-0025](../decisions/design/ADR-D-0025-experience-enters-durable-memory-by-one-path.md) and [ADR-D-0026](../decisions/design/ADR-D-0026-a-short-term-store-outside-core-memory-holds-recent-trace.md).

```text
structure within one event is immediate: the boundaries the application reports and binding to the scene, mechanically; finer segmentation is consolidation's
structure across events is consolidation: gist, state, patterns, beliefs
recent trace is reachable by scene, time, entity, and topic, never through the state or stored-intention routes, and the reader comprehends it as it reads
a change of state before consolidation is discovered at recall, not recorded at write
the short-term store is outside core memory; trace leaves it by consolidation, never by expiry, the out-of-band purge being the one other way out, and overlong retention is reported loudly
```

## 2.18 The timeline has no unexplained holes

Every span the character was present for is covered, however quiet: before consolidation by its trace, afterward by a durable account. A span is absent only when both are missing; an out-of-band purge lies outside this guarantee by ADR-D-0021's definition. "Nothing" may describe interpretation and never the record. Unconsolidated trace is never dropped, and consolidation never releases a span's trace without leaving an account of it. See [ADR-D-0027](../decisions/design/ADR-D-0027-every-span-of-presence-is-accounted-for.md).

## 2.19 Interpreted memory carries its evidence

Every interpreted memory states the evidence it rests on, and the write path rejects what that evidence cannot structurally support. The model supplies only judgment: what the trace already knows, the time, the scene, the entry, and whether it was the character's own output or something it perceived, is copied, and what can be counted is computed. Only a literal statement or an event supports a change of state. What a speaker stated is held as theirs from one instance, while what the character concludes for itself must be earned by a pattern. The character's own commitments rest on what it said or did, never on another's claim or its own note alone. Contradictions are held, not resolved, and what was experienced is never instruction to the one reflecting. Validation guarantees structure and grounding and never the truth of a judgment, which evaluation measures. See [ADR-D-0028](../decisions/design/ADR-D-0028-interpreted-memory-carries-its-evidence.md).

## 2.20 A memory is held as experience

No interpreted memory carries a confidence score or a numeric measure of emotion: how firmly something is held is read from how it was formed and what is linked to it. An episode keeps how it felt to the character, in words, and that is never revised; a later change of heart is a new memory beside it. How someone seemed is an impression, the character's own, and supports a conclusion about them only through a pattern. See [ADR-D-0030](../decisions/design/ADR-D-0030-interpreted-memory-carries-no-confidence-score.md) and [ADR-D-0033](../decisions/design/ADR-D-0033-a-memory-keeps-how-it-felt.md).

---

# 3. Version overview

| Version | Theme | Outcome |
|---|---|---|
| v0.1 | Starter episodic memory | Finished. Public graph-authoritative memory substrate with episodes, observations, entities, soft threads, derived memories, lifecycle facades, and continuity retrieval. |
| v0.1 backend | Storage contracts | Finished. Qdrant candidate recall, Oxigraph graph authority, stable IDs, vector metadata hints, graph triples, schema versions, bounded expansion support, and tests. |
| v0.1.1 | Persistent graph authority | Finished. Durable Oxigraph-backed graph authority, restart-safe retrieval, Qdrant/Oxigraph reconciliation, and persistence validation. |
| v0.1.2 | Continuous entity selectivity and retrieval guardrails | New. Use-case-agnostic guardrails for high-degree or low-selectivity entities, persistent retrieval statistics, continuous selectivity scoring, relation-specific fanout control, low-information co-occurrence prevention, and diagnostics. |
| v0.1.3 | Remember intake interfaces and deterministic write planning | Finished. Generation-ready write path with `RememberWritePlan`, memory candidates, validation, deterministic helpers, prepare/validate/commit flow, and shared manual/future-generated commit machinery. |
| v0.1.4 | Continuity evaluation harness | Finished. Deterministic long-horizon evaluation harness implemented in the public companion `CharacterMemoryEvals` repository as a development aid, not core library functionality: synthetic interaction fixtures, a minimal example assistant loop, continuity-oriented retrieval-quality metrics, selectivity/fanout measurement, and hub-entity stress scenarios. |
| v0.1.5 | Eval-driven v0.1 family closeout | Finished. Ran the evaluation harness across the v0.1 family, dispositioned eleven findings (none critical, none open), fixed deterministic vector admission and write-path warning diagnostics in the library, retained the measured defaults with a recorded basis (ADR-I-0022), adopted embedded persistent Oxigraph as the validated default (ADR-I-0021), and expanded the evaluation suite to 33 scenarios including benchmark-adapted and real-embedding fixtures. Closeout report: [`v0_1_5_closeout_report.md`](roadmap-phases/v0_1_5_closeout_report.md). |
| v0.1.6 | Embedded vector candidate recall | Finished 2026-09-04. An embedded vector candidate store on the in-process build of the service backend (Qdrant Edge) is the default vector mode at its exact-scan indexing threshold, so zero-infrastructure local deployments and the default test path need no external service; the service adapter remains the explicit service mode. The redesigned port reports recall completeness, accepts only object-type scope, and stores the five-field record shared by both adapters. Companion-repository evaluation work is tracked there. Decisions: ADR-I-0023 through ADR-I-0028. |
| v0.2 | Situated recall and scoped continuity | The scene on every memory and as the retrieval input, situated activation with a candidate route and an admission floor per cue kind, prospective memory (direction and due date on commitments and open loops, surfacing on their trigger), the currency invariant with staleness reported, treatment by supersession with write-path warnings, scene partitions as explicit policy, selectivity widening or its declination, a pack renderer, and an example loop. |
| v0.3 | Trace, consolidation, and reflection | A short-term store beside core memory written mechanically with no language-model call, recall across both stores including by topic, changes of state discovered at recall, reflection as the library's one producer of interpreted memory, with evidence rules enforced at the write path for reflection and deliberate callers alike, presence accounting, a project-owned default prompt run through a consumer-supplied model, and the behavioral evaluation tier. |
| v0.4 | Temporal validity, attribution, and entity evolution | Validity intervals and volatility, attribution completing the scene, entity aliases and roles over time, current-belief filtering as currency, and source reliability as scoped derived memories. The belief ontology stays behind ADR-D-0005's revisit clause. |
| v0.5 | Long-horizon shape | Currency at scale, consolidation of periphery into gist with provenance, query-time associative activation with no persisted structure, evidence-derived familiarity. Durable associative units enter only on measured demand. |
| Dissolved | Retrieval observability and governance | Delivered in the v0.1 family or moved to the phases and the evaluation repository that need its pieces (section 17). |
| v1.0 | Text-only release-ready state | Defined in section 1: the continuity situation catalog's situations met for text input at both evaluation tiers. Reached by the numbered phases above it. |
| Beyond text | Multimodal and embodied expansion (unnumbered horizon) | Voice beyond transcript, multimodal observations, situation frames, object/place/action memory, and embodied context through symbolic memory objects and opaque external source references. Raw media and sensor logs are not stored by Character Memory core. An aspiration past v1.0, never a v1.0 gate. |

The order from v0.2 onward is by builder-visible value with generation early, because every later structure needs a producer and the evaluation harness can now judge generated memory. Durable association structures stay demand-conditional because they create new edges and should be built only after query-time activation has been measured.

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
Opaque source-reference utilities
Schema/versioning utilities
Stable ID/IRI utilities
Test fixtures
Migration hooks
```

Phase 0 does not implement raw-log storage, raw-log search, public raw-reference resolution, or source-span utilities. Source-reference utilities represent caller-managed source material for provenance. v0.1.3 introduces source-span handling as part of generation-ready write planning.

## Implemented module layout

```text
src/
  adapters.rs
  adapters/
    openai.rs
    openai/
      embedding_provider.rs
    oxigraph.rs
    oxigraph/
      embedded.rs
      http.rs
      rdf_mapping.rs
      shared.rs
      sparql_selectors.rs
      tests.rs
      vocabulary.rs
    qdrant.rs
    qdrant/
      payload.rs
      store.rs
    stats.rs
    stats/
      in_memory.rs
      noop.rs
      sqlite.rs
  api.rs
  api/
    embedding.rs
    types.rs
    types/
      draft.rs
      lifecycle.rs
      retrieval.rs
      write_plan.rs
  composition.rs
  config.rs
  config/
    app_settings.rs
    embedding_provider_settings.rs
  domain.rs
  domain/
    schema.rs
    tests.rs
    write_validation.rs
  errors.rs
  lib.rs
  memory.rs
  models.rs
  models/
    vector.rs
    vector/
      candidate_record.rs
      embedding_model.rs
      record.rs
  policy.rs
  policy/
    embedding_surface.rs
    graph_expansion.rs
    retrieval_selectivity.rs
  ports.rs
  ports/
    embedder.rs
    graph_authority.rs
    retrieval_stats.rs
    source_reference.rs
    vector_candidate.rs
  test_support.rs
  usecases.rs
  usecases/
    correct_forget.rs
    link.rs
    reconciliation.rs
    remember.rs
    retrieve.rs
    write_planning.rs
tests/
  initialization_tests.rs
  public_facade_tests.rs
  retrieval_guardrails_tests.rs
  support/
    base.rs
    basic.rs
    persistent.rs
  write_planning_tests.rs
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

Detailed draft: [`v0_1_starter_episodic_memory.md`](roadmap-phases/v0_1_starter_episodic_memory.md)

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

Detailed draft: [`v0_1_storage_and_backend_contracts.md`](roadmap-phases/v0_1_storage_and_backend_contracts.md)

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

Detailed draft: [`v0_1_1_persistent_graph_authority.md`](roadmap-phases/v0_1_1_persistent_graph_authority.md)

## Intent

Make the v0.1 graph-authoritative architecture durable across process restarts before adding richer continuity and reflection features.

This phase closes the gap where Qdrant candidates may survive restart while the Oxigraph authority required to validate provenance, lifecycle, currentness, supersession, and links may not.

## Goals

```text
support embedded persistent Oxigraph storage configuration
preserve graph-authoritative state across process restarts
keep in-memory graph mode available for deterministic tests
validate restart-safe retrieval
detect Qdrant/Oxigraph drift
prevent vector-only candidates from becoming behavior-influencing memory
document persistence configuration and operational expectations
```

Embedded persistent Oxigraph is the application default.

Persistent graph mode is selected through `GRAPH_STORE_MODE=persistent`; in-memory graph mode is reserved for tests and explicit fixture runs through `GRAPH_STORE_MODE=in_memory`.

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
embedded persistent Oxigraph graph authority implementation
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

Detailed draft: [`v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md`](roadmap-phases/v0_1_2_continuous_entity_selectivity_retrieval_guardrails.md)

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

Detailed draft: [`v0_1_3_remember_intake_interfaces_deterministic_write_planning.md`](roadmap-phases/v0_1_3_remember_intake_interfaces_deterministic_write_planning.md)

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

the generation phase:
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
remember(input, options)
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

## Reflection's integration path

Reflection, the v0.3 producer of interpreted memory (section 14), produces `MemoryCandidate` and `RememberWritePlan` values for this path rather than bypassing validation and commit. There is one completion port and no set of per-step processors.

A reflected candidate passes validation or fails it with a diagnostic, and its trace stays until a plan commits. Richer admission states, such as deferred or needing review, are not part of the v0.3 design and enter only if its plan finds a need. v0.1.3 keeps candidate state simple.

---

# 10. v0.1.4: continuity evaluation harness

Detailed draft: [`v0_1_4_continuity_evaluation_harness.md`](roadmap-phases/v0_1_4_continuity_evaluation_harness.md)

## Intent

Close the gap between structural acceptance tests and the actual product goal before richer continuity features build on the v0.1 substrate.

The philosophy's success criteria are behavioral and longitudinal: recall after long gaps, entity anchoring without hub flooding, stable behavior shaped by history. The v0.1 family has so far been validated through structural acceptance tests and diagnostics. v0.1.4 adds a way to measure whether retrieval actually serves continuity under long-horizon workloads.

The intended sequence is:

```text
v0.1.3 completes the generation-ready write path
v0.1.4 builds the harness that exercises the full write and retrieve paths
v0.1.5 runs the harness, fixes what it reveals, and closes the v0.1 family
v0.1.6 makes zero-infrastructure local deployment possible and removes the service dependency from the default test path
then v0.2 builds scoped continuity on a measured substrate
```

## Goals

```text
build deterministic synthetic long-horizon interaction fixtures
cover months-scale episode accumulation, recurring entities, corrections, supersession, suppression, and thread drift
provide a minimal example assistant loop exercising remember/retrieve/correct/forget/link and prepare/validate/commit
define continuity-oriented retrieval-quality metrics
measure selectivity and fanout behavior under hub-entity stress
measure restart and persistence behavior under eval workloads
produce repeatable machine-readable eval reports
```

## Metrics direction

```text
continuity recall: relevant past episodes surface after long gaps without exact wording
entity continuity: recurring entities anchor recall without flooding context
temporal retrieval quality: recency, order, and interval relevance
correction safety: suppressed and superseded memories stay excluded
rationale quality: retrieved memories carry inspectable retrieval reasons
context pollution rate: low-relevance memories admitted into context packs
fanout discipline: bounded expansion under high-degree fixtures
```

## Non-goals

```text
learned retrieval policy
public benchmark publication
live LLM calls inside deterministic eval runs
CI-blocking quality gates
new memory object types
new public memory facade APIs
```

## Acceptance criteria

```text
Eval runs are deterministic and reproducible under fixed fixtures.
Fixtures include heterogeneous high-degree entities: people, places, projects, topics, objects, and arbitrary custom entities.
The harness exercises lifecycle facades and the prepare/validate/commit path.
Eval reports include metric values and per-query retrieval rationale samples.
Eval runs require no external LLM calls.
Selectivity and fanout measurements are recorded in a form usable for tuning defaults.
```

---

# 11. v0.1.5: eval-driven v0.1 family closeout

Detailed draft: [`v0_1_5_eval_driven_v0_1_family_closeout.md`](roadmap-phases/v0_1_5_eval_driven_v0_1_family_closeout.md)

Closeout report: [`v0_1_5_closeout_report.md`](roadmap-phases/v0_1_5_closeout_report.md) (findings and dispositions, shipped changes, confirmation evidence, deferred findings, v0.2 entry confirmation).

## Intent

Run the v0.1.4 harness against the full v0.1 family feature surface, identify weaknesses, fix them, and close out the v0.1 family before scoped continuity work begins.

## Goals

```text
run the evaluation harness across v0.1 through v0.1.4 behavior
record findings as a structured eval report
classify findings: fix now, defer with rationale and target phase, or accept as designed
fix revealed retrieval, guardrail, write-path, and persistence issues
tune selectivity and fanout defaults (alpha, gamma, relation budgets) from measured data
re-run the harness to confirm fixes and tuned defaults
declare the v0.1 family closed for v0.2 entry
```

## Non-goals

```text
v0.2 continuity concepts
new memory object types
new retrieval signals beyond tuning what exists
harness feature growth beyond what findings require
```

## Acceptance criteria

```text
Eval findings are recorded with severity and disposition.
Every fix-now finding is resolved and covered by a regression test or fixture.
Deferred findings carry rationale and a target phase.
Tuned defaults are documented together with the measurements that justified them.
All v0.1 through v0.1.4 acceptance criteria still pass after fixes.
v0.2 entry is explicitly confirmed against the closed v0.1 family.
```

---

# 12. v0.1.6: embedded vector candidate recall

Detailed draft: [`v0_1_6_embedded_vector_candidate_recall.md`](roadmap-phases/v0_1_6_embedded_vector_candidate_recall.md)

Decisions: ADR-I-0023 (embedded Qdrant Edge is the default vector candidate store), ADR-I-0024 (vector candidate recall reports completeness), ADR-I-0025 (the vector record is a read contract), ADR-I-0026 (raw vector baselines read the retrieval trace), ADR-I-0027 (the embedded engine runs on a blocking owner that flushes every write), ADR-I-0028 (vector prefilters require fully populated current columns and never match unknown values).

## Intent

Complete the zero-infrastructure local deployment story.
Graph authority already defaults to embedded persistent storage and retrieval statistics are file-backed; the vector candidate store is the only component that still requires an external service, which conflicts with desktop-companion and game deployments and keeps a service dependency in the default test path.
Because a second adapter must implement the vector port, this phase also settles the port contract that the structured-verdict work deferred: recall completeness is reported instead of silently degraded, the query is scope-only, and the stored payload is exactly what a reader consumes.

## Goals

```text
add an embedded vector candidate store on the in-process build of the service backend behind the existing vector port, selected by a store-mode setting with its own path setting, shipped at its exact-scan indexing threshold, and made the default vector mode on the phase's own parity evidence
make the port result carry a typed completeness verdict that the retrieval telemetry records and never repairs
reduce the vector payload to its read contract: identity, surface, schema version, and the embedded text as provenance of what was ranked
run one shared contract suite against both adapters, with the embedded adapter exercised unconditionally so the default test path needs no service
move the evaluation repository's vector-only baseline onto the retrieval trace so no consumer depends on a store's private schema
rule that a vector-layer predicate reads only synchronized or immutable values and never matches an unknown one, noting the candidate predicates a later phase may need
```

## Non-goals

```text
changing the authority split, or any retrieval semantics in the service mode for non-empty scopes and non-degenerate queries (the intended empty-scope and zero-norm rules are in scope)
deprecating or altering the service-mode adapter beyond the shared port contract
tuning the embedded index, quantization, or memory-mapping defaults (available in the engine; shipped at the exact-scan threshold this phase)
migration tooling between modes; rebuild from graph authority is the path
multi-process access to the embedded store
a public candidate-search facade
```

## Acceptance criteria

```text
Embedded mode constructs and serves retrieval without any running service.
The shared contract suite produces identical admitted candidate sets and orderings from both adapters while both are below their indexing thresholds; above them a recall comparison is recorded.
Deterministic admission holds in embedded mode; repeated runs are byte-identical.
Embedded state survives process restart.
Retrieval telemetry reports the completeness verdict for every retrieval in both modes.
The default test path requires no vector service; service-gated suites still execute under the service-backed CI job and cannot pass by skipping.
The evaluation repository's vector-only baseline produces its rows from the retrieval trace in both modes.
No public facade change beyond the telemetry field, the published maximum-surfaces-per-object-kind policy value, and the consuming awaitable close that releases the local stores (ADR-I-0027); no retrieval behavior change in service mode for non-empty scopes and non-degenerate queries (the intended empty-scope change: zero candidates, and boundary rejection of an empty configured scope).
```

---

# 13. v0.2: situated recall and scoped continuity

Detailed draft: [`v0_2_scoped_continuity_reflection.md`](roadmap-phases/v0_2_scoped_continuity_reflection.md)

The phase order from v0.2 onward was rearranged on 2026-09-17 by product value: what a builder observes on day one comes first, memory generation moves up because every later structure needs a producer and the evaluation harness can now judge generated memory, and the former observability phase is dissolved into the phases that need its pieces (section 17). The phase's shape was settled the same day against the catalog's section D, a day's recall.

## New concepts

```text
scene                       the circumstances a memory was formed in and the circumstances recall happens in: who is present, where, when, and what is in progress; "what" may reference a thread or open loop the application received from remember, or be inferred from the conversation's recent episodes; only the reference time is required, and a partial scene degrades gracefully
situated activation         recall is activation by the cues the scene supplies plus the topic; a candidate route per cue kind (content, entity and thread and place, time and date, stored trigger) and an admission floor per route (ADR-D-0022); purpose is never a cue (ADR-D-0023); the content, entity, and time routes are defined over a store-neutral candidate contract with a lookup per store, the graph and vectors in durable memory and each entry's text, participants, setting, and timestamp in the short-term store of v0.3, while the state and trigger routes read interpreted durable memory only
prospective memory          open loops and commitments carry a direction (actor, counterpart) and an optional due date, and surface on their trigger: a counterpart appearing, a topic arising, a date arriving
currency invariant          latest in its supersession chain and not resolved; selects versions, never items; staleness reported as age, never enforced (invariant 2.15)
treatment by supersession   a change in how a memory should be treated supersedes it with a restatement; write-path warnings for near-verbatim replacement text and for chain churn; expression quality is the writer's and is measured by the retelling-consistency situation
scene partition             an explicit application-chosen query-time policy over the scene (ADR-D-0019), never a default
pack renderer               a canonical rendering of a pack, gist and stance over quotation
example loop                a minimal retrieve, respond, remember loop in the library's own examples
```

## Goals

```text
let a character arrive carrying what the moment calls for: the people here, what was last said with them, what is owed, what is in progress, what fell due (catalog D1, D4, D5, D8, D11)
keep one character across settings: the scene travels with every memory, partitions are explicit policy
let "you said you would" work in both directions, and let a stored intention surface on its trigger (D7, D8)
admit what the scene brings up beside what the topic brings up, so derived state and evidence turns do not starve each other
apply selectivity beyond entity roots, or decline it with evidence
make time a retrieval cue, so that the README's temporal claim becomes true
```

## Inherited obligations from the v0.1.5 closeout

```text
own the deferred admission/ranking design item: it takes the shape of admission floors for the state and time routes beside the content route, and the ADR-I-0022 baselines are re-measured once
own the deferred selectivity-widening item (non-entity-keyed statistics or explicit declination)
build scoped/person-keyed evaluation scenarios (catalog B1 to B3) before implementing the scene; B1 and B2 measure the scene's presence and non-disclosure, never a failure to surface (ADR-D-0019)
answer the concurrent-facade-call question; with no background derivation inside the library the reflection-scheduling form dissolves
```

## Design items decided 2026-09-17

```text
currency is an invariant, not a retrieval driver; the stored current flag is at most a cache of the chain and joins the value-audit deletion candidates
purpose is emergent and never a field on the retrieval input
open loops and commitments carry an actor, a counterpart, and an optional due date
no reinforce operation; familiarity and stability derive from evidence at query time
the archived and deleted retention states and the archive-thread-derived-memories knob are value-audit deletion candidates; thread status keeps dormant and resolved as currency
the user and assistant entity types are value-audit deletion candidates under ADR-D-0020
scope keys on derived memories are derived from each memory's scene at write time and are never a caller-facing ID scheme; a stored scope object needs a consumer a graph query cannot serve
```

## Not in v0.2

```text
reflection that generates text: the generation phase (section 14); v0.2 may emit a "this scope has accumulated enough to reflect on" signal and nothing more
first-class OpenLoop, Commitment, CharacterSignal, RelationshipState object types: the subtypes stay (ADR-D-0005)
a current-state view type: a scene with no topic is the same retrieval with the content route empty
a scope hint by ID, a goal or purpose field, or any retrieval mode enumeration
involuntary recall from weak cues (D12): the long-horizon phase
the short-term store and the mechanical write: v0.3, beside reflection, since trace is released only by consolidation and would otherwise only accumulate
attribution fields beyond what the scene already implies (v0.4)
```

## Acceptance criteria

```text
With a scene and no topic, retrieval returns what the moment calls for: the people present's current state and last interaction, active loops and commitments in both directions, the activity's thread in order, items due, date matches, and recent high-salience episodes, with elapsed time since the pair last met.
A stored intention surfaces when its counterpart appears or its topic arises, and a promise surfaces on its due date, whatever the current topic.
A memory learned in one setting is admitted when retrieved for another, with its scene reported; a partition applied as a query option omits across the scene and the trace records the applied policy.
Under a loud topic, the state and time routes still admit their floor, and the ADR-I-0022 baselines are re-measured once.
Retrieval produces the temporal rationale category, and the catalog's D1, D4, D5, D7, D8, D9, D11, and D13 situations pass at the retrieval tier.
Currency never removes an item: every omission on lifecycle or currency grounds names a resolution, a supersession, or a suppression, and staleness is reported as age.
A superseding restatement that contains its predecessor nearly verbatim, and a chain that churns, each raise a write-path warning; the retelling-consistency situation is the behavioral check.
Selectivity beyond entity roots is either applied with its new signal or declined with recorded evidence.
The pack renderer and the example loop exist, and the README describes what ships.
```

---

# 14. v0.3: trace, consolidation, and reflection

Detailed draft: [`v0_3_memory_generation_and_reflection.md`](roadmap-phases/v0_3_memory_generation_and_reflection.md)

## Intent

Make the library able to form memory from experience under two constraints: a routine write costs no language-model call, and nothing enters lasting memory without stated evidence the write path can check; whether a judgment is right is measured, not guaranteed. Experience leaves a literal trace at once; the character reaches it immediately; reflection decides afterward what lasts. The behavioral standard is the catalog's sections E and F.

This phase moved up from its earlier position as v0.6 because the write-plan validation path exists (v0.1.3) and the evaluation harness can judge whether formed memory helps or pollutes (v0.1.4 onward). Its shape was worked out on 2026-09-19, subject to the acceptance of the decision records it cites, and replaces the earlier idea of generation on every remember call.

## New concepts

```text
short-term store      beside core memory: scene, raw snippet, time, kind, source pointer; indexed; released only by consolidation and never by expiry, with a loud escalating warning when trace is held too long; covered by purge (ADR-D-0026, ADR-D-0031)
mechanical write      no language-model call, no judgment; lexical indexing by default calls no model at all, and an opted-in vector index costs one embedding per write; exchanges, action lines, notes, and scene boundaries all land as trace (ADR-D-0025)
recall across stores  the short-term store joins the content, entity, and time routes; the state and trigger routes read durable memory only, and surfaced state reaches trace through the one re-cue hop; current-conversation items are marked; the reader comprehends trace as it reads
state before reflection   discovered at recall: a surfaced durable item cues the short-term store one bounded hop; an overlapping new line raises its stretch's reflection signal
entities as notions   an entity records that the character has someone or something in mind; its name, kind, standing, and sameness with another are beliefs with evidence, current by supersession; nothing is merged, split, or deleted (ADR-D-0034)
reflection            reads before it writes; a scope's pass and the day's pass; outputs are the existing memory kinds through prepare, validate, commit; trace is released only after commit
evidence rules        grounding by locator, register, own versus perceived, attribution copied or judged with its basis, the bar following attribution, computed promotion thresholds, contradictions held, trace untrusted (ADR-D-0028)
memory as experience    no confidence score and no numeric emotion; the gist says how the episode felt and that is never revised; impressions are the character's own observations (ADR-D-0030, ADR-D-0033)
the scene as perceived  only the time is required; descriptions, keys where they exist, perception labels; never resolved by the application (ADR-D-0029)
presence accounting   scene boundaries report presence; every present span is covered by its trace and then by a durable account, and only a span with neither is absent; an out-of-band purge is outside this guarantee (ADR-D-0027)
the processor         a project-owned, versioned, overridable default prompt; a consumer-implemented completion port; no model named, no client shipped; structured output validated; which prompt ran is reported on the outcome, not stored per memory (ADR-I-0035)
memory tool           optional; the character's model noting something; a short-term entry with its origin, a claim, never a durable write
```

## Decided constraints

A constraint below that rests on a decision record still marked proposed is provisional until the decider accepts that record, and changes or goes if the record does.

```text
the library executes reflection when the application calls it and starts none on its own; it reports accumulation with its reason, by stretch and for the store as a whole and under a key where the application gave one, since a scope is known only after consolidation, and selects bounded input by stretch; the application decides when
reflection is safe beside recall and writes, and whether to await it is the application's choice
exclusion is applied at the mechanical write, which keeps a marker and none of the content: content excluded when it is written is never stored, so it is never indexed, recalled, or sent to a model; content excluded later, while still unconsolidated, is never sent to a model from that moment on, and if a reflection had already sent it, that reflection writes nothing derived from it so long as the exclusion comes before its claim is marked committing; the trace is data to the model, never instruction
caller-authored plans are held to the same evidence rules with declared grounding, since the library never held the caller's source; existing callers, including the evaluation repository's hand-authored ingestion, must supply what each kind of candidate requires: declared grounding on observations, with register on those of speech, declared source grounding on caller-authored episodes, entities, and beliefs of sameness and containment, the supporting observations and episodes on every other interpreted memory, and attribution with its declared basis on all of them, which the no-backcompat ruling allows and the plan schedules
commitments and instructions are written with stability that keeps them from fading in ranking (ADR-D-0018)
"forget it" defaults to remembering the request; suppression is chosen only when wording and relationship warrant it
a request to reprocess an old scene works only while its trace is held or the application kept the source
ADR-D-0015 stays in force; ADR-I-0013 stays in force, and inference lives behind the completion port
```

## Open discussion

```text
purge propagation: when a source is purged, what happens to restated state that blended it with other sources; to be held before this phase's plan is approved
```

## Acceptance criteria

```text
A mechanical write makes no language-model call, no model call of any kind in the default lexical configuration, and writes only to the short-term store.
Something from earlier the same day is recalled by topic before any reflection, marked recent and unconsolidated.
A task completed or a fact changed before reflection is known at recall when its trace is reached by the recency floor, by topic, or by the re-cue hop from the state it bears on; a change that shares neither words nor meaning with that state and lies outside the recency floor is found at reflection, which the overlap signal and the accumulation warning exist to bring forward; the durable record is unchanged and the miss rate is measured.
Reflection runs end to end through a test double of the completion port; rule-breaking output commits nothing and releases nothing; a second run produces no duplicates.
A quiet present span has a durable account, an absent span has none, and recall tells them apart.
A trait from one episode, state from a non-literal observation, and a commitment from a claim about the character each fail validation; a hostile line produces no unsupported memory.
Names and references in trace become notions and beliefs about them: a reference reflection takes to be someone already known attaches to that notion, one it does not forms a new notion, and sameness between notions is a belief with evidence that a later reflection can supersede; nothing is merged.
A backlog is consolidated in order, and a later reflection can supersede an earlier one's conclusion.
The phase is judged on benchmark results obtained through mechanical writes plus reflection rather than hand-authored ingestion, and on the behavioral tier's first results; the harness work that produces them is planned in the evaluation repository.
The guide page on when to write and when to reflect exists, and the example loop uses both.
```

---

# 15. v0.4: temporal validity, attribution, and entity evolution

Detailed draft: [`v0_4_temporal_validity_attribution_entity_evolution.md`](roadmap-phases/v0_4_temporal_validity_attribution_entity_evolution.md)

## Intent

Give memories a validity in time, a source, and a history of the entities they concern, without introducing a belief ontology. This is the earlier factual-rigor phase narrowed to the parts with builder-visible value.

## In scope

```text
validity intervals and volatility on derived memories: valid from, valid until, review after
attribution: who asserted a memory and whether the character was there when it happened, on observations and derived memories, completing the scene; who-said-it on reflection outputs already arrives in v0.3 (ADR-D-0028)
entity aliases, roles, and relationships over time, without destructive overwrite
current-belief filtering as currency: a derived memory past the validity interval recorded when it was written is not current under invariant 2.15 and stays recallable; staleness without an interval is reported as age, never enforced
source reliability as derived memories about the source, scoped by domain, never a global score
```

## Demand-conditional, behind ADR-D-0005's revisit clause

```text
Assertion, Claim, EvidenceLink, BeliefAssessment, SourceAssessment as first-class objects
a normalized belief ontology and contradiction machinery
```

These enter only when measured use shows corrections and contradictions frequent enough that derived subtypes no longer carry them.

## Acceptance criteria

```text
A derived memory can carry a validity interval, and an expired one is omitted from current views with that reason and remains recallable.
Every observation and derived memory reports who asserted it and whether it was firsthand or told.
The system can represent that an entity had one name, role, or relationship during one interval and another later.
Hearsay and being-told-about-yourself scenarios (catalog C3, C5) pass at the retrieval tier.
Corrections and supersession remain provenance-preserving.
```

---

# 16. v0.5: long-horizon shape

Detailed draft: [`v0_5_long_horizon_shape_and_associative_recall.md`](roadmap-phases/v0_5_long_horizon_shape_and_associative_recall.md)

## Intent

Keep a memory that accumulates for years usable and human-comparable without losing anything: currency at scale, consolidation of periphery into gist with provenance, and query-time associative recall. Decay is not a mechanism (ADR-D-0018); bounded context comes from ranking, consolidation, and currency.

## In scope

```text
currency at scale: relationships that ended, threads that resolved, signals no longer reinforced leave current views and stay recallable
consolidation: periphery summarized into gist as derived memories with provenance, through the generation port
query-time associative activation over the existing graph, bounded by selectivity, with no persisted structure
recognition and familiarity derived from evidence: a returning stranger is recognized where a person would not
involuntary recall from weak partial cues (catalog D12), low precision and low cost when wrong
persisted retrieval footprints only if repeated-coactivation signals need history, decided by measurement
connections across scopes noticed at consolidation: the idea from one conversation that belongs to another person's birthday
which durable surfaces need vectors, decided by measurement, since scene, time, and entity routes reach what is not embedded
```

## Demand-conditional

```text
durable AssociativeUnit, AssociativeMembership, and AssociationSupport structures with member-level lifecycle (ADR-D-0013, ADR-D-0014, ADR-I-0014, ADR-I-0017 stay in force as the design for when they enter)
```

They enter only when long-horizon evaluations show query-time activation alone misses or costs too much.

## Acceptance criteria

```text
Long-horizon scenarios (catalog C2, C6) pass: intimates stay rich, periphery is gist, a departed person is remembered fully and no longer shapes default behavior.
Query-time activation can retrieve weakly related memories without creating durable pairwise edges, and the trace shows the activation path.
No memory's eligibility changes with time; every omission from a current view names a currency decision.
```

---

# 17. Dissolved: retrieval observability and governance

The earlier v0.4 phase is dissolved as of 2026-09-17 and has no draft. Its pieces went where they are needed:

```text
retrieval traces, section assignments, selectivity and expansion traces, lifecycle omissions   delivered in the v0.1 family; each later phase adds the trace fields its mechanism needs
RetrievalIntent (ADR-I-0016, unchanged: all five variants remain the enum)   Continuity is the only variant v0.2 implements, since a scene with no topic serves the current-state case; CurrentState, CorrectionReview, and SourceAudit are implemented by the phases that need them (temporal validity at the latest); AssociativeProbe with query-time activation
retention assessment and retention policy hooks   replaced by currency; the draft's archived state contradicted ADR-D-0018, and its redacted and deleted states contradicted ADR-D-0021
validation rules   the write path already validates; invariant checks over stores are evaluation-repository tooling (ADR-I-0019)
graph health reports and policy diagnostics   evaluation-repository reports over runs, not library surface
persisted first-class RetrievalTrace objects   only if repeated-coactivation signals need retrieval history (v0.5), decided by measurement
```

---

# 18. Beyond text: multimodal and embodied expansion (unnumbered horizon)

Detailed draft: [`beyond_text_multimodal_embodied_expansion.md`](roadmap-phases/beyond_text_multimodal_embodied_expansion.md)

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

This is an aspiration past the v1.0 text-only release-ready state. It carries no version number and is never a v1.0 gate.

---

# 19. Public API evolution

## v0.1 API

```rust
let memory = CharacterMemory::new(settings, collection_name).await?;
let stored = memory.remember(remember_input, remember_options).await?;
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
remember(input, options)
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

## v0.1.4 / v0.1.5 API surface

The continuity evaluation harness is implemented in the public companion `CharacterMemoryEvals` repository as a development and measurement aid, not core library functionality. It adds no public memory facade APIs.

v0.1.5 retained the measured configuration defaults (ADR-I-0022) and added no new facade methods; it did add public diagnostics — warning fields on lifecycle mutation outcomes and validation warnings on write plans, with their types re-exported from the crate root — and removed the Oxigraph service mode from the configuration surface (ADR-I-0021).

## v0.2 API additions

Illustrative shape; retrieval takes the present scene, built from the same information remember already takes (participants, source conversation, timestamps), and a renderer turns a pack into prompt text. There is no scope hint by ID, no purpose field, and no separate current-state call: a scene without a topic is the same retrieval. The scene is given as perceived: keys are shown here because this example has them, and a description in words, or nothing but the time, is equally valid (ADR-D-0029).

```rust
let scene = Scene::now()
    .with_participants([self_id, alice_id])
    .with_conversation("channel-42")
    .with_activity(thread_id);
let outcome = memory.retrieve(RetrievalContext::in_scene(scene).with_topic("what were we working on?")).await?;
let prompt_text = outcome.pack.render(RenderStyle::default());
```

## v0.3 API additions

Illustrative shape; the mechanical write, the signal, and reflection through the consumer's model.

```rust
let memory = memory.with_completion_provider(provider);
let scene = Scene::now().described("the kitchen of Kohta's house, evening; Kohta is cooking"); // as perceived; keys are optional
let written = memory.trace(&scene, Trace::perceived("raw text of what was heard")).await?; // Trace::own(..) for the character's output
if written.signal.level >= SignalLevel::Ready {
    // the application decides when; reflection may run in the background
    let outcome = memory.reflect(&scene, ReflectOptions::default()).await?;
}
let snapshot = memory.reflection_signal(&scene).await?; // optional: the same signal without a write or a recall
```

## v0.4 API additions

Illustrative shape; validity and attribution are fields on the existing drafts, and current-belief filtering is currency.

```rust
let draft = DerivedMemoryDraft::new(DerivedType::Claim, text)
    .valid_from(when)
    .asserted_by(source_entity_id, Attribution::Told);
```

## v0.5 API additions

Illustrative shape; activation is a retrieval option, not a new surface.

```rust
let context = RetrievalContext::new(query).with_intent(RetrievalIntent::AssociativeProbe);
```

---

# 20. YAGNI rules

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

Do not implement before durable associative units are demand-confirmed (v0.5 at the earliest):

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
