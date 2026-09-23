use std::{fs, path::Path};
use test_support::id;

use async_trait::async_trait;
use character_memory::{
    CharacterMemory, CollectionCompatibilityError, CollectionMismatch, ConfigValidationError,
    ConfigValidationReason, CustomError, EmbeddingError, EmbeddingProvider, EpisodeDraft,
    ForgetMemoryDraft, LifecycleTargetRef, MemoryId, MemoryObjectRef, ObjectType, ObservationDraft,
    RememberInput, RememberOptions, RetrievalCandidateLimits, RetrievalContext, Settings,
    VectorCandidateTrace, VectorIndexingCause, VectorRecallCompleteness,
};
use config::{builder::DefaultState, Config, ConfigBuilder};
use tempfile::TempDir;

#[path = "support/mod.rs"]
pub mod test_support;

#[tokio::test]
async fn injected_provider_opens_without_unused_settings_and_ignores_them_when_present() {
    for ignored_setting in [
        None,
        Some(("openai_api_key", "")),
        Some(("embedding_model", "not-an-openai-model")),
        Some(("oxigraph_path", "http://unused.invalid")),
        Some(("retrieval_stats_path", "")),
    ] {
        let temp = TempDir::new().unwrap();
        let mut builder = local_mode_settings(temp.path());
        if let Some((key, value)) = ignored_setting {
            builder = builder.set_override(key, value).unwrap();
        }
        let settings = Settings::new(builder.build().unwrap()).unwrap();
        let memory = CharacterMemory::new_with_embedding_provider(
            settings,
            "injected_without_placeholders".to_owned(),
            Box::new(constant_provider(2)),
        )
        .await
        .unwrap();
        remember_fixture(&memory).await;
        assert_eq!(ids(&episode_snapshot(&memory).await), vec![id(1), id(2)]);
        test_support::close_and_remove_root(memory, temp).await;
    }
}

#[tokio::test]
async fn injected_provider_dimension_must_match_existing_vector_storage() {
    let temp = TempDir::new().unwrap();
    let config = local_mode_settings(temp.path()).build().unwrap();
    let memory = CharacterMemory::new_with_embedding_provider(
        Settings::new(config.clone()).unwrap(),
        "injected_dimensions".to_owned(),
        Box::new(constant_provider(2)),
    )
    .await
    .unwrap();
    memory.close().await.unwrap();

    let result = CharacterMemory::new_with_embedding_provider(
        Settings::new(config).unwrap(),
        "injected_dimensions".to_owned(),
        Box::new(constant_provider(3)),
    )
    .await;
    assert!(matches!(
        result,
        Err(CustomError::CollectionIncompatible(
            CollectionCompatibilityError {
                mismatch: CollectionMismatch::VectorSize {
                    expected: 3,
                    actual: 2
                },
                ..
            }
        ))
    ));
    temp.close().unwrap();
}

#[tokio::test]
async fn persistent_graph_requires_a_path_at_facade_construction() {
    for path in [None, Some("")] {
        let temp = TempDir::new().unwrap();
        let mut builder = local_mode_settings(temp.path())
            .set_override("graph_store_mode", "persistent")
            .unwrap();
        if let Some(path) = path {
            builder = builder.set_override("oxigraph_path", path).unwrap();
        }
        let settings = Settings::new(builder.build().unwrap()).unwrap();
        let result = CharacterMemory::new_with_embedding_provider(
            settings,
            "missing_graph_path".to_owned(),
            Box::new(constant_provider(2)),
        )
        .await;
        assert!(matches!(
            result,
            Err(CustomError::ConfigValidation(ConfigValidationError {
                keys,
                reason: ConfigValidationReason::MissingForMode {
                    mode_key: "GRAPH_STORE_MODE",
                    mode: "persistent",
                },
            })) if keys == vec!["OXIGRAPH_PATH"]
        ));
        assert!(!temp.path().join("vectors").exists());
        temp.close().unwrap();
    }
}

