# CharacterMemory

## Construction

`CharacterMemory::new(settings, collection_name)` constructs the default OpenAI + Qdrant-backed instance.

For deterministic tests or custom embedding backends, use `CharacterMemory::new_with_embedding_provider(settings, collection_name, embed_provider)` with your own implementation of `EmbeddingProvider`.

## Running Tests

These integration tests require a local Qdrant instance reachable over gRPC (default port `6334`).

### Start Qdrant (Docker)

Using `docker run`:

```sh
docker run -d --name charactermemory-qdrant -p 6333:6333 -p 6334:6334 qdrant/qdrant:latest
```

Or using Compose:

```sh
docker compose -f docker-compose.qdrant.yml up -d
```

1. Copy `.env.example` to `.env`:
   ```sh
   cp .env.example .env
   ```
2. Fill in the required credentials in `.env`.
3. Run the tests normally:
   ```sh
   cargo test
   ```

Do not commit your `.env` file.
