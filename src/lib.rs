mod config;
mod errors;
// NOTE: internal implementation code lives under `crate::internal`.

pub mod api;
mod internal;

use async_trait::async_trait;

use crate::config::settings::{
    EmbeddingProviderSettings, GraphStoreMode as ConfigGraphStoreMode,
    RetrievalStatsStoreMode as ConfigRetrievalStatsStoreMode,
};
use crate::internal::infrastructures::external_services::{
    OpenAIEmbeddingProvider, QdrantVectorCandidateStore,
};
use crate::internal::infrastructures::graph::{
    OxigraphGraphAuthorityStore, OxigraphHttpGraphAuthorityStore,
};
use crate::internal::infrastructures::retrieval_stats::SqliteRetrievalStatsStore;
use crate::internal::models::vector::EmbeddingInput;
use crate::internal::repositories::{
    CorrectionForgetPipeline, GraphAuthorityStore, InMemoryRetrievalStatsStore, LinkPipeline,
    MemoryEmbedder, RememberPipeline, RememberPipelineDraft, RetrievalSelectivityPolicy,
    RetrievalStatsStore, RetrievePipeline, VectorCandidateStore,
};

// Re-export types for public use
pub use crate::api::embedding::EmbeddingProvider;
pub use crate::api::types::{
    default_retrieval_object_types, graph_uri, ArchivePolicy, ContextPackSection,
    ContinuityContextPack, ContinuitySectionLimits, CorrectMemoryDraft, CorrectionCascadePolicy,
    CorrectionLifecyclePolicy, CorrectionTarget, DeferredDestructiveLifecyclePolicy,
    DeferredLifecycleAction, DerivedMemory, DerivedMemoryDraft, DerivedType, DomainValidationError,
    DraftDefaults, Entity, EntityDraft, EntityType, Episode, EpisodeDraft, ExternalSourceReference,
    ForgetCascadePolicy, ForgetLifecyclePolicy, ForgetMemoryDraft,
    GraphExpansionBoundedFailureSummary, GraphExpansionBoundedFailureTrace,
    GraphExpansionBoundedReason, GraphExpansionOutcome, GraphExpansionTelemetry,
    GraphExpansionTrace, GraphRelationTrace, IncludedDerivedMemory, LifecycleDtoValidationError,
    LifecycleFilterAction, LifecycleFilterDecision, LifecycleFilterReason,
    LifecycleMutationOutcome, LifecycleMutationTrace, LifecycleOmissionSummary, LifecycleTargetRef,
    MemoryId, MemoryLink, MemoryLinkDraft, MemoryObject, MemoryObjectDraft, MemoryObjectRef,
    MemoryThread, MemoryThreadDraft, Modality, ObjectType, Observation, ObservationDraft,
    RelationType, RememberDraft, RememberOutcome, ReplacementDerivedMemoryDraft, RetentionState,
    RetrievalCandidateLimits, RetrievalContext, RetrievalGraphLimits, RetrievalLifecyclePolicy,
    RetrievalRationale, RetrievalTelemetry, RetrievalTrace, RetrieveOutcome, SectionAssignment,
    SectionPressureSummary, SelectivityCountScope, SelectivityDecision, SelectivityTelemetry,
    SelectivityTrace, SourceObjectCorrectionTarget, SourceProvenanceReference, Stability,
    StaleCandidateOmission, StaleCandidateOmissionSummary, StaleCandidateReason,
    SupersededByEvidence, SuppressionPolicy, ThreadStatus, VectorCandidateTrace,
    VectorIndexingFailure, VectorMaintenanceFailure, CURRENT_SCHEMA_VERSION,
    DEFAULT_SCHEMA_VERSION, EPISODIC_MEMORY_SCHEMA_VERSION,
};
pub use crate::config::settings::{
    GraphStoreMode, RetrievalStatsHealthFailMode, RetrievalStatsStoreMode, Settings,
};
pub use crate::errors::CustomError;

// Re-export for integration tests
pub mod test_utils {
    use crate::config::settings::Settings;
    use crate::errors::CustomError;

