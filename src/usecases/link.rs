// Typed-link pipeline used by the public facade and internal tests. Some
// helpers remain available for focused test and validation paths.
use crate::api::types::{DraftDefaults, LinkOutcome, MemoryLinkDraft};
use crate::domain::{MemoryLink, RelationType};
use crate::errors::CustomError;
use crate::ports::graph_authority::GraphAuthorityStore;
use crate::ports::retrieval_stats::RetrievalStatsStore;
use crate::usecases::StatsProjectionService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkAdmissionEvidence {
    ExplicitCallerIntent,
    #[cfg(test)]
    LowSelectivityCoOccurrenceOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkAdmissionDecision {
    Accepted,
    RejectedLowInformationCoOccurrence,
}

pub(crate) struct LinkPipeline<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    graph_store: &'a G,
    stats_store: &'a dyn RetrievalStatsStore,
}

impl<'a, G> LinkPipeline<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    #[cfg(test)]
    pub(crate) fn new(graph_store: &'a G) -> Self {
        Self {
            graph_store,
            stats_store: crate::adapters::stats::noop_retrieval_stats_store(),
        }
    }

    pub(crate) fn new_with_stats(
        graph_store: &'a G,
        stats_store: &'a dyn RetrievalStatsStore,
    ) -> Self {
        Self {
            graph_store,
            stats_store,
        }
    }

    pub(crate) async fn link(&self, draft: MemoryLinkDraft) -> Result<LinkOutcome, CustomError> {
        let mut defaults = DraftDefaults::generated();
        self.link_with_defaults(draft, &mut defaults).await
    }

    pub(crate) async fn link_with_defaults(
        &self,
        draft: MemoryLinkDraft,
        defaults: &mut DraftDefaults,
    ) -> Result<LinkOutcome, CustomError> {
        self.link_with_evidence(draft, defaults, LinkAdmissionEvidence::ExplicitCallerIntent)
            .await
    }

    async fn link_with_evidence(
        &self,
        draft: MemoryLinkDraft,
        defaults: &mut DraftDefaults,
        evidence: LinkAdmissionEvidence,
    ) -> Result<LinkOutcome, CustomError> {
        let link = draft.into_domain_with_defaults(defaults)?;
        if admit_link(&link, evidence) == LinkAdmissionDecision::RejectedLowInformationCoOccurrence
        {
            return Err(CustomError::LowInformationCoOccurrence { link_id: link.id });
        }
        self.graph_store
            .upsert_links(std::slice::from_ref(&link))
            .await?;
        let projection = StatsProjectionService::new(self.graph_store, self.stats_store)
            .project(&[], std::slice::from_ref(&link))
            .await;
        Ok(LinkOutcome {
            link,
            stats_update_status: projection.into_status(),
        })
    }
}

