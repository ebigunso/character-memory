use crate::errors::CustomError;

/// Repository trait for converting text into vector embeddings.
/// This trait defines the interface for embedding generation services.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait EmbeddingRepository: Send + Sync {
    /// Generates a vector embedding for the provided text.
    /// Returns a vector of f32 values representing the embedding, or an error if generation fails.
    fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, CustomError>;

    /// Generates embeddings for a batch of texts.
    /// Returns a vector of embeddings, one for each input text.
    fn bulk_generate_embeddings<'a>(&self, texts: &'a [&'a str]) -> Result<Vec<Vec<f32>>, CustomError>;
}

// Implement the trait for Box<dyn EmbeddingRepository> to allow using boxed trait objects
impl<T: EmbeddingRepository + ?Sized> EmbeddingRepository for Box<T> {
    fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, CustomError> {
        (**self).generate_embedding(text)
    }

    fn bulk_generate_embeddings<'a>(&self, texts: &'a [&'a str]) -> Result<Vec<Vec<f32>>, CustomError> {
        (**self).bulk_generate_embeddings(texts)
    }
}
