mod config;
mod databases;
mod errors;
mod models;
mod repositories;

use config::settings::Settings;
use crate::errors::custom::CustomError;
use crate::models::{Memory, MemoryInput, MemoryFilters};
use crate::repositories::{embedding_repository, memory_repository, vector_memory_repository};
use crate::databases::qdrant::QdrantDatabaseImpl;
use qdrant_client::Qdrant;

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
/// It encapsulates the required settings, repository configuration,
/// and exposes methods for memory manipulation.
pub struct AgentMemory {
    settings: Settings,
    collection_name: String,
    memory_repo: memory_repository::MemoryRepository<QdrantDatabaseImpl>,
}

impl AgentMemory {
    /// Constructs a new AgentMemory instance.
    /// The consumer must provide the application settings and the desired collection name.
    pub async fn new(settings: Settings, collection_name: String) -> Result<Self, CustomError> {
        // Create the embedding repository using provided settings.
        let embed_repo = embedding_repository::EmbeddingRepository::new(&settings)?;
        // Instantiate a Qdrant client using the connection string from settings by constructing a QdrantConfig.
        use qdrant_client::config::QdrantConfig;
        let q_config = QdrantConfig::from_url(settings.get_qdrant_connection());
        let qdrant_client = Qdrant::new(q_config)?;
        // Create the Qdrant database implementation.
        let qdrant_db = QdrantDatabaseImpl::new(qdrant_client);
        // Configure the vector memory repository. Adjust dimensions as needed.
        let vector_config = vector_memory_repository::VectorMemoryConfig::text_embedding_3_large(
            settings.get_qdrant_connection().to_string(),
            collection_name.clone()
        );
        // Create the vector memory repository.
        let vector_repo =
            vector_memory_repository::VectorMemoryRepository::new(qdrant_db, vector_config);
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
        filters: Option<MemoryFilters>
    ) -> Result<Vec<Memory>, CustomError> {
        let entries = self.memory_repo.search_memories(query, top_k, filters).await?;
        Ok(entries.into_iter().map(|entry| entry.into_public()).collect())
    }
}
