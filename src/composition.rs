use async_trait::async_trait;

use crate::adapters::oxigraph::OxigraphGraphAuthorityStore;
use crate::adapters::stats::{InMemoryRetrievalStatsStore, SqliteRetrievalStatsStore};
use crate::adapters::{
    OpenAIEmbeddingProvider, QdrantEdgeVectorCandidateStore, QdrantVectorCandidateStore,
};
use crate::api::embedding::EmbeddingProvider;
use crate::config::{
    GraphStoreMode as ConfigGraphStoreMode,
    RetrievalStatsStoreMode as ConfigRetrievalStatsStoreMode, Settings, VectorStoreMode,
};
use crate::domain::{MemoryId, MemoryObject, ObjectType};
use crate::errors::{
    ConfigValidationError, ConfigValidationReason, CustomError, EmbeddingError,
    RetrievalStatsHealthCause,
};
use crate::memory::CharacterMemory;
use crate::models::vector::EmbeddingInput;
use crate::policy::RetrievalSelectivityPolicy;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectQuery};
use crate::ports::retrieval_stats::{
    retrieval_stats_edges, retrieval_stats_object_states, RetrievalStatsHealthState,
    RetrievalStatsStore,
};
use crate::ports::vector_candidate::VectorCandidateStore;

pub(crate) struct MemoryComposition {
    pub(crate) character_id: MemoryId,
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
        character_id: MemoryId,
    ) -> Self {
        let settings = Settings::new(Default::default()).unwrap();
        Self {
            memory_composition: MemoryComposition {
                character_id,
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
        character_id: MemoryId,
    ) -> Self {
        Self {
            memory_composition: MemoryComposition {
                character_id,
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
    /// - `character_id`: The character's notion id. An existing graph store must belong to it.
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
        character_id: MemoryId,
    ) -> Result<Self, CustomError> {
        Self::construct(
            settings,
            collection_name,
            Some(embed_provider),
            character_id,
        )
        .await
    }

    async fn construct(
        settings: Settings,
        collection_name: String,
        embed_provider: Option<Box<dyn EmbeddingProvider>>,
        character_id: MemoryId,
    ) -> Result<Self, CustomError> {
        let (embed_provider, vector_size) = preflight(&settings, embed_provider)?;
        let persistent_graph_path = match settings.get_graph_store_mode() {
            ConfigGraphStoreMode::Persistent => Some(settings.get_oxigraph_path()?),
            ConfigGraphStoreMode::InMemory => None,
        };
        let graph_store = match persistent_graph_path {
            Some(path) => Box::new(OxigraphGraphAuthorityStore::new_persistent(path)?)
                as Box<dyn GraphAuthorityStore>,
            None => Box::new(OxigraphGraphAuthorityStore::new_in_memory()?),
        };
        graph_store.ensure_character_identity(character_id).await?;

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
        let stats_store = retrieval_stats_store(&settings, graph_store.as_ref()).await?;
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
            character_id,
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
    /// - `character_id`: The character's notion id, checked against the graph store before opening vectors
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `CharacterMemory` instance
    /// - `Err`: A `CustomError` if initialization fails
    pub async fn new(
        settings: Settings,
        collection_name: String,
        character_id: MemoryId,
    ) -> Result<Self, CustomError> {
        Self::construct(settings, collection_name, None, character_id).await
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
            settings.get_openai_api_key().to_owned(),
            settings.get_embedding_model()?,
        )),
    };
    Ok((embed_provider, vector_size))
}

pub(crate) async fn retrieval_stats_store(
    settings: &Settings,
    graph_store: &dyn GraphAuthorityStore,
) -> Result<Box<dyn RetrievalStatsStore>, CustomError> {
    match settings.get_retrieval_stats_store_mode() {
        ConfigRetrievalStatsStoreMode::Sqlite => {
            match SqliteRetrievalStatsStore::open(settings.get_retrieval_stats_path()) {
                Ok(store) => {
                    if matches!(store.health().await, Ok(health) if health.state == RetrievalStatsHealthState::Unhealthy)
                    {
                        // Failed recovery retains the original marker and conservative retrieval.
                        let _ = rebuild_retrieval_stats(&store, graph_store).await;
                    }
                    Ok(Box::new(store))
                }
                Err(error) => Ok(Box::new(InMemoryRetrievalStatsStore::unhealthy(
                    RetrievalStatsHealthCause::StoreInitialization { error },
                ))),
            }
        }
        ConfigRetrievalStatsStoreMode::InMemory => Ok(Box::new(InMemoryRetrievalStatsStore::new())),
    }
}

async fn rebuild_retrieval_stats(
    store: &SqliteRetrievalStatsStore,
    graph_store: &dyn GraphAuthorityStore,
) -> Result<(), CustomError> {
    let objects = graph_store
        .query_objects(&GraphObjectQuery::by_types(
            vec![
                ObjectType::Episode,
                ObjectType::Observation,
                ObjectType::DerivedMemory,
                ObjectType::MemoryLink,
            ],
            None,
        ))
        .await?;
    let links = objects
        .iter()
        .filter_map(|object| match object {
            MemoryObject::MemoryLink(link) => Some(link.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let derived_ids = objects
        .iter()
        .filter_map(|object| match object {
            MemoryObject::DerivedMemory(memory) => Some(memory.id),
            _ => None,
        })
        .collect::<Vec<_>>();
    let superseded = graph_store
        .query_superseded_derived_memory_ids(&derived_ids)
        .await?
        .into_iter()
        .collect();
    store.rebuild(
        &retrieval_stats_edges(&objects, &links, &superseded),
        &retrieval_stats_object_states(&objects, &superseded),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RelationType, RetentionState};
    use crate::errors::{GraphQueryError, RetrievalStatsStoreError};
    use crate::policy::retrieval_selectivity::{
        selectivity_plan_for_entity, SelectivityStatsContext,
    };
    use crate::ports::graph_authority::TraceMode;
    use crate::ports::retrieval_stats::{RetrievalStatsCounterKey, RetrievalStatsHealth};
    use crate::test_support::{in_memory_graph_store, representative_fixtures, TestGraphStore};
    use crate::usecases::stats_projection::StatsProjectionService;

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
    async fn sqlite_stats_open_failure_uses_conservative_fallback() {
        let settings = Settings::new(
            ::config::Config::builder()
                .set_override("retrieval_stats_store_mode", "sqlite")
                .unwrap()
                .set_override("retrieval_stats_path", ".")
                .unwrap()
                .build()
                .unwrap(),
        )
        .unwrap();

        let store = retrieval_stats_store(&settings, &crate::test_support::in_memory_graph_store())
            .await
            .unwrap();
        let health = store.health().await.unwrap();

        assert_eq!(health.state, RetrievalStatsHealthState::Unhealthy);
        assert!(matches!(
            health.last_error_cause,
            Some(RetrievalStatsHealthCause::StoreInitialization { .. })
        ));
    }

    #[tokio::test]
    async fn reopened_unhealthy_stats_match_fresh_graph_projection() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("stats.sqlite3");
        let graph = in_memory_graph_store();
        let mut fixtures = representative_fixtures();
        fixtures.suppressed_seed.retention_state = RetentionState::Active;
        fixtures.correction.retention_state = RetentionState::Suppressed;
        let mut objects = fixtures.objects();
        let mut suppressed_episode = fixtures.episode.clone();
        suppressed_episode.id = MemoryId::from_u128(901);
        suppressed_episode.retention_state = RetentionState::Suppressed;
        objects.push(MemoryObject::Episode(suppressed_episode));
        let mut links = fixtures.links();
        for (index, object) in [
            MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
            MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
            MemoryObject::DerivedMemory(fixtures.correction.clone()),
            MemoryObject::Observation(fixtures.salient_observation.clone()),
        ]
        .into_iter()
        .enumerate()
        {
            let mut link = fixtures.hub_links[0].clone();
            link.id = MemoryId::from_u128(910 + index as u128);
            link.to_id = object.id();
            link.to_type = object.object_type();
            link.relation = RelationType::AssociatedWith;
            links.push(link);
        }
        graph
            .upsert_objects_and_links(&objects, &links)
            .await
            .unwrap();
        let fresh =
            SqliteRetrievalStatsStore::open(directory.path().join("fresh.sqlite3")).unwrap();
        assert!(StatsProjectionService::new(&graph, &fresh)
            .project(&objects, &links)
            .await
            .causes
            .is_empty());

        let stale_entity = MemoryId::from_u128(999);
        let mut stale_episode = fixtures.episode.clone();
        stale_episode.id = MemoryId::from_u128(998);
        let mut stale_link = fixtures.hub_links[0].clone();
        stale_link.from_id = stale_entity;
        stale_link.to_id = stale_episode.id;
        stale_link.relation = RelationType::AssociatedWith;
        let stale_objects = [MemoryObject::Episode(stale_episode)];
        let stale = SqliteRetrievalStatsStore::open(&path).unwrap();
        stale
            .record_edges(&retrieval_stats_edges(
                &stale_objects,
                &[stale_link],
                &Default::default(),
            ))
            .await
            .unwrap();
        stale
            .record_object_states(&retrieval_stats_object_states(
                &stale_objects,
                &Default::default(),
            ))
            .await
            .unwrap();
        stale.mark_unhealthy(stats_failure()).await.unwrap();
        drop(stale);

        let recovered = retrieval_stats_store(&sqlite_stats_settings(&path), &graph)
            .await
            .unwrap();
        assert_eq!(
            recovered.health().await.unwrap(),
            RetrievalStatsHealth::default()
        );
        assert_eq!(
            recovered.global_episode_counter().await.unwrap(),
            fresh.global_episode_counter().await.unwrap()
        );
        for relation in [
            RelationType::Involves,
            RelationType::AssociatedWith,
            RelationType::About,
        ] {
            for object_type in [
                ObjectType::Episode,
                ObjectType::Observation,
                ObjectType::DerivedMemory,
            ] {
                assert_eq!(
                    recovered
                        .global_counter(relation, object_type)
                        .await
                        .unwrap(),
                    fresh.global_counter(relation, object_type).await.unwrap()
                );
                for entity_id in [
                    fixtures.user_entity.id,
                    fixtures.assistant_entity.id,
                    fixtures.hub_entity.id,
                    stale_entity,
                ] {
                    let key = RetrievalStatsCounterKey {
                        entity_id,
                        relation_kind: relation,
                        object_type,
                    };
                    assert_eq!(
                        recovered.counter(&key).await.unwrap(),
                        fresh.counter(&key).await.unwrap(),
                        "{key:?}"
                    );
                }
            }
        }

        let settings = sqlite_stats_settings(&path);
        let policy = RetrievalSelectivityPolicy::with_fanout_budgets(
            settings.get_selectivity_smoothing_alpha(),
            settings.get_selectivity_gamma(),
            settings
                .get_retrieval_fanout_budgets()
                .map(|(relation, kind, budget)| (relation, kind, budget.min(), budget.max())),
        );
        let mut plans = Vec::new();
        for store in [recovered.as_ref(), &fresh as &dyn RetrievalStatsStore] {
            let context = SelectivityStatsContext::load(store).await.unwrap();
            plans.push(
                selectivity_plan_for_entity(
                    fixtures.hub_entity.id,
                    1.0,
                    32,
                    store,
                    policy,
                    &context,
                    Default::default(),
                    TraceMode::Enabled,
                )
                .await
                .unwrap(),
            );
        }
        assert!(plans[0]
            .traces
            .iter()
            .any(|trace| trace.relation == RelationType::Involves && !trace.fallback));
        assert_eq!(plans[0].fanout_overrides, plans[1].fanout_overrides);
        assert_eq!(plans[0].traces, plans[1].traces);
        drop(recovered);
        assert_eq!(
            SqliteRetrievalStatsStore::open(&path)
                .unwrap()
                .health()
                .await
                .unwrap(),
            RetrievalStatsHealth::default()
        );
    }

    #[tokio::test]
    async fn failed_graph_recovery_preserves_health_cause_and_conservative_retrieval() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("stats.sqlite3");
        let cause = stats_failure();
        let store = SqliteRetrievalStatsStore::open(&path).unwrap();
        store.mark_unhealthy(cause.clone()).await.unwrap();
        drop(store);

        for graph in [
            TestGraphStore {
                query_error: Some(GraphQueryError::Selection {
                    detail: "read unavailable".to_owned(),
                }),
                ..Default::default()
            },
            TestGraphStore {
                fail_currency_query: true,
                ..Default::default()
            },
        ] {
            let settings = sqlite_stats_settings(&path);
            let store = retrieval_stats_store(&settings, &graph).await.unwrap();
            assert_eq!(
                store.health().await.unwrap(),
                RetrievalStatsHealth {
                    state: RetrievalStatsHealthState::Unhealthy,
                    last_error_cause: Some(cause.clone()),
                }
            );
            let context = SelectivityStatsContext::load(store.as_ref()).await.unwrap();
            let policy = RetrievalSelectivityPolicy::with_fanout_budgets(
                settings.get_selectivity_smoothing_alpha(),
                settings.get_selectivity_gamma(),
                settings
                    .get_retrieval_fanout_budgets()
                    .map(|(relation, kind, budget)| (relation, kind, budget.min(), budget.max())),
            );
            let plan = selectivity_plan_for_entity(
                MemoryId::from_u128(1),
                1.0,
                32,
                store.as_ref(),
                policy,
                &context,
                Default::default(),
                TraceMode::Enabled,
            )
            .await
            .unwrap();
            assert!(!plan.traces.is_empty());
            assert!(plan.traces.iter().all(|trace| trace.fallback));
        }
        assert_eq!(
            SqliteRetrievalStatsStore::open(&path)
                .unwrap()
                .health()
                .await
                .unwrap()
                .last_error_cause,
            Some(cause)
        );
    }

    #[tokio::test]
    async fn healthy_stats_reopen_without_reading_the_graph() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("stats.sqlite3");
        let store = SqliteRetrievalStatsStore::open(&path).unwrap();
        let objects = [MemoryObject::Episode(crate::test_support::simple_episode())];
        store
            .record_object_states(&retrieval_stats_object_states(
                &objects,
                &Default::default(),
            ))
            .await
            .unwrap();
        let expected = store.global_episode_counter().await.unwrap();
        drop(store);
        let graph = TestGraphStore {
            record_queries: true,
            ..Default::default()
        };
        let reopened = retrieval_stats_store(&sqlite_stats_settings(&path), &graph)
            .await
            .unwrap();
        assert_eq!(
            reopened.health().await.unwrap(),
            RetrievalStatsHealth::default()
        );
        assert_eq!(reopened.global_episode_counter().await.unwrap(), expected);
        assert!(graph.calls().is_empty());
    }

    fn sqlite_stats_settings(path: &std::path::Path) -> Settings {
        Settings::new(
            ::config::Config::builder()
                .set_override("retrieval_stats_store_mode", "sqlite")
                .unwrap()
                .set_override("retrieval_stats_path", path.to_str().unwrap())
                .unwrap()
                .build()
                .unwrap(),
        )
        .unwrap()
    }

    fn stats_failure() -> RetrievalStatsHealthCause {
        RetrievalStatsHealthCause::EdgeWrite {
            error: RetrievalStatsStoreError::Sqlite {
                detail: "interrupted projection".to_owned(),
            },
        }
    }

    struct MockEmbeddingProvider(usize);

    #[async_trait]
    impl EmbeddingProvider for MockEmbeddingProvider {
        fn vector_size(&self) -> usize {
            self.0
        }

        async fn generate_embedding<'a>(&self, _text: &'a str) -> Result<Vec<f32>, EmbeddingError> {
            unreachable!("preflight must not generate embeddings")
        }

        async fn bulk_generate_embeddings<'a>(
            &self,
            _texts: &'a [&'a str],
        ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
            unreachable!("preflight must not generate embeddings")
        }
    }

    fn preflight_error(settings: &Settings, vector_size: usize) -> CustomError {
        match preflight(settings, Some(Box::new(MockEmbeddingProvider(vector_size)))) {
            Ok(_) => panic!("preflight should reject the settings"),
            Err(error) => error,
        }
    }
}
