use async_trait::async_trait;
use character_memory::test_utils::load_test_settings;
use character_memory::{CharacterMemory, CustomError, EmbeddingProvider};
use qdrant_client::{Qdrant, QdrantError};
use std::io::ErrorKind;
use std::sync::Once;
use tonic::Code;
use uuid::Uuid;

static INIT: Once = Once::new();

// Initialize environment once for all tests
pub fn initialize() {
    INIT.call_once(|| {
        // Any global setup can go here
    });
}

// Create a unique collection name for parallel test execution
pub fn unique_collection_name() -> String {
    format!("test_collection_{}", Uuid::new_v4())
}

struct DeterministicEmbeddingProvider {
    vector_size: usize,
}

impl DeterministicEmbeddingProvider {
    fn new(vector_size: usize) -> Self {
        Self { vector_size }
    }

    fn vector_for_text(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0; self.vector_size];

        for token in text.split(|character: char| !character.is_alphanumeric()) {
            if token.is_empty() {
                continue;
            }

            let index = stable_hash(token) % self.vector_size;
            embedding[index] += 1.0;
        }

        if embedding.iter().all(|value| *value == 0.0) {
            embedding[0] = 1.0;
        }

        embedding
    }
}

#[async_trait]
impl EmbeddingProvider for DeterministicEmbeddingProvider {
    fn vector_size(&self) -> usize {
        self.vector_size
    }

    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, CustomError> {
        Ok(self.vector_for_text(text))
    }

    async fn bulk_generate_embeddings<'a>(
        &self,
        texts: &'a [&'a str],
    ) -> Result<Vec<Vec<f32>>, CustomError> {
        Ok(texts
            .iter()
            .map(|text| self.vector_for_text(text))
            .collect())
    }
}

fn stable_hash(text: &str) -> usize {
    text.bytes().fold(2166136261usize, |hash, byte| {
        hash.wrapping_mul(16777619) ^ usize::from(byte.to_ascii_lowercase())
    })
}

// Setup CharacterMemory instance with a unique collection
pub async fn try_setup_character_memory() -> Result<(CharacterMemory, String), CustomError> {
    initialize();

    let collection_name = unique_collection_name();

    // Use the load_test_settings function from the test_utils module
    let settings = load_test_settings()?;
    let embed_provider = Box::new(DeterministicEmbeddingProvider::new(
        settings.get_embedding_vector_size()?,
    ));

    let character_memory = CharacterMemory::new_with_embedding_provider(
        settings,
        collection_name.clone(),
        embed_provider,
    )
    .await?;

    Ok((character_memory, collection_name))
}

pub fn is_qdrant_unavailable_error(error: &QdrantError) -> bool {
    match error {
        QdrantError::ResponseError { status } => status.code() == Code::Unavailable,
        QdrantError::Reqwest(error) => error.is_connect() || error.is_timeout(),
        QdrantError::Io(error) => matches!(
            error.kind(),
            ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::TimedOut
        ),
        _ => false,
    }
}

// Cleanup after tests by deleting the collection
pub async fn cleanup_collection(collection_name: &str) {
    let settings = load_test_settings().expect("Failed to load settings from environment");

    let qdrant_url = settings.get_qdrant_connection().to_string();
    let client = Qdrant::from_url(&qdrant_url)
        .build()
        .expect("Failed to create Qdrant client");

    // Delete the collection
    let _ = client.delete_collection(collection_name).await;

    // We don't assert on the result because the collection might not exist
    // if the test failed before creating it
}
