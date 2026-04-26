# Character Memory

Character Memory is a Rust library for giving LLM assistants memory that shapes behavior over time.

It is built for persistent AI assistants and companions that should remember past interactions, recognize recurring entities, and maintain character continuity across sessions.

Instead of treating memory as a flat chat log or simple vector search, Character Memory stores episodic memories with temporal and relational structure.

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

Retrieval is hybrid:

- **Semantic retrieval:** finds memories with similar meaning
- **Temporal retrieval:** finds memories based on when they happened
- **Entity-based retrieval:** finds memories involving the same people, projects, places, or concepts
- **Relational retrieval:** helps connect memories through their relationships

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

`CharacterMemory::new(settings, collection_name)` constructs the default memory system.

By default, this uses:

- OpenAI for embeddings
- Qdrant for vector storage

```rust
let memory = CharacterMemory::new(settings, "my-assistant-memory");
```

For deterministic tests or custom embedding backends, use:

```rust
CharacterMemory::new_with_embedding_provider(
    settings,
    "my-assistant-memory",
    embed_provider,
);
```

Your custom provider must implement `EmbeddingProvider`.

This is useful when you want to:

- use a local embedding model
- avoid network calls in tests
- make tests deterministic
- integrate another embedding API

## Backends

The default implementation is backed by Qdrant.

Integration tests require a local Qdrant instance reachable over gRPC.

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

The current focus is building a reliable memory layer for LLM assistants that need persistent episodic memory, temporal awareness, and entity-based recall.
