// Remember pipeline used by the public facade and internal tests. Some
// builders remain available for focused test and validation paths.
use crate::api::types::{
    CommitOptions, DiagnosticSeverity, RememberDiagnostic, RememberDiagnosticCode,
    RememberDiagnostics, RememberOutcome, RememberWritePlan, RepairMarker, StatsUpdateStatus,
};
use crate::domain::{CandidateValidationStatus, MemoryLink, MemoryObject, MemoryObjectRef};
use crate::errors::CustomError;
use crate::models::vector::{EmbeddingInput, VectorRecord};
use crate::policy::embedding_surface::memory_object_vector_records;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectQuery};
use crate::ports::retrieval_stats::RetrievalStatsStore;
use crate::ports::vector_candidate::VectorCandidateStore;
use crate::usecases::{
    StatsProjectionService, VectorIndexingService, WritePlanCommitValues, WritePlanValidator,
};

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
        write_turn: &tokio::sync::Mutex<()>,
    ) -> Result<RememberOutcome, CustomError> {
        // Only request-owned values are prepared before the turn. Validation and all
        // graph-dependent decisions below see the preceding writer's completed state.
        let values = WritePlanCommitValues::from_plan(plan.clone());
        let vector_records = match &values {
            Ok(values) if options.update_vectors => {
                vector_records_for_targets(&values.objects, &values.vector_targets)
            }
            _ => Vec::new(),
        };
        let inputs = vector_records
            .iter()
            .map(VectorRecord::embedding_input)
            .collect::<Vec<_>>();
        let embeddings = if inputs.is_empty() {
            Ok(Vec::new())
        } else {
            self.embedder.embed_batch(&inputs).await
        };
        let _turn = write_turn.lock().await;
        let validation = WritePlanValidator::new(self.graph_store)
            .validate(&plan)
            .await?
            .into_result()?;
        let diagnostics =
            plan.diagnostics
                .clone()
                .with_validations(validation.validations.into_iter().filter(|validation| {
                    validation.status == CandidateValidationStatus::Invalid
                        || !validation.warnings.is_empty()
                }));
        let mut values = values?;
        super::scope::derive_scope_keys(self.graph_store, &mut values.objects).await?;

        self.reject_divergent_existing_writes(&values.objects, &values.links)
            .await?;

        self.persist_graph_then_repairable_parts(
            values.objects,
            values.links,
            vector_records,
            &inputs,
            embeddings,
            options,
        )
        .await
        .map(|mut outcome| {
            outcome.diagnostics.messages.extend(diagnostics.messages);
            outcome
                .diagnostics
                .validations
                .extend(diagnostics.validations);
            outcome
                .diagnostics
                .repair_needed
                .extend(diagnostics.repair_needed);
            outcome
        })
    }

    async fn persist_graph_then_repairable_parts(
        &self,
        objects: Vec<MemoryObject>,
        links: Vec<MemoryLink>,
        vector_records: Vec<VectorRecord>,
        inputs: &[EmbeddingInput],
        embeddings: Result<Vec<Vec<f32>>, CustomError>,
        options: CommitOptions,
    ) -> Result<RememberOutcome, CustomError> {
        self.graph_store
            .upsert_objects_and_links(&objects, &links)
            .await?;

        let mut outcome = graph_persisted_outcome(&objects, &links);
        if options.update_vectors {
            let mut predecessors = links
                .iter()
                .filter(|link| link.relation == crate::domain::RelationType::Supersedes)
                .map(|link| {
                    MemoryObjectRef::new(crate::domain::ObjectType::DerivedMemory, link.to_id)
                })
                .collect::<Vec<_>>();
            predecessors.sort_by_key(|object| object.stable_order_key());
            predecessors.dedup();
            if let Some(failure) =
                crate::usecases::vector_indexing::delete_vectors(self.vector_store, &predecessors)
                    .await?
            {
                let marker = RepairMarker::VectorMaintenance {
                    failure: crate::api::types::VectorMaintenanceFailure {
                        failures: vec![failure],
                    },
                };
                outcome.repair_needed.push(marker.clone());
                outcome.diagnostics.repair_needed.push(marker);
            }
            self.record_vector_outcome(&mut outcome, vector_records, inputs, embeddings)
                .await?;
        }

        if options.update_stats {
            self.record_stats_outcome(&mut outcome, &objects, &links)
                .await;
        }

        Ok(outcome)
    }

    async fn record_vector_outcome(
        &self,
        outcome: &mut RememberOutcome,
        vector_records: Vec<VectorRecord>,
        inputs: &[EmbeddingInput],
        embeddings: Result<Vec<Vec<f32>>, CustomError>,
    ) -> Result<(), CustomError> {
        if vector_records.is_empty() {
            return Ok(());
        }

        let indexing = VectorIndexingService::new(self.vector_store)
            .index(self.graph_store, vector_records, inputs, embeddings)
            .await?;
        outcome.vector_indexed_object_ids = indexing
            .indexed_objects
            .iter()
            .map(|object| object.id)
            .collect();
        if let Some(failure) = indexing.failure {
            let message = failure.cause.to_string();
            outcome.repair_needed.push(failure.clone().into());
            outcome.diagnostics =
                outcome
                    .diagnostics
                    .clone()
                    .with_message(RememberDiagnostic::new(
                        DiagnosticSeverity::Warning,
                        RememberDiagnosticCode::VectorIndexingFailed,
                        message,
                    ));
            outcome.vector_indexing_failure = Some(failure);
        }
        Ok(())
    }

    async fn record_stats_outcome(
        &self,
        outcome: &mut RememberOutcome,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) {
        let projection = StatsProjectionService::new(self.graph_store, self.stats_store)
            .project(objects, links)
            .await;
        match projection.causes.as_slice() {
            causes @ [_, ..] => {
                let diagnostic_code = if causes.iter().any(|cause| {
                    matches!(cause, crate::errors::StatsUpdateCause::HealthCheck { .. })
                }) {
                    RememberDiagnosticCode::StatsUpdateHealthCheckFailed
                } else {
                    RememberDiagnosticCode::StatsUpdateFailed
                };
                let message = causes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                outcome.stats_update_status = StatsUpdateStatus::failed(
                    Vec::new(),
                    projection.attempted_object_ids.clone(),
                    causes.to_vec(),
                );
                outcome.repair_needed.push(RepairMarker::StatsUpdate {
                    object_ids: projection.attempted_object_ids,
                    causes: causes.to_vec(),
                });
                outcome.diagnostics =
                    outcome
                        .diagnostics
                        .clone()
                        .with_message(RememberDiagnostic::new(
                            DiagnosticSeverity::Warning,
                            diagnostic_code,
                            message,
                        ));
            }
            [] => {
                outcome.stats_update_status =
                    StatsUpdateStatus::succeeded(projection.attempted_object_ids);
            }
        }
    }

    async fn reject_divergent_existing_writes(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), CustomError> {
        // The facade's turn covers both these reads and the following graph upsert.
        let refs = objects
            .iter()
            .map(MemoryObject::object_ref)
            .collect::<Vec<_>>();
        if !refs.is_empty() {
            for existing in self
                .graph_store
                .query_objects(&GraphObjectQuery::by_refs(refs))
                .await?
            {
                if let Some(planned) = objects
                    .iter()
                    .find(|object| object.object_ref() == existing.object_ref())
                {
                    if planned != &existing {
                        return Err(CustomError::DeterministicIdCollision {
                            object: planned.object_ref(),
                        });
                    }
                }
            }
        }

        if !links.is_empty() {
            let link_ids = links.iter().map(|link| link.id).collect::<Vec<_>>();
            let existing = self.graph_store.query_links_by_ids(&link_ids).await?;
            crate::usecases::link::reject_divergent_links(links, &existing)?;
        }

        Ok(())
    }
}

