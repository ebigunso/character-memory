/*
 * Module: embedding_repository.rs
 * Description: Implements functionality for converting text into vector embeddings using the OpenAI text-embedding-3-large model.
 * This repository abstracts the call to the OpenAI API and provides a clean internal API for generating embeddings.
 * Visibility: pub(crate)
 */

use crate::errors::custom::CustomError;

/// Initializes the embedding model by setting up necessary configurations such as
/// reading API credentials and validating connectivity to the OpenAI service. In a production
/// scenario, this function would perform these tasks. Here, the initialization is simulated.
///
/// Returns:
///   - Ok(()) on success.
///   - Err(CustomError) if initialization fails.
pub(crate) fn init_model() -> Result<(), CustomError> {
    // In a real implementation, load API credentials and verify the OpenAI endpoint.
    // For this demonstration, we assume initialization always succeeds.
    println!("Embedding repository: Model initialized successfully.");
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
pub(crate) fn generate_embedding(text: &str) -> Result<Vec<f32>, CustomError> {
    if text.trim().is_empty() {
        return Err(CustomError::EmbeddingGenerationError("Input text is empty.".into()));
    }

    // In production, send an HTTP request to the OpenAI API and process the response.
    println!("Embedding repository: Generating embedding for text: {}", text);
    // Dummy implementation: create a vector using text length and fixed values.
    let embedding = vec![text.len() as f32, 0.5, 1.0];
    Ok(embedding)
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
pub(crate) fn batch_generate_embeddings(texts: &[&str]) -> Result<Vec<Vec<f32>>, CustomError> {
    let mut embeddings = Vec::with_capacity(texts.len());
    for &text in texts {
        let emb = generate_embedding(text)?;
        embeddings.push(emb);
    }
    Ok(embeddings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_model() {
        let result = init_model();
        assert!(result.is_ok(), "Model initialization should succeed.");
    }

    #[test]
    fn test_generate_embedding_with_empty_text() {
        let result = generate_embedding("  ");
        assert!(result.is_err(), "Empty text should return an error.");
    }

    #[test]
    fn test_generate_embedding_with_valid_text() {
        let text = "hello, world";
        let result = generate_embedding(text);
        assert!(result.is_ok(), "Valid text should produce an embedding.");
        let embedding = result.unwrap();
        // Validate the dummy embedding: first element equals text length converted to f32.
        assert_eq!(embedding[0], text.len() as f32);
    }

    #[test]
    fn test_batch_generate_embeddings() {
        let texts = ["first test", "second test"];
        let result = batch_generate_embeddings(&texts);
        assert!(result.is_ok(), "Batch generation should succeed.");
        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), texts.len());
    }
}
