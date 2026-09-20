use crate::api::types::{
    CommitOptions, CorrectMemoryDraft, ForgetMemoryDraft, LifecycleMutationOutcome, LinkOutcome,
    MemoryLinkDraft, PrepareOptions, RememberInput, RememberOptions, RememberOutcome,
    RememberWritePlan, RetrievalContext, RetrieveOutcome,
};
use crate::composition::MemoryComposition;
use crate::domain::CandidateValidation;
use crate::errors::CustomError;
use crate::usecases::{
    CorrectionForgetPipeline, LinkPipeline, RememberPipeline, RetrievePipeline, WritePlanValidator,
};

#[cfg(test)]
mod notion_tests;
#[cfg(test)]
mod scene_tests;
#[cfg(test)]
mod write_turn_tests;

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
    // ponytail: one turn per memory value; revisit only if write throughput requires it.
    pub(crate) write_turn: tokio::sync::Mutex<()>,
}

impl CharacterMemory {
    /// Releases the local stores so their directories can be removed or reopened
    /// deterministically. Service vector storage requires no shutdown.
    ///
    /// This is not required for durability: acknowledged writes are already durable.
    /// Dropping without calling `close` remains valid and does not wait for the
    /// embedded vector owner's shutdown. Await this method before deleting stores.
    pub async fn close(self) -> Result<(), CustomError> {
        let result = self.memory_composition.vector_store.close().await;
        drop(self);
        result
    }

    /// Prepares a remember write plan without persisting graph, vector, or stats data.
    ///
    /// The default facade path uses fresh operation defaults, so identifiers generated
    /// during preparation differ across calls. Caller-supplied draft identifiers are
    /// preserved. Use the lower-level write-plan helper APIs with fixed defaults when
    /// byte-for-byte deterministic planning is required.
    pub async fn prepare(
        &self,
        input: RememberInput,
        options: PrepareOptions,
    ) -> Result<RememberWritePlan, CustomError> {
        let defaults = crate::usecases::write_planning::RememberPlanDefaults::generated();
        let plan = input.prepare_write_plan_with_options(
            &defaults,
            options.include_vector_index_candidates,
            options.include_stats_update_candidates,
        );
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
    /// Objects and links derived from memory supersedes lists are written in one graph batch.
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
        pipeline.commit(plan, options, &self.write_turn).await
    }

    /// Prepares, validates, and commits a remember input through the canonical write-plan path.
    pub async fn remember(
        &self,
        input: RememberInput,
        options: RememberOptions,
    ) -> Result<RememberOutcome, CustomError> {
        let plan = self.prepare(input, options.prepare).await?;
        self.commit(plan, options.commit).await
    }

    /// Persists a canonical typed relationship and reports its repairable stats projection.
    /// Existing IDs accept identical content only; an omitted creation time retains the stored time.
    pub async fn link(&self, draft: MemoryLinkDraft) -> Result<LinkOutcome, CustomError> {
        let _turn = self.write_turn.lock().await;
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

    /// Appends replacement memories and their derived Supersedes links without rewriting predecessors.
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
        .correct(draft, &self.write_turn)
        .await
    }

