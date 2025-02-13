use std::vec;

use qdrant_client::qdrant::vectors_output::VectorsOptions;
use qdrant_client::qdrant::{PointStruct, Filter, Condition, Range, PointId, SearchPointsBuilder, ScoredPoint};
use chrono::DateTime;
use uuid::Uuid;

use crate::databases::vector_database::VectorDatabase;
use crate::errors::custom::CustomError;
use crate::models::{MemoryEntry, MemoryType, MemoryFilters};

/// Configuration for VectorMemoryRepository
#[derive(Debug, Clone)]
pub(crate) struct VectorMemoryConfig {
    /// URL of the vector database server
    pub(crate) url: String,
    /// Name of the collection to store memories
    pub(crate) collection_name: String,
    /// Size of the embedding vectors
    pub(crate) vector_size: u64,
}

impl VectorMemoryConfig {
    /// Creates a new VectorMemoryConfig with the specified parameters
    pub(crate) fn new(url: String, collection_name: String, vector_size: u64) -> Self {
        Self {
            url,
            collection_name,
            vector_size,
        }
    }

    /// Creates a config for text-embedding-3-small (1536 dimensions)
    pub(crate) fn text_embedding_3_small(url: String, collection_name: String) -> Self {
        Self::new(url, collection_name, 1536)
    }

    /// Creates a config for text-embedding-3-large (3072 dimensions)
    pub(crate) fn text_embedding_3_large(url: String, collection_name: String) -> Self {
        Self::new(url, collection_name, 3072)
    }

    /// Creates a config for text-embedding-ada-002 (1536 dimensions)
    pub(crate) fn text_embedding_ada_002(url: String, collection_name: String) -> Self {
        Self::new(url, collection_name, 1536)
    }
}

/// Repository implementation for storing and retrieving memories using a vector database
pub(crate) struct VectorMemoryRepository<T: VectorDatabase> {
    client: T,
    config: VectorMemoryConfig,
}

impl<T: VectorDatabase> VectorMemoryRepository<T> {
    /// Creates a new VectorMemoryRepository instance
    pub(crate) fn new(client: T, config: VectorMemoryConfig) -> Self {
        Self { client, config }
    }

    /// Initializes the vector database collection if it doesn't exist
    ///
    /// This method ensures the collection exists with the correct configuration.
    /// If the collection already exists, it returns without making changes.
    pub(crate) async fn init_collection(&self) -> Result<(), CustomError> {
        // Check if collection exists
        let collections = self.client.list_collections().await?;
        if collections.iter().any(|name| name == &self.config.collection_name) {
            return Ok(());
        }

        // Create collection with the specified configuration
        self.client.create_collection(&self.config.collection_name, self.config.vector_size).await
    }

    /// Stores a new memory in the database
    ///
    /// This method performs validation and stores the memory using upsert semantics.
    /// If a memory with the same ID exists, it will be overwritten.
    ///
    /// # Errors
    /// - Returns MemoryValidation error if the embedding size doesn't match the configuration
    /// - Returns DatabaseError for any database operation failures
    pub(crate) async fn store_memory(&self, memory: &MemoryEntry) -> Result<(), CustomError> {
        if memory.embedding.len() != self.config.vector_size as usize {
            return Err(CustomError::MemoryValidation(format!(
                "Embedding size mismatch. Expected {}, got {}",
                self.config.vector_size,
                memory.embedding.len()
            )));
        }

        memory.validate()?;
        let point = self.memory_to_point(memory);
        self.client.upsert_points(&self.config.collection_name, vec![point]).await
    }

    /// Updates an existing memory in the database
    ///
    /// This is an alias for store_memory since both operations use upsert semantics.
    /// The method exists to provide semantic clarity when the intent is to update.
    pub(crate) async fn update_memory(&self, memory: &MemoryEntry) -> Result<(), CustomError> {
        self.store_memory(memory).await
    }

