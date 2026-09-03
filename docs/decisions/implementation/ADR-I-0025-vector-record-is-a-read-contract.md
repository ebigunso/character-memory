---
status: accepted
adr_type: implementation
date: 2026-09-02
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
warrant:
  warranted_by: "without this record, future work would likely re-add relationship, lifecycle, time, or readable-text columns to the vector record because the earlier payload design lists them as intended, or delete the embedded-text column as unread once surfaces become generated"
  detected_signals: "cross-boundary contract shape (two adapters mirror one record); rejected alternative likely to be re-proposed; premises likely to expire (a retrieval route may need a prefilter); cross-repository obligation (the evaluation baseline read a payload column)"
  cost_of_violation: "every column that returns without a reader is mirrored across two adapters, indexed at every collection initialisation, and carried stale by write paths that never update it; a column deleted as unread would erase the only record of what a generated vector embedded"
  cost_of_wrong_preservation: "if a retrieval route needs a prefilter and the five-column rule is preserved as prohibition rather than current state, the predicate is blocked instead of landing through the named re-entry path"
  cost_of_over_extension: "extending the rule to the graph store would strip graph authority of denormalised fields it legitimately owns"
depends_on: [implementation/ADR-I-0007-schema-versioning.md, implementation/ADR-I-0024-vector-candidate-recall-reports-completeness-and-prefilters-never-match-unknown.md]
implements: []
supersedes: [implementation/ADR-I-0005-qdrant-payload-vs-graph-authority.md, implementation/ADR-I-0002-natural-language-embedding-surfaces.md, implementation/ADR-I-0001-stable-cross-store-ids.md]
superseded_by: null
supersession_scope: partial
---

# ADR-I-0025: The vector record is a read contract — identity, surface, schema version, embedded text

## Context and Problem Statement

ADR-I-0005 decided that the vector payload stores filterable metadata and graph pointers, and the payload design note enumerated thirty-four fields with thirty-one of them indexed; the implemented manifest carried thirty-three with thirty indexed after the record-type field was dropped in the structured-verdict phase.
By the time the embedded adapter (ADR-I-0023) was designed, the library read back exactly three of those fields — object id, object type, surface — and the only external reader was the companion evaluation repository's vector-only baseline reading the readable text column.
The relationship hints were frozen at upsert and never updated by the link write path; the lifecycle hints described vectors the correction and forgetting paths delete; the readable text column duplicated graph text with a prefix removed; and every field was about to be mirrored into a second physical schema.
ADR-I-0002's implementation note said to "persist both `embedding_text` and `content_text` where useful", which left the two text columns' meanings undefined.
The forward-looking case for each family was analysed against the planned phases (scoped continuity, factual rigor and temporal validity, retrieval observability, associative recall, assisted remember, multimodal) before deciding.

## Decision Drivers

- A column earns its place when a reader exists; carrying it unread costs two adapter mappings, index creation per collection, a parity fixture, and the sync discipline ADR-I-0005 named in its own tradeoffs.
- Prefilter hints are only safe on immutable or synchronised data; the relationship, lifecycle, ranking, and mutable time hints were none of those.
- Re-adding an immutable column later is a backfill from graph authority, not a re-index.
- Once embedding surfaces are generated or caller-supplied (the assisted-remember phase; the write plan already carries a caller-supplied surface), the text a vector embeds is no longer re-derivable from graph authority, so it is provenance in the philosophy's sense.
- Read-out text is graph authority's job; the vector layer suggests, it does not describe.

## Decision

Both adapters persist exactly these fields per vector record: object id, object type, surface, schema version (ADR-I-0007), and `embedding_text`.

Three sentences govern the text columns:
Read-out text lives in graph authority.
The vector record stores only the embedded surface, as provenance of what was ranked.
Consumers needing candidate content hydrate by object id.

`content_text` is dropped.
The relationship refs (episode, observation, thread, entity, participant, speaker, supersedes), the lifecycle and currentness flags, the time hints, the ranking and salience hints, the object-specific hints, the graph URI, and the raw source reference leave the vector write path.
Dropping the graph URI partially supersedes ADR-I-0001's clause that every vector payload carries it: the stable object id remains the cross-store identity and the graph URI is derived from it by graph authority, so the pointer was a redundant copy of the id; ADR-I-0001's stable-id decision itself is unchanged.
The typed field manifest introduced in the structured-verdict phase remains the single source of both adapters' column sets and shrinks to the five entries.
ADR-I-0024 rules that a predicate reads only synchronised or immutable values and notes the two candidate predicates (a synchronised scope id; an immutable time window over `created_at` and `observed_at` backfilled from graph authority), so a returning column arrives with its predicate and its reader.

