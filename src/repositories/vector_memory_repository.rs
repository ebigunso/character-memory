use std::collections::HashMap;
use uuid::Uuid;
use chrono::DateTime;

use crate::databases::vector_database::VectorDatabase;
use crate::databases::domain_types::{DbPoint, DbSearchQuery, DbSearchResult};
use crate::errors::custom::CustomError;
use crate::models::internal::{MemoryEntry, MemoryType};
use crate::models::public::MemoryFilters;

/// Configuration for VectorMemoryRepository
///
/// * `url` - URL of the vector database server
/// * `collection_name` - Name of the collection to store memories
/// * `vector_size` - Size of the embedding vectors
#[derive(Debug, Clone)]
pub(crate) struct VectorMemoryConfig {
    pub(crate) url: String,
    pub(crate) collection_name: String,
    pub(crate) vector_size: u64,
}

impl VectorMemoryConfig {
    pub(crate) fn new(url: String, collection_name: String, vector_size: u64) -> Self {
        Self {
            url,
            collection_name,
            vector_size,
        }
    }

    pub(crate) fn text_embedding_3_small(url: String, collection_name: String) -> Self {
        Self::new(url, collection_name, 1536)
    }

    pub(crate) fn text_embedding_3_large(url: String, collection_name: String) -> Self {
        Self::new(url, collection_name, 3072)
    }

    pub(crate) fn text_embedding_ada_002(url: String, collection_name: String) -> Self {
        Self::new(url, collection_name, 1536)
    }
}

/// Repository implementation for storing and retrieving memories using a vector database.
pub(crate) struct VectorMemoryRepository<T: VectorDatabase> {
    client: T,
    config: VectorMemoryConfig,
}

impl<T: VectorDatabase> VectorMemoryRepository<T> {
    /// Creates a new VectorMemoryRepository instance.
    pub(crate) fn new(client: T, config: VectorMemoryConfig) -> Self {
        Self { client, config }
    }

    /// Initializes the vector database collection if it doesn't exist.
    /// If the collection already exists, it returns without changes.
    pub(crate) async fn init_collection(&self) -> Result<(), CustomError> {
        let collections = self.client.list_collections().await?;
        if collections.iter().any(|name| name == &self.config.collection_name) {
            return Ok(());
        }
        self.client.create_collection(&self.config.collection_name, self.config.vector_size).await
    }

    /// Stores a new memory in the database using upsert semantics.
    pub(crate) async fn store_memory(&self, memory: &MemoryEntry) -> Result<(), CustomError> {
        if memory.embedding.len() != self.config.vector_size as usize {
            return Err(CustomError::MemoryValidation(format!(
                "Embedding size mismatch. Expected {}, got {}",
                self.config.vector_size,
                memory.embedding.len()
            )));
        }
        memory.validate()?;
        let db_point = Self::memory_to_db_point(memory);
        self.client.upsert_points(&self.config.collection_name, vec![db_point]).await
    }

    /// Updates an existing memory in the database (alias to store_memory).
    pub(crate) async fn update_memory(&self, memory: &MemoryEntry) -> Result<(), CustomError> {
        self.store_memory(memory).await
    }

