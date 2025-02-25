use reqwest::blocking::Client;
use serde_json::json;

use crate::errors::CustomError;
use crate::config::settings::EmbeddingRepositorySettings;
use crate::repositories::EmbeddingRepository;

/// OpenAI-based implementation of the EmbeddingRepository trait.
///
/// # Description
///
/// This implementation uses the specified OpenAI embedding model to generate embeddings.
/// It handles the communication with OpenAI's API to convert text into vector embeddings.
///
/// # See also
///
/// - [`EmbeddingRepository`]
/// - [OpenAI Embeddings API](https://platform.openai.com/docs/api-reference/embeddings)
pub(crate) struct OpenAIEmbeddingRepository {
    api_key: String,
    model: String,
    client: Client,
}

impl OpenAIEmbeddingRepository {
    /// Creates a new OpenAIEmbeddingRepository instance.
    ///
    /// # Parameters
    ///
    /// - `settings`: Configuration settings containing the OpenAI API key
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `OpenAIEmbeddingRepository` instance
    /// - `Err`: A `CustomError` if initialization fails (e.g., missing API key)
    pub fn new(settings: EmbeddingRepositorySettings) -> Result<Self, CustomError> {
        if settings.api_key.trim().is_empty() {
            return Err(CustomError::EmbeddingInitializationError("OpenAI API key is not provided.".into()));
        }
        println!("OpenAI Embedding Repository: Initialized with {} model.", settings.model.as_str());
        Ok(OpenAIEmbeddingRepository {
            api_key: settings.api_key,
            model: settings.model.as_str().to_string(),
            client: Client::new(),
        })
    }
}

impl EmbeddingRepository for OpenAIEmbeddingRepository {
    fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, CustomError> {
        if text.trim().is_empty() {
            return Err(CustomError::EmbeddingGenerationError("Input text is empty.".into()));
        }
        let payload = json!({
            "model": &self.model,
            "input": text,
        });
        let response = self.client.post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .map_err(|e| CustomError::EmbeddingGenerationError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(CustomError::EmbeddingGenerationError(
                format!("OpenAI API error: {}", response.status())
            ));
        }
        let resp_json: serde_json::Value = response.json()
            .map_err(|e| CustomError::EmbeddingGenerationError(e.to_string()))?;
        let embedding = resp_json.get("data")
            .and_then(|data| data.get(0))
            .and_then(|item| item.get("embedding"))
            .and_then(|emb| emb.as_array())
            .ok_or_else(|| CustomError::EmbeddingGenerationError("Failed to parse embedding from API response".into()))?;
        let vec_embedding: Vec<f32> = embedding.iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        Ok(vec_embedding)
    }

    fn bulk_generate_embeddings(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for &text in texts {
            let emb = self.generate_embedding(text)?;
            embeddings.push(emb);
        }
        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::vector::EmbeddingModel;

    fn create_test_settings(api_key: &str) -> EmbeddingRepositorySettings {
        EmbeddingRepositorySettings::new(
            api_key.to_string(),
            EmbeddingModel::TextEmbedding3Large,
        )
    }

    #[test]
    fn test_new_with_valid_api() {
        let settings = create_test_settings("dummy_key");
        let repo = OpenAIEmbeddingRepository::new(settings);
        assert!(repo.is_ok(), "OpenAIEmbeddingRepository initialization should succeed with valid API key.");
    }

    #[test]
    fn test_new_with_empty_api() {
        let settings = create_test_settings("");
        let repo = OpenAIEmbeddingRepository::new(settings);
        assert!(repo.is_err(), "OpenAIEmbeddingRepository initialization should fail with empty API key.");
    }

    #[test]
    fn test_generate_embedding_with_empty_text() {
        let settings = create_test_settings("valid_key");
        let repo = OpenAIEmbeddingRepository::new(settings).unwrap();
        let result = repo.generate_embedding("  ");
        assert!(result.is_err(), "Empty text should return an error.");
    }

    #[test]
    fn test_generate_embedding_with_valid_text() {
        let text = "hello, world";
        let settings = create_test_settings("valid_api_key");
        let repo = OpenAIEmbeddingRepository::new(settings).unwrap();
        let result = repo.generate_embedding(text);
        assert!(result.is_err(), "Valid text with a dummy API key should produce an error during API call.");
    }

    #[test]
    fn test_batch_generate_embeddings() {
        let settings = create_test_settings("valid_api_key");
        let repo = OpenAIEmbeddingRepository::new(settings).unwrap();
        let texts = ["first test", "second test"];
        let result = repo.bulk_generate_embeddings(&texts);
        assert!(result.is_err(), "Batch generation should fail with a dummy API key.");
    }
}
