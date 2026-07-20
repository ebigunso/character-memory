// Remember pipeline used by the public facade and internal tests. Some
// builders remain available for focused test and validation paths.
use crate::api::types::{
    CommitOptions, DiagnosticSeverity, MemoryObjectRef, RememberDiagnostic, RememberDiagnostics,
    RememberWritePlan, RepairMarker, StatsUpdateStatus,
};
use crate::domain::{MemoryId, MemoryLink, MemoryObject, ObjectType};
use crate::errors::CustomError;
use crate::models::vector::{VectorRecord, VectorRecordEmbedding};
use crate::policy::memory_object_vector_record;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectQuery, GraphObjectRef};
use crate::ports::retrieval_stats::{
    record_stats_after_write, RetrievalStatsHealthState, RetrievalStatsStore,
};
use crate::ports::vector_candidate::VectorCandidateStore;
use crate::usecases::{WritePlanCommitValues, WritePlanValidator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RememberPipelineOutcome {
    pub(crate) persisted_object_ids: Vec<MemoryId>,
    pub(crate) persisted_link_ids: Vec<MemoryId>,
    pub(crate) vector_indexed_object_ids: Vec<MemoryId>,
    pub(crate) vector_indexing_failure: Option<VectorIndexingFailure>,
    pub(crate) stats_update_status: StatsUpdateStatus,
    pub(crate) repair_needed: Vec<RepairMarker>,
    pub(crate) diagnostics: RememberDiagnostics,
}

impl RememberPipelineOutcome {
    fn graph_persisted(objects: &[MemoryObject], links: &[MemoryLink]) -> Self {
        Self {
            persisted_object_ids: objects.iter().map(memory_object_id).collect(),
            persisted_link_ids: links.iter().map(|link| link.id).collect(),
            vector_indexed_object_ids: Vec::new(),
            vector_indexing_failure: None,
            stats_update_status: StatsUpdateStatus::default(),
            repair_needed: Vec::new(),
            diagnostics: RememberDiagnostics::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VectorIndexingFailure {
    pub(crate) unindexed_object_ids: Vec<MemoryId>,
    pub(crate) error_message: String,
}

pub(crate) struct RememberPipeline<'a, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    graph_store: &'a G,
    vector_store: &'a V,
    embedder: &'a E,
    stats_store: &'a dyn RetrievalStatsStore,
}

impl<'a, G, V, E> RememberPipeline<'a, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    #[cfg(test)]
    pub(crate) fn new(graph_store: &'a G, vector_store: &'a V, embedder: &'a E) -> Self {
        Self {
            graph_store,
            vector_store,
            embedder,
            stats_store: crate::adapters::stats::noop_retrieval_stats_store(),
        }
    }

    pub(crate) fn new_with_stats(
        graph_store: &'a G,
        vector_store: &'a V,
        embedder: &'a E,
        stats_store: &'a dyn RetrievalStatsStore,
    ) -> Self {
        Self {
            graph_store,
            vector_store,
            embedder,
            stats_store,
        }
    }

    pub(crate) async fn commit(
        &self,
        plan: RememberWritePlan,
        options: CommitOptions,
    ) -> Result<RememberPipelineOutcome, CustomError> {
        WritePlanValidator::new(self.graph_store)
            .validate(&plan)
            .await?
            .into_result()?;
        let diagnostics = plan.diagnostics.clone();
        let values = WritePlanCommitValues::from_plan(plan)?;
        let vector_targets = if options.update_vectors {
            VectorWriteIntent::PlanTargets(values.vector_targets)
        } else {
            VectorWriteIntent::None
        };

        self.reject_divergent_existing_writes(&values.objects, &values.links)
            .await?;

        self.persist_graph_then_repairable_parts(
            values.objects,
            values.links,
            vector_targets,
            options,
        )
        .await
        .map(|mut outcome| {
            outcome.diagnostics.messages.extend(diagnostics.messages);
            outcome
                .diagnostics
                .repair_needed
                .extend(diagnostics.repair_needed);
            outcome
                .diagnostics
                .candidate_counts
                .extend(diagnostics.candidate_counts);
            outcome
        })
    }

    async fn persist_graph_then_repairable_parts(
        &self,
        objects: Vec<MemoryObject>,
        links: Vec<MemoryLink>,
        vector_intent: VectorWriteIntent,
        options: CommitOptions,
    ) -> Result<RememberPipelineOutcome, CustomError> {
        self.graph_store.upsert_objects(&objects).await?;
        self.graph_store.upsert_links(&links).await?;

        let mut outcome = RememberPipelineOutcome::graph_persisted(&objects, &links);
        if options.update_vectors {
            let vector_records = match vector_intent {
                VectorWriteIntent::PlanTargets(targets) => {
                    vector_records_for_targets(&objects, &targets)
                }
                VectorWriteIntent::None => Vec::new(),
            };
            self.record_vector_outcome(&mut outcome, &vector_records)
                .await;
        }

        if options.update_stats {
            self.record_stats_outcome(&mut outcome, &objects, &links)
                .await;
        }

        Ok(outcome)
    }

    async fn record_vector_outcome(
        &self,
        outcome: &mut RememberPipelineOutcome,
        vector_records: &[VectorRecord],
    ) {
        if vector_records.is_empty() {
            return;
        }

        match self.index_vector_records(vector_records).await {
            Ok(indexed_ids) => {
                outcome.vector_indexed_object_ids = indexed_ids;
            }
            Err(error_message) => {
                let failure = VectorIndexingFailure {
                    unindexed_object_ids: vector_records
                        .iter()
                        .map(|record| record.object_id)
                        .collect(),
                    error_message,
                };
                outcome
                    .repair_needed
                    .push(crate::api::types::VectorIndexingFailure::from(failure.clone()).into());
                outcome.diagnostics =
                    outcome
                        .diagnostics
                        .clone()
                        .with_message(RememberDiagnostic::new(
                            DiagnosticSeverity::Warning,
                            "vector_indexing_failed",
                            failure.error_message.clone(),
                        ));
                outcome.vector_indexing_failure = Some(failure);
            }
        }
    }

    async fn record_stats_outcome(
        &self,
        outcome: &mut RememberPipelineOutcome,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) {
        let updated_ids = self.record_remember_stats_after_write(objects, links).await;
        match self.stats_store.health().await {
            Ok(health) if health.state == RetrievalStatsHealthState::Unhealthy => {
                let error_message = health
                    .last_error_message
                    .unwrap_or_else(|| "retrieval stats store is unhealthy".to_owned());
                outcome.stats_update_status = StatsUpdateStatus::failed(
                    Vec::new(),
                    updated_ids.clone(),
                    error_message.clone(),
                );
                outcome.repair_needed.push(RepairMarker::StatsUpdate {
                    object_ids: updated_ids,
                    error_message: error_message.clone(),
                });
                outcome.diagnostics =
                    outcome
                        .diagnostics
                        .clone()
                        .with_message(RememberDiagnostic::new(
                            DiagnosticSeverity::Warning,
                            "stats_update_failed",
                            error_message,
                        ));
            }
            Err(error) => {
                let error_message = error.to_string();
                outcome.stats_update_status = StatsUpdateStatus::failed(
                    Vec::new(),
                    updated_ids.clone(),
                    error_message.clone(),
                );
                outcome.repair_needed.push(RepairMarker::StatsUpdate {
                    object_ids: updated_ids,
                    error_message: error_message.clone(),
                });
                outcome.diagnostics =
                    outcome
                        .diagnostics
                        .clone()
                        .with_message(RememberDiagnostic::new(
                            DiagnosticSeverity::Warning,
                            "stats_update_health_check_failed",
                            error_message,
                        ));
            }
            _ => {
                outcome.stats_update_status = StatsUpdateStatus::succeeded(updated_ids);
            }
        }
    }

    async fn record_remember_stats_after_write(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Vec<MemoryId> {
        let endpoint_refs = remember_stats_endpoint_refs(objects, links);
        if endpoint_refs.is_empty() {
            record_stats_after_write(self.stats_store, objects, links).await;
            return objects.iter().map(memory_object_id).collect();
        }

        match self
            .graph_store
            .query_objects(&GraphObjectQuery::by_refs(endpoint_refs))
            .await
        {
            Ok(endpoint_objects) => {
                let stats_objects =
                    stats_objects_with_endpoint_lifecycle(objects, endpoint_objects);
                record_stats_after_write(self.stats_store, &stats_objects, links).await;
                stats_objects.iter().map(memory_object_id).collect()
            }
            Err(error) => {
                let error_message = error.to_string();
                record_stats_after_write(self.stats_store, objects, links).await;
                let _ = self.stats_store.mark_unhealthy(error_message).await;
                objects.iter().map(memory_object_id).collect()
            }
        }
    }

    async fn reject_divergent_existing_writes(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), CustomError> {
        // Known scale consideration: this checks planned IDs against current graph state without
        // a persisted operation ledger, which is acceptable for current write volumes.
        let refs = objects
            .iter()
            .map(|object| {
                let (id, object_type) = memory_object_identity(object);
                GraphObjectRef::new(id, object_type)
            })
            .collect::<Vec<_>>();
        if !refs.is_empty() {
            for existing in self
                .graph_store
                .query_objects(&GraphObjectQuery::by_refs(refs))
                .await?
            {
                if let Some(planned) = objects.iter().find(|object| {
                    memory_object_identity(object) == memory_object_identity(&existing)
                }) {
                    if planned != &existing {
                        return Err(validation_error(format!(
                            "write plan deterministic ID collided with existing divergent object content: {:?} {}",
                            memory_object_type(planned),
                            memory_object_id(planned)
                        )));
                    }
                }
            }
        }

        if !links.is_empty() {
            for existing in self.graph_store.list_diagnostic_links().await? {
                if let Some(planned) = links.iter().find(|link| link.id == existing.id) {
                    if planned != &existing {
                        return Err(validation_error(format!(
                            "write plan deterministic ID collided with existing divergent link content: {}",
                            planned.id
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    async fn index_vector_records(
        &self,
        vector_records: &[VectorRecord],
    ) -> Result<Vec<MemoryId>, String> {
        let embedding_inputs = vector_records
            .iter()
            .map(VectorRecord::embedding_input)
            .collect::<Vec<_>>();
        let embeddings = self
            .embedder
            .embed_batch(&embedding_inputs)
            .await
            .map_err(|error| error.to_string())?;

        if embeddings.len() != vector_records.len() {
            return Err(format!(
                "embedder returned {} embeddings for {} vector records",
                embeddings.len(),
                vector_records.len()
            ));
        }

        let record_embeddings = vector_records
            .iter()
            .zip(embeddings.iter())
            .map(|(record, embedding)| VectorRecordEmbedding::new(record, embedding.as_slice()))
            .collect::<Vec<_>>();
        self.vector_store
            .upsert_vector_records(&record_embeddings)
            .await
            .map_err(|error| error.to_string())?;

        Ok(vector_records
            .iter()
            .map(|record| record.object_id)
            .collect())
    }
}

enum VectorWriteIntent {
    PlanTargets(Vec<MemoryObjectRef>),
    None,
}

fn vector_records_for_targets(
    objects: &[MemoryObject],
    vector_targets: &[MemoryObjectRef],
) -> Vec<VectorRecord> {
    vector_targets
        .iter()
        .filter_map(|target| {
            objects.iter().find_map(|object| {
                (memory_object_id(object) == target.id
                    && memory_object_type(object) == target.object_type)
                    .then(|| memory_object_vector_record(object))
                    .flatten()
            })
        })
        .collect()
}

fn remember_stats_endpoint_refs(
    objects: &[MemoryObject],
    links: &[MemoryLink],
) -> Vec<GraphObjectRef> {
    let mut refs = Vec::new();
    for link in links {
        push_stats_endpoint_ref(&mut refs, objects, link.from_id, link.from_type);
        push_stats_endpoint_ref(&mut refs, objects, link.to_id, link.to_type);
    }
    refs
}

fn push_stats_endpoint_ref(
    refs: &mut Vec<GraphObjectRef>,
    objects: &[MemoryObject],
    object_id: MemoryId,
    object_type: ObjectType,
) {
    if !object_type_has_stats_state(object_type)
        || objects
            .iter()
            .any(|object| memory_object_identity(object) == (object_id, object_type))
        || refs.iter().any(|object_ref| {
            object_ref.object_id == object_id && object_ref.object_type == object_type
        })
    {
        return;
    }

    refs.push(GraphObjectRef::new(object_id, object_type));
}

fn stats_objects_with_endpoint_lifecycle(
    objects: &[MemoryObject],
    endpoint_objects: Vec<MemoryObject>,
) -> Vec<MemoryObject> {
    let mut stats_objects = objects.to_vec();
    for endpoint_object in endpoint_objects {
        if !stats_objects.iter().any(|object| {
            memory_object_identity(object) == memory_object_identity(&endpoint_object)
        }) {
            stats_objects.push(endpoint_object);
        }
    }
    stats_objects
}

fn object_type_has_stats_state(object_type: ObjectType) -> bool {
    matches!(
        object_type,
        ObjectType::Episode | ObjectType::Observation | ObjectType::DerivedMemory
    )
}

fn validation_error(error: impl ToString) -> CustomError {
    CustomError::MemoryValidation(error.to_string())
}

fn memory_object_id(object: &MemoryObject) -> MemoryId {
    memory_object_identity(object).0
}

fn memory_object_type(object: &MemoryObject) -> ObjectType {
    memory_object_identity(object).1
}

fn memory_object_identity(object: &MemoryObject) -> (MemoryId, ObjectType) {
    match object {
        MemoryObject::Episode(object) => (object.id, object.object_type),
        MemoryObject::Observation(object) => (object.id, object.object_type),
        MemoryObject::Entity(object) => (object.id, object.object_type),
        MemoryObject::MemoryThread(object) => (object.id, object.object_type),
        MemoryObject::DerivedMemory(object) => (object.id, object.object_type),
        MemoryObject::MemoryLink(object) => (object.id, object.object_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::{Arc, Mutex, MutexGuard};
    use uuid::Uuid;

    use crate::adapters::stats::InMemoryRetrievalStatsStore;
    use crate::api::types::{
        DerivedMemoryDraft, EntityDraft, EpisodeDraft, MemoryLinkDraft, MemoryThreadDraft,
        ObservationDraft, RememberInput,
    };
    use crate::domain::{DerivedType, EntityType, ObjectType, RelationType, RetentionState};
    use crate::models::vector::{EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch};
    use crate::ports::graph_authority::{GraphExpansion, GraphExpansionQuery, GraphObjectQuery};
    use crate::ports::retrieval_stats::{RetrievalStatsCounterKey, RetrievalStatsStore};
    use crate::test_support::{representative_fixtures, FakeGraphAuthorityStore};
    use crate::usecases::write_planning::RememberPlanDefaults;

    #[tokio::test]
    async fn persists_graph_objects_links_then_vectors_in_stable_order() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .commit(representative_plan(&ids), CommitOptions::default())
            .await
            .expect("remember draft should persist");

        assert_eq!(outcome.persisted_object_ids, expected_object_ids(&ids));
        assert_eq!(
            outcome.persisted_link_ids,
            vec![ids.inline_link, ids.extra_link]
        );
        assert_eq!(
            outcome.vector_indexed_object_ids,
            outcome.persisted_object_ids
        );
        assert_eq!(outcome.vector_indexing_failure, None);
        assert_eq!(
            graph.calls(),
            vec![
                StoreCall::GraphObjects(vec![
                    ids.episode,
                    ids.observation,
                    ids.entity,
                    ids.thread,
                    ids.derived,
                ]),
                StoreCall::GraphLinks(vec![ids.inline_link, ids.extra_link]),
            ]
        );
        assert_eq!(
            embedder.calls(),
            vec![StoreCall::EmbedBatch(vec![
                ids.episode,
                ids.observation,
                ids.entity,
                ids.thread,
                ids.derived,
            ])]
        );
        assert_eq!(
            vector.calls(),
            vec![StoreCall::VectorUpsert(vec![
                ids.episode,
                ids.observation,
                ids.entity,
                ids.thread,
                ids.derived,
            ])]
        );
    }

    #[tokio::test]
    async fn graph_object_failure_prevents_link_embedding_and_vector_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default().fail_objects();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .commit(representative_plan(&ids), CommitOptions::default())
            .await
            .unwrap_err();

        assert!(error.to_string().contains("object write failed"));
        assert_eq!(
            graph.calls(),
            vec![StoreCall::GraphObjects(vec![
                ids.episode,
                ids.observation,
                ids.entity,
                ids.thread,
                ids.derived,
            ])]
        );
        assert!(embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn graph_link_failure_prevents_embedding_and_vector_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default().fail_links();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .commit(representative_plan(&ids), CommitOptions::default())
            .await
            .unwrap_err();

        assert!(error.to_string().contains("link write failed"));
        assert_eq!(
            graph.calls(),
            vec![
                StoreCall::GraphObjects(vec![
                    ids.episode,
                    ids.observation,
                    ids.entity,
                    ids.thread,
                    ids.derived,
                ]),
                StoreCall::GraphLinks(vec![ids.inline_link, ids.extra_link]),
            ]
        );
        assert!(embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn validation_failure_prevents_all_store_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let mut invalid_episode = EpisodeDraft::new(" ");
        invalid_episode.id = Some(ids.episode);
        let plan = prepare_test_plan(RememberInput::new(" ").with_episode(invalid_episode));
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .commit(plan, CommitOptions::default())
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("episode summary must not be empty"));
        assert!(graph.calls().is_empty());
        assert!(embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn vector_upsert_failure_returns_partial_success_with_graph_ids() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default().fail_upsert();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .commit(representative_plan(&ids), CommitOptions::default())
            .await
            .expect("graph success with vector failure should return partial outcome");

        assert_eq!(outcome.persisted_object_ids, expected_object_ids(&ids));
        assert_eq!(
            outcome.persisted_link_ids,
            vec![ids.inline_link, ids.extra_link]
        );
        assert!(outcome.vector_indexed_object_ids.is_empty());
        let failure = outcome
            .vector_indexing_failure
            .expect("vector failure should be explicit");
        assert_eq!(failure.unindexed_object_ids, outcome.persisted_object_ids);
        assert!(failure.error_message.contains("vector write failed"));
    }

    #[tokio::test]
    async fn wrong_embedding_count_returns_clear_partial_failure_without_vector_write() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default().with_embedding_count(4);
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .commit(representative_plan(&ids), CommitOptions::default())
            .await
            .expect("graph success with embedding mismatch should return partial outcome");

        assert!(outcome.vector_indexed_object_ids.is_empty());
        let failure = outcome
            .vector_indexing_failure
            .expect("embedding mismatch should be explicit");
        assert_eq!(failure.unindexed_object_ids, outcome.persisted_object_ids);
        assert_eq!(
            failure.error_message,
            "embedder returned 4 embeddings for 5 vector records"
        );
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn remember_pipeline_records_stats_after_vector_attempt() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = RememberPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        pipeline
            .commit(
                prepare_test_plan(
                    RememberInput::new("stats after vector attempt")
                        .with_episode(episode_draft(ids.episode))
                        .with_entity(entity_draft(ids.entity))
                        .with_memory_link(typed_link_draft(
                            ids.extra_link,
                            ObjectType::Entity,
                            ids.entity,
                            RelationType::Mentions,
                            ObjectType::Episode,
                            ids.episode,
                        )),
                ),
                CommitOptions::default(),
            )
            .await
            .expect("stats should not change remember outcome");

        let counter = stats
            .counter(&RetrievalStatsCounterKey {
                entity_id: ids.entity,
                relation_kind: RelationType::Mentions,
                object_type: ObjectType::Episode,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 1);
        assert_eq!(counter.current_count, 1);
    }

    #[tokio::test]
    async fn remember_accepts_caller_supplied_associated_with_links() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = RememberPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        let outcome = pipeline
            .commit(
                prepare_test_plan(
                    RememberInput::new("caller supplied association")
                        .with_episode(episode_draft(ids.episode))
                        .with_entity(entity_draft(ids.entity))
                        .with_memory_link(typed_link_draft(
                            ids.extra_link,
                            ObjectType::Entity,
                            ids.entity,
                            RelationType::AssociatedWith,
                            ObjectType::Episode,
                            ids.episode,
                        )),
                ),
                CommitOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(outcome.persisted_link_ids, vec![ids.extra_link]);
        assert!(graph
            .calls()
            .contains(&StoreCall::GraphLinks(vec![ids.extra_link])));
        assert!(!embedder.calls().is_empty());
        assert!(!vector.calls().is_empty());
        assert_eq!(
            stats.rejected_low_information_link_count().await.unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn remember_pipeline_uses_existing_endpoint_lifecycle_for_link_stats() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph
            .upsert_objects(&[
                MemoryObject::Entity(fixtures.hub_entity.clone()),
                MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
            ])
            .await
            .unwrap();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = RememberPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        let outcome = pipeline
            .commit(
                prepare_test_plan(
                    RememberInput::new("existing endpoint stats").with_memory_link(
                        typed_link_draft(
                            id("550e8400-e29b-41d4-a716-446655443008"),
                            ObjectType::Entity,
                            fixtures.hub_entity.id,
                            RelationType::About,
                            ObjectType::DerivedMemory,
                            fixtures.suppressed_seed.id,
                        ),
                    ),
                ),
                CommitOptions::default(),
            )
            .await
            .expect("link-only remember should persist and record stats");

        let counter = stats
            .counter(&RetrievalStatsCounterKey {
                entity_id: fixtures.hub_entity.id,
                relation_kind: RelationType::About,
                object_type: ObjectType::DerivedMemory,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            fixtures.suppressed_seed.retention_state,
            RetentionState::Suppressed
        );
        assert!(!fixtures.suppressed_seed.is_current);
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 0);
        assert_eq!(counter.current_count, 0);
        assert!(outcome
            .stats_update_status
            .updated_object_ids
            .contains(&fixtures.suppressed_seed.id));
        let failed_ids = outcome
            .stats_update_status
            .failure
            .as_ref()
            .map(|failure| failure.failed_object_ids.as_slice())
            .unwrap_or_default();
        assert!(failed_ids.is_empty());

        let unhealthy_stats = InMemoryRetrievalStatsStore::unhealthy("repair required".to_owned());
        let repair_pipeline =
            RememberPipeline::new_with_stats(&graph, &vector, &embedder, &unhealthy_stats);
        let repair_outcome = repair_pipeline
            .commit(
                prepare_test_plan_with_seed(
                    RememberInput::new("existing endpoint repair").with_memory_link(
                        typed_link_draft(
                            id("550e8400-e29b-41d4-a716-446655443009"),
                            ObjectType::Entity,
                            fixtures.hub_entity.id,
                            RelationType::About,
                            ObjectType::DerivedMemory,
                            fixtures.suppressed_seed.id,
                        ),
                    ),
                    "remember-pipeline-repair",
                ),
                CommitOptions::default(),
            )
            .await
            .expect("stats repair outcome should still return graph success");

        let failure = repair_outcome
            .stats_update_status
            .failure
            .as_ref()
            .expect("unhealthy stats store should report failed update ids");
        assert!(failure
            .failed_object_ids
            .contains(&fixtures.suppressed_seed.id));
        assert!(repair_outcome.repair_needed.iter().any(|marker| matches!(
            marker,
            RepairMarker::StatsUpdate { object_ids, .. }
                if object_ids.contains(&fixtures.suppressed_seed.id)
        )));
    }

    #[derive(Debug, Clone, Copy)]
    struct FixedIds {
        entity: MemoryId,
        episode: MemoryId,
        observation: MemoryId,
        thread: MemoryId,
        derived: MemoryId,
        inline_link: MemoryId,
        extra_link: MemoryId,
    }

    fn fixed_ids() -> FixedIds {
        FixedIds {
            entity: id("550e8400-e29b-41d4-a716-446655443001"),
            episode: id("550e8400-e29b-41d4-a716-446655443002"),
            observation: id("550e8400-e29b-41d4-a716-446655443003"),
            thread: id("550e8400-e29b-41d4-a716-446655443004"),
            derived: id("550e8400-e29b-41d4-a716-446655443005"),
            inline_link: id("550e8400-e29b-41d4-a716-446655443006"),
            extra_link: id("550e8400-e29b-41d4-a716-446655443007"),
        }
    }

    fn representative_plan(ids: &FixedIds) -> RememberWritePlan {
        prepare_test_plan(
            RememberInput::new("Discussed stable remember ordering.")
                .with_episode(episode_draft(ids.episode))
                .with_observation(observation_draft(ids.observation, ids.episode))
                .with_entity(entity_draft(ids.entity))
                .with_memory_thread(thread_draft(ids.thread))
                .with_derived_memory(derived_draft(ids.derived, ids.episode, ids.observation))
                .with_memory_link(link_draft(ids.inline_link, ids.episode, ids.observation))
                .with_memory_link(typed_link_draft(
                    ids.extra_link,
                    ObjectType::DerivedMemory,
                    ids.derived,
                    RelationType::PartOfThread,
                    ObjectType::MemoryThread,
                    ids.thread,
                )),
        )
    }

    fn expected_object_ids(ids: &FixedIds) -> Vec<MemoryId> {
        vec![
            ids.episode,
            ids.observation,
            ids.entity,
            ids.thread,
            ids.derived,
        ]
    }

    fn prepare_test_plan(input: RememberInput) -> RememberWritePlan {
        prepare_test_plan_with_seed(input, "remember-pipeline-tests")
    }

    fn prepare_test_plan_with_seed(input: RememberInput, seed: &str) -> RememberWritePlan {
        input.prepare_write_plan_with_options(
            &RememberPlanDefaults::fixed(seed, timestamp()),
            true,
            true,
        )
    }

    fn entity_draft(id: MemoryId) -> EntityDraft {
        let mut draft = EntityDraft::new(EntityType::User, "Kohta");
        draft.id = Some(id);
        draft
    }

    fn episode_draft(id: MemoryId) -> EpisodeDraft {
        let mut draft = EpisodeDraft::new("Discussed stable remember ordering.");
        draft.id = Some(id);
        draft
    }

    fn observation_draft(id: MemoryId, episode_id: MemoryId) -> ObservationDraft {
        let mut draft = ObservationDraft::new(episode_id, "Stable vectors follow graph writes.");
        draft.id = Some(id);
        draft
    }

    fn thread_draft(id: MemoryId) -> MemoryThreadDraft {
        let mut draft = MemoryThreadDraft::new("Remember pipeline", "Graph before vectors.");
        draft.id = Some(id);
        draft
    }

    fn derived_draft(
        id: MemoryId,
        episode_id: MemoryId,
        observation_id: MemoryId,
    ) -> DerivedMemoryDraft {
        let mut draft = DerivedMemoryDraft::new(DerivedType::Reflection, "Order matters.")
            .with_source_episode(episode_id)
            .with_source_observation(observation_id);
        draft.id = Some(id);
        draft
    }

    fn link_draft(id: MemoryId, from_id: MemoryId, to_id: MemoryId) -> MemoryLinkDraft {
        typed_link_draft(
            id,
            ObjectType::Episode,
            from_id,
            RelationType::Mentions,
            ObjectType::Observation,
            to_id,
        )
    }

    fn typed_link_draft(
        id: MemoryId,
        from_type: ObjectType,
        from_id: MemoryId,
        relation: RelationType,
        to_type: ObjectType,
        to_id: MemoryId,
    ) -> MemoryLinkDraft {
        let mut draft = MemoryLinkDraft::new(from_type, from_id, relation, to_type, to_id);
        draft.id = Some(id);
        draft
    }

    fn id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-04-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum StoreCall {
        GraphObjects(Vec<MemoryId>),
        GraphLinks(Vec<MemoryId>),
        EmbedBatch(Vec<MemoryId>),
        VectorUpsert(Vec<MemoryId>),
    }

    #[derive(Debug, Default)]
    struct RecordingGraphStore {
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_objects: bool,
        fail_links: bool,
    }

    impl RecordingGraphStore {
        fn fail_objects(mut self) -> Self {
            self.fail_objects = true;
            self
        }

        fn fail_links(mut self) -> Self {
            self.fail_links = true;
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl GraphAuthorityStore for RecordingGraphStore {
        async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphObjects(
                objects.iter().map(memory_object_id).collect(),
            ));
            if self.fail_objects {
                return Err(CustomError::DatabaseError("object write failed".to_owned()));
            }
            Ok(())
        }

        async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphLinks(
                links.iter().map(|link| link.id).collect(),
            ));
            if self.fail_links {
                return Err(CustomError::DatabaseError("link write failed".to_owned()));
            }
            Ok(())
        }

        async fn upsert_objects_and_links(
            &self,
            objects: &[MemoryObject],
            links: &[MemoryLink],
        ) -> Result<(), CustomError> {
            self.upsert_objects(objects).await?;
            self.upsert_links(links).await
        }

        async fn query_objects(
            &self,
            _query: &GraphObjectQuery,
        ) -> Result<Vec<MemoryObject>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_provenance(
            &self,
            _query: &crate::ports::graph_authority::GraphDerivedMemoryProvenanceQuery,
        ) -> Result<Vec<crate::domain::DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_thread(
            &self,
            _query: &crate::ports::graph_authority::GraphDerivedMemoryThreadQuery,
        ) -> Result<Vec<crate::domain::DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn expand_bounded(
            &self,
            _query: &GraphExpansionQuery,
        ) -> Result<GraphExpansion, CustomError> {
            Ok(GraphExpansion::new(Vec::new(), Vec::new()))
        }

        async fn list_diagnostic_objects(&self) -> Result<Vec<MemoryObject>, CustomError> {
            Ok(Vec::new())
        }

        async fn list_diagnostic_links(&self) -> Result<Vec<MemoryLink>, CustomError> {
            Ok(Vec::new())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingVectorStore {
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_upsert: bool,
    }

    impl RecordingVectorStore {
        fn fail_upsert(mut self) -> Self {
            self.fail_upsert = true;
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl VectorCandidateStore for RecordingVectorStore {
        async fn upsert_vector_records(
            &self,
            records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::VectorUpsert(
                records
                    .iter()
                    .map(|record| record.record.object_id)
                    .collect(),
            ));
            if self.fail_upsert {
                return Err(CustomError::DatabaseError("vector write failed".to_owned()));
            }
            Ok(())
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

    #[derive(Debug, Default)]
    struct RecordingEmbedder {
        calls: Arc<Mutex<Vec<StoreCall>>>,
        embedding_count: Option<usize>,
    }

    impl RecordingEmbedder {
        fn with_embedding_count(mut self, embedding_count: usize) -> Self {
            self.embedding_count = Some(embedding_count);
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl MemoryEmbedder for RecordingEmbedder {
        async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
            Ok(vec![embedding_seed(input)])
        }

        async fn embed_batch(
            &self,
            inputs: &[EmbeddingInput],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            lock(&self.calls).push(StoreCall::EmbedBatch(
                inputs.iter().filter_map(|input| input.object_id).collect(),
            ));
            let count = self.embedding_count.unwrap_or(inputs.len());
            Ok(inputs
                .iter()
                .cycle()
                .take(count)
                .map(|input| vec![embedding_seed(input)])
                .collect())
        }
    }

    fn embedding_seed(input: &EmbeddingInput) -> f32 {
        input.text.len() as f32
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().expect("test mutex should not be poisoned")
    }
}
