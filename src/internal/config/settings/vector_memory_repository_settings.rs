use crate::internal::models::vector::EmbeddingModel;

/// Configuration for VectorMemoryRepository.
///
/// # Description
///
/// Contains settings required for configuring a vector database repository,
/// including connection details and collection configuration.
///
/// # Parameters
///
/// - `url`: URL of the vector database server
/// - `collection_name`: Name of the collection to store memories
/// - `model`: The embedding model to use, which determines vector dimensions
#[derive(Debug, Clone)]
pub(crate) struct VectorMemoryRepositorySettings {
    pub url: String,
    pub collection_name: String,
    pub model: EmbeddingModel,
}

impl VectorMemoryRepositorySettings {
    pub fn new(url: String, collection_name: String, model: EmbeddingModel) -> Self {
        Self {
            url,
            collection_name,
            model,
        }
    }
}
