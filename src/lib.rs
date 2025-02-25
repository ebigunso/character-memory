mod config;
mod errors;
mod models;
mod repositories;
mod infrastructures;

use config::settings::{Settings, VectorMemoryRepositorySettings, EmbeddingRepositorySettings};
use errors::CustomError;
use infrastructures::external_services::{OpenAIEmbeddingRepository, QdrantVectorMemoryRepository};
use models::vector::VectorMetadata;
use models::memory::dto::{Memory, MemoryFilters, MemoryInput};
use models::memory::MemoryType;
use repositories::MemoryRepository;

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
    pub async fn new(
        settings: Settings,
        collection_name: String,
    ) -> Result<Self, CustomError> {
        // Configure and create the embedding repository
        let embedding_settings = EmbeddingRepositorySettings::new(
            settings.get_openai_api_key().to_string(),
            settings.get_embedding_model()?
        );
        let embed_repo = Box::new(OpenAIEmbeddingRepository::new(embedding_settings)?);

        // Configure and create the vector memory repository
        let vector_memory_settings = VectorMemoryRepositorySettings::new(
            settings.get_qdrant_connection().to_string(),
            collection_name.clone(),
            settings.get_embedding_model()?
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
    pub async fn bulk_create_memories(&self, inputs: &[MemoryInput]) -> Result<Vec<Memory>, CustomError> {
        let metadata_list: Result<Vec<_>, _> = inputs.iter()
            .map(|input| VectorMetadata::from_memory_input(input.clone()))
            .collect();
        let metadata_list = metadata_list?;

        let entries = self.memory_repo.bulk_create_memories(&metadata_list).await?;
        Ok(entries.into_iter().map(|entry| entry.into_public()).collect())
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
    pub async fn get_memory_by_id(&self, id: uuid::Uuid) -> Result<Memory, CustomError> {
        let mem_entry = self.memory_repo.get_memory_by_id(id).await?;
        Ok(mem_entry.into_public())
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
    ) -> Result<Vec<Memory>, CustomError> {
        let entries = self.memory_repo.search_memories(query, top_k, filters).await?;
        Ok(entries.into_iter().map(|entry| entry.into_public()).collect())
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
    pub async fn delete_memory(&self, id: uuid::Uuid) -> Result<(), CustomError> {
        self.memory_repo.delete_memory(id).await
    }
}