#[tokio::test]
async fn openai_requires_its_key_and_model_at_facade_construction() {
    for (key, model, missing_field) in [
        (None, Some("text-embedding-3-small"), "OPENAI_API_KEY"),
        (Some(""), Some("text-embedding-3-small"), "OPENAI_API_KEY"),
        (Some("test-key"), None, "EMBEDDING_MODEL"),
        (Some("test-key"), Some(""), "EMBEDDING_MODEL"),
    ] {
        let mut builder = Config::builder();
        for (name, value) in [("openai_api_key", key), ("embedding_model", model)] {
            if let Some(value) = value {
                builder = builder.set_override(name, value).unwrap();
            }
        }
        let settings = Settings::new(builder.build().unwrap()).unwrap();
        let result = CharacterMemory::new(settings, "missing_openai_settings".to_owned()).await;
        assert!(matches!(
            result,
            Err(CustomError::ConfigValidation(ConfigValidationError {
                keys,
                reason: ConfigValidationReason::MissingForMode {
                    mode_key: "embedding provider",
                    mode: "openai",
                },
            })) if keys == vec![missing_field]
        ));
    }
}

#[tokio::test]
async fn flat_openai_settings_still_open_the_default_facade() {
    let temp = TempDir::new().unwrap();
    let config = local_mode_settings(temp.path())
        .set_override("openai_api_key", "test-key")
        .unwrap()
        .set_override("embedding_model", "text-embedding-3-small")
        .unwrap()
        .build()
        .unwrap();
    let settings = Settings::new(config).unwrap();
    assert_eq!(settings.get_embedding_vector_size().unwrap(), 1536);
    let memory = CharacterMemory::new(settings, "flat_openai_settings".to_owned())
        .await
        .unwrap();
    test_support::close_and_remove_root(memory, temp).await;
}

fn local_mode_settings(path: &Path) -> ConfigBuilder<DefaultState> {
    Config::builder()
        .set_override("vector_store_path", path.join("vectors").to_str().unwrap())
        .unwrap()
        .set_override("graph_store_mode", "in_memory")
        .unwrap()
        .set_override("retrieval_stats_store_mode", "in_memory")
        .unwrap()
}

#[tokio::test]
async fn empty_sqlite_path_fails_before_either_constructor_creates_stores() {
    for injected in [false, true] {
        let temp = TempDir::new().unwrap();
        let vector_path = temp.path().join("vectors");
        let graph_path = temp.path().join("graph");
        let config = local_mode_settings(temp.path())
            .set_override("graph_store_mode", "persistent")
            .unwrap()
            .set_override("oxigraph_path", graph_path.to_str().unwrap())
            .unwrap()
            .set_override("retrieval_stats_store_mode", "sqlite")
            .unwrap()
            .set_override("retrieval_stats_path", "")
            .unwrap()
            .set_override("openai_api_key", "test-key")
            .unwrap()
            .set_override("embedding_model", "text-embedding-3-small")
            .unwrap()
            .build()
            .unwrap();
        let settings = Settings::new(config).unwrap();
        let result = if injected {
            CharacterMemory::new_with_embedding_provider(
                settings,
                "preflight".to_owned(),
                Box::new(constant_provider(2)),
            )
            .await
        } else {
            CharacterMemory::new(settings, "preflight".to_owned()).await
        };
        assert!(matches!(
            result,
            Err(CustomError::ConfigValidation(ConfigValidationError {
                keys,
                reason: ConfigValidationReason::MissingForMode {
                    mode_key: "RETRIEVAL_STATS_STORE_MODE",
                    mode: "sqlite",
                },
            })) if keys == vec!["RETRIEVAL_STATS_PATH"]
        ));
        assert!(!vector_path.exists());
        assert!(!graph_path.exists());
        temp.close().unwrap();
    }
}

#[tokio::test]
async fn embedded_default_contract_is_service_free_restart_safe_and_canonical() {
    let temp = TempDir::new().unwrap();
    let collection = "embedded_contract";
    let memory = open_embedded(temp.path(), collection).await.unwrap();

    remember_fixture(&memory).await;
    let first = episode_snapshot(&memory).await;
    assert_eq!(ids(&first), vec![id(1), id(2)]);
    assert_eq!(first[0].score, first[1].score);
    assert_eq!(
        completeness(&memory).await,
        VectorRecallCompleteness::Exhaustive { scanned: 5 }
    );
    let observation = memory.retrieve(observation_query()).await.unwrap();
    let observation_trace = observation.trace.unwrap().vector_candidates;
    assert!(ids(&observation_trace).contains(&id(10)));
    assert!(observation_trace
        .iter()
        .all(|candidate| candidate.object.object_type == ObjectType::Observation));
    assert_eq!(
        observation.rationale.telemetry.vector_recall_completeness,
        VectorRecallCompleteness::Exhaustive { scanned: 5 }
    );

    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::episode(id(1)),
            "exercise vector delete",
        ))
        .await
        .unwrap();
    assert_eq!(ids(&episode_snapshot(&memory).await), vec![id(2), id(3)]);

    drop(memory);
    let reopened = open_embedded(temp.path(), collection).await.unwrap();
    assert_eq!(ids(&episode_snapshot(&reopened).await), vec![id(2), id(3)]);
    test_support::close_and_remove_root(reopened, temp).await;
}

