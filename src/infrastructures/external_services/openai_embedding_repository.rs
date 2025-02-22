use reqwest::blocking::Client;
use serde_json::json;

use crate::errors::CustomError;
use crate::config::settings::Settings;
use crate::repositories::EmbeddingRepository;

/// OpenAI-based implementation of the EmbeddingRepository trait.
/// This implementation uses the OpenAI text-embedding-3-large model to generate embeddings.
pub(crate) struct OpenAIEmbeddingRepository {
    api_key: String,
    client: Client,
}

impl OpenAIEmbeddingRepository {
    /// Creates a new OpenAIEmbeddingRepository instance.
    pub fn new(settings: &Settings) -> Result<Self, CustomError> {
        let api_key = settings.get_openai_api_key().to_string();
        if api_key.trim().is_empty() {
            return Err(CustomError::EmbeddingInitializationError("OPENAI_API_KEY is not provided.".into()));
        }
        println!("OpenAI Embedding Repository: Initialized with provided OpenAI API key.");
        Ok(OpenAIEmbeddingRepository {
            api_key,
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
            "model": "text-embedding-3-large",
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
    use secrecy::SecretString;

    fn dummy_settings_with_api(api: &str) -> Settings {
        Settings::new_for_tests(
            SecretString::new("dummy".into()),
            SecretString::new("dummy".into()),
            SecretString::new(api.into())
        )
    }

    #[test]
    fn test_new_with_valid_api() {
        let settings = dummy_settings_with_api("dummy_key");
        let repo = OpenAIEmbeddingRepository::new(&settings);
        assert!(repo.is_ok(), "OpenAIEmbeddingRepository initialization should succeed with valid API key.");
    }

    #[test]
    fn test_new_with_empty_api() {
        let settings = dummy_settings_with_api("");
        let repo = OpenAIEmbeddingRepository::new(&settings);
        assert!(repo.is_err(), "OpenAIEmbeddingRepository initialization should fail with empty API key.");
    }

    #[test]
    fn test_generate_embedding_with_empty_text() {
        let settings = dummy_settings_with_api("valid_key");
        let repo = OpenAIEmbeddingRepository::new(&settings).unwrap();
        let result = repo.generate_embedding("  ");
        assert!(result.is_err(), "Empty text should return an error.");
    }

    #[test]
    fn test_generate_embedding_with_valid_text() {
        let text = "hello, world";
        let settings = dummy_settings_with_api("valid_api_key");
        let repo = OpenAIEmbeddingRepository::new(&settings).unwrap();
        let result = repo.generate_embedding(text);
        assert!(result.is_err(), "Valid text with a dummy API key should produce an error during API call.");
    }

    #[test]
    fn test_batch_generate_embeddings() {
        let settings = dummy_settings_with_api("valid_api_key");
        let repo = OpenAIEmbeddingRepository::new(&settings).unwrap();
        let texts = ["first test", "second test"];
        let result = repo.bulk_generate_embeddings(&texts);
        assert!(result.is_err(), "Batch generation should fail with a dummy API key.");
    }
}
