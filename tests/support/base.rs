use async_trait::async_trait;
use character_memory::{
    CustomError, EmbeddingProvider, Settings, TransportStatus, VectorDatabaseError,
    VectorDatabaseErrorKind,
};
use config::Config;
use qdrant_client::Qdrant;
use uuid::Uuid;

pub fn load_test_settings() -> Result<Settings, CustomError> {
    dotenvy::dotenv().ok();

    let mut builder = Config::builder();
    for (environment_key, config_key) in [
        ("QDRANT_CONNECTION_STRING", "qdrant_connection_string"),
        ("OXIGRAPH_PATH", "oxigraph_path"),
        ("OPENAI_API_KEY", "openai_api_key"),
        ("EMBEDDING_MODEL", "embedding_model"),
    ] {
        let value = std::env::var(environment_key).map_err(|error| {
            CustomError::ConfigParseError(format!("{environment_key}: {error}"))
        })?;
        builder = builder
            .set_override(config_key, value)
            .map_err(config_error)?;
    }

    for (environment_key, config_key) in [
        ("GRAPH_STORE_MODE", "graph_store_mode"),
        ("RETRIEVAL_STATS_STORE_MODE", "retrieval_stats_store_mode"),
        ("RETRIEVAL_STATS_PATH", "retrieval_stats_path"),
        (
            "RETRIEVAL_STATS_HEALTH_FAIL_MODE",
            "retrieval_stats_health_fail_mode",
        ),
        ("SELECTIVITY_SMOOTHING_ALPHA", "selectivity_smoothing_alpha"),
        ("SELECTIVITY_GAMMA", "selectivity_gamma"),
        (
            "RETRIEVAL_FANOUT_ABOUT_ENTITY_DERIVED_MEMORY_MIN",
            "retrieval.fanout.about_entity.derived_memory.min",
        ),
        (
            "RETRIEVAL_FANOUT_ABOUT_ENTITY_DERIVED_MEMORY_MAX",
            "retrieval.fanout.about_entity.derived_memory.max",
        ),
        (
            "RETRIEVAL_FANOUT_PARTICIPANT_ENTITY_EPISODE_MIN",
            "retrieval.fanout.participant_entity.episode.min",
        ),
        (
            "RETRIEVAL_FANOUT_PARTICIPANT_ENTITY_EPISODE_MAX",
            "retrieval.fanout.participant_entity.episode.max",
        ),
        (
            "RETRIEVAL_FANOUT_PART_OF_THREAD_DERIVED_MEMORY_MIN",
            "retrieval.fanout.part_of_thread.derived_memory.min",
        ),
        (
            "RETRIEVAL_FANOUT_PART_OF_THREAD_DERIVED_MEMORY_MAX",
            "retrieval.fanout.part_of_thread.derived_memory.max",
        ),
    ] {
        if let Ok(value) = std::env::var(environment_key) {
            builder = builder
                .set_override(config_key, value)
                .map_err(config_error)?;
        }
    }

    Settings::new(builder.build().map_err(config_error)?)
}

pub fn unique_collection_name() -> String {
    format!("test_collection_{}", Uuid::new_v4())
}

pub struct DeterministicEmbeddingProvider {
    vector_size: usize,
}

impl DeterministicEmbeddingProvider {
    pub fn new(vector_size: usize) -> Self {
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

pub fn is_qdrant_unavailable_error(error: &VectorDatabaseError) -> bool {
    if error.backend != "qdrant" {
        return false;
    }

    let message = error.message.to_ascii_lowercase();
    error.status == Some(TransportStatus::Unavailable)
        || (error.kind == VectorDatabaseErrorKind::Response
            && message.contains("failed to connect")
            && message.contains("tcp connect error"))
        || matches!(
            error.kind,
            VectorDatabaseErrorKind::HttpConnect | VectorDatabaseErrorKind::HttpTimeout
        )
        || matches!(
            &error.kind,
            VectorDatabaseErrorKind::Io { io_kind }
                if matches!(
                    io_kind.as_str(),
                    "ConnectionRefused"
                        | "ConnectionReset"
                        | "ConnectionAborted"
                        | "NotConnected"
                        | "TimedOut"
                )
        )
}

pub async fn cleanup_collection(collection_name: &str) {
    let settings = load_test_settings().expect("Failed to load settings from environment");

    let qdrant_url = settings.get_qdrant_connection().to_string();
    let client = Qdrant::from_url(&qdrant_url)
        .build()
        .expect("Failed to create Qdrant client");

    let _ = client.delete_collection(collection_name).await;
}

pub fn config_error(error: config::ConfigError) -> CustomError {
    CustomError::ConfigParseError(error.to_string())
}