    /// Loads settings from environment variables for integration tests.
    ///
    /// # Important
    ///
    /// This function is intended ONLY for use in integration tests and should not be used in production code.
    /// A `.env` file in the project root directory will be loaded if present,
    /// otherwise existing environment variables are used.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `Settings` instance with configuration loaded from environment
    /// - `Err`: A `CustomError` if loading fails
    pub fn load_test_settings() -> Result<Settings, CustomError> {
        Settings::load()
    }
}

/// CharacterMemory provides a high-level API for memory operations.
///
/// # Description
///
/// This struct serves as the main entry point for memory operations,
/// providing a high-level interface for remembering typed memory objects,
/// linking canonical relationships, retrieving continuity context, and
/// applying lifecycle corrections or suppression.
pub struct CharacterMemory {
    memory_composition: MemoryComposition,
}

struct MemoryComposition {
    graph_store: Box<dyn GraphAuthorityStore>,
    vector_store: Box<dyn VectorCandidateStore>,
    embedder: Box<dyn MemoryEmbedder>,
    stats_store: Box<dyn RetrievalStatsStore>,
    selectivity_policy: RetrievalSelectivityPolicy,
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
        self.provider.generate_embedding(&input.text).await
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let texts: Vec<&str> = inputs.iter().map(|input| input.text.as_str()).collect();
        self.provider.bulk_generate_embeddings(&texts).await
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
                stats_store: Box::new(
                    crate::internal::repositories::InMemoryRetrievalStatsStore::new(),
                ),
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
            return Err(CustomError::EmbeddingInitializationError(format!(
                "Embedding provider vector size ({provider_vector_size}) does not match configured embedding model vector size ({expected_vector_size})."
            )));
        }

        let vector_store = QdrantVectorCandidateStore::new(
            settings.get_qdrant_connection(),
            collection_name,
            expected_vector_size as u64,
        )?;
        vector_store.init_collection().await?;
        let graph_store = match settings.get_graph_store_mode() {
            ConfigGraphStoreMode::Service => Box::new(OxigraphHttpGraphAuthorityStore::new(
                settings.get_oxigraph_endpoint()?,
            )?) as Box<dyn GraphAuthorityStore>,
            ConfigGraphStoreMode::Persistent => Box::new(
                OxigraphGraphAuthorityStore::new_persistent(settings.get_oxigraph_path()?)?,
            ),
            ConfigGraphStoreMode::InMemory => {
                Box::new(OxigraphGraphAuthorityStore::new_in_memory()?)
            }
        };
        let stats_store = retrieval_stats_store(&settings)?;
        let selectivity_policy = RetrievalSelectivityPolicy::try_new(
            settings.get_selectivity_smoothing_alpha(),
            settings.get_selectivity_gamma(),
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

    /// Persists a remember draft through the graph-authoritative write pipeline.
    pub async fn remember(&self, draft: RememberDraft) -> Result<RememberOutcome, CustomError> {
        let parts = self.memory_composition();
        let pipeline = RememberPipeline::new_with_stats(
            parts.graph_store.as_ref(),
            parts.vector_store.as_ref(),
            parts.embedder.as_ref(),
            parts.stats_store.as_ref(),
        );
        let outcome = pipeline
            .remember(RememberPipelineDraft::new(
                draft.object_drafts,
                draft.link_drafts,
            ))
            .await?;
        Ok(outcome.into())
    }

    /// Persists a canonical typed relationship through the graph-authoritative link pipeline.
    pub async fn link(&self, draft: MemoryLinkDraft) -> Result<MemoryLink, CustomError> {
        let parts = self.memory_composition();
        LinkPipeline::new_with_stats(parts.graph_store.as_ref(), parts.stats_store.as_ref())
            .link(draft)
            .await
    }

    /// Assembles a graph-verified continuity context pack through injected retrieval parts.
    pub async fn retrieve(
        &self,
        context: RetrievalContext,
    ) -> Result<RetrieveOutcome, CustomError> {
        let parts = self.memory_composition();
        RetrievePipeline::new_with_stats(
            parts.graph_store.as_ref(),
            parts.vector_store.as_ref(),
            parts.embedder.as_ref(),
            parts.stats_store.as_ref(),
            parts.selectivity_policy,
        )
        .retrieve(context)
        .await
    }

    /// Applies a non-destructive lifecycle correction through injected graph/vector parts.
    pub async fn correct(
        &self,
        draft: CorrectMemoryDraft,
    ) -> Result<LifecycleMutationOutcome, CustomError> {
        let parts = self.memory_composition();
        CorrectionForgetPipeline::new_with_stats(
            parts.graph_store.as_ref(),
            parts.vector_store.as_ref(),
            parts.embedder.as_ref(),
            parts.stats_store.as_ref(),
        )
        .correct(draft)
        .await
    }

    /// Applies suppression/archive lifecycle mutation through injected graph/vector parts.
    pub async fn forget(
        &self,
        draft: ForgetMemoryDraft,
    ) -> Result<LifecycleMutationOutcome, CustomError> {
        let parts = self.memory_composition();
        CorrectionForgetPipeline::new_with_stats(
            parts.graph_store.as_ref(),
            parts.vector_store.as_ref(),
            parts.embedder.as_ref(),
            parts.stats_store.as_ref(),
        )
        .forget(draft)
        .await
    }

    fn memory_composition(&self) -> &MemoryComposition {
        &self.memory_composition
    }
}

