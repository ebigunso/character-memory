use async_trait::async_trait;

use crate::adapters::oxigraph::OxigraphGraphAuthorityStore;
use crate::adapters::stats::{InMemoryRetrievalStatsStore, SqliteRetrievalStatsStore};
use crate::adapters::{
    OpenAIEmbeddingProvider, QdrantEdgeVectorCandidateStore, QdrantVectorCandidateStore,
};
use crate::api::embedding::EmbeddingProvider;
use crate::config::{
    EmbeddingProviderSettings, GraphStoreMode as ConfigGraphStoreMode,
    RetrievalStatsHealthFailMode, RetrievalStatsStoreMode as ConfigRetrievalStatsStoreMode,
    Settings, VectorStoreMode,
};
use crate::errors::{
    ConfigValidationError, ConfigValidationReason, CustomError, EmbeddingError,
    RetrievalStatsHealthCause,
};
use crate::memory::CharacterMemory;
use crate::models::vector::EmbeddingInput;
use crate::policy::RetrievalSelectivityPolicy;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::GraphAuthorityStore;
use crate::ports::retrieval_stats::RetrievalStatsStore;
use crate::ports::vector_candidate::VectorCandidateStore;

pub(crate) struct MemoryComposition {
    pub(crate) graph_store: Box<dyn GraphAuthorityStore>,
    pub(crate) vector_store: Box<dyn VectorCandidateStore>,
    pub(crate) embedder: Box<dyn MemoryEmbedder>,
    pub(crate) stats_store: Box<dyn RetrievalStatsStore>,
    pub(crate) selectivity_policy: RetrievalSelectivityPolicy,
}

struct EmbeddingProviderMemoryEmbedder {
    provider: Box<dyn EmbeddingProvider>,
}

impl EmbeddingProviderMemoryEmbedder {
    fn new(provider: Box<dyn EmbeddingProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl MemoryEmbedder for EmbeddingProviderMemoryEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        self.provider
            .generate_embedding(&input.text)
            .await
            .map_err(CustomError::from)
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let texts: Vec<&str> = inputs.iter().map(|input| input.text.as_str()).collect();
        self.provider
            .bulk_generate_embeddings(&texts)
            .await
            .map_err(CustomError::from)
    }
}

impl CharacterMemory {
    /// Builds CharacterMemory from provider-neutral graph, vector, and embedder parts.
    #[cfg(test)]
    pub(crate) fn from_parts(
        graph_store: Box<dyn GraphAuthorityStore>,
        vector_store: Box<dyn VectorCandidateStore>,
        embedder: Box<dyn MemoryEmbedder>,
    ) -> Self {
        let settings = Settings::new(Default::default()).unwrap();
        Self {
            memory_composition: MemoryComposition {
                graph_store,
                vector_store,
                embedder,
                stats_store: Box::new(crate::adapters::stats::InMemoryRetrievalStatsStore::new()),
                selectivity_policy: RetrievalSelectivityPolicy::with_fanout_budgets(
                    settings.get_selectivity_smoothing_alpha(),
                    settings.get_selectivity_gamma(),
                    settings.get_retrieval_fanout_budgets().map(
                        |(relation, object_type, budget)| {
                            (relation, object_type, budget.min(), budget.max())
                        },
                    ),
                ),
            },
            write_turn: tokio::sync::Mutex::new(()),
        }
    }

    fn from_parts_with_stats(
        graph_store: Box<dyn GraphAuthorityStore>,
        vector_store: Box<dyn VectorCandidateStore>,
        embedder: Box<dyn MemoryEmbedder>,
        stats_store: Box<dyn RetrievalStatsStore>,
        selectivity_policy: RetrievalSelectivityPolicy,
    ) -> Self {
        Self {
            memory_composition: MemoryComposition {
                graph_store,
                vector_store,
                embedder,
                stats_store,
                selectivity_policy,
            },
            write_turn: tokio::sync::Mutex::new(()),
        }
    }
}

