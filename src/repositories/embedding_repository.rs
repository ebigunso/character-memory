use crate::errors::CustomError;
use async_trait::async_trait;

/// Repository trait for converting text into vector embeddings.
///
/// # Description
///
/// This trait defines the interface for embedding generation services.
/// Implementations of this trait are responsible for converting text into
/// numerical vector representations suitable for semantic operations.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub(crate) trait EmbeddingRepository: Send + Sync {
    /// Generates a vector embedding for the provided text.
    ///
    /// # Parameters
    ///
    /// - `text`: The input text to generate an embedding for
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of f32 values representing the embedding
    /// - `Err`: A `CustomError` if generation fails
    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, CustomError>;

    /// Generates embeddings for a batch of texts.
    ///
    /// # Parameters
    ///
    /// - `texts`: A slice of text strings to generate embeddings for
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of embeddings, where each embedding is a vector of f32 values
    /// - `Err`: A `CustomError` if generation fails for any text in the batch
    async fn bulk_generate_embeddings<'a>(&self, texts: &'a [&'a str]) -> Result<Vec<Vec<f32>>, CustomError>;
}

// Implement the trait for Box<dyn EmbeddingRepository> to allow using boxed trait objects
#[async_trait]
impl<T: EmbeddingRepository + ?Sized> EmbeddingRepository for Box<T> {
    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, CustomError> {
        (**self).generate_embedding(text).await
    }

    async fn bulk_generate_embeddings<'a>(&self, texts: &'a [&'a str]) -> Result<Vec<Vec<f32>>, CustomError> {
        (**self).bulk_generate_embeddings(texts).await
    }
}
