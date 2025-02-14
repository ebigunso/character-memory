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

/// Initializes the embedding model by setting up necessary configurations such as
/// reading API credentials and validating connectivity to the OpenAI service. In a production
/// scenario, this function would perform these tasks. Here, the initialization is simulated.
///
/// Returns:
///   - Ok(()) on success.
///   - Err(CustomError) if initialization fails.
pub(crate) fn init_model(settings: &Settings) -> Result<(), CustomError> {
    let api_key = settings.get_openai_api_key();
    if api_key.trim().is_empty() {
        println!("Embedding repository: Warning - OPENAI_API_KEY is not provided. Dummy embeddings will be used.");
    } else {
        println!("Embedding repository: Initialized with provided OpenAI API key.");
        // Optionally, perform a connectivity test to the OpenAI API.
    }
    Ok(())
}

/// Generates a vector embedding for the provided text by invoking the OpenAI text-embedding-3-large API.
/// For demonstration purposes, this function simulates embedding generation by constructing a dummy vector.
///
/// Parameters:
///   - text: Input string for which the embedding is generated.
///
/// Returns:
///   - Ok(Vec<f32>) containing the embedding vector if successful.
///   - Err(CustomError) if the input text is empty or generation fails.
pub(crate) fn generate_embedding(settings: &Settings, text: &str) -> Result<Vec<f32>, CustomError> {
    if text.trim().is_empty() {
        return Err(CustomError::EmbeddingGenerationError("Input text is empty.".into()));
    }
    let api_key = settings.get_openai_api_key();
    if api_key.trim().is_empty() {
        println!("Embedding repository: OPENAI_API_KEY not provided. Using dummy embedding.");
        let embedding = vec![text.len() as f32, 0.5, 1.0];
        return Ok(embedding);
    }
    let client = Client::new();
    let payload = json!({
        "model": "text-embedding-3-large",
        "input": text,
    });
    let response = client.post("https://api.openai.com/v1/embeddings")
        .bearer_auth(api_key)
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

/// Generates embeddings for a batch of texts. This function calls generate_embedding for each text
/// and accumulates the results.
///
/// Parameters:
///   - texts: A slice of string slices for which embeddings should be generated.
///
/// Returns:
///   - Ok(Vec<Vec<f32>>): A vector of embeddings corresponding to each input text.
///   - Err(CustomError) if any invocation of generate_embedding fails.
pub(crate) fn batch_generate_embeddings(settings: &Settings, texts: &[&str]) -> Result<Vec<Vec<f32>>, CustomError> {
    let mut embeddings = Vec::with_capacity(texts.len());
    for &text in texts {
        let emb = generate_embedding(settings, text)?;
        embeddings.push(emb);
    }
    Ok(embeddings)
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
    fn test_init_model() {
        let settings = dummy_settings_with_api("");
        let result = init_model(&settings);
        assert!(result.is_ok(), "Model initialization should succeed.");
    }

    #[test]
    fn test_generate_embedding_with_empty_text() {
        let settings = dummy_settings_with_api("");
        let result = generate_embedding(&settings, "  ");
        assert!(result.is_err(), "Empty text should return an error.");
    }

    #[test]
    fn test_generate_embedding_with_valid_text() {
        let text = "hello, world";
        let settings = dummy_settings_with_api("");
        let result = generate_embedding(&settings, text);
        assert!(result.is_ok(), "Valid text should produce an embedding.");
        let embedding = result.unwrap();
        assert_eq!(embedding[0], text.len() as f32);
    }

    #[test]
    fn test_batch_generate_embeddings() {
        let settings = dummy_settings_with_api("");
        let texts = ["first test", "second test"];
        let result = batch_generate_embeddings(&settings, &texts);
        assert!(result.is_ok(), "Batch generation should succeed.");
        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), texts.len());
    }
}