#[tokio::test]
async fn close_releases_local_stores_for_immediate_removal_and_fresh_reopen() {
    let temp = TempDir::new().unwrap();
    let collection = "close_release";
    let config = test_support::persistent_settings(temp.path())
        .build()
        .unwrap();
    let memory = open(Config::builder().add_source(config.clone()), collection)
        .await
        .unwrap();
    remember_fixture(&memory).await;
    assert!(temp.path().join("stats.sqlite3").is_file());
    assert!(temp.path().join("graph").is_dir());
    memory.close().await.unwrap();
    fs::remove_dir_all(temp.path()).unwrap();
    assert!(!temp.path().exists());

    let reopened = open(Config::builder().add_source(config), collection)
        .await
        .unwrap();
    assert!(episode_snapshot(&reopened).await.is_empty());
    remember_fixture(&reopened).await;
    assert_eq!(ids(&episode_snapshot(&reopened).await), vec![id(1), id(2)]);
    test_support::close_and_remove_root(reopened, temp).await;
}

#[tokio::test]
async fn default_construction_rejects_a_missing_vector_store_path() {
    let settings = Settings::new(common_settings().build().unwrap()).unwrap();
    let vector_size = settings.get_embedding_vector_size().unwrap();

    let error = match CharacterMemory::new_with_embedding_provider(
        settings,
        "missing_vector_path".to_owned(),
        Box::new(test_support::TestEmbeddingProvider::new(
            vector_size,
            move |_: &str| constant_embedding(vector_size),
        )),
    )
    .await
    {
        Ok(_) => panic!("the embedded default must require VECTOR_STORE_PATH"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        CustomError::ConfigValidation(ConfigValidationError {
            keys,
            reason: ConfigValidationReason::MissingValue,
        }) if keys == vec!["VECTOR_STORE_PATH"]
    ));
}

#[tokio::test]
async fn embedded_zero_norm_contract_rejects_records_and_exhaustively_scores_queries() {
    let temp = TempDir::new().unwrap();
    let memory = open_embedded_for_zero_norm(temp.path(), "embedded_zero_norm")
        .await
        .unwrap();

    assert_zero_norm_contract(&memory).await;
    test_support::close_and_remove_root(memory, temp).await;
}

#[tokio::test]
async fn service_and_embedded_share_the_zero_norm_contract() {
    if std::env::var_os("REQUIRE_QDRANT_TESTS").is_none() {
        return;
    }

    dotenvy::dotenv().ok();
    let temp = TempDir::new().unwrap();
    let collection = test_support::unique_collection_name();
    let embedded = open_embedded_for_zero_norm(temp.path(), "parity_zero_norm")
        .await
        .unwrap();
    let service = open_service_for_zero_norm(&collection).await.unwrap();

    let result = async {
        assert_zero_norm_contract(&embedded).await;
        assert_zero_norm_contract(&service).await;
    }
    .await;

    test_support::close_and_remove_root(embedded, temp).await;
    service.close().await.unwrap();
    test_support::cleanup_collection(&collection).await;
    result
}

#[tokio::test]
async fn service_and_embedded_admit_identical_candidates_in_identical_order() {
    if std::env::var_os("REQUIRE_QDRANT_TESTS").is_none() {
        return;
    }

    let temp = TempDir::new().unwrap();
    let collection = test_support::unique_collection_name();
    let embedded = open_embedded(temp.path(), "parity_embedded").await.unwrap();
    let service = open_service(&collection).await.unwrap();

    let result = async {
        remember_fixture(&embedded).await;
        remember_fixture(&service).await;
        let embedded_trace = episode_snapshot(&embedded).await;
        let service_trace = episode_snapshot(&service).await;

        assert_eq!(embedded_trace, service_trace);
        assert_eq!(
            completeness(&embedded).await,
            VectorRecallCompleteness::Exhaustive { scanned: 5 }
        );
        assert!(matches!(
            completeness(&service).await,
            VectorRecallCompleteness::BoundaryTieClosed { fetched: 5 }
        ));
    }
    .await;

    test_support::close_and_remove_root(embedded, temp).await;
    service.close().await.unwrap();
    test_support::cleanup_collection(&collection).await;
    result
}

