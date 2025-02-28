# Vector Database Memory Schema & Payload Specification

This document defines the schema for storing memory entries in Qdrant, detailing the intended structure and data types. It distinguishes between **episodic** and **semantic** memories using a shared common structure, while allowing episodic entries to include additional metadata. The specification leverages Qdrant's supported payload types (e.g., `uuid`, `keyword`, `datetime`) to enforce consistency and enable effective filtering.

## Schema Quick Reference

| Field          | Type         | Required For | Description                                   |
|----------------|--------------|--------------|-----------------------------------------------|
| id             | `uuid`       | All          | Unique identifier linking to graph database   |
| memory_type    | `keyword`    | All          | Either "episodic" or "semantic"               |
| content        | `keyword`    | All          | Raw textual content for vector generation      |
| timestamp      | `datetime`   | Episodic     | Event occurrence time (RFC 3339)              |
| location_text  | `keyword`    | Episodic     | Textual location description                  |
| participants   | `keyword[]`  | Episodic     | Array of involved entities                    |

*Note: A `location_geo` field (using Qdrant's `geo` type for geographic coordinates) is considered as a future enhancement and is not included in the current schema requirements.*

## 1. Common Fields

Every memory entry—whether episodic or semantic—**must include** the following fields:

- **id**
  - **Type**: `uuid`
  - **Description**: A unique identifier for the memory entry. This field is critical for linking Qdrant entries to corresponding nodes in the graph database.
  - **Example**:
    ```json
    "id": "123e4567-e89b-12d3-a456-426614174000"
    ```

- **memory_type**
  - **Type**: `keyword` (string value)
  - **Allowed Values**: `"episodic"` or `"semantic"`
  - **Description**: Differentiates between episodic (event-based) and semantic (general knowledge) memories. This field is used for filtering and ensures that queries can target the correct type of memory.
  - **Example**:
    ```json
    "memory_type": "episodic"
    ```

- **content**
  - **Type**: `keyword` (string value)
  - **Description**: The raw textual content from which the vector is generated. This field serves as a reference for verification, reprocessing, or fallback retrieval.
  - **Example**:
    ```json
    "content": "Discussed plans for the weekend at Café Central."
    ```

## 2. Episodic-Specific Fields

These fields **must be included only** in entries where `memory_type` is set to `"episodic"`. Note that while the schema currently requires a textual representation of location, future enhancements may introduce precise geographic data if needed.

- **timestamp**
  - **Type**: `datetime`
  - **Description**: The date and time when the event occurred or when the memory was created. This field is essential for time-based filtering and chronological queries in the graph database.
  - **Format**: RFC 3339 (e.g., `"2025-02-02T14:00:00Z"`)
  - **Example**:
    ```json
    "timestamp": "2025-02-02T14:00:00Z"
    ```

- **location_text**
  - **Type**: `keyword`
  - **Description**: A textual representation of the location. This field is intended to capture vague, abstract, or digital spaces where a physical location is not applicable. Examples include names of digital platforms, virtual meeting rooms, or general location descriptors like `"Café Central"` or `"Slack Channel #general"`.
  - **Example**:
    ```json
    "location_text": "Café Central"
    ```

- **participants**
  - **Type**: Array of `keyword` values
  - **Description**: A list of individuals or entities involved in the event. This helps in establishing relationships within the graph database and supports queries to retrieve all memories associated with a particular participant.
  - **Example**:
    ```json
    "participants": ["Alice", "Bob"]
    ```

## 3. Example Payloads

### Episodic Memory Entry

Using a textual description of the location:

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "memory_type": "episodic",
  "content": "Discussed plans for the weekend at Café Central.",
  "timestamp": "2025-02-02T14:00:00Z",
  "location_text": "Café Central",
  "participants": ["Alice", "Bob"]
}
```

If the event took place in a digital space:

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174002",
  "memory_type": "episodic",
  "content": "Joined a meeting in the virtual conference room.",
  "timestamp": "2025-02-02T16:00:00Z",
  "location_text": "Virtual Conference Room",
  "participants": ["Alice", "Charlie"]
}
```

### Semantic Memory Entry

Semantic memories do not require any location data:

```json
{
  "id": "123e4567-e89b-12d3-a456-426614174001",
  "memory_type": "semantic",
  "content": "Alice is a software engineer living in New York."
}
```

## 4. Integration with the Graph Database

- **Linking Entries**:
  The `id` field (of type `uuid`) is the bridge between Qdrant entries and corresponding nodes in the graph database. It ensures that any memory stored in Qdrant can be easily referenced, updated, or linked to related graph data.

- **Filtering Considerations**:
  Qdrant enables filtering based on payload types. For example, to filter for episodic memories within a specific date range, you might use a filter such as:

  ```json
  {
    "must": [
      { "key": "memory_type", "match": { "value": "episodic" } },
      { "key": "timestamp", "range": { "gte": "2025-02-01T00:00:00Z", "lte": "2025-02-03T00:00:00Z" } }
    ]
  }
  ```

  This filter will automatically ignore entries that do not contain the episodic-specific keys, such as semantic memories. Likewise, filtering on location can be done by applying conditions on `location_text` based on the type of query required.

## Future Considerations

- **Geographic Coordinates (`location_geo`)**:
  While the current schema excludes geographic coordinates to keep the implementation simple and within scope, future versions of the system may benefit from capturing precise location data. If a requirement arises to support spatial queries (e.g., "Show me all events near Berlin"), the schema can be extended to include a `location_geo` field (of type `geo`). This would be integrated in a way that complements the existing `location_text` field without disrupting the current functionality.
