# v0.5 Design Draft: Controlled Associative Recall and Clustering

## Version intent

Add human-like serendipitous associative recall without weakening the v0.1.2 guard against low-information pairwise graph growth.

The system should support:

```text
"This reminds me of that."
```

without creating:

```text
every memory sharing a broad entity is permanently associated with every other memory sharing that entity.
```

## Design priority order

This phase should optimize in this order:

```text
1. Retrieval quality
2. Retrieval-time performance
3. Management overhead reduction
```

Management cost matters, but it should not be reduced by flattening away member-level status, provenance, or retrieval rationale needed for high-quality long-term recall.

## Core design decision

Use graph-internal associative structures with member-level lifecycle.

Do not use:

```text
separate weak hint store
ordinary low-value pairwise associated_with edges
cluster-level status as a substitute for member-level status
```

Use:

```text
AssociativeUnit
AssociativeMembership
AssociationSupport
```

## Why cluster-level status is not enough

An active associative cluster may receive new possible members over time.

A new memory that appears related to an active cluster should not automatically be treated as equally established with existing core members.

Example:

```text
AssociativeUnit:
  Alice + rainy-weather planning
  status = Active

Existing members:
  Alice mentioned rainy weather.
  Alice talked about buying rain boots.

New possible member:
  Alice mentioned waterproof paint.
```

The unit may be active, but the new memory should begin as a candidate or peripheral member.

Therefore the model needs two lifecycle levels:

```text
AssociativeUnit lifecycle:
  Is this associative structure valid, active, retired, or rejected?

AssociativeMembership lifecycle:
  Is this specific memory a candidate, active, rejected, or retired member of the unit, and what role does it play?
```

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

## AssociativeUnit

An `AssociativeUnit` represents an associative recall structure.

It may be shaped as:

```text
Pair
CueBundle
Cluster
ScopePattern
```

It may have lifecycle status:

```text
Candidate
Active
Retired
Rejected
```

Suggested fields:

```text
id
unit_type
status
scope_id
cue_entity_ids
cue_concept_ids
cue_terms
summary_text
strength
salience
confidence
created_at
updated_at
last_reinforced_at
review_after
membership_materialization
```

`membership_materialization` may be:

```text
Full
Partial
ExemplarOnly
SummaryOnly
```

Default preference should be `Full` when practical. Exemplar-only or summary-only clusters should be explicit, not silently treated as complete membership.

## AssociativeMembership

An `AssociativeMembership` represents one memory's participation in an `AssociativeUnit`.

Suggested fields:

```text
id
unit_id
member_memory_id
membership_status
member_role
membership_strength
membership_confidence
membership_salience
supporting_signal_count
added_at
updated_at
last_reinforced_at
review_after
rationale
```

Possible `membership_status` values:

```text
Candidate
Active
Retired
Rejected
```

Possible `member_role` values:

```text
Core
Exemplar
Peripheral
Bridge
Outlier
```

Meaning:

```text
Core:
  central member that strongly defines the associative unit

Exemplar:
  good member to show when summarizing or sampling the unit

Peripheral:
  related but not central

Bridge:
  member that connects this unit to another theme or scope

Outlier:
  weak or unusual member that may be useful in exploratory recall but should not influence ordinary retrieval strongly
```

An active unit may contain memberships with different statuses and roles. For example, it may include active-status core-role members, candidate-status peripheral-role members, active-status bridge-role members, active-status outlier-role members, or retired-status memberships retained for audit/history.

## AssociationSupport

`AssociationSupport` explains why an associative unit or membership exists.

Suggested fields:

```text
id
support_target_id
support_kind
source_memory_ids
retrieval_trace_id
weight
created_at
rationale
```

Support kinds may include:

```text
SemanticCoactivation
SharedSelectiveEntity
SharedConcept
SameScope
SameThread
TemporalProximity
RepeatedRetrievalTogether
ExplicitApplicationLink
ReflectionRationale
CorrectionChain
CommitmentLifecycle
```

Association support should be used to explain promotion, demotion, decay, or retrieval inclusion decisions.

## Query-time activation

Before creating or updating any persistent associative unit, retrieval should be able to activate memories through the existing graph.

Activation uses:

```text
semantic similarity
entity cues
concept cues
thread cues
scope cues
temporal cues
salience
currentness
correction/supersession relevance
selectivity scores
```

Low-selectivity cues transfer less activation. High-selectivity cues transfer more activation.

Query-time activation supports serendipitous recall without immediately creating durable graph structure.

## Persistence rule

Do not create associative units for every weak coactivation.

Create or update associative units only when one or more of these holds:

