use agent_memory::test_utils::load_test_settings;
use agent_memory::{AgentMemory, CustomError, EmbeddingProvider, Memory};
use async_trait::async_trait;
use qdrant_client::Qdrant;
use std::env;
use std::sync::Once;
use tokio::time::{sleep, Duration};
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

fn embedding_vector_size_from_env() -> usize {
    match env::var("EMBEDDING_MODEL")
        .expect("EMBEDDING_MODEL must be set for integration tests")
        .trim()
    {
        "text-embedding-3-small" | "text-embedding-ada-002" => 1536,
        "text-embedding-3-large" => 3072,
        other => panic!("Unsupported EMBEDDING_MODEL for integration tests: {other}"),
    }
}

// Setup AgentMemory instance with a unique collection
pub async fn setup_agent_memory() -> (AgentMemory, String) {
    initialize();

    let collection_name = unique_collection_name();

    // Use the load_test_settings function from the test_utils module
    let settings = load_test_settings().expect("Failed to load settings from environment");
    let embed_provider = Box::new(DeterministicEmbeddingProvider::new(
        embedding_vector_size_from_env(),
    ));

    let agent_memory =
        AgentMemory::new_with_embedding_provider(settings, collection_name.clone(), embed_provider)
            .await
            .expect("Failed to create AgentMemory");

    agent_memory
        .init_storage()
        .await
        .expect("Failed to initialize storage");

    (agent_memory, collection_name)
}

#[allow(dead_code)]
pub async fn wait_for_memory(agent_memory: &AgentMemory, memory_id: Uuid) -> Memory {
    let mut last_error = None;

    for _attempt in 0..20 {
        match agent_memory.get_memory_by_id(memory_id).await {
            Ok(memory) => return memory,
            Err(error) => last_error = Some(error),
        }

        sleep(Duration::from_millis(50)).await;
    }

    panic!("Memory {memory_id} was not visible in Qdrant after retries: {last_error:?}");
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
