mod config;
mod errors;
mod models;
mod repositories;
mod infrastructures;

use config::settings::Settings;
use config::database_settings::DatabaseSettings;
use errors::custom::CustomError;
use models::public::memory::Memory;
use models::public::memory_input::MemoryInput;
use models::public::memory_filters::MemoryFilters;
use repositories::memory_repository;
use infrastructures::external_services::{
    openai_embedding_repository::OpenAIEmbeddingRepository,
    qdrant_vector_memory_repository::QdrantVectorMemoryRepository,
};

/// Initialize the library with externally supplied settings.
#[allow(unused_variables)]
pub fn init(settings: Settings) -> Result<(), CustomError> {
    // Initialization logic here.
    Ok(())
}

/// Initialize the library by loading settings from the environment.
/// This function is primarily intended for integration tests.
#[allow(dead_code)]
pub(crate) fn init_from_env() -> Result<(), CustomError> {
    let settings = Settings::load()?;
    init(settings)
}

/// AgentMemory provides a high-level API for memory operations.
/// The public API no longer exposes any Qdrant-specific types.
pub struct AgentMemory {
    settings: Settings,
    collection_name: String,
    memory_repo: memory_repository::MemoryRepository,
}

impl AgentMemory {
    /// Constructs a new AgentMemory instance.
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
        let memory_repo = memory_repository::MemoryRepository::new(embed_repo, vector_repo);

        Ok(Self {
            settings,
            collection_name,
            memory_repo,
        })
    }

    /// Creates a new memory entry.
    pub async fn create_memory(&self, input: MemoryInput) -> Result<Memory, CustomError> {
        let mem_entry = self.memory_repo.create_memory(input).await?;
        Ok(mem_entry.into_public())
    }

    /// Retrieves a memory entry by its unique identifier.
    pub async fn get_memory_by_id(&self, id: uuid::Uuid) -> Result<Memory, CustomError> {
        let mem_entry = self.memory_repo.get_memory_by_id(id).await?;
        Ok(mem_entry.into_public())
    }

    /// Updates an existing memory entry.
    pub async fn update_memory(&self, input: MemoryInput) -> Result<Memory, CustomError> {
        let mem_entry = self.memory_repo.update_memory(input).await?;
        Ok(mem_entry.into_public())
    }

    /// Deletes a memory entry by its unique identifier.
    pub async fn delete_memory(&self, id: uuid::Uuid) -> Result<(), CustomError> {
        self.memory_repo.delete_memory(id).await
    }

    /// Searches for memory entries that are semantically similar to the query.
    pub async fn search_memories(
        &self,
        query: &str,
        top_k: usize,
        filters: Option<MemoryFilters>,
    ) -> Result<Vec<Memory>, CustomError> {
        let entries = self.memory_repo.search_memories(query, top_k, filters).await?;
        Ok(entries.into_iter().map(|entry| entry.into_public()).collect())
    }
}
