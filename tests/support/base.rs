use std::{path::Path, time::Duration};

use async_trait::async_trait;
use character_memory::{CharacterMemory, CustomError, EmbeddingError, EmbeddingProvider, Settings};
use config::{builder::DefaultState, Config, ConfigBuilder};
use qdrant_client::{config::QdrantConfig, Qdrant};
use tempfile::TempDir;
use uuid::Uuid;

/// Settings for the embedded vector store under `root/vectors` with in-memory graph and
/// stats stores. Built from explicit overrides only: no environment variable is read.
pub fn embedded_settings(root: &Path) -> ConfigBuilder<DefaultState> {
    Config::builder()
        .set_override("vector_store_path", path_string(&root.join("vectors")))
        .unwrap()
        .set_override("embedding_model", "text-embedding-3-small")
        .unwrap()
        .set_override("graph_store_mode", "in_memory")
        .unwrap()
        .set_override("retrieval_stats_store_mode", "in_memory")
        .unwrap()
}

pub async fn open(
    builder: ConfigBuilder<DefaultState>,
    collection_name: String,
) -> Result<CharacterMemory, CustomError> {
    let settings = Settings::new(builder.build().unwrap())?;
    let embed_provider = Box::new(deterministic_provider(
        settings.get_embedding_vector_size()?,
    ));
    CharacterMemory::new_with_embedding_provider(settings, collection_name, embed_provider).await
}

/// Closes the facade so its local stores release their files, then removes the store root.
pub async fn close_and_remove_root(memory: CharacterMemory, root: TempDir) {
    let path = root.path().to_path_buf();
    memory.close().await.expect("facade should close");
    root.close().expect("store root should be removed");
    assert!(!path.exists(), "store root should not remain at {path:?}");
}

pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn unique_collection_name() -> String {
    format!("test_collection_{}", Uuid::new_v4())
}

pub struct TestEmbeddingProvider<F> {
    vector_size: usize,
    vector: F,
}

impl<F> TestEmbeddingProvider<F> {
    pub fn new(vector_size: usize, vector: F) -> Self {
        Self {
            vector_size,
            vector,
        }
    }
}

#[async_trait]
impl<F> EmbeddingProvider for TestEmbeddingProvider<F>
where
    F: Fn(&str) -> Vec<f32> + Send + Sync,
{
    fn vector_size(&self) -> usize {
        self.vector_size
    }
    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, EmbeddingError> {
        Ok((self.vector)(text))
    }
    async fn bulk_generate_embeddings<'a>(
        &self,
        texts: &'a [&'a str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(texts.iter().map(|text| (self.vector)(text)).collect())
    }
}

pub fn deterministic_provider(vector_size: usize) -> impl EmbeddingProvider {
    TestEmbeddingProvider::new(vector_size, move |text: &str| {
        deterministic_embedding(text, vector_size)
    })
}

fn deterministic_embedding(text: &str, vector_size: usize) -> Vec<f32> {
    let mut embedding = vec![0.0; vector_size];

    for token in text.split(|character: char| !character.is_alphanumeric()) {
        if token.is_empty() {
            continue;
        }

        let index = stable_hash(token) % vector_size;
        embedding[index] += 1.0;
    }

    if embedding.iter().all(|value| *value == 0.0) {
        embedding[0] = 1.0;
    }

    embedding
}

pub async fn open_with_provider(
    builder: ConfigBuilder<DefaultState>,
    collection: String,
    provider: impl EmbeddingProvider + 'static,
) -> Result<CharacterMemory, CustomError> {
    CharacterMemory::new_with_embedding_provider(
        Settings::new(builder.build().unwrap())?,
        collection,
        Box::new(provider),
    )
    .await
}

pub fn id(n: u128) -> character_memory::MemoryId {
    character_memory::MemoryId::from_u128(n)
}

fn stable_hash(text: &str) -> usize {
    text.bytes().fold(2166136261usize, |hash, byte| {
        hash.wrapping_mul(16777619) ^ usize::from(byte.to_ascii_lowercase())
    })
}

/// Deletes a collection on the Qdrant service named by `QDRANT_CONNECTION_STRING`.
/// Only the opt-in service parity tests reach the service, so only they call this.
pub async fn cleanup_collection(collection_name: &str) {
    let qdrant_url = std::env::var("QDRANT_CONNECTION_STRING")
        .expect("QDRANT_CONNECTION_STRING is set for service parity tests");
    let client = Qdrant::new(QdrantConfig::from_url(&qdrant_url).timeout(Duration::from_secs(30)))
        .expect("Failed to create Qdrant client");

    if let Err(delete_error) = client.delete_collection(collection_name).await {
        match client.collection_exists(collection_name).await {
            Ok(false) => {}
            Ok(true) => {
                eprintln!(
                    "warning: Qdrant collection {collection_name:?} remains after cleanup failed: {delete_error}"
                );
            }
            Err(probe_error) => {
                eprintln!(
                    "warning: could not verify Qdrant collection {collection_name:?} after cleanup failed: delete error: {delete_error}; existence probe error: {probe_error}"
                );
            }
        }
    }
}

pub fn parse_id(value: &str) -> character_memory::MemoryId {
    value.parse().unwrap()
}
pub fn scene_time() -> chrono::DateTime<chrono::Utc> {
    "2026-09-21T12:00:00Z".parse().unwrap()
}
pub fn time_at_minute(offset: i64) -> chrono::DateTime<chrono::Utc> {
    "2026-09-21T00:00:00Z"
        .parse::<chrono::DateTime<chrono::Utc>>()
        .unwrap()
        + chrono::Duration::minutes(offset)
}

pub fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}
pub fn derived_ids(result: &character_memory::RetrieveOutcome) -> Vec<character_memory::MemoryId> {
    result
        .pack
        .derived_memories
        .iter()
        .map(|entry| entry.memory.id)
        .collect()
}
pub fn keyed(n: u128) -> character_memory::SceneParticipant {
    character_memory::SceneParticipant {
        key: Some(id(n)),
        ..Default::default()
    }
}
