# High-Level Design & Development Roadmap

## 1. Overview

This repository implements a **memory storage + retrieval library** built on:

* **Qdrant** for vector embeddings and lightweight payload filtering.
* **Oxigraph** (RDF/SPARQL) for explicit relationships and structured queries.
* A stable cross-store join key: the same **UUID `id`** exists in Qdrant payloads and the RDF graph (as a deterministic IRI).

The library’s job is to reliably **ingest**, **index**, **query**, and **return structured retrieval results**. **LLM integration is intentionally not implemented here** and is handled by callers.

---

## 2. Scope and Non-Goals

### 2.1 Library scope

This repository provides:

* Memory ingest/upsert into **Qdrant** (vectors + payload schema).
* Memory graph modeling and upsert into **Oxigraph** (RDF triples).
* Hybrid retrieval APIs (vector-first, graph-first, and combined) producing a **RetrievalBundle**.
* Deterministic ID/IRI strategy, schema/versioning conventions, and regression tests for Qdrant filters and SPARQL queries.
* Performance controls for graph expansion (breadth/depth limits, caching hooks, hub-entity handling).

### 2.2 Non-goals (explicit boundary)

This repository does **not** provide:

* LLM calls, prompt templates, agent orchestration, or tool-calling loops.
* “Planning” and “iterative retrieval” driven by LLM outputs.
* UI/UX, chat endpoints, or product-level assistant behavior.

**Callers** (apps/services) own LLM selection, prompting, iterative reasoning, and rendering—using this library’s retrieval outputs.

---

## 3. System Invariants (Non-Negotiables)

1. **Shared ID discipline**

   * Every memory has a UUID `id`.
   * Qdrant payload stores `id`; Oxigraph uses a deterministic `memory_iri(id)` resource.

2. **Qdrant payload schema is canonical for the vector layer**

   * Always: `id`, `memory_type`, `content`
   * Episodic-only: `timestamp`, `location_text`, `participants`

  Notes:

  * Payload stores metadata only (no embedding vectors in payload).

  Filtering notes:

  * `participants` and `location_text` should support word-level full-text matching (Qdrant text index + text match conditions).

3. **Graph layer tolerates partial information**

  * Missing fields simply mean “missing triples,” never ingest failures.
  * This does not override vector-layer requirements for episodic memories: episodic records must include timestamp/location/participants (use explicit placeholders like "unknown" instead of null).

4. **Hybrid retrieval is an API contract**

   * Typical library flow: vector search → candidate ids → graph expansion → merged bundle.

---

## 4. Technology Stack (Library)

* **Vector DB:** Qdrant
* **Graph DB:** Oxigraph (RDF store; queried via SPARQL)
* **Embeddings:** caller-provided embedding function or pluggable provider interface (the library defines the interface; implementation can be bundled or injected depending on your packaging preference)

### 4.1 Oxigraph modeling implications

* “Nodes” are RDF resources; “edges” are RDF triples.
* “Node types” are RDF classes; properties are predicates.
* Queries are **SPARQL**; core SPARQL must be regression-tested.

Recommended IRI schemes (example):

* Memory: `urn:am:memory:<uuid>`
* Entity: `urn:am:entity:<normalized-or-hash>`
* Location: `urn:am:location:<normalized-or-hash>`
* Date: `urn:am:date:<YYYY-MM-DD>` (optional)

---

## 5. Public API Contract (Library → Caller)

Callers should integrate against a small, stable set of interfaces.

Design principle:

* **Usability first:** callers should use a single, high-level hybrid retrieval API (`hybrid_search`) rather than composing vector-only and graph-only stages.
* **Traceability separately:** internal stage outputs (vector candidates/scores, graph expansions, merge decisions) can be exposed as an *optional* retrieval trace for debugging and provenance, without requiring callers to implement selection logic.

### 5.1 Core types

* `MemoryRecord`

  * `id: UUID`
  * `memory_type: episodic | semantic`
  * `content: string`
  * episodic-only (required): `timestamp`, `location_text`, `participants[]`

    * Use explicit placeholder values (e.g., `location_text = "unknown"`) instead of null when the value is not known.

* `RetrievalBundle`

  * `query`
  * `results[]`: `{ id, content_excerpt, payload_metadata, graph_context }`
  * `entities[]`, `locations[]`, `time_anchors[]` (normalized aggregates)
  * `provenance`: stable IDs suitable for caller-side citations

  Optional trace (debug/provenance aid; not required for callers):

  * `trace?`: includes internal stage diagnostics such as vector candidate ids + scores, graph expansion summary, and merge/ranking rationale.

### 5.2 Core operations (shape, not exact signatures)

* `upsert(record) -> id`
* `get_by_id(id) -> record + graph_context`
* `hybrid_search(query, filters, policy) -> RetrievalBundle`
* Optional: `graph_query_*` helpers (by entity, location, time range, etc.)

Implementation note:

* The hybrid flow may internally perform vector search and graph expansion. These are not necessarily exposed as stable, user-facing APIs.

### 5.3 Caller responsibility

* Construct prompts / plans using `RetrievalBundle`.
* Call any LLM and manage iterative loops.
* Render results, maintain chat state, decide UX.

---

## 6. Repository Layout Guidance (to prevent scope creep)

* `/src` (or library crate): storage, retrieval, schemas, query builders, tests
* `/docs`: library docs + system context (no executable LLM integration)
* `/examples` (optional): **non-core** demonstration of caller integration (can include LLM calls, but must not be required for library build/test)

---

