use crate::errors::custom::CustomError;
use qdrant_client::config::QdrantConfig;
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
    },
}

impl DatabaseSettings {
    /// Creates a new Qdrant database settings instance
    pub fn qdrant(url: String) -> Self {
        Self::Qdrant { url }
    }

    /// Internal helper to create the appropriate database implementation
    pub(crate) async fn create_database(&self) -> Result<Box<dyn super::vector_database::VectorDatabase + Send + Sync>, CustomError> {
        match self {
            DatabaseSettings::Qdrant { url } => {
                let config = QdrantConfig::from_url(url);
                let client = Qdrant::new(config)?;
                Ok(Box::new(super::qdrant::QdrantDatabaseImpl::new(client)))
            }
        }
    }
}
