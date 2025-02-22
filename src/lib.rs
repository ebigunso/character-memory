mod config;
mod errors;
mod models;
mod repositories;
mod infrastructures;

use config::settings::{Settings, DatabaseSettings};
use errors::CustomError;
use models::public::Memory;
use models::public::MemoryInput;
use models::public::MemoryFilters;
use repositories::MemoryRepository;
use infrastructures::external_services::{OpenAIEmbeddingRepository, QdrantVectorMemoryRepository};

/// AgentMemory provides a high-level API for memory operations.
pub struct AgentMemory {
    memory_repo: MemoryRepository,
}

impl AgentMemory {
    /// Constructs a new AgentMemory instance.
    ///
    /// # Arguments
    /// * `settings` - Configuration settings for the memory system
    /// * `collection_name` - Name of the vector collection to use
    /// * `database_settings` - Database-specific configuration settings
    ///
    /// # Returns
    /// A Result containing the new AgentMemory instance or a CustomError if initialization fails.
    pub async fn new(
        settings: Settings,
        collection_name: String,
        database_settings: DatabaseSettings,
    ) -> Result<Self, CustomError> {
        // Create the embedding repository using provided settings
        let embed_repo = Box::new(OpenAIEmbeddingRepository::new(&settings)?);

        // Configure and create the vector memory repository
        let vector_config = database_settings.create_vector_memory_config(collection_name.clone());
        let vector_repo = Box::new(QdrantVectorMemoryRepository::new(vector_config)?);
        // Assemble the high-level MemoryRepository.
        let memory_repo = MemoryRepository::new(embed_repo, vector_repo);

        Ok(Self { memory_repo })
    }

    /// Creates a new memory entry.
    ///
    /// # Arguments
    /// * `input` - A MemoryInput struct containing the data for the new memory entry
    ///
    /// # Returns
    /// A Result containing the created Memory entry or a CustomError if the operation fails.
    pub async fn create_memory(&self, input: MemoryInput) -> Result<Memory, CustomError> {
        let mem_entry = self.memory_repo.create_memory(input).await?;
        Ok(mem_entry.into_public())
    }

    /// Creates multiple memory entries in a batch.
    ///
    /// # Arguments
    /// * `inputs` - A slice of MemoryInput structs containing the data for each memory entry.
    ///
    /// # Returns
    /// A Result containing a vector of created Memory entries or a CustomError if the operation fails.
    pub async fn bulk_create_memories(&self, inputs: &[MemoryInput]) -> Result<Vec<Memory>, CustomError> {
        let entries = self.memory_repo.bulk_create_memories(inputs).await?;
        Ok(entries.into_iter().map(|entry| entry.into_public()).collect())
    }

    /// Retrieves a memory entry by its unique identifier.
    ///
    /// # Arguments
    /// * `id` - The UUID of the memory entry to retrieve
    ///
    /// # Returns
    /// A Result containing the requested Memory entry or a CustomError if the operation fails.
    pub async fn get_memory_by_id(&self, id: uuid::Uuid) -> Result<Memory, CustomError> {
        let mem_entry = self.memory_repo.get_memory_by_id(id).await?;
        Ok(mem_entry.into_public())
    }

    /// Searches for memory entries that are semantically similar to the query.
    ///
    /// # Arguments
    /// * `query` - The search query string to find similar memories
    /// * `top_k` - The maximum number of results to return
    /// * `filters` - Optional filters to apply to the search results
    ///
    /// # Returns
    /// A Result containing a vector of Memory entries or a CustomError if the operation fails.
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
    /// # Arguments
    /// * `input` - A MemoryInput struct containing the updated data for the memory entry
    ///
    /// # Returns
    /// A Result containing the updated Memory entry or a CustomError if the operation fails.
    pub async fn update_memory(&self, input: MemoryInput) -> Result<Memory, CustomError> {
        let mem_entry = self.memory_repo.update_memory(input).await?;
        Ok(mem_entry.into_public())
    }

    /// Deletes a memory entry by its unique identifier.
    ///
    /// # Arguments
    /// * `id` - The UUID of the memory entry to delete
    ///
    /// # Returns
    /// A Result containing unit type or a CustomError if the operation fails.
    pub async fn delete_memory(&self, id: uuid::Uuid) -> Result<(), CustomError> {
        self.memory_repo.delete_memory(id).await
    }
}
