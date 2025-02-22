use qdrant_client::Qdrant;

use crate::errors::CustomError;
use crate::config::enums::EmbeddingModel;
use crate::config::settings::VectorMemorySettings;

/// Settings for configuring the vector database.
///
/// # Description
///
/// Currently only supports Qdrant, but designed to be extensible
/// for future database implementations.
///
/// # See also
///
/// - [`VectorMemorySettings`]
/// - [`EmbeddingModel`]
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
    /// Creates a new Qdrant database settings instance.
    ///
    /// # Parameters
    ///
    /// - `url`: Connection URL for the Qdrant server
    /// - `model`: The embedding model to use
    ///
    /// # Returns
    ///
    /// A new `DatabaseSettings::Qdrant` instance
    pub fn qdrant(url: String, model: EmbeddingModel) -> Self {
        Self::Qdrant { url, model }
    }

    /// Creates the appropriate vector memory configuration based on the database type.
    ///
    /// # Parameters
    ///
    /// - `collection_name`: Name of the collection to store memories
    ///
    /// # Returns
    ///
    /// A `VectorMemorySettings` instance configured for the specific database type
    pub(crate) fn create_vector_memory_config(&self, collection_name: String) -> VectorMemorySettings {
        match self {
            DatabaseSettings::Qdrant { url, model } => {
                VectorMemorySettings::new(url.clone(), collection_name, *model)
            }
        }
    }

    /// Internal helper to create the appropriate database client.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `Qdrant` client instance
    /// - `Err`: A `CustomError` if client creation fails
    pub(crate) async fn create_database_client(&self) -> Result<Qdrant, CustomError> {
        match self {
            DatabaseSettings::Qdrant { url, .. } => {
                let config = qdrant_client::config::QdrantConfig::from_url(url);
                Ok(Qdrant::new(config)?)
            }
        }
    }
}
