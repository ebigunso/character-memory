mod config;
mod errors;
mod infrastructures;
mod models;
mod repositories;

use uuid::Uuid;

use crate::config::settings::{EmbeddingRepositorySettings, VectorMemoryRepositorySettings};
use crate::infrastructures::external_services::{
    OpenAIEmbeddingRepository, QdrantVectorMemoryRepository,
};
use crate::models::vector::VectorMetadata;
use crate::repositories::MemoryRepository;

// Re-export types for public use
pub use crate::config::settings::Settings;
pub use crate::errors::CustomError;
pub use crate::models::memory::dto::{Memory, MemoryFilters, MemoryInput, ScoredMemory};
pub use crate::models::memory::MemoryType;
pub use crate::models::memory::{MemoryEntry, ScoredMemoryEntry};
pub use crate::repositories::{EmbeddingRepository, VectorMemoryRepository};

// Re-export for integration tests
pub mod test_utils {
    use crate::config::settings::Settings;
    use crate::errors::CustomError;

    /// Loads settings from environment variables for integration tests.
    ///
    /// # Important
    ///
    /// This function is intended ONLY for use in integration tests and should not be used in production code.
    /// A `.env` file in the project root directory will be loaded if present,
    /// otherwise existing environment variables are used.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `Settings` instance with configuration loaded from environment
    /// - `Err`: A `CustomError` if loading fails
    pub fn load_test_settings() -> Result<Settings, CustomError> {
        Settings::load()
    }
}

/// AgentMemory provides a high-level API for memory operations.
///
/// # Description
///
/// This struct serves as the main entry point for memory operations,
/// providing a high-level interface for storing, retrieving, and
/// searching memory entries.
pub struct AgentMemory {
    memory_repo: MemoryRepository,
}

impl AgentMemory {
    /// Constructs a new AgentMemory instance from caller-provided repositories.
    ///
    /// # Description
    ///
    /// This constructor allows callers to inject custom embedding generation and vector storage
    /// implementations. This is useful for deterministic tests and for plugging in alternative
    /// backends.
    ///
    /// # Parameters
    ///
    /// - `embed_repo`: Embedding generator implementation
    /// - `vector_repo`: Vector memory storage implementation
    pub fn new_with_repositories(
        embed_repo: Box<dyn EmbeddingRepository>,
        vector_repo: Box<dyn VectorMemoryRepository>,
    ) -> Self {
        let memory_repo = MemoryRepository::new(embed_repo, vector_repo);
        Self { memory_repo }
    }

    /// Constructs a new AgentMemory instance.
    ///
    /// # Parameters
    ///
    /// - `settings`: Configuration settings for the memory system
    /// - `collection_name`: Name of the vector collection to use
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `AgentMemory` instance
    /// - `Err`: A `CustomError` if initialization fails
    pub async fn new(settings: Settings, collection_name: String) -> Result<Self, CustomError> {
        // Configure and create the embedding repository
        let embedding_settings = EmbeddingRepositorySettings::new(
            settings.get_openai_api_key().to_string(),
            settings.get_embedding_model()?,
        );
        let embed_repo = Box::new(OpenAIEmbeddingRepository::new(embedding_settings)?);

        // Configure and create the vector memory repository
        let vector_memory_settings = VectorMemoryRepositorySettings::new(
            settings.get_qdrant_connection().to_string(),
            collection_name.clone(),
            settings.get_embedding_model()?,
        );
        let vector_repo = Box::new(QdrantVectorMemoryRepository::new(vector_memory_settings)?);
        // Assemble the high-level MemoryRepository.
        let memory_repo = MemoryRepository::new(embed_repo, vector_repo);

        Ok(Self { memory_repo })
    }

    /// Initializes the storage systems.
    ///
    /// # Description
    ///
    /// Ensures all required storage systems are properly initialized before any operations are performed.
    /// This should be called during application startup.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if initialization succeeds
    /// - `Err`: A `CustomError` if initialization fails
    pub async fn init_storage(&self) -> Result<(), CustomError> {
        self.memory_repo.init_storage().await
    }

    /// Creates a new memory entry.
    ///
    /// # Parameters
    ///
    /// - `input`: A `MemoryInput` containing the data for the new memory entry
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `Memory` containing the created entry
    /// - `Err`: A `CustomError` if the operation fails
    pub async fn create_memory(&self, input: MemoryInput) -> Result<Memory, CustomError> {
        let metadata = VectorMetadata::from_memory_input(input)?;
        let mem_entry = self.memory_repo.create_memory(metadata).await?;
        Ok(mem_entry.into_public())
    }

