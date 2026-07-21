use crate::api::types::{
    CommitOptions, CorrectMemoryDraft, ForgetMemoryDraft, LifecycleMutationOutcome,
    MemoryLinkDraft, PrepareOptions, RememberInput, RememberOptions, RememberOutcome,
    RememberWritePlan, RetrievalContext, RetrieveOutcome,
};
use crate::composition::MemoryComposition;
use crate::domain::{CandidateValidation, MemoryLink};
use crate::errors::CustomError;
use crate::usecases::{
    CorrectionForgetPipeline, LinkPipeline, RememberPipeline, RetrievePipeline, WritePlanValidator,
};

/// CharacterMemory provides a high-level API for memory operations.
///
/// # Description
///
/// This struct serves as the main entry point for memory operations,
/// providing a high-level interface for remembering typed memory objects,
/// linking canonical relationships, retrieving continuity context, and
/// applying lifecycle corrections or suppression.
pub struct CharacterMemory {
    pub(crate) memory_composition: MemoryComposition,
}

impl CharacterMemory {
    /// Prepares a remember write plan without persisting graph, vector, or stats data.
    ///
    /// The default facade path uses fresh operation defaults, so repeated calls with
    /// the same input produce distinct plan/object identifiers. Use the lower-level
    /// write-plan helper APIs with fixed defaults when byte-for-byte deterministic
    /// planning is required.
    pub async fn prepare(
        &self,
        input: RememberInput,
        options: PrepareOptions,
    ) -> Result<RememberWritePlan, CustomError> {
        let defaults = crate::usecases::write_planning::RememberPlanDefaults::generated();
        let mut plan = input.prepare_write_plan_with_options(
            &defaults,
            options.include_vector_index_candidates,
            options.include_stats_update_candidates,
        );
        if let Some(idempotency_key) = options.idempotency_key {
            plan.idempotency_key = idempotency_key;
        }
        Ok(plan)
    }

    /// Validates a remember write plan against current graph state without persisting anything.
    pub async fn validate_plan(
        &self,
        plan: &RememberWritePlan,
    ) -> Result<Vec<CandidateValidation>, CustomError> {
        let parts = self.memory_composition();
        Ok(WritePlanValidator::new(parts.graph_store.as_ref())
            .validate(plan)
            .await?
            .validations)
    }

    /// Commits a remember write plan after revalidating it against current graph state.
    ///
    /// Graph-authoritative writes are critical and fail the operation. Vector indexing and
    /// retrieval-stats updates are repairable and are reported in the returned outcome.
    pub async fn commit(
        &self,
        plan: RememberWritePlan,
        options: CommitOptions,
    ) -> Result<RememberOutcome, CustomError> {
        let parts = self.memory_composition();
        let pipeline = RememberPipeline::new_with_stats(
            parts.graph_store.as_ref(),
            parts.vector_store.as_ref(),
            parts.embedder.as_ref(),
            parts.stats_store.as_ref(),
        );
        let outcome = pipeline.commit(plan, options).await?;
        Ok(outcome.into())
    }