# Roadmap A: Library Roadmap (In-Repository)

## Phase L0: Foundations (Contracts, IDs, Schema Versioning, Test Harness)

**Deliverables**

* Canonical domain model: `MemoryRecord`, `MemoryType`, `RetrievalBundle`
* UUID minting strategy + idempotent upsert semantics
* Deterministic IRI generation utilities (`memory_iri`, `entity_iri`, `location_iri`, optional `date_iri`)
* Schema/versioning conventions (Qdrant collection, RDF namespace)
* Golden fixtures: episodic full, episodic (with explicit "unknown" placeholders), semantic

**Acceptance criteria**

* Stable IRIs for same inputs; round-trip payload validity tests
* Fixtures validate optionality rules without special casing
* CI includes unit tests for ID/IRI + schema validation

---

## Phase L1: Qdrant Vector Layer MVP (Payload + Filters + Search)

**Deliverables**

* Qdrant collection setup with payload schema:

  * required: `id`, `memory_type`, `content`
  * episodic-only: `timestamp`, `location_text`, `participants`
* Payload excludes embedding vectors (vectors are stored only in the Qdrant vector field)
* Text indexes for word-level partial match filtering:

  * `participants` (multilingual text index)
  * `location_text` (multilingual text index)
* Upsert + vector search (top-k) + filter combinations
* Minimal operational tooling: create collection, basic health check

**Acceptance criteria**

* Controlled dataset yields deterministic expected top-k
* Filter correctness: memory_type + time range + participants + location_text

  * Participants/location filtering supports word-level full-text matching (not within-word substring matching unless explicitly added later).

---

## Phase L2: Oxigraph Graph Layer MVP (RDF Vocabulary + Upsert + Core Queries)

**Deliverables**

* RDF vocabulary (`am:`) with minimal classes/predicates:

  * Classes: `am:EpisodicMemory`, `am:SemanticMemory`, `am:Entity`, `am:Location`, optional `am:Date`
  * Predicates: `am:involves`, `am:happenedAt`, optional `am:recordedIn` or literal `am:timestamp`, `am:mentions`
* Deterministic mapping: `MemoryRecord` → RDF triples
* SPARQL query helpers (core):

  * context by memory id
  * by entity
  * by location
  * by time range (depending on timestamp modeling)

**Acceptance criteria**

* For each fixture, graph lookups return correct bindings
* SPARQL regression tests lock query semantics

---

## Phase L3: Hybrid Retrieval (Vector → Graph Expansion → RetrievalBundle)

**Deliverables**

* Hybrid flow:

  1. vector search → candidate ids
  2. graph expansion around ids (entities/location/time)
  3. merge into RetrievalBundle with stable ordering rules
* Graph-first structured retrieval APIs:

  * “memories involving entity X”
  * “memories at location Y”
  * “memories between dates”
  * composable constraints where feasible

**Acceptance criteria**

* End-to-end tests confirm RetrievalBundle contains:

  * candidate memories with provenance
  * graph-enriched context
  * deterministic merge behavior (de-dupe + ordering)

---

## Phase L4: Performance and Operational Hardening

**Deliverables**

* Graph expansion policy controls:

  * max breadth/depth, pagination, timeouts
  * hub-entity handling (bounded fan-out)
* Caching hooks (optional) for hot SPARQL patterns
* Indexing/optimization guidance for common predicates and time queries (as supported by Oxigraph)

**Acceptance criteria**

* Stress tests show bounded runtime on “hub entity” scenarios
* No unbounded expansions; predictable worst-case behavior

---

## Phase L5: Extensions (Schema Evolution Without LLM Dependency)

**Deliverables (choose as needed)**

* Memory consolidation primitives (storage-level):

  * store summary memories as `semantic` with links to source ids in graph
* Prospective memory primitives (storage-level):

  * task/reminder records as memory subtype or dedicated schema extension (still retrieval-only)
* Multimodal hooks (storage-level only):

  * store media references (URI + metadata) and graph predicates (no embedding/LLM logic required)

**Acceptance criteria**

* Backward-compatible migrations or explicit version bumps
* Existing queries continue to function or fail loudly with clear migration steps

---

## Phase L6: Optional Examples (Caller Integration Demonstrations, Non-Core)

**Deliverables**

* Examples that show how a caller can:

  * call `hybrid_search`
  * turn RetrievalBundle into a prompt
  * perform an external LLM call
* These examples must be isolated and not required for core build/test.

**Acceptance criteria**

* Examples compile/run independently
* Core library remains LLM-free and deterministic in CI

---

# Roadmap B: Application / Caller Roadmap (Out of Repository, Context Only)

These phases are **not implemented** in this library. They are included to clarify how the library is intended to be used.

## Phase C1: LLM Answering (Single-Step Grounded Responses)

* Caller uses `RetrievalBundle` as the grounding substrate.
* Caller owns prompt design, citations, and response formatting.

## Phase C2: Iterative Retrieval & Planning

* Caller uses an LLM (or other logic) to produce structured constraints (entities/time/location).
* Caller re-invokes library APIs with refined filters and graph expansions.

## Phase C3: UI/UX and Product Features

* Timeline views, memory browsing, media rendering, notifications/reminders, etc.

---

## 7. Documentation Notes

* All “assistant behavior” is described only as **integration context**.
* The library documentation should emphasize:

  * stable storage/retrieval semantics,
  * deterministic IDs and query behavior,
  * explicit contracts (`MemoryRecord`, `RetrievalBundle`),
  * and clear integration points for callers.