    /// Creates multiple memory entries in a batch.
    ///
    /// # Parameters
    ///
    /// - `inputs`: A slice of `MemoryInput` containing the data for each memory entry
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `Memory` containing the created entries
    /// - `Err`: A `CustomError` if the operation fails
    pub async fn bulk_create_memories(
        &self,
        inputs: &[MemoryInput],
    ) -> Result<Vec<Memory>, CustomError> {
        let metadata_list: Result<Vec<_>, _> = inputs
            .iter()
            .map(|input| VectorMetadata::from_memory_input(input.clone()))
            .collect();
        let metadata_list = metadata_list?;

        let entries = self
            .memory_repo
            .bulk_create_memories(&metadata_list)
            .await?;
        Ok(entries
            .into_iter()
            .map(|entry| entry.into_public())
            .collect())
    }

    /// Retrieves a memory entry by its unique identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: The UUID of the memory entry to retrieve
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `Memory` containing the requested entry
    /// - `Err`: A `CustomError` if the operation fails
    pub async fn get_memory_by_id(&self, id: Uuid) -> Result<Memory, CustomError> {
        let mem_entry = self.memory_repo.get_memory_by_id(id).await?;
        Ok(mem_entry.into_public())
    }

    /// Retrieves multiple memory entries by their unique identifiers.
    ///
    /// # Parameters
    ///
    /// - `ids`: A slice of UUIDs of the memory entries to retrieve
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `Memory` containing the requested entries
    /// - `Err`: A `CustomError` if the operation fails
    pub async fn get_memories_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Memory>, CustomError> {
        let mem_entries = self.memory_repo.get_memories_by_ids(ids).await?;
        Ok(mem_entries
            .into_iter()
            .map(|entry| entry.into_public())
            .collect())
    }

    /// Searches for memory entries that are semantically similar to the query.
    ///
    /// # Parameters
    ///
    /// - `query`: The search query string to find similar memories
    /// - `top_k`: The maximum number of results to return
    /// - `filters`: Optional filters to apply to the search results
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `Memory` containing the search results
    /// - `Err`: A `CustomError` if the operation fails
    pub async fn search_memories(
        &self,
        query: &str,
        top_k: usize,
        filters: Option<MemoryFilters>,
    ) -> Result<Vec<ScoredMemory>, CustomError> {
        let entries = self
            .memory_repo
            .search_memories(query, top_k, filters)
            .await?;

        Ok(entries
            .into_iter()
            .map(|entry| entry.into_public())
            .collect())
    }

    /// Updates an existing memory entry.
    ///
    /// # Parameters
    ///
    /// - `input`: A `MemoryInput` containing the updated data and ID of the entry to update
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `Memory` containing the updated entry
    /// - `Err`: A `CustomError` if:
    ///     - The input does not contain an ID
    ///     - The update operation fails
    pub async fn update_memory(&self, input: MemoryInput) -> Result<Memory, CustomError> {
        let metadata = VectorMetadata::from_memory_input(input)?;
        let mem_entry = self.memory_repo.update_memory(metadata).await?;
        Ok(mem_entry.into_public())
    }

    /// Deletes a memory entry by its unique identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: The UUID of the memory entry to delete
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if deletion succeeds
    /// - `Err`: A `CustomError` if the operation fails
    pub async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        self.memory_repo.delete_memory(id).await
    }
}