    /// Prepares, validates, and commits a remember input through the canonical write-plan path.
    pub async fn remember(
        &self,
        input: RememberInput,
        options: RememberOptions,
    ) -> Result<RememberOutcome, CustomError> {
        let plan = self.prepare(input, options.prepare).await?;
        self.validate_plan(&plan).await?;
        self.commit(plan, options.commit).await
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
        draft.validate()?;
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
        draft.validate()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::embedding::EmbeddingProvider;
    use crate::composition::retrieval_stats_store;
    use crate::config::Settings;
    use crate::ports::embedder::MemoryEmbedder;
    use crate::ports::graph_authority::GraphAuthorityStore;
    use crate::ports::vector_candidate::VectorCandidateStore;
    use crate::*;
    use async_trait::async_trait;
    use secrecy::SecretString;
    use uuid::Uuid;

    use crate::api::types::{EntityDraft, MemoryLinkDraft, PrepareOptions};
    use crate::domain::{EntityType, ObjectType, RelationType};
    use crate::models::vector::{
        EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch, VectorRecordEmbedding,
        VectorSurface,
    };
    use crate::policy::memory_object_vector_record;
    use crate::test_support::{
        representative_fixtures, DeterministicMemoryEmbedder, FakeGraphAuthorityStore,
        FakeVectorCandidateStore,
    };

    #[tokio::test]
    async fn injected_facade_remembers_through_the_write_plan_path() {
        let memory = injected_memory();
        let entity_id = id("550e8400-e29b-41d4-a716-446655445001");
        let mut entity = EntityDraft::new(EntityType::User, "Kohta");
        entity.id = Some(entity_id);

        let outcome = memory
            .remember(
                RememberInput::new("Kohta").with_entity(entity),
                RememberOptions::default(),
            )
            .await
            .expect("remember facade should persist through injected parts");

        assert!(outcome.persisted_object_ids.contains(&entity_id));
        assert_eq!(outcome.persisted_link_ids, Vec::<MemoryId>::new());
        assert!(outcome.vector_indexed_object_ids.contains(&entity_id));
        assert_eq!(outcome.vector_indexing_failure, None);
    }

    #[tokio::test]
    async fn remember_surfaces_write_plan_validation_warnings() {
        let memory = injected_memory();
        let episode_id = id("550e8400-e29b-41d4-a716-446655445011");
        let mut episode = EpisodeDraft::new("echoed source content");
        episode.id = Some(episode_id);
        let observation = ObservationDraft::new(episode_id, "echoed source content");

        let outcome = memory
            .remember(
                RememberInput::new("echoed source content")
                    .with_episode(episode)
                    .with_observation(observation),
                RememberOptions::default(),
            )
            .await
            .expect("remember should accept warning-bearing write plans");

        let validation = outcome
            .diagnostics
            .validations
            .iter()
            .find(|validation| {
                validation.candidate_index == 1
                    && validation.candidate_kind == MemoryCandidateKind::Observation
            })
            .expect("remember outcome should preserve the warning-bearing validation");
        assert_eq!(validation.status, CandidateValidationStatus::Valid);
        assert!(validation.errors.is_empty());
        assert_eq!(
            validation.warnings,
            vec![CandidateValidationIssue::DuplicateObservationEcho {
                echo_surface: "echoed source content".to_owned(),
                matching_episode_ids: vec![episode_id],
            }]
        );

        let messages = outcome
            .diagnostics
            .messages
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == RememberDiagnosticCode::WritePlanValidationWarning
            })
            .collect::<Vec<_>>();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].severity, DiagnosticSeverity::Warning);
    }

    #[tokio::test]
    async fn prepare_and_validate_plan_do_not_persist() {
        let memory = injected_memory();

        let plan = memory
            .prepare(
                RememberInput::new("prepare only"),
                PrepareOptions::default(),
            )
            .await
            .unwrap();
        let validations = memory.validate_plan(&plan).await.unwrap();

        assert!(validations
            .iter()
            .all(|validation| validation.status == CandidateValidationStatus::Valid));
        let graph = memory.memory_composition.graph_store.as_ref();
        assert_eq!(graph.list_diagnostic_objects().await.unwrap().len(), 0);
        assert_eq!(graph.list_diagnostic_links().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn commit_revalidates_against_current_graph_state() {
        let memory = injected_memory();
        let missing_entity_id = id("550e8400-e29b-41d4-a716-446655445021");
        let plan = memory
            .prepare(
                RememberInput::new("commit revalidation").with_entity_id(missing_entity_id),
                PrepareOptions::default(),
            )
            .await
            .unwrap();

        let error = memory
            .commit(plan, CommitOptions::default())
            .await
            .expect_err("missing link target should reject during commit revalidation");

        assert_validation_rejection_has_unknown_ref(
            error,
            MemoryCandidateKind::MemoryLink,
            MemoryObjectRef::new(ObjectType::Entity, missing_entity_id),
        );
    }

    #[tokio::test]
    async fn remember_returns_structured_validation_rejection() {
        let memory = injected_memory();
        let missing_entity_id = id("550e8400-e29b-41d4-a716-446655445022");

        let error = memory
            .remember(
                RememberInput::new("remember rejection").with_entity_id(missing_entity_id),
                RememberOptions::default(),
            )
            .await
            .expect_err("remember should return the structured commit rejection");

        assert_validation_rejection_has_unknown_ref(
            error,
            MemoryCandidateKind::MemoryLink,
            MemoryObjectRef::new(ObjectType::Entity, missing_entity_id),
        );
    }

    #[tokio::test]
    async fn commit_retry_is_idempotent_and_rejects_divergent_content() {
        let memory = injected_memory();
        let mut plan = memory
            .prepare(
                RememberInput::new("same content"),
                PrepareOptions::default(),
            )
            .await
            .unwrap();

        memory
            .commit(plan.clone(), CommitOptions::default())
            .await
            .unwrap();
        memory
            .commit(plan.clone(), CommitOptions::default())
            .await
            .expect("exact retry should be accepted");

        if let MemoryCandidate::Episode(candidate) = &mut plan.candidates[0] {
            candidate.draft.summary = "divergent content".to_owned();
        }
        let error = memory
            .commit(plan, CommitOptions::default())
            .await
            .expect_err("same deterministic IDs with different content should reject");

        assert!(error.to_string().contains("deterministic ID collided"));
    }

    #[tokio::test]
    async fn retry_after_vector_failure_does_not_duplicate_graph_writes() {
        let memory = CharacterMemory::from_parts(
            Box::new(FakeGraphAuthorityStore::new()),
            Box::new(FailingVectorCandidateStore),
            Box::new(DeterministicMemoryEmbedder::new(8)),
        );
        let plan = memory
            .prepare(
                RememberInput::new("repairable vector failure"),
                PrepareOptions::default(),
            )
            .await
            .unwrap();

        let first = memory
            .commit(plan.clone(), CommitOptions::default())
            .await
            .unwrap();
        let second = memory.commit(plan, CommitOptions::default()).await.unwrap();

        assert!(first.vector_indexing_failure.is_some());
        assert!(second.vector_indexing_failure.is_some());
        assert_eq!(
            first
                .repair_needed
                .iter()
                .filter(|marker| matches!(marker, RepairMarker::VectorIndex { .. }))
                .count(),
            1
        );
        let graph = memory.memory_composition.graph_store.as_ref();
        assert_eq!(graph.list_diagnostic_objects().await.unwrap().len(), 2);
        assert_eq!(graph.list_diagnostic_links().await.unwrap().len(), 0);
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
            CustomError::Embedding(EmbeddingError::ProviderVectorSizeMismatch {
                expected: 1536,
                actual: 8,
            })
        ));
    }

    #[tokio::test]
    async fn constructor_rejects_persistent_endpoint_url_before_qdrant_contact() {
        let settings = Settings::new(
            ::config::Config::builder()
                .set_override("qdrant_connection_string", "http://127.0.0.1:1")
                .unwrap()
                .set_override("oxigraph_path", "http://127.0.0.1:7878")
                .unwrap()
                .set_override("openai_api_key", "dummy-key")
                .unwrap()
                .set_override("embedding_model", "text-embedding-3-small")
                .unwrap()
                .set_override("graph_store_mode", "persistent")
                .unwrap()
                .set_override("retrieval_stats_store_mode", "in_memory")
                .unwrap()
                .build()
                .unwrap(),
        )
        .unwrap();
        let vector_size = settings.get_embedding_vector_size().unwrap();

        let error = match CharacterMemory::new_with_embedding_provider(
            settings,
            "graph_config_fails_before_qdrant".to_owned(),
            Box::new(FixedEmbeddingProvider::new(vector_size)),
        )
        .await
        {
            Ok(_) => panic!("constructor should reject the removed service endpoint"),
            Err(error) => error,
        };
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
                .set_override("qdrant_connection_string", "external_qdrant")
                .unwrap()
                .set_override("oxigraph_path", "external_oxigraph")
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
            crate::ports::retrieval_stats::RetrievalStatsHealthState::Unhealthy
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

    async fn retrieval_memory() -> (CharacterMemory, crate::test_support::RepresentativeFixtures) {
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
        crate::test_support::RepresentativeFixtures,
        MemoryId,
    ) {
        lifecycle_memory_with_replacement(id("550e8400-e29b-41d4-a716-446655449100")).await
    }

    async fn lifecycle_memory_with_replacement(
        replacement_id: MemoryId,
    ) -> (
        CharacterMemory,
        crate::test_support::RepresentativeFixtures,
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
        fixtures: &crate::test_support::RepresentativeFixtures,
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
        fixtures: &crate::test_support::RepresentativeFixtures,
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
        fixtures: &crate::test_support::RepresentativeFixtures,
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
        ) -> Result<Vec<crate::models::vector::VectorCandidateDiagnosticRecord>, CustomError>
        {
            Ok(Vec::new())
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FailingVectorCandidateStore;

    #[async_trait]
    impl VectorCandidateStore for FailingVectorCandidateStore {
        async fn upsert_vector_records(
            &self,
            _records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            Err(CustomError::VectorDatabaseError(VectorDatabaseError::new(
                "test",
                VectorDatabaseErrorKind::Response,
                None,
                "vector store unavailable",
            )))
        }

        async fn search_candidates(
            &self,
            _query: &VectorCandidateSearch,
        ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
            Ok(Vec::new())
        }

        async fn list_candidate_diagnostics(
            &self,
        ) -> Result<Vec<crate::models::vector::VectorCandidateDiagnosticRecord>, CustomError>
        {
            Ok(Vec::new())
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            Ok(())
        }
    }

    fn assert_validation_rejection_has_unknown_ref(
        error: CustomError,
        expected_kind: MemoryCandidateKind,
        expected_ref: MemoryObjectRef,
    ) {
        let CustomError::WritePlanValidationRejected { validations } = error else {
            panic!("expected structured write-plan validation rejection, got {error:?}");
        };
        assert!(
            validations.iter().any(|validation| {
                validation.candidate_kind == expected_kind
                    && validation.status == CandidateValidationStatus::Invalid
                    && validation.errors.iter().any(|error| {
                        matches!(
                            error,
                            CandidateValidationIssue::UnknownObjectRef { referenced, .. }
                                if *referenced == expected_ref
                        )
                    })
            }),
            "expected invalid {expected_kind:?} validation for {expected_ref:?}, got {validations:?}"
        );
    }

    fn id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }
}