    /// Deletes a memory by its ID.
    pub(crate) async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        self.client
            .delete_points(&self.config.collection_name, vec![id.to_string()])
            .await
    }

    /// Searches for memories using a query vector and optional filters.
    pub(crate) async fn search_memory(
        &self,
        query_vector: &[f32],
        top_k: usize,
        filters: Option<&MemoryFilters>,
    ) -> Result<Vec<MemoryEntry>, CustomError> {
        if query_vector.len() != self.config.vector_size as usize {
            return Err(CustomError::MemoryValidation(format!(
                "Query vector size mismatch. Expected {}, got {}",
                self.config.vector_size,
                query_vector.len()
            )));
        }
        let filter = if let Some(filters) = filters {
            let mut must_conditions = Vec::new();
            if let Some(mem_type) = &filters.memory_type {
                must_conditions.push(serde_json::json!({
                    "key": "memory_type",
                    "match": { "value": mem_type.to_lowercase() }
                }));
            }
            if filters.start_date.is_some() || filters.end_date.is_some() {
                let mut range_obj = serde_json::Map::new();
                if let Some(start) = filters.start_date {
                    range_obj.insert("gte".to_string(), serde_json::json!(start.timestamp()));
                }
                if let Some(end) = filters.end_date {
                    range_obj.insert("lte".to_string(), serde_json::json!(end.timestamp()));
                }
                must_conditions.push(serde_json::json!({
                    "key": "timestamp",
                    "range": range_obj
                }));
            }
            Some(serde_json::json!({
                "must": must_conditions
            }))
        } else {
            None
        };

        let db_query = DbSearchQuery {
            collection_name: self.config.collection_name.clone(),
            vector: query_vector.to_vec(),
            limit: top_k as u64,
            filter,
            with_payload: true,
        };
        let results = self.client.search_points(&db_query).await?;
        let mut memories = Vec::with_capacity(results.len());
        for res in results {
            memories.push(Self::scored_point_to_memory(&res)?);
        }
        Ok(memories)
    }

    /// Inserts multiple memories in a single operation.
    pub(crate) async fn bulk_insert(&self, memories: &[MemoryEntry]) -> Result<(), CustomError> {
        let mut points = Vec::new();
        for memory in memories {
            if memory.embedding.len() != self.config.vector_size as usize {
                return Err(CustomError::MemoryValidation(format!(
                    "Embedding size mismatch. Expected {}, got {}",
                    self.config.vector_size,
                    memory.embedding.len()
                )));
            }
            memory.validate()?;
            points.push(Self::memory_to_db_point(memory));
        }
        self.client.upsert_points(&self.config.collection_name, points).await
    }

    // Helper: Convert a MemoryEntry into a domain DbPoint.
    fn memory_to_db_point(memory: &MemoryEntry) -> DbPoint {
        let mut payload_map = HashMap::new();
        payload_map.insert(
            "memory_type".to_string(),
            serde_json::Value::String(format!("{:?}", memory.memory_type).to_lowercase()),
        );
        payload_map.insert(
            "content".to_string(),
            serde_json::Value::String(memory.content.clone()),
        );
        if let Some(timestamp) = &memory.timestamp {
            payload_map.insert(
                "timestamp".to_string(),
                serde_json::Value::Number(serde_json::Number::from(timestamp.timestamp())),
            );
        }
        if let Some(location) = &memory.location_text {
            payload_map.insert(
                "location_text".to_string(),
                serde_json::Value::String(location.clone()),
            );
        }
        if let Some(participants) = &memory.participants {
            payload_map.insert(
                "participants".to_string(),
                serde_json::Value::Array(
                    participants.iter().map(|p| serde_json::Value::String(p.clone())).collect()
                ),
            );
        }
        DbPoint {
            id: Some(memory.id.to_string()),
            payload: payload_map,
            vector: memory.embedding.clone(),
        }
    }

    // Helper: Convert a domain DbSearchResult into a MemoryEntry.
    fn scored_point_to_memory(result: &DbSearchResult) -> Result<MemoryEntry, CustomError> {
        let payload = &result.payload;
        let memory_type_str = payload.get("memory_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CustomError::DatabaseError("Missing memory_type in payload".to_string()))?;
        let memory_type = match memory_type_str {
            "episodic" => MemoryType::Episodic,
            _ => MemoryType::Semantic,
        };
        let content = payload.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CustomError::DatabaseError("Missing content in payload".to_string()))?
            .to_string();
        let embedding = result.vector.clone();
        // Parse the id from the string.
        let id = Uuid::parse_str(&result.id)
            .map_err(|e| CustomError::DatabaseError(format!("Invalid UUID format: {}", e)))?;
        // Optional fields
        let timestamp = payload.get("timestamp")
            .and_then(|v| v.as_i64())
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .map(|dt| dt);
        let location_text = payload.get("location_text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let participants = payload.get("participants")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_owned()))
                    .collect::<Vec<_>>()
            });
        Ok(MemoryEntry {
            id,
            memory_type,
            content,
            embedding,
            timestamp,
            location_text,
            participants,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MemoryInput;
    use crate::databases::vector_database::MockVectorDatabase;
    use mockall::predicate::*;
    use chrono::Utc;
    use serde_json::Value;
    use std::collections::HashMap;

    // Common test setup
    fn create_test_config() -> VectorMemoryConfig {
        VectorMemoryConfig::text_embedding_3_small(
            "http://localhost:6334".to_string(),
            "test_memories".to_string(),
        )
    }

    fn create_test_memory(vector_size: usize) -> MemoryEntry {
        let input = MemoryInput {
            content: "Test memory".to_string(),
            memory_type: "episodic".to_string(),
            timestamp: Some(Utc::now()),
            location_text: Some("Test Location".to_string()),
            participants: Some(vec!["Alice".to_string(), "Bob".to_string()]),
        };
        MemoryEntry::new(input, vec![0.1; vector_size]).unwrap()
    }

    // CRUD Operation Tests
    mod crud_tests {
        use super::*;

        #[tokio::test]
        async fn test_init_collection() {
            let config = create_test_config();
            let mut mock_db = MockVectorDatabase::new();

            // Setup mock expectations
            mock_db.expect_list_collections()
                .times(1)
                .returning(|| Ok(vec![]));

            mock_db.expect_create_collection()
                .withf(|name: &str, size: &u64| {
                    name == "test_memories" && *size == 1536
                })
                .times(1)
                .returning(|_, _| Ok(()));

            let repo = VectorMemoryRepository::new(mock_db, config);
            assert!(repo.init_collection().await.is_ok());
        }

        #[tokio::test]
        async fn test_store_memory() {
            let config = create_test_config();
            let vector_size = config.vector_size as usize;
            let mut mock_db = MockVectorDatabase::new();
            let memory = create_test_memory(vector_size);
            let memory_id = memory.id.clone();

            mock_db.expect_upsert_points()
                .withf(move |collection: &str, points: &Vec<DbPoint>| {
                    collection == "test_memories" && points.len() == 1 &&
                    points[0].id.as_ref().map_or(false, |id| id == &memory_id.to_string())
                })
                .times(1)
                .returning(|_, _| Ok(()));

            let repo = VectorMemoryRepository::new(mock_db, config);
            assert!(repo.store_memory(&memory).await.is_ok());
        }

        #[tokio::test]
        async fn test_update_memory() {
            let config = create_test_config();
            let vector_size = config.vector_size as usize;
            let mut mock_db = MockVectorDatabase::new();
            let memory = create_test_memory(vector_size);
            let memory_id = memory.id.clone();

            mock_db.expect_upsert_points()
                .withf(move |collection: &str, points: &Vec<DbPoint>| {
                    collection == "test_memories" &&
                    points.len() == 1 &&
                    points[0].payload.get("memory_type").map_or(false, |v| v.as_str().map_or(false, |s| s == "episodic")) &&
                    points[0].payload.get("content").map_or(false, |v| v.as_str().map_or(false, |s| s == "Test memory")) &&
                    points[0].id.as_ref().map_or(false, |id| id == &memory_id.to_string())
                })
                .times(1)
                .returning(|_, _| Ok(()));

            let repo = VectorMemoryRepository::new(mock_db, config);
            assert!(repo.update_memory(&memory).await.is_ok());
        }

        #[tokio::test]
        async fn test_delete_memory() {
            let config = create_test_config();
            let vector_size = config.vector_size as usize;
            let mut mock_db = MockVectorDatabase::new();
            let memory = create_test_memory(vector_size);
            let memory_id = memory.id.clone();

            mock_db.expect_delete_points()
                .withf(move |collection: &str, ids: &Vec<String>| {
                    collection == "test_memories" &&
                    ids.get(0).map_or(false, |id| id == &memory_id.to_string())
                })
                .times(1)
                .returning(|_, _| Ok(()));

            let repo = VectorMemoryRepository::new(mock_db, config);
            assert!(repo.delete_memory(memory.id).await.is_ok());
        }

        #[tokio::test]
        async fn test_search_memory() {
            let config = create_test_config();
            let vector_size = config.vector_size as usize;
            let mut mock_db = MockVectorDatabase::new();
            let memory = create_test_memory(vector_size);

            let mut payload: HashMap<String, Value> = HashMap::new();
            payload.insert("memory_type".to_string(), Value::from("episodic"));
            payload.insert("content".to_string(), Value::from("Test memory"));
            if let Some(ts) = memory.timestamp.as_ref() {
                payload.insert("timestamp".to_string(), Value::from(ts.timestamp()));
            }
            payload.insert("location_text".to_string(), Value::from("Test Location"));
            payload.insert("participants".to_string(), Value::from(vec!["Alice".to_string(), "Bob".to_string()]));

            let search_result = vec![DbSearchResult {
                id: memory.id.to_string(),
                payload: payload.clone(),
                vector: vec![0.1; vector_size],
                score: 0.9,
            }];

            mock_db.expect_search_points()
                .withf({
                    let vs = vector_size;
                    move |search: &DbSearchQuery| {
                        search.collection_name == "test_memories" &&
                        search.vector.len() == vs &&
                        search.limit == 1 &&
                        search.filter.is_none() &&
                        search.with_payload
                    }
                })
                .times(1)
                .returning(move |_| Ok(search_result.clone()));

            let repo = VectorMemoryRepository::new(mock_db, config);
            let results = repo.search_memory(&memory.embedding, 1, None).await.unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].content, memory.content);
        }
    }

    // Search and Filter Tests
    mod search_tests {
        use super::*;
        #[tokio::test]
        async fn test_search_with_filters() {
            let config = create_test_config();
            let vector_size = config.vector_size as usize;
            let mut mock_db = MockVectorDatabase::new();

            // Setup mock expectations for init_collection
            mock_db.expect_list_collections()
                .times(1)
                .returning(|| Ok(vec![]));

            mock_db.expect_create_collection()
                .withf(|name: &str, size: &u64| {
                    name == "test_memories" && *size == 1536
                })
                .times(1)
                .returning(|_, _| Ok(()));

            let now = Utc::now();
            let yesterday = now - chrono::Duration::days(1);
            let tomorrow = now + chrono::Duration::days(1);

            // Create test memories
            let memory1 = MemoryEntry::new(
                MemoryInput {
                    content: "Memory 1".to_string(),
                    memory_type: "episodic".to_string(),
                    timestamp: Some(yesterday),
                    location_text: Some("Location A".to_string()),
                    participants: Some(vec!["Alice".to_string()]),
                },
                vec![0.1; vector_size],
            ).unwrap();

            let memory2 = MemoryEntry::new(
                MemoryInput {
                    content: "Memory 2".to_string(),
                    memory_type: "episodic".to_string(),
                    timestamp: Some(tomorrow),
                    location_text: Some("Location B".to_string()),
                    participants: Some(vec!["Bob".to_string()]),
                },
                vec![0.1; vector_size],
            ).unwrap();

            // Setup mock for search with filters
            let mut payload = HashMap::new();
            payload.insert("memory_type".to_string(), Value::from("episodic"));
            payload.insert("content".to_string(), Value::from("Memory 2"));
            if let Some(ts) = memory2.timestamp.as_ref() {
                payload.insert("timestamp".to_string(), Value::from(ts.timestamp()));
            }
            payload.insert("location_text".to_string(), Value::from("Location B"));
            payload.insert("participants".to_string(), Value::from(vec!["Bob".to_string()]));

            let search_result = vec![DbSearchResult {
                id: memory2.id.to_string(),
                payload,
                vector: vec![0.1; vector_size],
                score: 0.9,
            }];

            mock_db.expect_search_points()
                .withf({
                    let vs = vector_size;
                    move |search: &DbSearchQuery| {
                        search.collection_name == "test_memories" &&
                        search.vector.len() == vs &&
                        search.limit == 10 &&
                        search.with_payload &&
                        if let Some(ref filt) = search.filter {
                            if let Some(obj) = filt.as_object() {
                                if let Some(must_val) = obj.get("must") {
                                    if let Some(arr) = must_val.as_array() {
                                        arr.len() == 2
                                    } else { false }
                                } else { false }
                            } else { false }
                        } else { false }
                    }
                })
                .times(1)
                .returning(move |_| Ok(search_result.clone()));

            // Create repository with mock
            let repo = VectorMemoryRepository::new(mock_db, config);

            // Initialize collection
            repo.init_collection().await.unwrap();

            // Test date range filtering – note that in the repository the filters are not yet applied,
            // so the underlying search query will have filter == None. This test expects that when filters are provided,
            // the repository builds a filter (whose JSON "must" array has 2 elements).
            let filters = MemoryFilters {
                memory_type: Some("episodic".to_string()),
                start_date: Some(now),
                end_date: Some(tomorrow + chrono::Duration::hours(1)),
                location_text: None,
                participants: None,
            };

            let results = repo
                .search_memory(&vec![0.1; vector_size], 10, Some(&filters))
                .await
                .unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].content, "Memory 2");
        }
    }

    // Validation Tests
    mod validation_tests {
        use super::*;

        #[tokio::test]
        async fn test_invalid_vector_size() {
            let config = create_test_config();
            let mock_db = MockVectorDatabase::new();
            let repo = VectorMemoryRepository::new(mock_db, config);

            let input = MemoryInput {
                content: "Test memory".to_string(),
                memory_type: "semantic".to_string(),
                timestamp: None,
                location_text: None,
                participants: None,
            };

            // Create memory with wrong vector size
            let memory = MemoryEntry::new(input, vec![0.1; 512]).unwrap();

            // Test store with wrong vector size
            let result = repo.store_memory(&memory).await;
            assert!(matches!(result, Err(CustomError::MemoryValidation(_))));
        }
    }
}
