// Provider-neutral embedding boundary used by remember, retrieve, and
// lifecycle vector maintenance.
use async_trait::async_trait;

use crate::errors::CustomError;
use crate::models::vector::EmbeddingInput;

#[async_trait]
pub(crate) trait MemoryEmbedder: Send + Sync {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError>;

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError>;
}

#[async_trait]
impl<T: MemoryEmbedder + ?Sized> MemoryEmbedder for Box<T> {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        (**self).embed(input).await
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        (**self).embed_batch(inputs).await
    }
}