pub(crate) fn admit_link(
    link: &MemoryLink,
    evidence: LinkAdmissionEvidence,
) -> LinkAdmissionDecision {
    if link.relation != RelationType::AssociatedWith {
        return LinkAdmissionDecision::Accepted;
    }

    match evidence {
        #[cfg(test)]
        LinkAdmissionEvidence::LowSelectivityCoOccurrenceOnly => {
            LinkAdmissionDecision::RejectedLowInformationCoOccurrence
        }
        LinkAdmissionEvidence::ExplicitCallerIntent => LinkAdmissionDecision::Accepted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use crate::adapters::stats::InMemoryRetrievalStatsStore;
    use crate::domain::{
        DerivedMemory, DomainValidationError, MemoryId, MemoryObject, ObjectType, RelationType,
        RetentionState, DEFAULT_SCHEMA_VERSION,
    };
    use crate::errors::{RetrievalStatsHealthCause, RetrievalStatsStoreError, StatsUpdateCause};
    use crate::ports::graph_authority::{
        GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
        GraphExpansion, GraphExpansionQuery, GraphObjectQuery,
    };
    use crate::ports::retrieval_stats::{
        RetrievalStatsCounter, RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth,
        RetrievalStatsObjectState, RetrievalStatsStore,
    };
    use crate::test_support::{in_memory_graph_store, representative_fixtures};

    #[tokio::test]
    async fn persists_caller_supplied_link_as_graph_authoritative_record() {
        let graph = in_memory_graph_store();
        let fixtures = representative_fixtures();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        let pipeline = LinkPipeline::new(&graph);
        let mut defaults = DraftDefaults::at(timestamp());
        let mut draft = MemoryLinkDraft::new(
            ObjectType::Entity,
            fixtures.hub_entity.id,
            RelationType::Involves,
            ObjectType::Episode,
            fixtures.episode.id,
        );
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655444001"));
        draft.confidence = 0.42;
        draft.rationale = Some("Task_4 typed link pipeline test.".to_owned());

        let persisted = pipeline
            .link_with_defaults(draft, &mut defaults)
            .await
            .expect("valid typed link should persist")
            .link;

        assert_eq!(persisted.id, id("550e8400-e29b-41d4-a716-446655444001"));
        assert_eq!(persisted.object_type, ObjectType::MemoryLink);
        assert_eq!(persisted.from_id, fixtures.hub_entity.id);
        assert_eq!(persisted.from_type, ObjectType::Entity);
        assert_eq!(persisted.to_id, fixtures.episode.id);
        assert_eq!(persisted.to_type, ObjectType::Episode);
        assert_eq!(persisted.relation, RelationType::Involves);
        assert_eq!(persisted.confidence, 0.42);
        assert_eq!(persisted.created_at, timestamp());
        assert_eq!(persisted.schema_version, DEFAULT_SCHEMA_VERSION);

        let expansion = graph
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 1, 2)
                    .with_allowed_object_types(vec![ObjectType::Episode]),
            )
            .await
            .unwrap();
        assert_eq!(expansion.links, vec![persisted]);
        assert_eq!(
            expansion.objects,
            vec![
                MemoryObject::Entity(fixtures.hub_entity),
                MemoryObject::Episode(fixtures.episode),
            ]
        );
    }

    #[tokio::test]
    async fn rejects_invalid_confidence_before_graph_write() {
        let graph = in_memory_graph_store();
        let pipeline = LinkPipeline::new(&graph);
        let mut draft = valid_link_draft();
        draft.confidence = 1.1;

        let error = pipeline.link(draft).await.unwrap_err();

        assert!(matches!(
            error,
            CustomError::DomainValidation(DomainValidationError::InvalidScore {
                field: "MemoryLink.confidence",
                value: 1.1,
            })
        ));
    }

    #[tokio::test]
    async fn rejects_self_links_before_graph_write() {
        let graph = in_memory_graph_store();
        let pipeline = LinkPipeline::new(&graph);
        let object_id = id("550e8400-e29b-41d4-a716-446655444010");
        let draft = MemoryLinkDraft::new(
            ObjectType::Observation,
            object_id,
            RelationType::AssociatedWith,
            ObjectType::Observation,
            object_id,
        );

        let error = pipeline.link(draft).await.unwrap_err();

        assert!(matches!(
            error,
            CustomError::DomainValidation(DomainValidationError::SelfLink {
                object_type: ObjectType::Observation,
                id,
            }) if id == object_id
        ));
    }

    #[tokio::test]
    async fn rejects_memory_link_endpoints_before_graph_write() {
        let graph = in_memory_graph_store();
        let pipeline = LinkPipeline::new(&graph);
        let draft = MemoryLinkDraft::new(
            ObjectType::MemoryLink,
            id("550e8400-e29b-41d4-a716-446655444020"),
            RelationType::AssociatedWith,
            ObjectType::Entity,
            id("550e8400-e29b-41d4-a716-446655444021"),
        );

        let error = pipeline.link(draft).await.unwrap_err();

        assert!(matches!(
            error,
            CustomError::DomainValidation(DomainValidationError::UnsupportedMemoryLinkEndpoint {
                field: "MemoryLink.from_type",
            })
        ));
    }

    #[tokio::test]
    async fn link_pipeline_records_entity_relation_stats_after_graph_success() {
        let graph = in_memory_graph_store();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);
        let draft = valid_link_draft();
        let entity_id = draft.to_id;

        let persisted = pipeline.link(draft).await.unwrap().link;

        let counter = stats
            .counter(&RetrievalStatsCounterKey {
                entity_id,
                relation_kind: persisted.relation,
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
    async fn link_pipeline_records_endpoint_lifecycle_state_in_stats() {
        let graph = in_memory_graph_store();
        let fixtures = representative_fixtures();
        let mut suppressed_episode = fixtures.episode.clone();
        suppressed_episode.retention_state = RetentionState::Suppressed;
        graph
            .upsert_objects(&[
                MemoryObject::Entity(fixtures.hub_entity.clone()),
                MemoryObject::Episode(suppressed_episode.clone()),
            ])
            .await
            .unwrap();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);
        let draft = MemoryLinkDraft::new(
            ObjectType::Entity,
            fixtures.hub_entity.id,
            RelationType::Involves,
            ObjectType::Episode,
            suppressed_episode.id,
        );

        let persisted = pipeline.link(draft).await.unwrap().link;

        let counter = stats
            .counter(&RetrievalStatsCounterKey {
                entity_id: fixtures.hub_entity.id,
                relation_kind: persisted.relation,
                object_type: ObjectType::Episode,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 0);
        assert_eq!(counter.current_count, 0);
    }

    #[tokio::test]
    async fn low_information_guard_rejects_weak_associated_with_candidate_without_graph_write() {
        let graph = in_memory_graph_store();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);
        let mut defaults = DraftDefaults::at(timestamp());

        let error = pipeline
            .link_with_evidence(
                associated_with_link_draft(),
                &mut defaults,
                LinkAdmissionEvidence::LowSelectivityCoOccurrenceOnly,
            )
            .await
            .unwrap_err();

        let rejected_link_id = id("550e8400-e29b-41d4-a716-446655444042");
        assert!(matches!(
            error,
            CustomError::LowInformationCoOccurrence { link_id } if link_id == rejected_link_id
        ));
        assert!(graph
            .query_links_by_ids(&[rejected_link_id])
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn explicit_intent_allows_associated_with_links_by_default() {
        let graph = in_memory_graph_store();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);

        let persisted = pipeline
            .link(associated_with_link_draft())
            .await
            .unwrap()
            .link;

        assert_eq!(persisted.relation, RelationType::AssociatedWith);
    }

    #[tokio::test]
    async fn entity_neutral_low_information_guard_does_not_check_roles() {
        let graph = in_memory_graph_store();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);

        for (from_type, to_type) in [
            (ObjectType::Episode, ObjectType::Episode),
            (ObjectType::DerivedMemory, ObjectType::Observation),
        ] {
            let mut draft = associated_with_link_draft();
            draft.from_type = from_type;
            draft.to_type = to_type;
            let rejected_link_id = draft.id.unwrap();
            let mut defaults = DraftDefaults::at(timestamp());
            let error = pipeline
                .link_with_evidence(
                    draft,
                    &mut defaults,
                    LinkAdmissionEvidence::LowSelectivityCoOccurrenceOnly,
                )
                .await
                .unwrap_err();

            assert!(matches!(
                error,
                CustomError::LowInformationCoOccurrence { link_id } if link_id == rejected_link_id
            ));
        }
    }

    #[tokio::test]
    async fn link_pipeline_records_fallback_stats_when_endpoint_lookup_fails() {
        let graph = QueryObjectsFailingGraph::default();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);
        let draft = valid_link_draft();
        let entity_id = draft.to_id;

        let persisted = pipeline.link(draft).await.unwrap().link;

        let counter = stats
            .counter(&RetrievalStatsCounterKey {
                entity_id,
                relation_kind: persisted.relation,
                object_type: ObjectType::Episode,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 1);
        assert_eq!(counter.current_count, 1);
        assert_eq!(
            stats.health().await.unwrap().state,
            crate::ports::retrieval_stats::RetrievalStatsHealthState::Unhealthy
        );
    }

    #[tokio::test]
    async fn link_outcome_preserves_all_stats_failures() {
        let graph = in_memory_graph_store();
        let fixtures = representative_fixtures();
        graph
            .upsert_objects(&[
                MemoryObject::Entity(fixtures.hub_entity.clone()),
                MemoryObject::Episode(fixtures.episode.clone()),
            ])
            .await
            .unwrap();
        let stats = DualFailingStatsStore;
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);
        let draft = MemoryLinkDraft::new(
            ObjectType::Entity,
            fixtures.hub_entity.id,
            RelationType::Involves,
            ObjectType::Episode,
            fixtures.episode.id,
        );

        let outcome = pipeline
            .link(draft)
            .await
            .expect("stats degradation should remain a repairable link outcome");

        assert_eq!(outcome.link.from_id, fixtures.hub_entity.id);
        assert!(outcome.stats_update_status.updated_object_ids.is_empty());
        let failure = outcome
            .stats_update_status
            .failure
            .as_ref()
            .expect("stats failure must be visible");
        assert_eq!(failure.failed_object_ids, vec![fixtures.episode.id]);
        assert!(matches!(
            failure.causes.as_slice(),
            [
                StatsUpdateCause::EdgeWrite { .. },
                StatsUpdateCause::ObjectStateWrite { .. }
            ]
        ));
    }

    struct DualFailingStatsStore;

    #[async_trait]
    impl RetrievalStatsStore for DualFailingStatsStore {
        async fn record_edges(
            &self,
            _edges: &[RetrievalStatsEdge],
        ) -> Result<(), RetrievalStatsStoreError> {
            Err(RetrievalStatsStoreError::Sqlite {
                detail: "stats edge write failed".to_owned(),
            })
        }

        async fn record_object_states(
            &self,
            _states: &[RetrievalStatsObjectState],
        ) -> Result<(), RetrievalStatsStoreError> {
            Err(RetrievalStatsStoreError::Sqlite {
                detail: "stats object-state write failed".to_owned(),
            })
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
            Ok(RetrievalStatsHealth::default())
        }

        async fn mark_unhealthy(
            &self,
            _cause: RetrievalStatsHealthCause,
        ) -> Result<(), RetrievalStatsStoreError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct QueryObjectsFailingGraph {
        links: std::sync::Mutex<Vec<MemoryLink>>,
    }

    #[async_trait]
    impl GraphAuthorityStore for QueryObjectsFailingGraph {
        async fn upsert_objects(&self, _objects: &[MemoryObject]) -> Result<(), CustomError> {
            Ok(())
        }

        async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
            self.links.lock().unwrap().extend_from_slice(links);
            Ok(())
        }

        async fn upsert_objects_and_links(
            &self,
            _objects: &[MemoryObject],
            links: &[MemoryLink],
        ) -> Result<(), CustomError> {
            self.upsert_links(links).await
        }

        async fn query_objects(
            &self,
            _query: &GraphObjectQuery,
        ) -> Result<Vec<MemoryObject>, crate::errors::GraphQueryError> {
            Err(crate::errors::GraphQueryError::Selection {
                detail: "endpoint lifecycle lookup failed".to_owned(),
            })
        }

        async fn query_links_by_ids(
            &self,
            _link_ids: &[MemoryId],
        ) -> Result<Vec<MemoryLink>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_provenance(
            &self,
            _query: &GraphDerivedMemoryProvenanceQuery,
        ) -> Result<Vec<DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_thread(
            &self,
            _query: &GraphDerivedMemoryThreadQuery,
        ) -> Result<Vec<DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn expand_bounded(
            &self,
            _query: &GraphExpansionQuery,
        ) -> Result<GraphExpansion, CustomError> {
            Ok(GraphExpansion::new(Vec::new(), Vec::new()))
        }
    }

    fn valid_link_draft() -> MemoryLinkDraft {
        let mut draft = MemoryLinkDraft::new(
            ObjectType::Episode,
            id("550e8400-e29b-41d4-a716-446655444030"),
            RelationType::Mentions,
            ObjectType::Entity,
            id("550e8400-e29b-41d4-a716-446655444031"),
        );
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655444032"));
        draft
    }

    fn associated_with_link_draft() -> MemoryLinkDraft {
        let mut draft = MemoryLinkDraft::new(
            ObjectType::Observation,
            id("550e8400-e29b-41d4-a716-446655444040"),
            RelationType::AssociatedWith,
            ObjectType::Observation,
            id("550e8400-e29b-41d4-a716-446655444041"),
        );
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655444042"));
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
}
