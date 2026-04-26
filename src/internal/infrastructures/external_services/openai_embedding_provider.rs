use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::errors::CustomError;
use crate::internal::config::settings::EmbeddingProviderSettings;
use crate::EmbeddingProvider;

/// OpenAI-based implementation of the EmbeddingProvider trait.
///
/// # Description
///
/// This implementation uses the specified OpenAI embedding model to generate embeddings.
/// It handles the communication with OpenAI's API to convert text into vector embeddings.
///
/// # See also
///
/// - [`EmbeddingProvider`]
/// - [OpenAI Embeddings API](https://platform.openai.com/docs/api-reference/embeddings)
pub(crate) struct OpenAIEmbeddingProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl OpenAIEmbeddingProvider {
    /// Creates a new OpenAIEmbeddingProvider instance.
    ///
    /// # Parameters
    ///
    /// - `settings`: Configuration settings containing the OpenAI API key
    ///
    /// # Returns
    ///
    /// - `Ok`: A new `OpenAIEmbeddingProvider` instance
    /// - `Err`: A `CustomError` if initialization fails (e.g., missing API key)
    pub fn new(settings: EmbeddingProviderSettings) -> Result<Self, CustomError> {
        if settings.api_key.trim().is_empty() {
            return Err(CustomError::EmbeddingInitializationError(
                "OpenAI API key is not provided.".into(),
            ));
        }
        println!(
            "OpenAI Embedding Provider: Initialized with {} model.",
            settings.model.as_str()
        );
        Ok(OpenAIEmbeddingProvider {
            api_key: settings.api_key,
            model: settings.model.as_str().to_string(),
            client: Client::new(),
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, CustomError> {
        if text.trim().is_empty() {
            return Err(CustomError::EmbeddingGenerationError(
                "Input text is empty.".into(),
            ));
        }
        let payload = json!({
            "model": &self.model,
            "input": text,
        });
        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| CustomError::EmbeddingGenerationError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(CustomError::EmbeddingGenerationError(format!(
                "OpenAI API error: {}",
                response.status()
            )));
        }
        let resp_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CustomError::EmbeddingGenerationError(e.to_string()))?;
        let embedding = resp_json
            .get("data")
            .and_then(|data| data.get(0))
            .and_then(|item| item.get("embedding"))
            .and_then(|emb| emb.as_array())
            .ok_or_else(|| {
                CustomError::EmbeddingGenerationError(
                    "Failed to parse embedding from API response".into(),
                )
            })?;
        let vec_embedding: Vec<f32> = embedding
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        Ok(vec_embedding)
    }

    async fn bulk_generate_embeddings<'a>(
        &self,
        texts: &'a [&'a str],
    ) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for &text in texts {
            let emb = self.generate_embedding(text).await?;
            embeddings.push(emb);
        }
        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::models::vector::EmbeddingModel;

    fn create_test_settings(api_key: &str) -> EmbeddingProviderSettings {
        EmbeddingProviderSettings::new(api_key.to_string(), EmbeddingModel::TextEmbedding3Large)
    }

    #[test]
    fn test_new_with_valid_api() {
        let settings = create_test_settings("dummy_key");
        let provider = OpenAIEmbeddingProvider::new(settings);
        assert!(
            provider.is_ok(),
            "OpenAIEmbeddingProvider initialization should succeed with valid API key."
        );
    }

    #[test]
    fn test_new_with_empty_api() {
        let settings = create_test_settings("");
        let provider = OpenAIEmbeddingProvider::new(settings);
        assert!(
            provider.is_err(),
            "OpenAIEmbeddingProvider initialization should fail with empty API key."
        );
    }

    #[tokio::test]
    async fn test_generate_embedding_with_empty_text() {
        let settings = create_test_settings("valid_key");
        let provider = OpenAIEmbeddingProvider::new(settings).unwrap();
        let result = provider.generate_embedding("  ").await;
        assert!(result.is_err(), "Empty text should return an error.");
    }

    #[tokio::test]
    async fn test_generate_embedding_with_valid_text() {
        let text = "hello, world";
        let settings = create_test_settings("valid_api_key");
        let provider = OpenAIEmbeddingProvider::new(settings).unwrap();
        let result = provider.generate_embedding(text).await;
        assert!(
            result.is_err(),
            "Valid text with a dummy API key should produce an error during API call."
        );
    }

    #[tokio::test]
    async fn test_batch_generate_embeddings() {
        let settings = create_test_settings("valid_api_key");
        let provider = OpenAIEmbeddingProvider::new(settings).unwrap();
        let texts = ["first test", "second test"];
        let result = provider.bulk_generate_embeddings(&texts).await;
        assert!(
            result.is_err(),
            "Batch generation should fail with a dummy API key."
        );
    }
}
