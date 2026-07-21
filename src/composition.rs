use async_trait::async_trait;

use crate::adapters::oxigraph::OxigraphGraphAuthorityStore;
use crate::adapters::stats::{InMemoryRetrievalStatsStore, SqliteRetrievalStatsStore};
use crate::adapters::{OpenAIEmbeddingProvider, QdrantVectorCandidateStore};
use crate::api::embedding::EmbeddingProvider;
use crate::config::{
    EmbeddingProviderSettings, GraphStoreMode as ConfigGraphStoreMode,
    RetrievalStatsHealthFailMode, RetrievalStatsStoreMode as ConfigRetrievalStatsStoreMode,
    Settings,
};
use crate::errors::{CustomError, EmbeddingError};
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
            .map_err(normalize_embedding_error)
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let texts: Vec<&str> = inputs.iter().map(|input| input.text.as_str()).collect();
        self.provider
            .bulk_generate_embeddings(&texts)
            .await
            .map_err(normalize_embedding_error)
    }
}

fn normalize_embedding_error(error: CustomError) -> CustomError {
    match error {
        CustomError::Embedding(_) => error,
        error => EmbeddingError::Unrecognized(error.to_string()).into(),
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
        Self {
            memory_composition: MemoryComposition {
                graph_store,
                vector_store,
                embedder,
                stats_store: Box::new(crate::adapters::stats::InMemoryRetrievalStatsStore::new()),
                selectivity_policy: RetrievalSelectivityPolicy::default(),
            },
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
        }
    }
}

impl CharacterMemory {
    /// Constructs a new CharacterMemory instance using a caller-provided embedding provider.
    ///
    /// # Description
    ///
    /// This constructor allows callers to inject custom embedding generation while using the
    /// default graph-authoritative storage composition.
    ///
    /// # Parameters
    ///
    /// - `settings`: Global configuration used to derive the Qdrant connection and embedding
    ///   model settings required to initialize the Qdrant candidate collection.
    /// - `collection_name`: The name of the Qdrant collection where memory vectors will be
    ///   stored and queried.
    /// - `embed_provider`: A boxed implementation of [`EmbeddingProvider`] that is responsible
    ///   for generating embeddings from input data.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok(Self)`: A new [`CharacterMemory`] instance backed by Oxigraph graph authority and
    ///   Qdrant vector candidate recall.
    /// - `Err(CustomError)`: Returned if any error occurs while resolving configuration from
    ///   `settings` or initializing the Oxigraph graph authority and Qdrant vector candidate
    ///   store.
    pub async fn new_with_embedding_provider(
        settings: Settings,
        collection_name: String,
        embed_provider: Box<dyn EmbeddingProvider>,
    ) -> Result<Self, CustomError> {
        let expected_vector_size = settings.get_embedding_vector_size()?;
        let provider_vector_size = embed_provider.vector_size();
        if provider_vector_size != expected_vector_size {
            return Err(EmbeddingError::ProviderVectorSizeMismatch {
                expected: expected_vector_size,
                actual: provider_vector_size,
            }
            .into());
        }

        let persistent_graph_path = match settings.get_graph_store_mode() {
            ConfigGraphStoreMode::Persistent => Some(settings.get_oxigraph_path()?),
            ConfigGraphStoreMode::InMemory => None,
        };

        let vector_store = QdrantVectorCandidateStore::new(
            settings.get_qdrant_connection(),
            collection_name,
            expected_vector_size as u64,
        )?;
        vector_store.init_collection().await?;
        let graph_store = match persistent_graph_path {
            Some(path) => Box::new(OxigraphGraphAuthorityStore::new_persistent(path)?)
                as Box<dyn GraphAuthorityStore>,
            None => Box::new(OxigraphGraphAuthorityStore::new_in_memory()?),
        };
        let stats_store = retrieval_stats_store(&settings)?;
        let fanout_budgets = settings.get_retrieval_fanout_budgets().into_iter().map(
            |(relation, object_type, budget)| (relation, object_type, budget.min(), budget.max()),
        );
        let selectivity_policy = RetrievalSelectivityPolicy::try_new_with_fanout_budgets(
            settings.get_selectivity_smoothing_alpha(),
            settings.get_selectivity_gamma(),
            fanout_budgets,
        )?;

        Ok(Self::from_parts_with_stats(
            graph_store,
            Box::new(vector_store),
            Box::new(EmbeddingProviderMemoryEmbedder::new(embed_provider)),
            stats_store,
            selectivity_policy,
        ))
    }

    /// Constructs a new CharacterMemory instance.
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
        // Configure and create the embedding provider
        let embedding_settings = EmbeddingProviderSettings::new(
            settings.get_openai_api_key().to_string(),
            settings.get_embedding_model()?,
        );
        let embed_provider = Box::new(OpenAIEmbeddingProvider::new(embedding_settings)?);

        Self::new_with_embedding_provider(settings, collection_name, embed_provider).await
    }
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
                        Ok(Box::new(InMemoryRetrievalStatsStore::unhealthy(format!(
                            "sqlite retrieval stats unavailable; using in-memory fallback: {error}"
                        ))))
                    }
                },
            }
        }
        ConfigRetrievalStatsStoreMode::InMemory => Ok(Box::new(InMemoryRetrievalStatsStore::new())),
    }
}
