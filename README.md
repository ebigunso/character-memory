# AgentMemory

## Construction

`AgentMemory::new(settings, collection_name)` constructs the default OpenAI + Qdrant-backed instance.

For deterministic tests or custom backends, use `AgentMemory::new_with_repositories(embed_repo, vector_repo)` with your own implementations of `EmbeddingRepository` and `VectorMemoryRepository`.

## Running Tests

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
