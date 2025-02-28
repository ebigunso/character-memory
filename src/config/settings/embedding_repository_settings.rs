use crate::models::vector::EmbeddingModel;

/// Settings for configuring the OpenAI Embedding Repository.
///
/// # Description
///
/// This struct contains the necessary configuration settings for
/// initializing and using the OpenAI Embedding Repository.
#[derive(Clone)]
pub(crate) struct EmbeddingRepositorySettings {
    /// The API key for authenticating with OpenAI's API
    pub(crate) api_key: String,
    /// The embedding model to use for generating embeddings
    pub(crate) model: EmbeddingModel,
}

impl EmbeddingRepositorySettings {
    /// Creates a new instance of EmbeddingRepositorySettings.
    ///
    /// # Parameters
    ///
    /// - `api_key`: The OpenAI API key
    /// - `model`: The embedding model to use
    ///
    /// # Returns
    ///
    /// A new `EmbeddingRepositorySettings` instance
    pub(crate) fn new(api_key: String, model: EmbeddingModel) -> Self {
        Self { api_key, model }
    }
}
