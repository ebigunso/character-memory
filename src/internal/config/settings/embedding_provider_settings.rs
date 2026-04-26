use crate::internal::models::vector::EmbeddingModel;

/// Settings for configuring the OpenAI embedding provider.
///
/// # Description
///
/// This struct contains the necessary configuration settings for
/// initializing and using the OpenAI embedding provider.
#[derive(Clone)]
pub(crate) struct EmbeddingProviderSettings {
    /// The API key for authenticating with OpenAI's API
    pub(crate) api_key: String,
    /// The embedding model to use for generating embeddings
    pub(crate) model: EmbeddingModel,
}

impl EmbeddingProviderSettings {
    /// Creates a new instance of EmbeddingProviderSettings.
    ///
    /// # Parameters
    ///
    /// - `api_key`: The OpenAI API key
    /// - `model`: The embedding model to use
    ///
    /// # Returns
    ///
    /// A new `EmbeddingProviderSettings` instance
    pub(crate) fn new(api_key: String, model: EmbeddingModel) -> Self {
        Self { api_key, model }
    }
}