impl CharacterMemory {
    /// Constructs a new CharacterMemory instance using a caller-provided embedding provider.
    ///
    /// # Description
    ///
    /// This constructor allows callers to inject custom embedding generation while using the
    /// default graph-authoritative storage composition. Vector candidate recall uses the embedded
    /// store by default and requires `VECTOR_STORE_PATH`; callers can explicitly select service
    /// mode, which instead requires `QDRANT_CONNECTION_STRING`.
    /// OpenAI settings are ignored; the injected provider supplies the vector dimension.
    /// Existing vector storage must have that same dimension.
    ///
    /// # Parameters
    ///
    /// - `settings`: Global configuration used to select and initialize the vector candidate
    ///   backend.
    /// - `collection_name`: The name of the vector collection where memory vectors will be stored
    ///   and queried.
    /// - `embed_provider`: A boxed implementation of [`EmbeddingProvider`] that is responsible
    ///   for generating embeddings from input data.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok(Self)`: A new [`CharacterMemory`] instance backed by Oxigraph graph authority and the
    ///   configured vector candidate store.
    /// - `Err(CustomError)`: Returned if any error occurs while resolving configuration from
    ///   `settings` or initializing the Oxigraph graph authority and configured vector candidate
    ///   store.
    pub async fn new_with_embedding_provider(
        settings: Settings,
        collection_name: String,
        embed_provider: Box<dyn EmbeddingProvider>,
    ) -> Result<Self, CustomError> {
        Self::construct(settings, collection_name, Some(embed_provider)).await
    }

    async fn construct(
        settings: Settings,
        collection_name: String,
        embed_provider: Option<Box<dyn EmbeddingProvider>>,
    ) -> Result<Self, CustomError> {
        let (embed_provider, vector_size) = preflight(&settings, embed_provider)?;
        let persistent_graph_path = match settings.get_graph_store_mode() {
            ConfigGraphStoreMode::Persistent => Some(settings.get_oxigraph_path()?),
            ConfigGraphStoreMode::InMemory => None,
        };

        let vector_store: Box<dyn VectorCandidateStore> = match settings.get_vector_store_mode() {
            VectorStoreMode::Embedded => Box::new(
                QdrantEdgeVectorCandidateStore::open(
                    settings.get_vector_store_path()?,
                    collection_name,
                    vector_size,
                )
                .await?,
            ),
            VectorStoreMode::Service => {
                let store = QdrantVectorCandidateStore::new(
                    settings.get_service_qdrant_connection()?,
                    collection_name,
                    vector_size as u64,
                )?;
                store.init_collection().await?;
                Box::new(store)
            }
        };
        let graph_store = match persistent_graph_path {
            Some(path) => Box::new(OxigraphGraphAuthorityStore::new_persistent(path)?)
                as Box<dyn GraphAuthorityStore>,
            None => Box::new(OxigraphGraphAuthorityStore::new_in_memory()?),
        };
        let stats_store = retrieval_stats_store(&settings)?;
        let fanout_budgets =
            settings
                .get_retrieval_fanout_budgets()
                .map(|(relation, object_type, budget)| {
                    (relation, object_type, budget.min(), budget.max())
                });
        let selectivity_policy = RetrievalSelectivityPolicy::with_fanout_budgets(
            settings.get_selectivity_smoothing_alpha(),
            settings.get_selectivity_gamma(),
            fanout_budgets,
        );

        Ok(Self::from_parts_with_stats(
            graph_store,
            vector_store,
            Box::new(EmbeddingProviderMemoryEmbedder::new(embed_provider)),
            stats_store,
            selectivity_policy,
        ))
    }

    /// Constructs a new CharacterMemory instance.
    ///
    /// Vector candidate recall uses the embedded store by default and requires
    /// `VECTOR_STORE_PATH`. Explicit service mode instead requires
    /// `QDRANT_CONNECTION_STRING`.
    /// OpenAI requires both `OPENAI_API_KEY` and `EMBEDDING_MODEL`.
    ///
    /// # Parameters
    ///
    /// - `settings`: Configuration settings for the memory system
    /// - `collection_name`: Name of the vector collection to use
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `CharacterMemory` instance
    /// - `Err`: A `CustomError` if initialization fails
    pub async fn new(settings: Settings, collection_name: String) -> Result<Self, CustomError> {
        Self::construct(settings, collection_name, None).await
    }
}