fn vector_records_for_targets(
    objects: &[MemoryObject],
    vector_targets: &[MemoryObjectRef],
) -> Vec<VectorRecord> {
    vector_targets
        .iter()
        .filter_map(|target| objects.iter().find(|object| object.object_ref() == *target))
        .flat_map(memory_object_vector_records)
        .collect()
}

fn graph_persisted_outcome(objects: &[MemoryObject], links: &[MemoryLink]) -> RememberOutcome {
    RememberOutcome {
        persisted_object_ids: objects.iter().map(MemoryObject::id).collect(),
        persisted_link_ids: links.iter().map(|link| link.id).collect(),
        vector_indexed_object_ids: Vec::new(),
        vector_indexing_failure: None,
        stats_update_status: StatsUpdateStatus::default(),
        repair_needed: Vec::new(),
        diagnostics: RememberDiagnostics::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ScopeKey;
    use crate::ports::graph_authority::GraphExpansionFilteredNode;
    use crate::ports::graph_authority::GraphExpansionLifecyclePolicy;
    use crate::test_support::parse_id as id;
    use crate::test_support::write_time as timestamp;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex, MutexGuard};

    use crate::adapters::oxigraph::OxigraphGraphAuthorityStore;
    use crate::adapters::stats::InMemoryRetrievalStatsStore;
    use crate::api::types::{
        CandidateProvenance, DerivedMemoryDraft, EntityDraft, EpisodeDraft, MemoryCandidate,
        MemoryLinkCandidate, MemoryLinkDraft, MemoryThreadDraft, ObservationDraft, RememberInput,
    };
    use crate::domain::{
        CandidateValidationIssue, DerivedType, MemoryCandidateKind, MemoryId, ObjectType,
        RelationType, DEFAULT_SCHEMA_VERSION,
    };
    use crate::errors::{
        RetrievalStatsHealthCause, RetrievalStatsStoreError, StatsUpdateCause, VectorDatabaseError,
        VectorDatabaseErrorKind, VectorIndexingCause,
    };
    use crate::models::vector::{EmbeddingInput, VectorCandidateSearch, VectorRecordEmbedding};
    use crate::ports::graph_authority::{GraphExpansion, GraphExpansionQuery, GraphObjectQuery};
    use crate::ports::retrieval_stats::{
        RetrievalStatsCounter, RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth,
        RetrievalStatsObjectState, RetrievalStatsStore,
    };
    use crate::ports::vector_candidate::VectorCandidateRecall;
    use crate::test_support::{
        in_memory_graph_store, representative_fixtures, TemporaryVectorCandidateStore,
    };
    use crate::usecases::write_planning::RememberPlanDefaults;

    #[tokio::test]
    async fn prepared_successor_validates_and_commits_derived_links_without_predecessor_write() {
        let fixtures = representative_fixtures();
        let graph = in_memory_graph_store();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        let successor_id = MemoryId::from_u128(901);
        let mut draft = DerivedMemoryDraft::new(DerivedType::Correction, "A corrected preference.")
            .with_source_episode(fixtures.episode.id);
        draft.id = Some(successor_id);
        draft.supersedes = vec![fixtures.user_preference.id];
        let plan =
            prepare_test_plan(RememberInput::new("Correction source.").with_derived_memory(draft));
        assert!(WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap()
            .is_valid());
        let vector = RecordingVectorStore::new().await;
        let embedder = RecordingEmbedder::default();
        let outcome = RememberPipeline::new(&graph, &vector, &embedder)
            .commit(plan, CommitOptions::default(), &tokio::sync::Mutex::new(()))
            .await
            .unwrap();
        assert!(!outcome
            .persisted_object_ids
            .contains(&fixtures.user_preference.id));
        assert_eq!(
            graph
                .query_objects(&GraphObjectQuery::by_ids(vec![fixtures.user_preference.id]))
                .await
                .unwrap(),
            vec![MemoryObject::DerivedMemory(
                fixtures.user_preference.clone()
            )]
        );
        assert_eq!(
            graph
                .query_superseded_derived_memory_ids(&[fixtures.user_preference.id, successor_id])
                .await
                .unwrap(),
            vec![fixtures.user_preference.id]
        );
        let links = graph
            .query_links_by_ids(&outcome.persisted_link_ids)
            .await
            .unwrap();
        assert!(links.iter().any(|link| link.from_id == successor_id
            && link.to_id == fixtures.user_preference.id
            && link.relation == RelationType::Supersedes));
    }

    #[tokio::test]
    async fn failed_currency_lookup_reports_repair_without_indexing_or_guessing_stats() {
        let mut graph = RecordingGraphStore {
            fail_currency_query: true,
            ..RecordingGraphStore::default()
        };
        let vector = RecordingVectorStore::new().await;
        let embedder = RecordingEmbedder::default();
        let stats = InMemoryRetrievalStatsStore::new();
        let outcome = RememberPipeline::new_with_stats(&graph, &vector, &embedder, &stats)
            .commit(
                representative_plan(&fixed_ids()),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();
        assert!(matches!(
            outcome
                .stats_update_status
                .failure
                .as_ref()
                .unwrap()
                .causes
                .as_slice(),
            [StatsUpdateCause::GraphRead {
                error: crate::errors::GraphQueryError::Selection { .. }
            }]
        ));
        assert_eq!(
            serde_json::to_value(&outcome.stats_update_status.failure.as_ref().unwrap().causes[0])
                .unwrap()["cause"],
            "graph_read"
        );
        let health_cause = stats.health().await.unwrap().last_error_cause.unwrap();
        assert!(matches!(
            health_cause,
            RetrievalStatsHealthCause::GraphRead {
                error: crate::errors::GraphQueryError::Selection { .. }
            }
        ));
        assert_eq!(
            serde_json::to_value(&health_cause).unwrap()["operation"],
            "graph_read"
        );
        assert!(outcome
            .repair_needed
            .iter()
            .any(|marker| matches!(marker, RepairMarker::StatsUpdate { .. })));
        assert!(stats
            .global_counter(RelationType::About, ObjectType::DerivedMemory)
            .await
            .unwrap()
            .is_none());
        assert!(matches!(
            outcome.vector_indexing_failure.unwrap().cause,
            VectorIndexingCause::GraphQuery(crate::errors::GraphQueryError::Selection { .. })
        ));
        assert!(outcome.vector_indexed_object_ids.is_empty());
        assert_eq!(
            outcome.persisted_object_ids,
            expected_object_ids(&fixed_ids())
        );
        assert!(!embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
        graph.fail_currency_query = false;
        let retry = RememberPipeline::new_with_stats(&graph, &vector, &embedder, &stats)
            .commit(
                representative_plan(&fixed_ids()),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();
        assert!(retry.vector_indexing_failure.is_none());
        assert_eq!(
            retry.vector_indexed_object_ids,
            expected_vector_ids(&fixed_ids())
        );
    }

    #[tokio::test]
    async fn persists_graph_objects_links_then_vectors_in_stable_order() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore {
            calls: graph.calls.clone(),
            ..RecordingVectorStore::new().await
        };
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .commit(
                representative_plan(&ids),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("remember draft should persist");

        assert_eq!(outcome.persisted_object_ids, expected_object_ids(&ids));
        // Ruling 69: the structural ObservedIn follows the authored links.
        assert_eq!(outcome.persisted_link_ids.len(), 3);
        assert_eq!(
            outcome.persisted_link_ids[..2],
            [ids.inline_link, ids.extra_link]
        );
        assert_eq!(outcome.vector_indexed_object_ids, expected_vector_ids(&ids));
        assert_eq!(outcome.vector_indexing_failure, None);
        let calls = graph.calls();
        let last_graph_write = calls
            .iter()
            .rposition(|call| matches!(call, StoreCall::GraphObjects(_) | StoreCall::GraphLinks(_)))
            .unwrap();
        let first_vector_write = calls
            .iter()
            .position(|call| matches!(call, StoreCall::VectorUpsert(_)))
            .unwrap();
        assert!(last_graph_write < first_vector_write);
        let stored = graph
            .query_objects(&GraphObjectQuery::by_ids(expected_object_ids(&ids)))
            .await
            .unwrap();
        assert_eq!(
            stored
                .iter()
                .map(MemoryObject::id)
                .collect::<std::collections::HashSet<_>>(),
            expected_object_ids(&ids).into_iter().collect()
        );
        let stored_links = graph
            .query_links_by_ids(&[ids.inline_link, ids.extra_link])
            .await
            .unwrap();
        assert_eq!(
            stored_links
                .iter()
                .map(|link| link.id)
                .collect::<std::collections::HashSet<_>>(),
            [ids.inline_link, ids.extra_link].into_iter().collect()
        );
    }

    #[tokio::test]
    async fn bounded_link_collision_query_rejects_divergent_existing_content() {
        let graph = in_memory_graph_store();
        let vector = RecordingVectorStore::new().await;
        let embedder = RecordingEmbedder::default();
        let existing = representative_fixtures().links()[0].clone();
        graph
            .upsert_links(std::slice::from_ref(&existing))
            .await
            .unwrap();
        let mut divergent = existing.clone();
        divergent.rationale = Some("Changed caller rationale.".to_owned());

        let error = RememberPipeline::new(&graph, &vector, &embedder)
            .reject_divergent_existing_writes(&[], &[divergent])
            .await
            .expect_err("divergent content under an existing link ID must reject");

        assert!(
            matches!(error, CustomError::DeterministicIdCollision { object }
            if object == MemoryObjectRef::new(ObjectType::MemoryLink, existing.id))
        );
    }

    #[tokio::test]
    async fn graph_object_failure_prevents_link_and_vector_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default().fail_objects();
        let vector = RecordingVectorStore::new().await;
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .commit(
                representative_plan(&ids),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap_err();

        assert!(matches!(error, CustomError::DatabaseError(_)));
        assert!(!embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn graph_link_failure_prevents_vector_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default().fail_links();
        let vector = RecordingVectorStore::new().await;
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .commit(
                representative_plan(&ids),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap_err();

        assert!(matches!(error, CustomError::DatabaseError(_)));
        assert!(!embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
        assert!(graph
            .query_objects(&GraphObjectQuery::by_ids(expected_object_ids(&ids)))
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn validation_failure_prevents_all_store_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::new().await;
        let embedder = RecordingEmbedder::default();
        let mut invalid_episode = EpisodeDraft::new(" ");
        invalid_episode.id = Some(ids.episode);
        let plan = prepare_test_plan(RememberInput::new(" ").with_episode(invalid_episode));
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .commit(plan, CommitOptions::default(), &tokio::sync::Mutex::new(()))
            .await
            .unwrap_err();

        let CustomError::WritePlanValidationRejected { validations } = error else {
            panic!("expected structured plan rejection");
        };
        assert!(validations
            .iter()
            .any(
                |validation| validation.candidate_kind == MemoryCandidateKind::Episode
                    && validation.status == CandidateValidationStatus::Invalid
                    && validation
                        .errors
                        .contains(&CandidateValidationIssue::EmptyEpisodeSummary)
            ));
        assert!(graph.calls().is_empty());
        assert!(embedder.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn vector_upsert_failure_returns_partial_success_with_graph_ids() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::new().await.fail_upsert();
        let embedder = RecordingEmbedder::default();
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .commit(
                representative_plan(&ids),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("graph success with vector failure should return partial outcome");

        assert_eq!(outcome.persisted_object_ids, expected_object_ids(&ids));
        // Ruling 69: the structural ObservedIn follows the authored links.
        assert_eq!(outcome.persisted_link_ids.len(), 3);
        assert_eq!(
            outcome.persisted_link_ids[..2],
            [ids.inline_link, ids.extra_link]
        );
        assert!(outcome.vector_indexed_object_ids.is_empty());
        let failure = outcome
            .vector_indexing_failure
            .expect("vector failure should be explicit");
        assert_eq!(failure.unindexed_object_ids(), expected_vector_ids(&ids));
        assert!(matches!(
            failure.cause,
            VectorIndexingCause::VectorDatabase(VectorDatabaseError {
                backend,
                kind: VectorDatabaseErrorKind::Response,
                ..
            }) if backend == "test"
        ));
    }

    #[tokio::test]
    async fn wrong_embedding_count_returns_clear_partial_failure_without_vector_write() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::new().await;
        let embedder = RecordingEmbedder::default().with_embedding_count(3);
        let pipeline = RememberPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .commit(
                representative_plan(&ids),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("graph success with embedding mismatch should return partial outcome");

        assert!(outcome.vector_indexed_object_ids.is_empty());
        let failure = outcome
            .vector_indexing_failure
            .expect("embedding mismatch should be explicit");
        assert_eq!(failure.unindexed_object_ids(), expected_vector_ids(&ids));
        assert_eq!(
            failure.cause,
            VectorIndexingCause::CardinalityMismatch {
                expected: 4,
                actual: 3,
            }
        );
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn remember_pipeline_records_stats_after_vector_attempt() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::default();
        let vector = RecordingVectorStore::new().await;
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
                            RelationType::Involves,
                            ObjectType::Episode,
                            ids.episode,
                        )),
                ),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("stats should not change remember outcome");

        let counter = stats
            .counter(&RetrievalStatsCounterKey {
                entity_id: ids.entity,
                relation_kind: RelationType::Involves,
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
        let vector = RecordingVectorStore::new().await;
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
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();

        // Ruling 69: ObservedIn accompanies the caller's association.
        assert_eq!(outcome.persisted_link_ids.len(), 2);
        assert_eq!(outcome.persisted_link_ids[0], ids.extra_link);
        let stored = graph.query_links_by_ids(&[ids.extra_link]).await.unwrap();
        assert!(matches!(stored.as_slice(), [link]
            if link.id == ids.extra_link && link.relation == RelationType::AssociatedWith));
        assert!(!embedder.calls().is_empty());
        assert!(!vector.calls().is_empty());
    }

    #[tokio::test]
    async fn remember_pipeline_uses_existing_endpoint_lifecycle_for_link_stats() {
        let fixtures = representative_fixtures();
        let graph = in_memory_graph_store();
        graph
            .upsert_objects(&[
                MemoryObject::Entity(fixtures.hub_entity.clone()),
                MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
            ])
            .await
            .unwrap();
        let vector = RecordingVectorStore::new().await;
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
                            RelationType::AssociatedWith,
                            ObjectType::DerivedMemory,
                            fixtures.suppressed_seed.id,
                        ),
                    ),
                ),
                CommitOptions::default(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("link-only remember should persist and record stats");

        let counter = stats
            .counter(&RetrievalStatsCounterKey {
                entity_id: fixtures.hub_entity.id,
                relation_kind: RelationType::AssociatedWith,
                object_type: ObjectType::DerivedMemory,
            })
            .await
            .unwrap()
            .unwrap();
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
    }

    #[tokio::test]
    async fn link_only_stats_failures_preserve_endpoint_ids_and_repair_causes() {
        let fixtures = representative_fixtures();
        let episode_id = fixtures.episode.id;
        let observation_id = fixtures.salient_observation.id;
        let graph = RecordingGraphStore::default()
            .with_query_objects(vec![
                MemoryObject::Episode(fixtures.episode),
                MemoryObject::Observation(fixtures.salient_observation),
            ])
            .await
            .fail_id_queries();
        let vector = RecordingVectorStore::new().await;
        let embedder = RecordingEmbedder::default();
        let stats = EdgeFailingStatsStore::default();
        let pipeline = RememberPipeline::new_with_stats(&graph, &vector, &embedder, &stats);
        let mut link = typed_link_draft(
            id("550e8400-e29b-41d4-a716-446655443010"),
            ObjectType::Episode,
            episode_id,
            RelationType::Mentions,
            ObjectType::Observation,
            observation_id,
        );
        link.created_at = Some(timestamp());
        link.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let plan = RememberWritePlan::new().with_candidate(MemoryCandidate::MemoryLink(
            MemoryLinkCandidate::new(
                link,
                CandidateProvenance::caller("exercise link-only stats hydration"),
            ),
        ));

        let outcome = pipeline
            .commit(plan, CommitOptions::default(), &tokio::sync::Mutex::new(()))
            .await
            .expect("stats hydration failure should remain a repairable graph success");
        let expected_ids = vec![episode_id, observation_id];
        let failure = outcome
            .stats_update_status
            .failure
            .as_ref()
            .expect("endpoint hydration failure should be published");

        assert_eq!(failure.failed_object_ids, expected_ids);
        assert!(matches!(
            failure.causes.as_slice(),
            [
                StatsUpdateCause::GraphRead { .. },
                StatsUpdateCause::EdgeWrite { .. }
            ]
        ));
        assert!(outcome.repair_needed.iter().any(|marker| matches!(
            marker,
            RepairMarker::StatsUpdate { object_ids, causes }
                if object_ids == &expected_ids
                    && causes == &failure.causes
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

    fn expected_vector_ids(ids: &FixedIds) -> Vec<MemoryId> {
        vec![ids.episode, ids.observation, ids.thread, ids.derived]
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
        let mut draft = EntityDraft::new();
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum StoreCall {
        GraphObjects(Vec<MemoryId>),
        GraphLinks(Vec<MemoryId>),
        EmbedBatch(Vec<MemoryId>),
        VectorUpsert(Vec<MemoryId>),
    }

    struct RecordingGraphStore {
        store: OxigraphGraphAuthorityStore,
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_objects: bool,
        fail_links: bool,
        fail_id_queries: bool,
        fail_currency_query: bool,
    }

    impl Default for RecordingGraphStore {
        fn default() -> Self {
            Self {
                store: in_memory_graph_store(),
                calls: Arc::default(),
                fail_objects: false,
                fail_links: false,
                fail_id_queries: false,
                fail_currency_query: false,
            }
        }
    }

    #[derive(Debug, Default)]
    struct EdgeFailingStatsStore {
        health: Mutex<RetrievalStatsHealth>,
    }

    #[async_trait]
    impl RetrievalStatsStore for EdgeFailingStatsStore {
        async fn record_edges(
            &self,
            _edges: &[RetrievalStatsEdge],
        ) -> Result<(), RetrievalStatsStoreError> {
            Err(RetrievalStatsStoreError::Sqlite {
                detail: "edge write failed".to_owned(),
            })
        }

        async fn record_object_states(
            &self,
            _states: &[RetrievalStatsObjectState],
        ) -> Result<(), RetrievalStatsStoreError> {
            Ok(())
        }

        async fn counter(
            &self,
            _key: &RetrievalStatsCounterKey,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(None)
        }

        async fn global_counter(
            &self,
            _relation_kind: RelationType,
            _object_type: ObjectType,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(None)
        }

        async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
            Ok(lock(&self.health).clone())
        }
        async fn global_episode_counter(
            &self,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(None)
        }

        async fn mark_unhealthy(
            &self,
            cause: RetrievalStatsHealthCause,
        ) -> Result<(), RetrievalStatsStoreError> {
            *lock(&self.health) = RetrievalStatsHealth {
                state: crate::ports::retrieval_stats::RetrievalStatsHealthState::Unhealthy,
                last_error_cause: Some(cause),
            };
            Ok(())
        }
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

        async fn with_query_objects(self, objects: Vec<MemoryObject>) -> Self {
            self.store.upsert_objects(&objects).await.unwrap();
            self
        }

        fn fail_id_queries(mut self) -> Self {
            self.fail_id_queries = true;
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl GraphAuthorityStore for RecordingGraphStore {
        async fn query_anniversaries(
            &self,
            date: chrono::NaiveDate,
            participants: &[crate::domain::MemoryId],
            limit: usize,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Vec<(crate::ports::graph_authority::GraphMemoryRank, bool)>, CustomError>
        {
            let _ = (date, participants, limit, policy);
            Ok(Vec::new())
        }

        async fn query_episodes_by_time(
            &self,
            start: Option<chrono::DateTime<chrono::Utc>>,
            end: chrono::DateTime<chrono::Utc>,
            limit: usize,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Vec<crate::ports::graph_authority::GraphMemoryRank>, CustomError> {
            self.store
                .query_episodes_by_time(start, end, limit, policy)
                .await
        }

        async fn query_episode_occasions(
            &self,
            episodes: &[crate::domain::MemoryObjectRef],
        ) -> Result<crate::policy::graph_expansion::ParticipantOccasions, CustomError> {
            self.store.query_episode_occasions(episodes).await
        }

        async fn query_last_interaction(
            &self,
            participant: MemoryId,
            reference_time: chrono::DateTime<chrono::Utc>,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Option<(MemoryId, chrono::DateTime<chrono::Utc>)>, CustomError> {
            self.store
                .query_last_interaction(participant, reference_time, policy)
                .await
        }

        async fn query_notions_known_as(
            &self,
            name: &str,
        ) -> Result<Vec<MemoryId>, crate::errors::GraphQueryError> {
            self.store.query_notions_known_as(name).await
        }

        async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphObjects(
                objects.iter().map(MemoryObject::id).collect(),
            ));
            if self.fail_objects {
                return Err(CustomError::DatabaseError("object write failed".to_owned()));
            }
            self.store.upsert_objects(objects).await
        }

        async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphLinks(
                links.iter().map(|link| link.id).collect(),
            ));
            if self.fail_links {
                return Err(CustomError::DatabaseError("link write failed".to_owned()));
            }
            self.store.upsert_links(links).await
        }

        async fn upsert_objects_and_links(
            &self,
            objects: &[MemoryObject],
            links: &[MemoryLink],
        ) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphObjects(
                objects.iter().map(MemoryObject::id).collect(),
            ));
            if self.fail_objects {
                return Err(CustomError::DatabaseError("object write failed".to_owned()));
            }
            lock(&self.calls).push(StoreCall::GraphLinks(
                links.iter().map(|link| link.id).collect(),
            ));
            if self.fail_links {
                return Err(CustomError::DatabaseError("link write failed".to_owned()));
            }
            self.store.upsert_objects_and_links(objects, links).await
        }

        async fn query_objects(
            &self,
            query: &GraphObjectQuery,
        ) -> Result<Vec<MemoryObject>, crate::errors::GraphQueryError> {
            if self.fail_id_queries && matches!(query, GraphObjectQuery::ByIds(_)) {
                return Err(crate::errors::GraphQueryError::Selection {
                    detail: "endpoint lifecycle lookup failed".to_owned(),
                });
            }

            self.store.query_objects(query).await
        }

        async fn query_superseded_derived_memory_ids(
            &self,
            memory_ids: &[crate::domain::MemoryId],
        ) -> Result<Vec<crate::domain::MemoryId>, crate::errors::GraphQueryError> {
            if self.fail_currency_query {
                return Err(crate::errors::GraphQueryError::Selection {
                    detail: "currency lookup failed".to_owned(),
                });
            }
            self.store
                .query_superseded_derived_memory_ids(memory_ids)
                .await
        }

        async fn query_links_by_ids(
            &self,
            link_ids: &[MemoryId],
        ) -> Result<Vec<MemoryLink>, CustomError> {
            self.store.query_links_by_ids(link_ids).await
        }

        async fn query_derived_memories_by_provenance(
            &self,
            query: &crate::ports::graph_authority::GraphDerivedMemoryProvenanceQuery,
        ) -> Result<Vec<crate::domain::DerivedMemory>, CustomError> {
            self.store.query_derived_memories_by_provenance(query).await
        }

        async fn query_derived_memories_by_thread(
            &self,
            query: &crate::ports::graph_authority::GraphDerivedMemoryThreadQuery,
        ) -> Result<
            (
                Vec<crate::domain::DerivedMemory>,
                Vec<GraphExpansionFilteredNode>,
            ),
            CustomError,
        > {
            self.store.query_derived_memories_by_thread(query).await
        }

        async fn query_thread_state(
            &self,
            query: &crate::ports::graph_authority::GraphDerivedMemoryThreadQuery,
            limit: usize,
        ) -> Result<
            (
                Vec<crate::ports::graph_authority::GraphMemoryRank>,
                Vec<crate::ports::graph_authority::GraphExpansionFilteredNode>,
            ),
            CustomError,
        > {
            let _ = (query, limit);
            self.store.query_thread_state(query, limit).await
        }

        async fn query_scope_state(
            &self,
            key: &ScopeKey,
            policy: GraphExpansionLifecyclePolicy,
            limit: usize,
        ) -> Result<
            (
                Vec<crate::ports::graph_authority::GraphMemoryRank>,
                Vec<GraphExpansionFilteredNode>,
            ),
            CustomError,
        > {
            self.store.query_scope_state(key, policy, limit).await
        }

        async fn expand_bounded(
            &self,
            query: &GraphExpansionQuery,
        ) -> Result<GraphExpansion, CustomError> {
            self.store.expand_bounded(query).await
        }
    }

    #[derive(Debug)]
    struct RecordingVectorStore {
        inner: TemporaryVectorCandidateStore,
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_upsert: bool,
    }

    impl RecordingVectorStore {
        async fn new() -> Self {
            Self {
                inner: TemporaryVectorCandidateStore::open(1).await,
                calls: Arc::default(),
                fail_upsert: false,
            }
        }

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
        async fn close(&self) -> Result<(), CustomError> {
            self.inner.close().await
        }

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
                return Err(CustomError::VectorDatabaseError(VectorDatabaseError::new(
                    "test",
                    VectorDatabaseErrorKind::Response,
                    None,
                    "vector write failed",
                )));
            }
            self.inner.upsert_vector_records(records).await
        }

        async fn search_candidates(
            &self,
            query: &VectorCandidateSearch,
        ) -> Result<VectorCandidateRecall, CustomError> {
            self.inner.search_candidates(query).await
        }

        async fn delete_candidates(&self, objects: &[MemoryObjectRef]) -> Result<(), CustomError> {
            self.inner.delete_candidates(objects).await
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