```text
same memories coactivate repeatedly
same cue bundle recurs
same scope + cue + members appear across retrievals
high-salience memories coactivate
reflection identifies a pattern
application explicitly creates a link
the unit would reduce repeated expensive retrieval
```

## Promotion policy

A candidate membership may become active only with enough support.

Suggested rule:

```text
promote if:
  membership_strength >= configured_threshold
  AND supporting_signal_count >= 2
  AND support is not broad-entity-only
```

Promotion signals may include:

```text
strong semantic similarity to unit summary/cues
same scoped continuity context
same thread
high salience
repeated retrieval coactivation
temporal pattern
explicit user/application link
reflection rationale
shared selective cue
```

A membership must not be promoted solely because it shares a low-selectivity entity with the unit.

## Decay and retirement policy

Candidate memberships should decay if not reinforced.

Possible rules:

```text
Candidate -> Retired
  if not reinforced after review_after

Candidate -> Rejected
  if judged noisy or contradicted

Active membership role -> Peripheral
  if the unit evolves away from that member but the membership remains valid

Active -> Retired
  if rarely retrieved, low salience, and no longer useful even as a peripheral member

Any membership -> InvalidForRetrieval
  if source memory is suppressed or deleted
```

Lifecycle/currentness/suppression of the underlying memory remains authoritative in Oxigraph.

Associative membership cannot override suppression, deletion, supersession, or currentness.

## Retrieval behavior

Retrieval should use both unit-level and membership-level state.

For ordinary retrieval:

```text
use Active units
prefer Core and Exemplar memberships
include Active memberships when query support is strong
avoid Candidate-status or Peripheral-role memberships unless the query specifically supports them
```

For exploratory associative retrieval:

```text
Candidate-status memberships and Peripheral or Bridge-role memberships may be considered,
but expansion remains bounded and explainable.
```

For broad entity-only queries:

```text
return summaries, top salient memories, active threads, and core/exemplar memberships only
do not expand all candidate-status or peripheral-role members
```

For cue-specific queries:

```text
use semantic/time/thread/scope/salience support to include relevant candidate-status or peripheral-role members
```

## Maintenance strategy

Do not recluster on every write.

On normal memory write:

```text
store ordinary entity/thread/concept links
update retrieval stats
mark relevant scopes/entities/concepts dirty
```

At retrieval time:

```text
use query-time activation
consider matching associative units
score memberships
include bounded top members
```

After retrieval:

```text
reinforce only memberships actually included in final context,
selected by reranker,
used in answer,
or marked useful by the application
```

During consolidation:

```text
process dirty scopes or units
promote candidate memberships
demote weak active memberships
retire stale candidate memberships
merge overlapping units
split incoherent units
update summaries
select exemplars
```

## Goals

```text
support human-like serendipitous recall
avoid broad-entity clique growth
represent associative structures inside graph authority
track member-level status, role, strength, and rationale
support query-time activation before durable association
promote associations only with repeated or multi-signal support
use summaries and exemplars for retrieval quality, not as hidden substitutes for lost membership
keep cluster expansion bounded and explainable
```

## Non-goals

```text
ordinary pairwise weak associated_with edges
separate non-graph weak hint store
global graph centrality as memory importance
unbounded spreading activation
automatic clique creation around recurring entities
summary-only clusters by default
cluster membership overriding memory lifecycle/currentness
```

## Acceptance criteria

```text
An active AssociativeUnit can contain memberships with Candidate, Active, Retired, or Rejected status and Core, Exemplar, Peripheral, Bridge, or Outlier role as appropriate.
A new memory can be proposed as a Candidate membership in an Active unit without being treated as equally established.
Ordinary retrieval prefers Core, Exemplar, and strongly supported Active memberships.
Candidate memberships require direct query support or exploratory mode before inclusion.
Broad entity-only evidence cannot promote a membership.
Repeated coactivation or multi-signal support can promote a membership.
Suppressed, deleted, non-current, or superseded memories are excluded by graph authority even if they remain cluster members.
Query-time activation can retrieve weakly related memories without creating durable pairwise edges.
AssociativeUnit expansion is bounded by selectivity, relation policy, lifecycle, currentness, scope, and membership status.
Cluster summaries preserve provenance to source episodes, derived memories, or memberships.
Diagnostics can explain why a member was included, excluded, promoted, demoted, retired, or rejected.
```

## Revisit when

Revisit if member-level lifecycle proves too heavy for implementation. The first simplification should be reducing optional fields, not removing member-level status.

Do not collapse to cluster-level status only unless retrieval quality tests show member-level status is unnecessary.