async fn open_embedded(path: &Path, collection: &str) -> Result<CharacterMemory, CustomError> {
    open(
        common_settings()
            .set_override("vector_store_path", path.to_string_lossy().into_owned())
            .unwrap(),
        collection,
    )
    .await
}

async fn open_service(collection: &str) -> Result<CharacterMemory, CustomError> {
    dotenvy::dotenv().ok();
    let connection = std::env::var("QDRANT_CONNECTION_STRING")
        .map_err(|error| CustomError::ConfigParseError(error.to_string()))?;
    open(
        common_settings()
            .set_override("vector_store_mode", "service")
            .unwrap()
            .set_override("qdrant_connection_string", connection)
            .unwrap(),
        collection,
    )
    .await
}

async fn open_embedded_for_zero_norm(
    path: &Path,
    collection: &str,
) -> Result<CharacterMemory, CustomError> {
    open_for_zero_norm(
        common_settings()
            .set_override("vector_store_path", path.to_string_lossy().into_owned())
            .unwrap(),
        collection,
    )
    .await
}

async fn open_service_for_zero_norm(collection: &str) -> Result<CharacterMemory, CustomError> {
    let connection = std::env::var("QDRANT_CONNECTION_STRING")
        .map_err(|error| CustomError::ConfigParseError(error.to_string()))?;
    open_for_zero_norm(
        common_settings()
            .set_override("vector_store_mode", "service")
            .unwrap()
            .set_override("qdrant_connection_string", connection)
            .unwrap(),
        collection,
    )
    .await
}

async fn open(
    builder: ConfigBuilder<DefaultState>,
    collection: &str,
) -> Result<CharacterMemory, CustomError> {
    let settings = Settings::new(builder.build().unwrap())?;
    let vector_size = settings.get_embedding_vector_size()?;
    CharacterMemory::new_with_embedding_provider(
        settings,
        collection.to_owned(),
        Box::new(test_support::TestEmbeddingProvider::new(
            vector_size,
            move |_: &str| constant_embedding(vector_size),
        )),
    )
    .await
}

async fn open_for_zero_norm(
    builder: ConfigBuilder<DefaultState>,
    collection: &str,
) -> Result<CharacterMemory, CustomError> {
    let settings = Settings::new(builder.build().unwrap())?;
    let vector_size = settings.get_embedding_vector_size()?;
    CharacterMemory::new_with_embedding_provider(
        settings,
        collection.to_owned(),
        Box::new(ZeroNormFixtureEmbeddingProvider {
            vector_size,
            zero_texts: [
                "Episode summary: Episode summary".to_owned(),
                ZERO_NORM_QUERY.to_owned(),
            ],
            fixture_embedding: vec![0.0; vector_size],
        }),
    )
    .await
}

fn common_settings() -> ConfigBuilder<DefaultState> {
    Config::builder()
        .set_override("oxigraph_path", "unused-in-memory")
        .unwrap()
        .set_override("openai_api_key", "unused")
        .unwrap()
        .set_override("embedding_model", "text-embedding-3-small")
        .unwrap()
        .set_override("graph_store_mode", "in_memory")
        .unwrap()
        .set_override("retrieval_stats_store_mode", "in_memory")
        .unwrap()
}