fn preflight(
    settings: &Settings,
    embed_provider: Option<Box<dyn EmbeddingProvider>>,
) -> Result<(Box<dyn EmbeddingProvider>, usize), CustomError> {
    let vector_size = match &embed_provider {
        Some(provider) => provider.vector_size(),
        None => {
            settings.require_openai_api_key()?;
            settings.get_embedding_vector_size()?
        }
    };
    if vector_size == 0 {
        return Err(EmbeddingError::InvalidVectorSize {
            actual: vector_size,
        }
        .into());
    }
    if settings.get_graph_store_mode() == ConfigGraphStoreMode::Persistent {
        settings.get_oxigraph_path()?;
    }
    match settings.get_vector_store_mode() {
        VectorStoreMode::Embedded => {
            settings.get_vector_store_path()?;
        }
        VectorStoreMode::Service => {
            settings.get_service_qdrant_connection()?;
        }
    }
    if settings.get_retrieval_stats_store_mode() == ConfigRetrievalStatsStoreMode::Sqlite
        && settings.get_retrieval_stats_path().as_os_str().is_empty()
    {
        return Err(ConfigValidationError {
            keys: vec!["RETRIEVAL_STATS_PATH"],
            reason: ConfigValidationReason::MissingForMode {
                mode_key: "RETRIEVAL_STATS_STORE_MODE",
                mode: "sqlite",
            },
        }
        .into());
    }

    // Build the default provider only after all required settings have been admitted.
    let embed_provider = match embed_provider {
        Some(provider) => provider,
        None => Box::new(OpenAIEmbeddingProvider::new(
            EmbeddingProviderSettings::new(
                settings.get_openai_api_key().to_owned(),
                settings.get_embedding_model()?,
            ),
        )?),
    };
    Ok((embed_provider, vector_size))
}

pub(crate) fn retrieval_stats_store(
    settings: &Settings,
) -> Result<Box<dyn RetrievalStatsStore>, CustomError> {
    match settings.get_retrieval_stats_store_mode() {
        ConfigRetrievalStatsStoreMode::Sqlite => {
            match SqliteRetrievalStatsStore::open(settings.get_retrieval_stats_path()) {
                Ok(store) => Ok(Box::new(store)),
                Err(error) => match settings.get_retrieval_stats_health_fail_mode() {
                    RetrievalStatsHealthFailMode::Conservative => {
                        Ok(Box::new(InMemoryRetrievalStatsStore::unhealthy(
                            RetrievalStatsHealthCause::StoreInitialization { error },
                        )))
                    }
                },
            }
        }
        ConfigRetrievalStatsStoreMode::InMemory => Ok(Box::new(InMemoryRetrievalStatsStore::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::embedding::MockEmbeddingProvider;
    use crate::ports::retrieval_stats::RetrievalStatsHealthState;

    #[test]
    fn preflight_rejects_zero_provider_vector_size_before_storage_init() {
        let settings = Settings::new(::config::Config::default()).unwrap();

        let error = preflight_error(&settings, 0);

        assert!(matches!(
            error,
            CustomError::Embedding(EmbeddingError::InvalidVectorSize { actual: 0 })
        ));
    }

    #[test]
    fn preflight_rejects_persistent_endpoint_url_before_vector_store_admission() {
        let settings = Settings::new(
            ::config::Config::builder()
                .set_override("qdrant_connection_string", "http://127.0.0.1:1")
                .unwrap()
                .set_override("oxigraph_path", "http://127.0.0.1:7878")
                .unwrap()
                .set_override("graph_store_mode", "persistent")
                .unwrap()
                .set_override("retrieval_stats_store_mode", "in_memory")
                .unwrap()
                .build()
                .unwrap(),
        )
        .unwrap();

        let error = preflight_error(&settings, 3);

        let CustomError::ConfigValidation(ConfigValidationError { keys, reason }) = error else {
            panic!("expected configuration validation error");
        };
        assert_eq!(keys, vec!["OXIGRAPH_PATH"]);
        assert_eq!(
            reason,
            ConfigValidationReason::OutOfDomain {
                expected: "a local filesystem path",
                actual: "http://127.0.0.1:7878".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn sqlite_stats_open_failure_uses_configured_conservative_fallback() {
        let settings = Settings::new(
            ::config::Config::builder()
                .set_override("retrieval_stats_store_mode", "sqlite")
                .unwrap()
                .set_override("retrieval_stats_path", ".")
                .unwrap()
                .set_override("retrieval_stats_health_fail_mode", "conservative")
                .unwrap()
                .build()
                .unwrap(),
        )
        .unwrap();

        let store = retrieval_stats_store(&settings).unwrap();
        let health = store.health().await.unwrap();

        assert_eq!(health.state, RetrievalStatsHealthState::Unhealthy);
        assert!(matches!(
            health.last_error_cause,
            Some(RetrievalStatsHealthCause::StoreInitialization { .. })
        ));
    }

    fn preflight_error(settings: &Settings, vector_size: usize) -> CustomError {
        let mut provider = MockEmbeddingProvider::new();
        provider.expect_vector_size().return_const(vector_size);
        match preflight(settings, Some(Box::new(provider))) {
            Ok(_) => panic!("preflight should reject the settings"),
            Err(error) => error,
        }
    }
}
