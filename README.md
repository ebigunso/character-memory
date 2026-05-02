# Character Memory

Character Memory is a Rust library for giving LLM assistants memory that shapes behavior over time.

It is built for persistent AI assistants and companions that should remember past interactions, recognize recurring entities, and maintain character continuity across sessions.

Instead of treating memory as a chat log or simple vector search, Character Memory stores episodic memories with temporal and relational structure.

It helps an assistant remember:

- what happened
- when it happened
- who or what was involved
- how past events relate to each other

To aid in character continuity.

A character is not just a prompt. A character is shaped by remembered experience.

## Why Character Memory?

Modern LLM agents are usually built around task execution.

They can plan, call tools, and complete workflows, but they often lack persistent memory of lived interaction. This makes them feel stateless: each conversation may be useful, but the assistant does not develop a stable sense of continuity.

Character Memory is designed for systems where past interactions matter.

Use it when you want an assistant to remember things like:

- recurring people, projects, places, and topics
- past conversations with the user
- important events and decisions
- relationship history
- preferences that emerged over time
- memories that are relevant because of time, entities, or meaning

## Core idea

Character Memory stores memories as episodes.

An episode is a remembered event or interaction. Each episode can be connected to time, entities, and related memories.

Retrieval is graph-authoritative and hybrid:

- **Vector candidate recall:** uses Qdrant to find semantically similar memory objects
- **Graph expansion:** uses Oxigraph as the authority for entities, threads, provenance, lifecycle state, and links
- **Temporal retrieval:** includes memories based on when they happened
- **Entity-based retrieval:** includes memories involving the same people, projects, places, or concepts
- **Continuity retrieval:** returns a structured `ContinuityContextPack` rather than a generic ranked list

This allows an assistant to retrieve memories in a way that is closer to human recall than plain vector search.

## What this is not

Character Memory is not:

- a generic vector database wrapper
- a chat history dump
- a simple user profile store
- a task-agent framework
- a replacement for an LLM

It is a memory layer for persistent AI assistants and companions.

## Typical usage

A typical assistant loop looks like this:

1. The user says something.
2. The assistant retrieves relevant memories.
3. The retrieved memories are added to the LLM context.
4. The assistant responds.
5. Important parts of the interaction are stored as new memories.
6. Over time, memories reinforce character continuity.

Conceptually:

```text
user message
    ↓
retrieve relevant memories
    ↓
LLM prompt with memory context
    ↓
assistant response
    ↓
store new episode
    ↓
future interactions become more continuous
```

## Construction

`CharacterMemory::new(settings, collection_name).await?` constructs the default memory system.

By default, this uses:

- OpenAI for embeddings
- Qdrant for vector candidate recall and payload filtering
- Oxigraph service mode for graph-authoritative memory objects, relationships, provenance, and lifecycle state

```rust
let memory = CharacterMemory::new(settings, "my-assistant-memory".to_owned()).await?;
```

For deterministic tests or custom embedding backends, use:

```rust
let memory = CharacterMemory::new_with_embedding_provider(
    settings,
    "my-assistant-memory".to_owned(),
    embed_provider,
).await?;
```

Your custom provider must implement `EmbeddingProvider`.

This is useful when you want to:

- use a local embedding model
- avoid embedding-provider network calls in tests
- make tests deterministic
- integrate another embedding API

## Backends

The default implementation is backed by Qdrant and an Oxigraph HTTP service.

Qdrant is used for vector candidate recall. Oxigraph is the graph authority for memory objects, links, provenance, currentness, and lifecycle filtering. Local application construction defaults to `GRAPH_STORE_MODE=service` with `OXIGRAPH_CONNECTION_STRING=http://localhost:7878`. Embedded filesystem persistence remains available with `GRAPH_STORE_MODE=persistent`; deterministic tests and fixtures use `GRAPH_STORE_MODE=in_memory`.

Raw source material, such as chat or voice transcripts, is caller-owned in v0.1 and is not stored by the default graph/vector backends. Memory objects may preserve `raw_ref` source pointers for provenance, but those pointers are not the transcript content and do not imply a public raw-resolution API.

Integration tests that exercise external vector storage require a local Qdrant instance reachable over gRPC.

The default gRPC port is `6334`.

### Start Qdrant with Docker

```sh
docker run -d \
  --name charactermemory-qdrant \
  -p 6333:6333 \
  -p 6334:6334 \
  qdrant/qdrant:latest
```

Or using Docker Compose:

```sh
docker compose -f docker-compose.qdrant.yml up -d
```

### Start Oxigraph with Docker

```sh
docker compose -f docker-compose.oxigraph.yml up -d
```

The default Oxigraph HTTP endpoint is `http://localhost:7878`.

Live Oxigraph smoke tests use a separate container, port, and volume:

```sh
docker compose -f docker-compose.oxigraph.test.yml up -d
```

The default live-test Oxigraph endpoint is `http://localhost:7879`. The smoke test cleans up the named graphs it creates.

## Running tests

1. Copy `.env.example` to `.env`:

   ```sh
   cp .env.example .env
   ```

2. Fill in the required credentials in `.env`.

3. Run the tests:

   ```sh
   cargo test
   ```

Do not commit your `.env` file.

## Status

Character Memory is under active development.

The v0.1 public architecture is graph-authoritative episodic continuity memory: public construction and facades compose an embedder, Qdrant candidate recall, and Oxigraph graph authority.

v0.1 does not store raw transcripts directly in graph/vector storage, run a reflection scheduler, implement a normalized belief ontology, support multimodal memory, or perform physical redaction/delete as a default lifecycle operation.

Production raw transcript storage is caller-owned and deferred. No public raw-reference resolution API is part of v0.1.