## Implementation Impact

- The vector record type and the surface builders lose the hint carriers; the payload map serialises five fields for both adapters, which share the engine family's payload conventions.
- The service adapter stops creating per-field payload indexes for dropped fields.
- The companion evaluation repository's vector-only baseline stops reading the readable text column and sources item text from its own ingest records (ADR-I-0026).
- The payload design note's field categories and indexing policy are superseded by this record and carry a supersession note.
- No migration: under the Compatibility Policy, existing stores are rebuilt from graph authority.

## Considered Options

1. Five-column read contract; keep `embedding_text` only; drop the hint families with named re-entry paths.
2. Keep both text columns.
3. Drop both text columns.
4. Keep the unread hints for the planned phases.
5. Keep only the two immutable timestamp columns as a hedge against top-K starvation.

## Decision Outcome

Chosen option: **Option 1**.
It stores what is read, keeps the one column that becomes non-re-derivable, and prices re-entry honestly.

### Rejected Alternatives

Option 2: `content_text` is a deterministic function of graph object fields at every surface builder, so it never carries information graph authority lacks, and its one reader moves to its own ingest records; rejected outright.
Option 3: `embedding_text` is cheap and becomes the only record of what a generated or caller-supplied vector embeds; rejected outright.
Option 4: no planned phase names a vector-layer predicate the existing fields could serve without new synchronisation work — scoped retrieval needs a scope id kept in sync by linking, temporal validity is a ranking property of new claim objects, salience evolves by reinforcement, and lifecycle hints describe vectors the write path deletes; reopened only through the re-entry paths.
Option 5 is the only subset with a forward-looking case that survives the synchronisation test; it was declined because no phase document asks for the predicate and the columns backfill cheaply when one does.

## Consequences

- Positive: both adapters mirror one five-field manifest; the embedded shard stores those fields as payload beside the vector with keyword indexes on object id (the delete selector) and object type (the scope predicate); ADR-I-0023 owns the physical layout.
- Positive: the embedded surface is preserved as vector provenance before surfaces become generated.
- Negative / tradeoffs: a future scoped or time-bounded prefilter requires a backfill and a schema-version step rather than a query-only change; the re-entry paths make that step predictable.

## Decision Boundary

Invariant: the vector record carries only fields a reader consumes, plus the embedded surface as provenance and the schema version ADR-I-0007 requires; readable content is hydrated from graph authority by object id; a returning hint arrives with its predicate and parity fixture through ADR-I-0024's re-entry paths.

Not covered: the physical encoding of each column per adapter, and graph authority's own denormalised fields.

## Validation

- A census of both repositories shows no reader of the dropped fields and no reader of `content_text`.
- The manifest test asserts the five entries; the parity suite serialises and reads them through both adapters.
- The evaluation baseline reproduces its results with text sourced from ingest records.

## Revisit When

- A retrieval route needs a scoped or time-bounded semantic search — add the column under ADR-I-0024's prefilter rule; this record's invariant is satisfied by a column that arrives with its reader.
- The assisted-remember phase makes the embedding surface a graph-authoritative provenance artifact — the vector copy becomes a cache and this record's provenance argument moves to the graph.
- A re-indexing workflow appears that cannot rebuild from graph authority — the readable-text question reopens with that workflow as its reader.

## Consultation impact

Question asked: whether the unread hint families and the readable text column should be kept for planned phases; ruling adopted the five-column contract with the three governing sentences and the named re-entry paths.

## More Information

- ADR-I-0005 remains authoritative for graph authority over relationships; this record supersedes its payload field list and its "payload metadata as candidate filter" implementation guidance.
- ADR-I-0002 remains authoritative for natural-language embedding surfaces; this record supersedes only its note to persist both text columns.
- ADR-I-0024 (completeness verdict and prefilter rule), ADR-I-0023 (embedded shard layout), ADR-I-0026 (evaluation baseline reader).