    /// Deletes a memory by its ID
    pub(crate) async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        let point_id: PointId = id.to_string().into();
        let selector = vec![point_id].into();
        self.client.delete_points(&self.config.collection_name, &selector).await
    }

    /// Searches for memories using a query vector and optional filters
    ///
    /// # Arguments
    /// * `query_vector` - The vector to search for similar memories
    /// * `top_k` - Maximum number of results to return
    /// * `filters` - Optional filters to apply to the search
    ///
    /// # Errors
    /// - Returns MemoryValidation error if the query vector size doesn't match the configuration
    /// - Returns DatabaseError for any database operation failures
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

        let mut search_req = SearchPointsBuilder::new(
            &self.config.collection_name,
            query_vector.to_vec(),
            top_k as u64
        )
        .with_payload(true);

        if let Some(f) = filters {
            search_req = search_req.filter(self.build_filter(f));
        }

        let search_req = search_req.build();

        let points = self.client.search_points(&search_req).await?;
        let mut memories = Vec::with_capacity(points.len());
        for point in points {
            memories.push(self.scored_point_to_memory(&point)?);
        }
        Ok(memories)
    }

    /// Inserts multiple memories in a single operation
    ///
    /// # Errors
    /// - Returns MemoryValidation error if any memory's embedding size doesn't match
    /// - Returns DatabaseError for any database operation failures
    pub(crate) async fn bulk_insert(&self, memories: &[MemoryEntry]) -> Result<(), CustomError> {
        // Validate all memories first
        for memory in memories {
            if memory.embedding.len() != self.config.vector_size as usize {
                return Err(CustomError::MemoryValidation(format!(
                    "Embedding size mismatch. Expected {}, got {}",
                    self.config.vector_size,
                    memory.embedding.len()
                )));
            }
            memory.validate()?;
        }

        let points: Vec<PointStruct> = memories.iter().map(|m| self.memory_to_point(m)).collect();
        self.client.upsert_points(&self.config.collection_name, points).await
    }

    // Helper methods
    fn memory_to_point(&self, memory: &MemoryEntry) -> PointStruct {
        use std::collections::HashMap;
        use qdrant_client::qdrant::Value;

        let mut payload_map = HashMap::new();

        // Add required fields
        payload_map.insert(
            "memory_type".to_string(),
            Value::from(format!("{:?}", memory.memory_type).to_lowercase())
        );
        payload_map.insert(
            "content".to_string(),
            Value::from(memory.content.clone())
        );

        // Add episodic-specific fields if present
        if let Some(timestamp) = &memory.timestamp {
            payload_map.insert(
                "timestamp".to_string(),
                Value::from(timestamp.timestamp())
            );
        }
        if let Some(location) = &memory.location_text {
            payload_map.insert(
                "location_text".to_string(),
                Value::from(location.clone())
            );
        }
        if let Some(participants) = &memory.participants {
            payload_map.insert(
                "participants".to_string(),
                Value::from(participants.clone())
            );
        }

        return PointStruct {
            id: Some(format!("{:?}", memory.id).into()),
            payload: payload_map,
            vectors: Some(memory.embedding.clone().into()),
        };
    }

    fn scored_point_to_memory(&self, point: &ScoredPoint) -> Result<MemoryEntry, CustomError> {
        let payload = &point.payload;

        // Extract required fields with proper error handling
        let memory_type_str = payload.get("memory_type")
            .ok_or_else(|| CustomError::DatabaseError("Missing memory_type in payload".to_string()))?
            .as_str()
            .ok_or_else(|| CustomError::DatabaseError("Invalid memory_type format".to_string()))?;

        let memory_type = match memory_type_str.as_str() {
            "episodic" => MemoryType::Episodic,
            _ => MemoryType::Semantic,
        };

        let content = payload.get("content")
            .ok_or_else(|| CustomError::DatabaseError("Missing content in payload".to_string()))?
            .as_str()
            .ok_or_else(|| CustomError::DatabaseError("Invalid content format".to_string()))?
            .to_string();

        let embedding = if let Some(vectors) = &point.vectors {
            if let Some(VectorsOptions::Vector(ref vec)) = vectors.vectors_options {
                vec.data.clone()
            } else {
                return Err(CustomError::DatabaseError("Invalid vectors format".to_string()));
            }
        } else {
            return Err(CustomError::DatabaseError("Missing vectors in point".to_string()));
        };

        let id = point.id.as_ref()
            .ok_or_else(|| CustomError::DatabaseError("Missing point ID".to_string()))?;
        let id = Uuid::parse_str(&format!("{:?}", id))
            .map_err(|e| CustomError::DatabaseError(format!("Invalid UUID format: {}", e)))?;

        // Extract optional fields
        let timestamp = payload
            .get("timestamp")
            .and_then(|v| v.as_integer())
            .map(|ts| DateTime::from_timestamp(ts, 0)
            .ok_or_else(|| CustomError::DatabaseError("Invalid timestamp value".to_string())))
            .transpose()?;

        let location_text = payload
            .get("location_text")
            .and_then(|v| v.as_str().map(|s| s.to_owned()));

        let participants = payload
            .get("participants")
            .and_then(|v| {
                v.as_list()?
                    .iter()
                    .map(|item| {
                        item.as_str().map(|s| s.to_owned())
                    })
                    .collect::<Option<Vec<String>>>()
            });

        return Ok(MemoryEntry {
            id,
            memory_type,
            content,
            embedding,
            timestamp,
            location_text,
            participants,
        });
    }

    fn build_filter(&self, filters: &MemoryFilters) -> Filter {
        let mut conditions = Vec::new();

        if let Some(memory_type) = &filters.memory_type {
            conditions.push(Condition::matches("memory_type", memory_type.to_lowercase()));
        }

        // Handle date range filtering
        let mut date_range = Range::default();
        let mut has_date_filter = false;

        if let Some(start_date) = &filters.start_date {
            date_range.gte = Some((start_date.timestamp() as f64).into());
            has_date_filter = true;
        }

        if let Some(end_date) = &filters.end_date {
            date_range.lte = Some((end_date.timestamp() as f64).into());
            has_date_filter = true;
        }

        if has_date_filter {
            conditions.push(Condition::range("timestamp", date_range));
        }

        if let Some(location) = &filters.location_text {
            conditions.push(Condition::matches("location_text", location.clone()));
        }

        if let Some(participants) = &filters.participants {
            for participant in participants {
                conditions.push(Condition::matches("participants", participant.clone()));
            }
        }

        Filter::must(conditions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MemoryInput;
    use crate::databases::vector_database::MockVectorDatabase;
    use mockall::predicate::*;
    use chrono::Utc;
    use qdrant_client::qdrant::{PointsSelector, SearchPoints, Value, VectorsOutput, VectorOutput};
    use qdrant_client::qdrant::vectors_output::VectorsOptions;
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
        use qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Points;

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
                .withf(move |collection: &str, points: &Vec<PointStruct>| {
                    collection == "test_memories" && points.len() == 1 &&
                    format!("{:?}", points[0].id.as_ref().unwrap()) == format!("{:?}", memory_id)
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
                .withf(move |collection: &str, points: &Vec<PointStruct>| {
                    collection == "test_memories" && points.len() == 1 &&
                    format!("{:?}", points[0].id.as_ref().unwrap()) == format!("{:?}", memory_id)
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
                .withf(move |collection: &str, selector: &PointsSelector| {
                    collection == "test_memories" &&
                    if let Some(points_selector) = &selector.points_selector_one_of {
                        matches!(points_selector, Points(ids) if ids.ids.get(0).map_or(false, |id|
                            format!("{:?}", id) == format!("{:?}", memory_id)
                        ))
                    } else {
                        false
                    }
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

            let mut payload = HashMap::new();
            payload.insert("memory_type".to_string(), Value::from("episodic"));
            payload.insert("content".to_string(), Value::from("Test memory"));
            if let Some(ts) = memory.timestamp.as_ref() {
                payload.insert("timestamp".to_string(), Value::from(ts.timestamp()));
            }
            payload.insert("location_text".to_string(), Value::from("Test Location"));
            payload.insert("participants".to_string(), Value::from(vec!["Alice".to_string(), "Bob".to_string()]));

            let search_result = vec![ScoredPoint {
                id: Some(memory.id.to_string().into()),
                payload,
                vectors: Some(VectorsOutput {
                    vectors_options: Some(VectorsOptions::Vector(
                        VectorOutput {
                            indices: None,
                            vector: Some(qdrant_client::qdrant::vector_output::Vector::Dense(
                                qdrant_client::qdrant::DenseVector {
                                    data: vec![0.1; vector_size]
                                }
                            )),
                            vectors_count: Some(1),
                            data: vec![0.1; vector_size],
                        }
                    ))
                }),
                score: 0.9,
                version: 1,
                shard_key: None,
                order_value: None,
            }];

            mock_db.expect_search_points()
                .withf({
                    let vs = vector_size;
                    move |search: &SearchPoints| {
                        search.collection_name == "test_memories" &&
                        search.vector.len() == vs &&
                        search.limit == 1
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
        use qdrant_client::qdrant::DenseVector;
        use qdrant_client::qdrant::vector_output::Vector;

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

            let search_result = vec![ScoredPoint {
                id: Some(memory2.id.to_string().into()),
                payload,
                vectors: Some(VectorsOutput {
                    vectors_options: Some(VectorsOptions::Vector(
                        VectorOutput {
                            indices: None,
                            vector: Some(Vector::Dense(
                                DenseVector {
                                    data: vec![0.1; vector_size]
                                }
                            )),
                            vectors_count: Some(1),
                            data: vec![0.1; vector_size],
                        }
                    ))
                }),
                score: 0.9,
                version: 1,
                shard_key: None,
                order_value: None,
            }];

            mock_db.expect_search_points()
                .withf({
                    let vs = vector_size;
                    move |search: &SearchPoints| {
                        search.collection_name == "test_memories" &&
                        search.vector.len() == vs &&
                        search.limit == 10 &&
                        matches!(search.filter.as_ref(), Some(filter) if filter.must.len() == 2)
                    }
                })
                .times(1)
                .returning(move |_| Ok(search_result.clone()));

            // Create repository with mock
            let repo = VectorMemoryRepository::new(mock_db, config);

            // Initialize collection
            repo.init_collection().await.unwrap();

            // Test date range filtering
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