#[cfg(test)]
mod injection_tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct FakeEmbeddingRepository;

    impl FakeEmbeddingRepository {
        fn embed_for(text: &str) -> Vec<f32> {
            // Deterministic, fixed-size embedding for unit tests.
            vec![text.len() as f32, 1.0, 0.0]
        }
    }

    #[async_trait]
    impl EmbeddingRepository for FakeEmbeddingRepository {
        async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, CustomError> {
            Ok(Self::embed_for(text))
        }

        async fn bulk_generate_embeddings<'a>(
            &self,
            texts: &'a [&'a str],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            Ok(texts.iter().map(|t| Self::embed_for(t)).collect())
        }
    }

    #[derive(Clone, Default)]
    struct InMemoryVectorRepository {
        entries: Arc<Mutex<HashMap<Uuid, MemoryEntry>>>,
    }

    impl InMemoryVectorRepository {
        fn new() -> Self {
            Self::default()
        }
    }

    fn dot(a: &[f32], b: &[f32]) -> Result<f32, CustomError> {
        if a.len() != b.len() {
            return Err(CustomError::DatabaseError(
                "Vector length mismatch".to_string(),
            ));
        }
        Ok(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum())
    }

    #[async_trait]
    impl VectorMemoryRepository for InMemoryVectorRepository {
        async fn init_collection(&self) -> Result<(), CustomError> {
            Ok(())
        }

        async fn store_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
            let mut guard = self
                .entries
                .lock()
                .map_err(|_| CustomError::DatabaseError("Lock poisoned".to_string()))?;
            guard.insert(memory.id, memory.clone());
            Ok(())
        }

        async fn update_memory<'a>(&'a self, memory: &'a MemoryEntry) -> Result<(), CustomError> {
            self.store_memory(memory).await
        }

        async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
            let mut guard = self
                .entries
                .lock()
                .map_err(|_| CustomError::DatabaseError("Lock poisoned".to_string()))?;
            guard.remove(&id);
            Ok(())
        }

        async fn search_memory<'a>(
            &'a self,
            query_vector: &'a [f32],
            top_k: usize,
            _filters: Option<&'a MemoryFilters>,
        ) -> Result<Vec<ScoredMemoryEntry>, CustomError> {
            let guard = self
                .entries
                .lock()
                .map_err(|_| CustomError::DatabaseError("Lock poisoned".to_string()))?;

            let mut scored: Vec<ScoredMemoryEntry> = guard
                .values()
                .map(|entry| {
                    let score = dot(query_vector, &entry.embedding)?;
                    Ok::<ScoredMemoryEntry, CustomError>(ScoredMemoryEntry {
                        entry: entry.clone(),
                        score,
                    })
                })
                .collect::<Result<_, _>>()?;

            scored.sort_by(|a, b| b.score.total_cmp(&a.score));
            scored.truncate(top_k);
            Ok(scored)
        }

        async fn bulk_insert<'a>(&'a self, memories: &'a [MemoryEntry]) -> Result<(), CustomError> {
            for memory in memories {
                self.store_memory(memory).await?;
            }
            Ok(())
        }

        async fn get_memories_by_ids<'a>(
            &'a self,
            ids: &'a [Uuid],
        ) -> Result<Vec<MemoryEntry>, CustomError> {
            let guard = self
                .entries
                .lock()
                .map_err(|_| CustomError::DatabaseError("Lock poisoned".to_string()))?;

            let mut out = Vec::with_capacity(ids.len());
            for id in ids {
                let entry = guard.get(id).ok_or_else(|| {
                    CustomError::DatabaseError(format!("Memory with ID {id} not found"))
                })?;
                out.push(entry.clone());
            }
            Ok(out)
        }
    }

    #[tokio::test]
    async fn injected_repositories_create_and_store_memory_deterministically() {
        let vector_repo = InMemoryVectorRepository::new();
        let store_handle = vector_repo.entries.clone();

        let agent = AgentMemory::new_with_repositories(
            Box::new(FakeEmbeddingRepository),
            Box::new(vector_repo),
        );

        let id = Uuid::new_v4();
        let input = MemoryInput {
            id: Some(id),
            content: "hello".to_string(),
            memory_type: MemoryType::Semantic,
            timestamp: None,
            location_text: None,
            participants: None,
        };

        let created = agent.create_memory(input).await.expect("create succeeds");
        assert_eq!(created.id, id);
        assert_eq!(created.content, "hello");

        let stored = store_handle
            .lock()
            .expect("lock")
            .get(&id)
            .cloned()
            .expect("stored entry exists");
        assert_eq!(
            stored.embedding,
            FakeEmbeddingRepository::embed_for("hello")
        );
    }

    #[tokio::test]
    async fn injected_vector_repo_search_orders_by_score() {
        let vector_repo = InMemoryVectorRepository::new();
        let agent = AgentMemory::new_with_repositories(
            Box::new(FakeEmbeddingRepository),
            Box::new(vector_repo),
        );

        let short_id = Uuid::new_v4();
        let long_id = Uuid::new_v4();

        agent
            .create_memory(MemoryInput {
                id: Some(short_id),
                content: "aaaaa".to_string(),
                memory_type: MemoryType::Semantic,
                timestamp: None,
                location_text: None,
                participants: None,
            })
            .await
            .expect("create short");

        agent
            .create_memory(MemoryInput {
                id: Some(long_id),
                content: "aaaaaaaaaa".to_string(),
                memory_type: MemoryType::Semantic,
                timestamp: None,
                location_text: None,
                participants: None,
            })
            .await
            .expect("create long");

        let results = agent
            .search_memories("aaaaa", 2, None)
            .await
            .expect("search succeeds");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].memory.id, long_id);
        assert_eq!(results[1].memory.id, short_id);
        assert!(results[0].score >= results[1].score);
    }
}
