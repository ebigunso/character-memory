use crate::errors::custom::CustomError;
use crate::config::embedding_model::EmbeddingModel;
use crate::config::vector_memory_config::VectorMemoryConfig;
use qdrant_client::Qdrant;

/// Settings for configuring the vector database.
/// Currently only supports Qdrant, but designed to be extensible
/// for future database implementations.
#[derive(Clone)]
pub enum DatabaseSettings {
    /// Settings for a Qdrant database instance
    Qdrant {
        /// Connection URL for the Qdrant server
        url: String,
        /// The embedding model to use
        model: EmbeddingModel,
    },
}

impl DatabaseSettings {
    /// Creates a new Qdrant database settings instance
    pub fn qdrant(url: String, model: EmbeddingModel) -> Self {
        Self::Qdrant { url, model }
    }

    /// Creates the appropriate vector memory configuration based on the database type
    pub(crate) fn create_vector_memory_config(&self, collection_name: String) -> VectorMemoryConfig {
        match self {
            DatabaseSettings::Qdrant { url, model } => {
                VectorMemoryConfig::new(url.clone(), collection_name, *model)
            }
        }
    }

    /// Internal helper to create the appropriate database client
    pub(crate) async fn create_database_client(&self) -> Result<Qdrant, CustomError> {
        match self {
            DatabaseSettings::Qdrant { url, .. } => {
                let config = qdrant_client::config::QdrantConfig::from_url(url);
                Ok(Qdrant::new(config)?)
            }
        }
    }
}