fn retrieval_stats_store(settings: &Settings) -> Result<Box<dyn RetrievalStatsStore>, CustomError> {
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

impl From<crate::internal::repositories::RememberPipelineOutcome> for RememberOutcome {
    fn from(value: crate::internal::repositories::RememberPipelineOutcome) -> Self {
        Self {
            persisted_object_ids: value.persisted_object_ids,
            persisted_link_ids: value.persisted_link_ids,
            vector_indexed_object_ids: value.vector_indexed_object_ids,
            vector_indexing_failure: value.vector_indexing_failure.map(Into::into),
        }
    }
}

impl From<crate::internal::repositories::InternalVectorIndexingFailure> for VectorIndexingFailure {
    fn from(value: crate::internal::repositories::InternalVectorIndexingFailure) -> Self {
        Self {
            unindexed_object_ids: value.unindexed_object_ids,
            error_message: value.error_message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use secrecy::SecretString;
    use uuid::Uuid;

    use crate::api::types::{EntityDraft, EntityType, ObjectType, RelationType};
    use crate::internal::models::vector::{
        memory_object_vector_record, EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch,
        VectorRecordEmbedding, VectorSurface,
    };
    use crate::internal::repositories::test_support::{
        representative_fixtures, DeterministicMemoryEmbedder, FakeGraphAuthorityStore,
        FakeVectorCandidateStore,
    };

    #[tokio::test]
    async fn injected_facade_remembers_backend_free_drafts() {
        let memory = injected_memory();
        let entity_id = id("550e8400-e29b-41d4-a716-446655445001");
        let mut entity = EntityDraft::new(EntityType::User, "Kohta");
        entity.id = Some(entity_id);

        let outcome = memory
            .remember(RememberDraft::new([MemoryObjectDraft::Entity(entity)]))
            .await
            .expect("remember facade should persist through injected parts");

        assert_eq!(outcome.persisted_object_ids, vec![entity_id]);
        assert_eq!(outcome.persisted_link_ids, Vec::<MemoryId>::new());
        assert_eq!(outcome.vector_indexed_object_ids, vec![entity_id]);
        assert_eq!(outcome.vector_indexing_failure, None);
    }

    #[tokio::test]
    async fn injected_facade_links_canonical_relationships() {
        let memory = injected_memory();
        let from_id = id("550e8400-e29b-41d4-a716-446655445010");
        let to_id = id("550e8400-e29b-41d4-a716-446655445011");
        let mut draft = MemoryLinkDraft::new(
            ObjectType::Entity,
            from_id,
            RelationType::Mentions,
            ObjectType::Episode,
            to_id,
        );
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655445012"));

        let link = memory
            .link(draft)
            .await
            .expect("link facade should persist through injected graph store");

        assert_eq!(link.from_id, from_id);
        assert_eq!(link.to_id, to_id);
        assert_eq!(link.relation, RelationType::Mentions);
    }

    #[tokio::test]
    async fn injected_facade_retrieves_with_graph_vector_and_embedder_parts() {
        let (memory, fixtures) = retrieval_memory().await;

        let outcome = memory
            .retrieve(RetrievalContext::new("deterministic preferences").with_trace())
            .await
            .expect("retrieve facade should assemble through injected parts");

        assert_eq!(outcome.pack.preferences.len(), 1);
        assert_eq!(
            outcome.pack.preferences[0].memory.id,
            fixtures.user_preference.id
        );
        assert_eq!(outcome.rationale.vector_candidate_count, 1);
        assert_eq!(outcome.rationale.graph_verified_count, 1);
        assert_eq!(outcome.trace.as_ref().unwrap().vector_candidates.len(), 1);
    }

    #[tokio::test]
    async fn injected_facade_corrects_derived_memory_and_retrieval_excludes_superseded_memory() {
        let (memory, fixtures, replacement_id) = lifecycle_memory().await;

        let outcome = memory
            .correct(derived_correction_draft(
                &fixtures,
                replacement_id,
                fixtures.user_preference.id,
            ))
            .await
            .expect("correct facade should use injected lifecycle pipeline");

        assert!(outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                fixtures.user_preference.id,
            )));
        assert!(outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                replacement_id,
            )));
        assert!(outcome
            .trace
            .as_ref()
            .unwrap()
            .superseded_by
            .iter()
            .any(|evidence| {
                evidence.superseded_memory_id == fixtures.user_preference.id
                    && evidence.superseded_by_memory_id == replacement_id
            }));

        let normal = memory
            .retrieve(RetrievalContext::new("corrected deterministic preference").with_trace())
            .await
            .expect("normal retrieval should use graph lifecycle filters");
        assert!(pack_contains_derived_memory(&normal.pack, replacement_id));
        assert!(!pack_contains_derived_memory(
            &normal.pack,
            fixtures.user_preference.id,
        ));
        assert!(normal
            .trace
            .as_ref()
            .unwrap()
            .lifecycle_filter_decisions
            .iter()
            .any(|decision| {
                decision.object.id == fixtures.user_preference.id
                    && decision.action == LifecycleFilterAction::Omitted
            }));

        let mut historical =
            RetrievalContext::new("corrected deterministic preference").with_trace();
        historical.lifecycle_policy.include_suppressed = true;
        historical.lifecycle_policy.include_non_current = true;
        historical.lifecycle_policy.include_superseded = true;
        let historical = memory
            .retrieve(historical)
            .await
            .expect("historical opt-in should keep lifecycle state inspectable");
        assert!(pack_contains_derived_memory(
            &historical.pack,
            fixtures.user_preference.id,
        ));
        assert!(historical
            .trace
            .as_ref()
            .unwrap()
            .lifecycle_filter_decisions
            .iter()
            .any(|decision| {
                decision.object.id == fixtures.user_preference.id
                    && decision.superseded_by.contains(&replacement_id)
                    && decision.action == LifecycleFilterAction::Included
            }));
    }

    #[tokio::test]
    async fn injected_facade_corrects_episode_and_observation_provenanced_derived_memories() {
        let (memory, fixtures, episode_replacement_id) =
            lifecycle_memory_with_replacement(id("550e8400-e29b-41d4-a716-446655449200")).await;

        let episode_outcome = memory
            .correct(source_object_correction_draft(
                &fixtures,
                SourceObjectCorrectionTarget::Episode {
                    id: fixtures.episode.id,
                    original_raw_ref: fixtures.episode.raw_ref.clone(),
                    original_source_ref: fixtures.episode.source_conversation_id.clone(),
                },
                episode_replacement_id,
            ))
            .await
            .expect("episode correction should supersede affected derived memories");

        assert!(episode_outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                fixtures.user_preference.id,
            )));
        assert!(!episode_outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::Episode,
                fixtures.episode.id
            )));
        let episode_retrieval = memory
            .retrieve(RetrievalContext::new("episode corrected preference"))
            .await
            .unwrap();
        assert!(pack_contains_derived_memory(
            &episode_retrieval.pack,
            episode_replacement_id,
        ));
        assert!(!pack_contains_derived_memory(
            &episode_retrieval.pack,
            fixtures.user_preference.id,
        ));

        let (memory, fixtures, observation_replacement_id) =
            lifecycle_memory_with_replacement(id("550e8400-e29b-41d4-a716-446655449201")).await;
        let observation_outcome = memory
            .correct(source_object_correction_draft(
                &fixtures,
                SourceObjectCorrectionTarget::Observation {
                    id: fixtures.salient_observation.id,
                    original_raw_ref: fixtures.salient_observation.raw_ref.clone(),
                    original_source_ref: fixtures.episode.source_conversation_id.clone(),
                },
                observation_replacement_id,
            ))
            .await
            .expect("observation correction should supersede affected derived memories");

        assert!(observation_outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                fixtures.user_preference.id,
            )));
        assert!(!observation_outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::Observation,
                fixtures.salient_observation.id,
            )));
        let observation_retrieval = memory
            .retrieve(RetrievalContext::new("observation corrected preference"))
            .await
            .unwrap();
        assert!(pack_contains_derived_memory(
            &observation_retrieval.pack,
            observation_replacement_id,
        ));
        assert!(!pack_contains_derived_memory(
            &observation_retrieval.pack,
            fixtures.user_preference.id,
        ));
    }

    #[tokio::test]
    async fn injected_facade_forgets_derived_memory_and_source_objects_without_deletion() {
        let (memory, fixtures, _) = lifecycle_memory().await;

        let derived_outcome = memory
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::derived_memory(fixtures.user_preference.id),
                    "Suppress stale derived preference.",
                )
                .with_trace(),
            )
            .await
            .expect("derived forget should use injected lifecycle pipeline");
        assert_eq!(
            derived_outcome.graph_mutated_object_ids,
            vec![MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                fixtures.user_preference.id,
            )]
        );
        let normal = memory
            .retrieve(RetrievalContext::new("deterministic local fakes"))
            .await
            .unwrap();
        assert!(!pack_contains_derived_memory(
            &normal.pack,
            fixtures.user_preference.id,
        ));

        let (memory, fixtures, _) = lifecycle_memory().await;
        let source_outcome = memory
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::episode(fixtures.episode.id),
                    "Suppress source episode and dependent derived memories.",
                )
                .with_trace(),
            )
            .await
            .expect("source forget should cascade to provenanced derived memories");
        assert!(source_outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::Episode,
                fixtures.episode.id,
            )));
        assert!(source_outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                fixtures.user_preference.id,
            )));

        let source_retrieval = memory
            .retrieve(RetrievalContext::new("deterministic local fakes").with_trace())
            .await
            .unwrap();
        assert!(!source_retrieval
            .pack
            .relevant_episodes
            .iter()
            .any(|episode| episode.id == fixtures.episode.id));
        assert!(!pack_contains_derived_memory(
            &source_retrieval.pack,
            fixtures.user_preference.id,
        ));
    }

    #[tokio::test]
    async fn injected_facade_archives_memory_thread() {
        let (memory, fixtures, _) = lifecycle_memory().await;

        let outcome = memory
            .forget(
                ForgetMemoryDraft::archive_thread(fixtures.soft_thread.id, "Archive soft thread.")
                    .with_trace(),
            )
            .await
            .expect("thread forget should archive through injected lifecycle pipeline");

        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![MemoryObjectRef::new(
                ObjectType::MemoryThread,
                fixtures.soft_thread.id,
            )]
        );
        let normal = memory
            .retrieve(RetrievalContext::new("contract test support"))
            .await
            .unwrap();
        assert!(!normal
            .pack
            .active_threads
            .iter()
            .any(|thread| thread.id == fixtures.soft_thread.id));
    }

    #[tokio::test]
    async fn constructor_rejects_embedding_provider_vector_size_mismatch_before_storage_init() {
        let settings = Settings::new_for_tests(
            SecretString::new("not-a-qdrant-url".into()),
            SecretString::new("memory://local".into()),
            SecretString::new("dummy-key".into()),
            SecretString::new("text-embedding-3-small".into()),
        );

        let error = match CharacterMemory::new_with_embedding_provider(
            settings,
            "mismatched_provider_vectors".to_owned(),
            Box::new(FixedEmbeddingProvider::new(8)),
        )
        .await
        {
            Ok(_) => panic!("constructor should reject mismatched provider vector size"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            CustomError::EmbeddingInitializationError(message)
                if message.contains("8") && message.contains("1536")
        ));
    }

    #[tokio::test]
    async fn sqlite_stats_open_failure_uses_configured_conservative_fallback() {
        let settings = Settings::new(
            ::config::Config::builder()
                .set_override("qdrant_connection_string", "external_qdrant")
                .unwrap()
                .set_override("oxigraph_connection_string", "external_oxigraph")
                .unwrap()
                .set_override("openai_api_key", "external_openai")
                .unwrap()
                .set_override("embedding_model", "TextEmbedding3Small")
                .unwrap()
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

        assert_eq!(
            health.state,
            crate::internal::repositories::RetrievalStatsHealthState::Unhealthy
        );
        assert!(health
            .last_error_message
            .as_deref()
            .unwrap_or_default()
            .contains("using in-memory fallback"));
    }

    fn injected_memory() -> CharacterMemory {
        CharacterMemory::from_parts(
            Box::new(FakeGraphAuthorityStore::new()),
            Box::new(FakeVectorCandidateStore::new()),
            Box::new(DeterministicMemoryEmbedder::new(8)),
        )
    }

    async fn retrieval_memory() -> (
        CharacterMemory,
        crate::internal::repositories::test_support::RepresentativeFixtures,
    ) {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FixedVectorCandidateStore::new(vec![VectorCandidateMatch::new(
            fixtures.user_preference.id,
            ObjectType::DerivedMemory,
            VectorSurface::DerivedText,
            0.95,
        )]);
        let memory = CharacterMemory::from_parts(
            Box::new(graph),
            Box::new(vector),
            Box::new(FixedMemoryEmbedder::new(vec![1.0, 0.0])),
        );

        (memory, fixtures)
    }

    async fn lifecycle_memory() -> (
        CharacterMemory,
        crate::internal::repositories::test_support::RepresentativeFixtures,
        MemoryId,
    ) {
        lifecycle_memory_with_replacement(id("550e8400-e29b-41d4-a716-446655449100")).await
    }

    async fn lifecycle_memory_with_replacement(
        replacement_id: MemoryId,
    ) -> (
        CharacterMemory,
        crate::internal::repositories::test_support::RepresentativeFixtures,
        MemoryId,
    ) {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
        let objects = [
            MemoryObject::Episode(fixtures.episode.clone()),
            MemoryObject::Observation(fixtures.salient_observation.clone()),
            MemoryObject::MemoryThread(fixtures.soft_thread.clone()),
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
        ];
        for object in objects {
            let record = memory_object_vector_record(&object).unwrap();
            vector
                .upsert_vector_records(&[VectorRecordEmbedding::new(
                    &record,
                    &[1.0, 0.0, 0.0, 0.0],
                )])
                .await
                .unwrap();
        }

        let memory = CharacterMemory::from_parts(
            Box::new(graph),
            Box::new(vector),
            Box::new(DeterministicMemoryEmbedder::new(4)),
        );

        (memory, fixtures, replacement_id)
    }

    fn derived_correction_draft(
        fixtures: &crate::internal::repositories::test_support::RepresentativeFixtures,
        replacement_id: MemoryId,
        superseded_id: MemoryId,
    ) -> CorrectMemoryDraft {
        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "Prefer corrected deterministic lifecycle facade coverage.",
        )
        .with_source_episode(fixtures.episode.id)
        .with_source_observation(fixtures.salient_observation.id)
        .with_superseded_memory(superseded_id);
        replacement.id = Some(replacement_id);
        replacement.original_source_provenance =
            SourceProvenanceReference::episode(fixtures.episode.id)
                .with_external_ref(ExternalSourceReference::raw("raw://original/preference"));
        replacement.correction_origin_provenance =
            SourceProvenanceReference::observation(fixtures.salient_observation.id)
                .with_external_ref(ExternalSourceReference::raw("raw://correction/facade"));

        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::derived_memory(superseded_id),
            "Correct stale derived memory through injected facade.",
        )
        .with_replacement(replacement)
        .with_superseded_derived_memory(superseded_id)
        .with_trace();
        draft.correction_origin =
            SourceProvenanceReference::observation(fixtures.salient_observation.id)
                .with_external_ref(ExternalSourceReference::raw("raw://correction/facade"));
        draft
    }

    fn source_object_correction_draft(
        fixtures: &crate::internal::repositories::test_support::RepresentativeFixtures,
        target: SourceObjectCorrectionTarget,
        replacement_id: MemoryId,
    ) -> CorrectMemoryDraft {
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(target),
            "Correct source-provenanced derived behavior through injected facade.",
        )
        .with_replacement(derived_correction_replacement(fixtures, replacement_id))
        .with_trace();
        draft.correction_origin =
            SourceProvenanceReference::observation(fixtures.salient_observation.id)
                .with_external_ref(ExternalSourceReference::raw(
                    "raw://correction/source-object",
                ));
        draft
    }

    fn derived_correction_replacement(
        fixtures: &crate::internal::repositories::test_support::RepresentativeFixtures,
        replacement_id: MemoryId,
    ) -> ReplacementDerivedMemoryDraft {
        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "Correct source-provenanced deterministic facade behavior.",
        )
        .with_source_episode(fixtures.episode.id)
        .with_source_observation(fixtures.salient_observation.id);
        replacement.id = Some(replacement_id);
        replacement.original_source_provenance =
            SourceProvenanceReference::episode(fixtures.episode.id)
                .with_external_ref(ExternalSourceReference::raw("raw://original/source-object"));
        replacement.correction_origin_provenance =
            SourceProvenanceReference::observation(fixtures.salient_observation.id)
                .with_external_ref(ExternalSourceReference::raw(
                    "raw://correction/source-object",
                ));
        replacement
    }

    fn pack_contains_derived_memory(pack: &ContinuityContextPack, memory_id: MemoryId) -> bool {
        pack.derived_memories
            .iter()
            .chain(pack.preferences.iter())
            .chain(pack.relationship_notes.iter())
            .chain(pack.open_loops.iter())
            .chain(pack.commitments.iter())
            .chain(pack.character_signals.iter())
            .any(|included| included.memory.id == memory_id)
    }

    #[derive(Debug)]
    struct FixedEmbeddingProvider {
        vector_size: usize,
    }

    impl FixedEmbeddingProvider {
        fn new(vector_size: usize) -> Self {
            Self { vector_size }
        }
    }

    #[async_trait]
    impl EmbeddingProvider for FixedEmbeddingProvider {
        fn vector_size(&self) -> usize {
            self.vector_size
        }

        async fn generate_embedding<'a>(&self, _text: &'a str) -> Result<Vec<f32>, CustomError> {
            Ok(vec![0.0; self.vector_size])
        }

        async fn bulk_generate_embeddings<'a>(
            &self,
            texts: &'a [&'a str],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            Ok(vec![vec![0.0; self.vector_size]; texts.len()])
        }
    }

    #[derive(Debug)]
    struct FixedMemoryEmbedder {
        embedding: Vec<f32>,
    }

    impl FixedMemoryEmbedder {
        fn new(embedding: Vec<f32>) -> Self {
            Self { embedding }
        }
    }

    #[async_trait]
    impl MemoryEmbedder for FixedMemoryEmbedder {
        async fn embed(&self, _input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
            Ok(self.embedding.clone())
        }

        async fn embed_batch(
            &self,
            inputs: &[EmbeddingInput],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            Ok(vec![self.embedding.clone(); inputs.len()])
        }
    }

    #[derive(Debug)]
    struct FixedVectorCandidateStore {
        candidates: Vec<VectorCandidateMatch>,
    }

    impl FixedVectorCandidateStore {
        fn new(candidates: Vec<VectorCandidateMatch>) -> Self {
            Self { candidates }
        }
    }

    #[async_trait]
    impl VectorCandidateStore for FixedVectorCandidateStore {
        async fn upsert_vector_records(
            &self,
            _records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn search_candidates(
            &self,
            query: &VectorCandidateSearch,
        ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
            let mut candidates = self.candidates.clone();
            candidates.truncate(query.limit);
            Ok(candidates)
        }

        async fn list_candidate_diagnostics(
            &self,
        ) -> Result<
            Vec<crate::internal::models::vector::VectorCandidateDiagnosticRecord>,
            CustomError,
        > {
            Ok(Vec::new())
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            Ok(())
        }
    }

    fn id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }
}