async fn remember_fixture(memory: &CharacterMemory) {
    for value in 1..=4 {
        let mut episode = EpisodeDraft::new(format!("tie cohort episode {value}"));
        episode.id = Some(id(value));
        let outcome = memory
            .remember(
                RememberInput::new(format!("tie cohort source {value}")).with_episode(episode),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        assert!(outcome.vector_indexing_failure.is_none());
    }
    let mut observation = ObservationDraft::new(id(1), "shared deterministic tie cohort");
    observation.id = Some(id(10));
    let outcome = memory
        .remember(
            RememberInput::new("scope filter fixture").with_observation(observation),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    assert!(outcome.vector_indexing_failure.is_none());
}

const ZERO_NORM_QUERY: &str = "zero norm query";

async fn assert_zero_norm_contract(memory: &CharacterMemory) {
    let object = MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(1));
    let mut rejected = EpisodeDraft::new("Episode summary");
    rejected.id = Some(object.id);
    let outcome = memory
        .remember(
            RememberInput::new("zero norm record fixture").with_episode(rejected),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let failure = outcome
        .vector_indexing_failure
        .expect("zero-norm record must produce a typed indexing failure");
    assert!(failure.unindexed_objects.contains(&object));
    assert_eq!(
        failure.cause,
        VectorIndexingCause::ZeroNormEmbedding { object }
    );
    let empty = memory.retrieve(episode_query()).await.unwrap();
    assert!(empty.trace.unwrap().vector_candidates.is_empty());
    for value in 2..=3 {
        let mut episode = EpisodeDraft::new(format!("positive episode {value}"));
        episode.id = Some(id(value));
        let outcome = memory
            .remember(
                RememberInput::new(format!("positive source {value}")).with_episode(episode),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        assert!(outcome.vector_indexing_failure.is_none());
    }

    let mut query = RetrievalContext::new(ZERO_NORM_QUERY).with_trace();
    query.object_type_defaults = vec![ObjectType::Episode];
    query.candidate_limits.max_vector_candidates = 2;
    let outcome = memory.retrieve(query).await.unwrap();
    let candidates = outcome.trace.unwrap().vector_candidates;
    assert_eq!(ids(&candidates), vec![id(2), id(3)]);
    assert!(candidates.iter().all(|candidate| candidate.score == 0.0));
    assert_eq!(
        outcome.rationale.telemetry.vector_recall_completeness,
        VectorRecallCompleteness::Exhaustive { scanned: 2 }
    );
}

async fn episode_snapshot(memory: &CharacterMemory) -> Vec<VectorCandidateTrace> {
    let outcome = memory.retrieve(episode_query()).await.unwrap();
    outcome.trace.unwrap().vector_candidates
}

async fn completeness(memory: &CharacterMemory) -> VectorRecallCompleteness {
    memory
        .retrieve(episode_query())
        .await
        .unwrap()
        .rationale
        .telemetry
        .vector_recall_completeness
}

fn episode_query() -> RetrievalContext {
    let mut context = RetrievalContext::new("shared deterministic tie cohort").with_trace();
    context.object_type_defaults = vec![ObjectType::Episode];
    context.candidate_limits = RetrievalCandidateLimits {
        max_vector_candidates: 2,
        max_graph_roots: 2,
    };
    context
}

fn observation_query() -> RetrievalContext {
    let mut context = RetrievalContext::new("shared deterministic tie cohort").with_trace();
    context.object_type_defaults = vec![ObjectType::Observation];
    context
}

fn ids(trace: &[VectorCandidateTrace]) -> Vec<MemoryId> {
    trace.iter().map(|candidate| candidate.object.id).collect()
}

struct ZeroNormFixtureEmbeddingProvider {
    vector_size: usize,
    zero_texts: [String; 2],
    fixture_embedding: Vec<f32>,
}

impl ZeroNormFixtureEmbeddingProvider {
    fn embedding(&self, text: &str) -> Vec<f32> {
        if self.zero_texts.iter().any(|zero_text| zero_text == text) {
            self.fixture_embedding.clone()
        } else {
            constant_embedding(self.vector_size)
        }
    }
}

#[async_trait]
impl EmbeddingProvider for ZeroNormFixtureEmbeddingProvider {
    fn vector_size(&self) -> usize {
        self.vector_size
    }

    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(self.embedding(text))
    }

    async fn bulk_generate_embeddings<'a>(
        &self,
        texts: &'a [&'a str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(texts.iter().map(|text| self.embedding(text)).collect())
    }
}

fn constant_embedding(size: usize) -> Vec<f32> {
    let mut embedding = vec![0.0; size];
    embedding[0] = 1.0;
    embedding
}

fn constant_provider(size: usize) -> impl EmbeddingProvider {
    test_support::TestEmbeddingProvider::new(size, move |_: &str| constant_embedding(size))
}
