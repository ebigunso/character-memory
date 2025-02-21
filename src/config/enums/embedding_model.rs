/// Available embedding models and their corresponding vector sizes
#[derive(Debug, Clone, Copy)]
pub enum EmbeddingModel {
    /// OpenAI text-embedding-3-small model (1536 dimensions)
    TextEmbedding3Small,
    /// OpenAI text-embedding-3-large model (3072 dimensions)
    TextEmbedding3Large,
    /// OpenAI text-embedding-ada-002 model (1536 dimensions)
    TextEmbeddingAda002,
}
