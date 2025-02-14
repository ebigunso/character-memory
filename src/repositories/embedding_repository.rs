/*
 * Module: embedding_repository.rs
 * Description: Implements functionality for converting text into vector embeddings using the OpenAI text-embedding-3-large model.
 * This repository abstracts the call to the OpenAI API and provides a clean internal API for generating embeddings.
 * Visibility: pub(crate)
 */

use crate::errors::custom::CustomError;
use crate::config::settings::Settings;
use reqwest::blocking::Client;
use serde_json::json;

pub(crate) struct EmbeddingRepository {
    api_key: String,
    client: Client,
}

impl EmbeddingRepository {
    pub(crate) fn new(settings: &Settings) -> Result<Self, CustomError> {
        let api_key = settings.get_openai_api_key();
        if api_key.trim().is_empty() {
            return Err(CustomError::EmbeddingInitializationError("OPENAI_API_KEY is not provided.".into()));
        }
        println!("Embedding repository: Initialized with provided OpenAI API key.");
        Ok(EmbeddingRepository {
            api_key,
            client: Client::new(),
        })
    }

    /// Generates a vector embedding for the provided text by invoking the OpenAI text-embedding-3-large API.
    /// Returns a vector of f32 values representing the embedding, or an error if the API call fails.
    pub(crate) fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, CustomError> {
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

    /// Generates embeddings for a batch of texts.
    /// Calls generate_embedding for each text and accumulates the results.
    pub(crate) fn batch_generate_embeddings(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, CustomError> {
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
    use crate::config::settings::Settings;
    use secrecy::SecretString;

    fn dummy_settings_with_api(api: &str) -> Settings {
        Settings {
            qdrant_connection_string: SecretString::new("dummy".into()),
            oxigraph_connection_string: SecretString::new("dummy".into()),
            openai_api_key: SecretString::new(api.into()),
        }
    }

    #[test]
    fn test_new_with_valid_api() {
        let settings = dummy_settings_with_api("dummy_key");
        let repo = EmbeddingRepository::new(&settings);
        assert!(repo.is_ok(), "EmbeddingRepository initialization should succeed with valid API key.");
    }

    #[test]
    fn test_new_with_empty_api() {
        let settings = dummy_settings_with_api("");
        let repo = EmbeddingRepository::new(&settings);
        assert!(repo.is_err(), "EmbeddingRepository initialization should fail with empty API key.");
    }

    #[test]
    fn test_generate_embedding_with_empty_text() {
        let settings = dummy_settings_with_api("valid_key");
        let repo = EmbeddingRepository::new(&settings).unwrap();
        let result = repo.generate_embedding("  ");
        assert!(result.is_err(), "Empty text should return an error.");
    }

    #[test]
    fn test_generate_embedding_with_valid_text() {
        let text = "hello, world";
        let settings = dummy_settings_with_api("valid_api_key");
        let repo = EmbeddingRepository::new(&settings).unwrap();
        let result = repo.generate_embedding(text);
        assert!(result.is_err(), "Valid text with a dummy API key should produce an error during API call.");
    }

    #[test]
    fn test_batch_generate_embeddings() {
        let settings = dummy_settings_with_api("valid_api_key");
        let repo = EmbeddingRepository::new(&settings).unwrap();
        let texts = ["first test", "second test"];
        let result = repo.batch_generate_embeddings(&texts);
        assert!(result.is_err(), "Batch generation should fail with a dummy API key.");
    }
}
