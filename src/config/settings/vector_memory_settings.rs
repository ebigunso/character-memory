use crate::config::enums::EmbeddingModel;

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
/// - `vector_size`: Size of the embedding vectors in dimensions
#[derive(Debug, Clone)]
pub struct VectorMemorySettings {
    pub url: String,
    pub collection_name: String,
    pub vector_size: u64,
}

impl VectorMemorySettings {
    pub fn new(url: String, collection_name: String, model: EmbeddingModel) -> Self {
        let vector_size = match model {
            EmbeddingModel::TextEmbedding3Small => 1536,
            EmbeddingModel::TextEmbedding3Large => 3072,
            EmbeddingModel::TextEmbeddingAda002 => 1536,
        };
        Self {
            url,
            collection_name,
            vector_size,
        }
    }
}
