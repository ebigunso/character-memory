# AgentMemory

## Running Tests

These integration tests require a local Qdrant instance reachable over gRPC (default port `6334`).

### Start Qdrant (Docker)

Using `docker run`:

```sh
docker run -d --name agentmemory-qdrant -p 6333:6333 -p 6334:6334 qdrant/qdrant:latest
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
