use super::embedding_model::EmbeddingModel;

/// Configuration for VectorMemoryRepository
///
/// * `url` - URL of the vector database server
/// * `collection_name` - Name of the collection to store memories
/// * `vector_size` - Size of the embedding vectors
#[derive(Debug, Clone)]
pub struct VectorMemoryConfig {
    pub url: String,
    pub collection_name: String,
    pub vector_size: u64,
}

impl VectorMemoryConfig {
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
