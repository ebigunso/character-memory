# Graph Database Schema Design

This document describes the Oxigraph/RDF graph schema design for Character Memory. It focuses on why the graph is shaped this way, not on exhaustively restating the Rust mapping code.

The graph store is the authority for memory objects, relationships, provenance, lifecycle state, currentness, and bounded expansion. Qdrant can suggest candidates, but Oxigraph decides what those candidates mean and whether they belong in retrieved context.

## Design Goal

Character Memory needs more than a ranked list of text snippets. It needs to answer continuity questions:

```text
What happened?
Who or what was involved?
Which memories came from this episode?
Which memories are current?
Which memories were corrected or suppressed?
What nearby context should travel with this candidate?
```

Those questions are graph questions. The schema therefore prioritizes stable identity, typed object boundaries, inspectable links, lifecycle filtering, and bounded traversal.

## Backend Boundary

Embedded in-memory Oxigraph is the default graph authority. Persistent Oxigraph storage configuration is v0.1.1 future work.

The public domain model does not expose Oxigraph types. Domain objects are mapped into RDF at the infrastructure edge. This keeps the public API stable if the backing graph implementation changes later.

Raw transcript storage remains outside the graph boundary in v0.1. The graph may carry source pointers, but production raw storage is caller-owned/deferred and raw-reference resolution is not a public graph API.

## Identity Model

Every graph resource uses a deterministic URI derived from object type and UUID:

```text
urn:cmem:episode:<uuid>
urn:cmem:observation:<uuid>
urn:cmem:entity:<uuid>
urn:cmem:thread:<uuid>
urn:cmem:derived-memory:<uuid>
urn:cmem:link:<uuid>
```

The graph also stores the UUID, object type, graph URI, and schema version as literal properties.

This is redundant by design. The URI is efficient for graph edges, while the literal fields make debugging, migration checks, and cross-store joins easier. Qdrant carries the same `object_id` and `graph_uri` so vector candidates can be joined back to graph truth without guessing.

## Object Classes

The graph uses one RDF class per canonical memory object:

```text
Episode
Observation
Entity
MemoryThread
DerivedMemory
MemoryLink
```

This object-backed graph is the core philosophical choice. Episodes, observations, and derived memories are different kinds of memory evidence. They should not collapse into one generic note because correction, provenance, and retrieval policy need to treat them differently.

## Relationship Strategy

The graph stores relationships in two forms:

1. Direct typed relation triples between resources.
2. Reified `MemoryLink` resources that preserve link identity, endpoint types, confidence, rationale, and creation time.

The direct triples make traversal simple:

```text
<derived-memory> urn:cmem:relation:derived_from <episode>
<derived-memory> urn:cmem:relation:part_of_thread <thread>
```

The reified `MemoryLink` object keeps the relationship inspectable as domain data:

```text
<link> from <derived-memory>
<link> to <thread>
<link> relation "part_of_thread"
<link> confidence "0.9"
<link> rationale "..."
```

This dual representation avoids a bad tradeoff. Direct triples alone are easy to traverse but lose link metadata. Reified links alone preserve metadata but make common traversal heavier. Keeping both gives retrieval fast graph expansion and keeps explanations auditable.

## Provenance Shape

Derived memories carry explicit provenance edges to episodes and observations:

```text
derivedFromEpisode
derivedFromObservation
```

This is intentionally narrower than a generic "source" blob. Corrections and forget operations need to find derived memories affected by a source episode or source observation. Dedicated provenance predicates make that query direct and keep source-cascade behavior deterministic.

Raw source material is not stored in the graph. Objects may carry `rawRef` pointers so callers can associate memories with original transcript material elsewhere. A `rawRef` is a source pointer, not the transcript content.

## Lifecycle Shape

Lifecycle state is graph-authoritative. The graph stores fields such as:

```text
retentionState
isCurrent
supersedes
threadStatus
```

The reason is correctness under partial vector maintenance failure. If a memory is corrected or suppressed in the graph but Qdrant still returns a stale vector, retrieval must omit it by consulting graph lifecycle state.

Supersession is represented as a relationship rather than overwriting history. That preserves the prior memory for audit/historical retrieval while making the replacement memory visible by default.

## Core Predicate Groups

The vocabulary is grouped by purpose.

### Common Object Properties

```text
objectId
objectType
graphUri
schemaVersion
createdAt
updatedAt
```

These make graph resources self-describing and migration-aware.

### Episode And Observation Properties

```text
modality
sourceConversationId
startedAt
endedAt
participantEntity
summary
rawRef
episode
speakerEntity
observedAt
text
salienceScore
retentionState
```

Episodes summarize interaction spans. Observations represent salient pieces inside or from those spans. This lets retrieval include either broad context or specific evidence without treating raw transcripts as memory objects.

### Entity And Thread Properties

```text
entityType
name
alias
canonicalKey
title
threadStatus
lastTouchedAt
summary
```

Entities and threads are continuity anchors. They help memories cluster around people, projects, topics, open loops, and recurring concerns.

### Derived Memory Properties

```text
derivedType
text
derivedFromEpisode
derivedFromObservation
partOfThread
aboutEntity
confidence
stability
isCurrent
supersedes
salienceScore
retentionState
```

Derived memories are explicit interpretations: preferences, reflections, relationship notes, commitments, corrections, and similar continuity signals. They need provenance and lifecycle fields because they are the most likely memory type to be corrected over time.

### Link Properties

```text
from
fromType
to
toType
relation
rationale
confidence
createdAt
```

The endpoint type literals are redundant with endpoint URIs, but they make link validation and diagnostics straightforward and avoid requiring URI parsing to understand a link.

## Query Patterns The Schema Optimizes

The schema is designed around retrieval and lifecycle operations:

```text
resolve vector candidates by object id / graph URI
expand nearby objects through typed links
find derived memories by source episode or observation
find derived memories by thread
exclude suppressed, archived, deleted, non-current, or superseded objects
trace why a relationship was included
bound traversal by depth, fanout, object type, relation type, and lifecycle
```

The graph is not optimized for unconstrained exploration. Character Memory retrieval should be bounded because user and assistant entities can become hubs. The schema supports expansion, but the retrieval layer controls fanout and limits.

## Why Not Store Everything As Triples Only

The embedded Oxigraph adapter also keeps canonical domain objects in memory for contract reads while materializing RDF triples. That is an implementation boundary, not a rejection of RDF.

It keeps the graph authority simple and deterministic:

- domain objects can round-trip without lossy RDF reconstruction
- tests can validate graph behavior without committing to full SPARQL object hydration yet
- RDF triples still exist for relationship mapping and future query expansion

Persistent graph storage can later move more responsibility into SPARQL-backed queries without changing the public object model. That persistent authority work remains scoped to v0.1.1-era follow-up, not the v0.1 in-memory boundary.

## Cross-Store Contract

Qdrant and Oxigraph share stable IDs and graph URIs, but they do not share authority.

```text
Qdrant   recalls candidates and applies coarse payload filters
Oxigraph verifies existence, relationships, provenance, lifecycle, and context
```

This is why the vector payload intentionally duplicates some graph-derived hints. Duplication is acceptable for speed as long as retrieval treats those hints as non-authoritative.

## Future Revisit Points

Revisit this design when:

- v0.1.1 persistent Oxigraph storage becomes configurable
- SPARQL query helpers replace more in-memory object-side reads
- belief/claim tracking adds richer factual rigor semantics
- some relation types become important enough to deserve specialized objects
- graph migrations need compatibility across stored schema versions
