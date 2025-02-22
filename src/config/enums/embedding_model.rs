/// Available embedding models and their corresponding vector sizes.
///
/// # Description
///
/// This enum represents the supported OpenAI embedding models,
/// each with its specific vector dimension size.
///
/// # See also
///
/// - [OpenAI Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)
#[derive(Debug, Clone, Copy)]
pub enum EmbeddingModel {
    /// OpenAI text-embedding-3-small model.
    ///
    /// # Description
    ///
    /// A smaller, more efficient model with 1536-dimensional embeddings
    TextEmbedding3Small,

    /// OpenAI text-embedding-3-large model.
    ///
    /// # Description
    ///
    /// A larger model with 3072-dimensional embeddings for higher accuracy
    TextEmbedding3Large,

    /// OpenAI text-embedding-ada-002 model.
    ///
    /// # Description
    ///
    /// Legacy model with 1536-dimensional embeddings
    TextEmbeddingAda002,
}