    /// Applies suppression with an optional thread-member cascade.
    ///
    /// Forgetting a thread preserves its status and vector.
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
        .forget(draft, &self.write_turn)
        .await
    }

    fn memory_composition(&self) -> &MemoryComposition {
        &self.memory_composition
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::embedder::MemoryEmbedder;
    use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectQuery};
    use crate::ports::vector_candidate::{VectorCandidateRecall, VectorCandidateStore};
    use crate::*;
    use async_trait::async_trait;
    use uuid::Uuid;

    use crate::api::types::{EntityDraft, MemoryLinkDraft, PrepareOptions};
    use crate::domain::{ObjectType, RelationType};
    use crate::models::vector::{
        CanonicalCandidates, EmbeddingInput, VectorCandidateSearch, VectorRecordEmbedding,
    };
    use crate::policy::memory_object_vector_record;
    use crate::test_support::{
        in_memory_graph_store, representative_fixtures, DeterministicMemoryEmbedder,
        TemporaryVectorCandidateStore,
    };

    #[tokio::test]
    async fn injected_facade_remembers_through_the_write_plan_path() {
        let memory = injected_memory().await;
        let entity_id = id("550e8400-e29b-41d4-a716-446655445001");
        let mut entity = EntityDraft::new();
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
        assert!(!outcome.vector_indexed_object_ids.contains(&entity_id));
        assert_eq!(outcome.vector_indexing_failure, None);
    }

    #[tokio::test]
    async fn remember_surfaces_write_plan_validation_warnings() {
        let memory = injected_memory().await;
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
    async fn prepare_generates_fresh_candidate_ids() {
        let memory = injected_memory().await;
        let input = RememberInput::new("fresh candidate ids");
        let options = PrepareOptions::default();

        let first = memory
            .prepare(input.clone(), options.clone())
            .await
            .unwrap();
        let second = memory.prepare(input, options).await.unwrap();
        memory.close().await.unwrap();

        assert!(matches!(
            (&first.candidates[0], &second.candidates[0]),
            (MemoryCandidate::Episode(first), MemoryCandidate::Episode(second))
                if first.draft.id.is_some() && second.draft.id.is_some()
                    && first.draft.id != second.draft.id
        ));
    }

    #[tokio::test]
    async fn prepare_and_validate_plan_do_not_persist() {
        let memory = injected_memory().await;

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
        let objects = graph
            .query_objects(&GraphObjectQuery::by_types(
                vec![
                    ObjectType::Episode,
                    ObjectType::Observation,
                    ObjectType::Entity,
                    ObjectType::MemoryThread,
                    ObjectType::DerivedMemory,
                ],
                None,
            ))
            .await
            .unwrap();
        assert!(objects.is_empty());
    }

    #[tokio::test]
    async fn commit_revalidates_against_current_graph_state() {
        let memory = injected_memory().await;
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
        let memory = injected_memory().await;
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
        let memory = injected_memory().await;
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

        assert!(matches!(
            error,
            CustomError::DeterministicIdCollision { object }
                if object.object_type == ObjectType::Episode
        ));
    }

    #[tokio::test]
    async fn supersession_requires_an_already_stored_predecessor() {
        let memory = injected_memory().await;
        let episode_id = MemoryId::from_u128(1100);
        let predecessor_id = MemoryId::from_u128(1101);
        let mut episode = EpisodeDraft::new("Supersession source.");
        episode.id = Some(episode_id);
        let mut predecessor =
            DerivedMemoryDraft::new(DerivedType::UserPreference, "Old preference.")
                .with_source_episode(episode_id);
        predecessor.id = Some(predecessor_id);
        let mut successor = DerivedMemoryDraft::new(DerivedType::UserPreference, "New preference.")
            .with_source_episode(episode_id);
        successor.supersedes = vec![predecessor_id];
        let plan = memory
            .prepare(
                RememberInput::new("Supersession source.")
                    .with_episode(episode)
                    .with_derived_memory(predecessor)
                    .with_derived_memory(successor),
                PrepareOptions::default(),
            )
            .await
            .unwrap();
        let expected = CandidateValidationIssue::UnknownObjectRef {
            role: CandidateReferenceRole::SupersededMemory,
            referenced: MemoryObjectRef::new(ObjectType::DerivedMemory, predecessor_id),
        };
        assert!(
            matches!(memory.commit(plan, CommitOptions::default()).await,
            Err(CustomError::WritePlanValidationRejected { validations })
                if validations.iter().any(|item| item.errors.contains(&expected)))
        );
        memory.close().await.unwrap();
    }

    #[tokio::test]
    async fn older_plan_replay_keeps_predecessor_out_of_vector_index() {
        let memory = injected_memory().await;
        let episode_id = MemoryId::from_u128(1000);
        let predecessor_id = MemoryId::from_u128(1001);
        let successor_id = MemoryId::from_u128(1002);
        let mut episode = EpisodeDraft::new("Preference source.");
        episode.id = Some(episode_id);
        let mut predecessor =
            DerivedMemoryDraft::new(DerivedType::UserPreference, "Original preference.")
                .with_source_episode(episode_id);
        predecessor.id = Some(predecessor_id);
        let old_plan = memory
            .prepare(
                RememberInput::new("Preference source.")
                    .with_episode(episode)
                    .with_derived_memory(predecessor),
                PrepareOptions::default(),
            )
            .await
            .unwrap();
        memory
            .commit(old_plan.clone(), CommitOptions::default())
            .await
            .unwrap();
        let mut successor =
            DerivedMemoryDraft::new(DerivedType::UserPreference, "Updated preference.")
                .with_source_episode(episode_id);
        successor.id = Some(successor_id);
        successor.supersedes = vec![predecessor_id];
        memory
            .remember(
                RememberInput::new("Preference update.").with_derived_memory(successor),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        let replay = memory
            .commit(old_plan, CommitOptions::default())
            .await
            .expect("older identical plan is accepted");
        assert!(!replay.vector_indexed_object_ids.contains(&predecessor_id));
        assert!(replay.vector_indexing_failure.is_none());
        let recall = memory
            .memory_composition
            .vector_store
            .search_candidates(&VectorCandidateSearch::new(
                vec![1.0; 8],
                100,
                vec![ObjectType::DerivedMemory],
            ))
            .await
            .unwrap();
        assert_eq!(
            recall
                .candidates
                .iter()
                .map(|candidate| candidate.object_id)
                .collect::<Vec<_>>(),
            vec![successor_id]
        );
        memory.close().await.unwrap();
    }

    #[tokio::test]
    async fn older_correction_replay_keeps_superseded_replacement_out_of_index() {
        let (memory, fixtures, replacement_id) = lifecycle_memory().await;
        let old_correction =
            derived_correction_draft(&fixtures, replacement_id, fixtures.user_preference.id);
        memory.correct(old_correction.clone()).await.unwrap();
        let newest_id = MemoryId::from_u128(1003);
        memory
            .correct(derived_correction_draft(
                &fixtures,
                newest_id,
                replacement_id,
            ))
            .await
            .unwrap();
        let replay = memory
            .correct(old_correction)
            .await
            .expect("older identical correction is accepted");
        assert!(replay.graph_mutated_object_ids.is_empty());
        assert!(replay.graph_mutated_link_ids.is_empty());
        assert!(!replay
            .vector_maintained_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                replacement_id
            )));
        assert!(replay.vector_maintenance_failure.is_none());
        let recall = memory
            .memory_composition
            .vector_store
            .search_candidates(&VectorCandidateSearch::new(
                vec![1.0; 4],
                100,
                vec![ObjectType::DerivedMemory],
            ))
            .await
            .unwrap();
        assert_eq!(
            recall
                .candidates
                .iter()
                .map(|candidate| candidate.object_id)
                .collect::<Vec<_>>(),
            vec![newest_id]
        );
        memory.close().await.unwrap();
    }

    #[tokio::test]
    async fn link_cannot_overwrite_generated_supersession() {
        let (memory, fixtures, replacement_id) = lifecycle_memory().await;
        let correction = memory
            .correct(derived_correction_draft(
                &fixtures,
                replacement_id,
                fixtures.user_preference.id,
            ))
            .await
            .unwrap();
        let link_id = correction.graph_mutated_link_ids[0];
        let graph = memory.memory_composition.graph_store.as_ref();
        let original = graph.query_links_by_ids(&[link_id]).await.unwrap();
        let mut replacement = MemoryLinkDraft::new(
            ObjectType::DerivedMemory,
            replacement_id,
            RelationType::About,
            ObjectType::DerivedMemory,
            fixtures.user_preference.id,
        );
        replacement.id = Some(link_id);
        replacement.created_at = Some(original[0].created_at);
        let error = memory
            .link(replacement)
            .await
            .expect_err("generated supersession is immutable");
        assert!(
            matches!(error, CustomError::DeterministicIdCollision { object }
            if object == MemoryObjectRef::new(ObjectType::MemoryLink, link_id))
        );
        assert_eq!(
            graph.query_links_by_ids(&[link_id]).await.unwrap(),
            original
        );
        assert_eq!(
            graph
                .query_superseded_derived_memory_ids(&[fixtures.user_preference.id])
                .await
                .unwrap(),
            vec![fixtures.user_preference.id]
        );
        let retrieved = memory
            .retrieve(RetrievalContext::new("corrected preference"))
            .await
            .unwrap();
        assert!(!pack_contains_derived_memory(
            &retrieved.pack,
            fixtures.user_preference.id
        ));
        memory.close().await.unwrap();
    }

    #[tokio::test]
    async fn link_replay_accepts_equal_content_and_rejects_divergence() {
        let memory = injected_memory().await;
        let link_id = MemoryId::from_u128(1010);
        let mut draft = MemoryLinkDraft::new(
            ObjectType::Episode,
            MemoryId::from_u128(1011),
            RelationType::Mentions,
            ObjectType::Entity,
            MemoryId::from_u128(1012),
        );
        draft.id = Some(link_id);
        let first = memory.link(draft.clone()).await.unwrap();
        let replay = memory
            .link(draft.clone())
            .await
            .expect("identical link replay is accepted");
        assert_eq!(replay.link, first.link);
        draft.created_at = Some(first.link.created_at);
        assert_eq!(memory.link(draft.clone()).await.unwrap().link, first.link);
        let mut changed_timestamp = draft.clone();
        changed_timestamp.created_at = Some(first.link.created_at + chrono::Duration::seconds(1));
        assert!(matches!(memory.link(changed_timestamp).await.unwrap_err(),
            CustomError::DeterministicIdCollision { object } if object.id == link_id));
        draft.rationale = Some("different content".to_owned());
        let error = memory
            .link(draft)
            .await
            .expect_err("a plain link also rejects divergent content");
        assert!(
            matches!(error, CustomError::DeterministicIdCollision { object }
            if object == MemoryObjectRef::new(ObjectType::MemoryLink, link_id))
        );
        assert_eq!(
            memory
                .memory_composition
                .graph_store
                .query_links_by_ids(&[link_id])
                .await
                .unwrap(),
            vec![first.link]
        );
        memory.close().await.unwrap();
    }

    #[tokio::test]
    async fn retry_after_vector_failure_does_not_duplicate_graph_writes() {
        let memory = CharacterMemory::from_parts(
            Box::new(in_memory_graph_store()),
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
        let objects = graph
            .query_objects(&GraphObjectQuery::by_types(
                vec![
                    ObjectType::Episode,
                    ObjectType::Observation,
                    ObjectType::Entity,
                    ObjectType::MemoryThread,
                    ObjectType::DerivedMemory,
                ],
                None,
            ))
            .await
            .unwrap();
        assert_eq!(objects.len(), 2);
    }

    #[tokio::test]
    async fn injected_facade_links_canonical_relationships() {
        let memory = injected_memory().await;
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

        let outcome = memory
            .link(draft)
            .await
            .expect("link facade should persist through injected graph store");

        assert_eq!(outcome.link.from_id, from_id);
        assert_eq!(outcome.link.to_id, to_id);
        assert_eq!(outcome.link.relation, RelationType::Mentions);
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
        memory.close().await.unwrap();
    }

    #[tokio::test]
    async fn retrieve_rejects_an_empty_configured_object_type_scope_at_the_boundary() {
        let memory = injected_memory().await;
        let mut context = RetrievalContext::new("invalid empty scope");
        context.object_type_defaults.clear();

        let error = memory.retrieve(context).await.unwrap_err();

        assert!(matches!(
            error,
            CustomError::ConfigValidation(ConfigValidationError {
                keys,
                reason: ConfigValidationReason::OutOfDomain {
                    expected: "at least one retrieval object type",
                    actual,
                },
            }) if keys == vec!["object_type_defaults"] && actual == "[]"
        ));
    }

    #[tokio::test]
    async fn injected_facade_corrects_derived_memory_and_retrieval_excludes_superseded_memory() {
        let (memory, fixtures, replacement_id) = lifecycle_memory().await;

        let mut draft =
            derived_correction_draft(&fixtures, replacement_id, fixtures.user_preference.id);
        let replacement = &mut draft.replacement_derived_memories[0];
        replacement.derived_from_episode_ids = vec![fixtures.episode.id; 2];
        replacement.derived_from_observation_ids = vec![fixtures.salient_observation.id; 2];
        replacement.thread_ids = vec![fixtures.soft_thread.id; 2];
        replacement.entity_ids = vec![fixtures.user_entity.id; 2];
        replacement.supersedes = vec![fixtures.user_preference.id; 2];
        let outcome = memory.correct(draft.clone()).await.unwrap();
        memory
            .correct(draft)
            .await
            .expect("identical repeated-ID correction replays");
        let stored = memory
            .memory_composition
            .graph_store
            .query_objects(&GraphObjectQuery::by_ids(vec![replacement_id]))
            .await
            .unwrap();
        let MemoryObject::DerivedMemory(replacement) = &stored[0] else {
            panic!("expected replacement")
        };
        assert_eq!(
            replacement.derived_from_episode_ids,
            vec![fixtures.episode.id]
        );
        assert_eq!(
            replacement.derived_from_observation_ids,
            vec![fixtures.salient_observation.id]
        );
        assert_eq!(replacement.thread_ids, vec![fixtures.soft_thread.id]);
        assert_eq!(replacement.entity_ids, vec![fixtures.user_entity.id]);
        assert_eq!(replacement.supersedes, vec![fixtures.user_preference.id]);

        assert!(!outcome
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
            .graph_relations
            .iter()
            .any(|relation| {
                relation.from.id == replacement_id
                    && relation.to.id == fixtures.user_preference.id
                    && relation.relation == RelationType::Supersedes
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
                    original_setting_key: fixtures.episode.scene.setting.key.clone(),
                },
                episode_replacement_id,
            ))
            .await
            .expect("episode correction should supersede affected derived memories");

        assert!(!episode_outcome
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
                    original_setting_key: fixtures.episode.scene.setting.key.clone(),
                },
                observation_replacement_id,
            ))
            .await
            .expect("observation correction should supersede affected derived memories");

        assert!(!observation_outcome
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
    async fn injected_facade_forget_keeps_memory_thread_reachable() {
        let (memory, fixtures, _) = lifecycle_memory().await;

        let outcome = memory
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::MemoryThread(fixtures.soft_thread.id),
                    "Forget thread without cascading.",
                )
                .with_trace(),
            )
            .await
            .expect("thread forget should complete through injected lifecycle pipeline");

        assert!(outcome.graph_mutated_object_ids.is_empty());
        assert!(outcome.vector_maintained_object_ids.is_empty());
        let normal = memory
            .retrieve(RetrievalContext::new("contract test support"))
            .await
            .unwrap();
        assert!(normal
            .pack
            .active_threads
            .iter()
            .any(|thread| thread.id == fixtures.soft_thread.id));
    }

    async fn injected_memory() -> CharacterMemory {
        CharacterMemory::from_parts(
            Box::new(in_memory_graph_store()),
            Box::new(TemporaryVectorCandidateStore::open(8).await),
            Box::new(DeterministicMemoryEmbedder::new(8)),
        )
    }

    async fn retrieval_memory() -> (CharacterMemory, crate::test_support::RepresentativeFixtures) {
        let fixtures = representative_fixtures();
        let graph = in_memory_graph_store();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(2).await;
        let record = memory_object_vector_record(&MemoryObject::DerivedMemory(
            fixtures.user_preference.clone(),
        ))
        .unwrap();
        vector
            .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &[1.0, 0.0])])
            .await
            .unwrap();
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
        let graph = in_memory_graph_store();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
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
            query: &VectorCandidateSearch,
        ) -> Result<VectorCandidateRecall, CustomError> {
            Ok(VectorCandidateRecall {
                candidates: CanonicalCandidates::new([]),
                completeness: if query.limit == 0 || query.object_types.is_empty() {
                    crate::api::types::retrieval::VectorRecallCompleteness::NotRequested
                } else {
                    crate::api::types::retrieval::VectorRecallCompleteness::Exhaustive {
                        scanned: 0,
                    }
                },
            })
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
