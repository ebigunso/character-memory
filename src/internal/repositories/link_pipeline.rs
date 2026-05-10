// Typed-link pipeline used by the public facade and internal tests.
use crate::api::types::{DraftDefaults, MemoryLink, MemoryLinkDraft, ObjectType};
use crate::errors::CustomError;
use crate::internal::repositories::{
    record_stats_after_write, GraphAuthorityStore, GraphObjectQuery, GraphObjectRef,
    RetrievalStatsStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkAdmissionEvidence {
    ExplicitCallerIntent,
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
            stats_store: crate::internal::repositories::noop_retrieval_stats_store(),
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

    pub(crate) async fn link(&self, draft: MemoryLinkDraft) -> Result<MemoryLink, CustomError> {
        let mut defaults = DraftDefaults::generated();
        self.link_with_defaults(draft, &mut defaults).await
    }

    pub(crate) async fn link_with_defaults(
        &self,
        draft: MemoryLinkDraft,
        defaults: &mut DraftDefaults,
    ) -> Result<MemoryLink, CustomError> {
        self.link_with_evidence(draft, defaults, LinkAdmissionEvidence::ExplicitCallerIntent)
            .await
    }

    async fn link_with_evidence(
        &self,
        draft: MemoryLinkDraft,
        defaults: &mut DraftDefaults,
        evidence: LinkAdmissionEvidence,
    ) -> Result<MemoryLink, CustomError> {
        let link = draft
            .into_domain_with_defaults(defaults)
            .map_err(validation_error)?;
        if admit_link(&link, evidence) == LinkAdmissionDecision::RejectedLowInformationCoOccurrence
        {
            if let Err(error) = self
                .stats_store
                .record_rejected_low_information_link()
                .await
            {
                let _ = self.stats_store.mark_unhealthy(error.to_string()).await;
            }
            return Err(CustomError::MemoryValidation(
                "low-information co-occurrence link rejected".to_owned(),
            ));
        }
        self.graph_store
            .upsert_links(std::slice::from_ref(&link))
            .await?;
        self.record_link_stats_after_write(&link).await;
        Ok(link)
    }

    async fn record_link_stats_after_write(&self, link: &MemoryLink) {
        let endpoint_refs = link_stats_endpoint_refs(link);
        if endpoint_refs.is_empty() {
            record_stats_after_write(self.stats_store, &[], std::slice::from_ref(link)).await;
            return;
        }

        match self
            .graph_store
            .query_objects(&GraphObjectQuery::by_refs(endpoint_refs))
            .await
        {
            Ok(objects) => {
                record_stats_after_write(self.stats_store, &objects, std::slice::from_ref(link))
                    .await;
            }
            Err(error) => {
                let _ = self.stats_store.mark_unhealthy(error.to_string()).await;
            }
        }
    }
}

fn link_stats_endpoint_refs(link: &MemoryLink) -> Vec<GraphObjectRef> {
    let mut refs = Vec::new();
    if object_type_has_stats_state(link.from_type) {
        refs.push(GraphObjectRef::new(link.from_id, link.from_type));
    }
    if object_type_has_stats_state(link.to_type) {
        refs.push(GraphObjectRef::new(link.to_id, link.to_type));
    }
    refs
}

fn object_type_has_stats_state(object_type: ObjectType) -> bool {
    matches!(
        object_type,
        ObjectType::Episode | ObjectType::Observation | ObjectType::DerivedMemory
    )
}

fn admit_link(link: &MemoryLink, evidence: LinkAdmissionEvidence) -> LinkAdmissionDecision {
    if link.relation == crate::api::types::RelationType::AssociatedWith
        && evidence == LinkAdmissionEvidence::LowSelectivityCoOccurrenceOnly
    {
        LinkAdmissionDecision::RejectedLowInformationCoOccurrence
    } else {
        LinkAdmissionDecision::Accepted
    }
}

fn validation_error(error: impl ToString) -> CustomError {
    CustomError::MemoryValidation(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use crate::api::types::{
        MemoryId, MemoryObject, ObjectType, RelationType, RetentionState, DEFAULT_SCHEMA_VERSION,
    };
    use crate::internal::repositories::test_support::{
        representative_fixtures, FakeGraphAuthorityStore,
    };
    use crate::internal::repositories::{
        GraphAuthorityStore, GraphExpansionQuery, InMemoryRetrievalStatsStore,
        RetrievalStatsCounterKey, RetrievalStatsStore,
    };

    #[tokio::test]
    async fn persists_caller_supplied_link_as_graph_authoritative_record() {
        let graph = FakeGraphAuthorityStore::new();
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
            .expect("valid typed link should persist");

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
        let graph = FakeGraphAuthorityStore::new();
        let pipeline = LinkPipeline::new(&graph);
        let mut draft = valid_link_draft();
        draft.confidence = 1.1;

        let error = pipeline.link(draft).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("MemoryLink.confidence must be in 0.0..=1.0"));
    }

    #[tokio::test]
    async fn rejects_self_links_before_graph_write() {
        let graph = FakeGraphAuthorityStore::new();
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

        assert!(error
            .to_string()
            .contains("cannot point from an object to itself"));
    }

    #[tokio::test]
    async fn rejects_memory_link_endpoints_before_graph_write() {
        let graph = FakeGraphAuthorityStore::new();
        let pipeline = LinkPipeline::new(&graph);
        let draft = MemoryLinkDraft::new(
            ObjectType::MemoryLink,
            id("550e8400-e29b-41d4-a716-446655444020"),
            RelationType::AssociatedWith,
            ObjectType::Entity,
            id("550e8400-e29b-41d4-a716-446655444021"),
        );

        let error = pipeline.link(draft).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("cannot point at MemoryLink endpoints"));
    }

    #[tokio::test]
    async fn link_pipeline_uses_graph_store_only() {
        let graph = FakeGraphAuthorityStore::new();
        let pipeline = LinkPipeline::new(&graph);

        let persisted = pipeline.link(valid_link_draft()).await.unwrap();

        assert_eq!(persisted.object_type, ObjectType::MemoryLink);
    }

    #[tokio::test]
    async fn link_pipeline_records_entity_relation_stats_after_graph_success() {
        let graph = FakeGraphAuthorityStore::new();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);
        let draft = valid_link_draft();
        let entity_id = draft.to_id;

        let persisted = pipeline.link(draft).await.unwrap();

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
        let graph = FakeGraphAuthorityStore::new();
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

        let persisted = pipeline.link(draft).await.unwrap();

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
        let graph = FakeGraphAuthorityStore::new();
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

        assert!(error
            .to_string()
            .contains("low-information co-occurrence link rejected"));
        assert_eq!(
            stats.rejected_low_information_link_count().await.unwrap(),
            1
        );
        assert!(graph.list_diagnostic_links().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn explicit_intent_allows_associated_with_links_by_default() {
        let graph = FakeGraphAuthorityStore::new();
        let stats = InMemoryRetrievalStatsStore::new();
        let pipeline = LinkPipeline::new_with_stats(&graph, &stats);

        let persisted = pipeline.link(associated_with_link_draft()).await.unwrap();

        assert_eq!(persisted.relation, RelationType::AssociatedWith);
        assert_eq!(
            stats.rejected_low_information_link_count().await.unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn entity_neutral_low_information_guard_does_not_check_roles() {
        for evidence in [
            LinkAdmissionEvidence::LowSelectivityCoOccurrenceOnly,
            LinkAdmissionEvidence::ExplicitCallerIntent,
        ] {
            let decision = admit_link(&associated_with_link(), evidence);
            assert_eq!(
                decision,
                if evidence == LinkAdmissionEvidence::LowSelectivityCoOccurrenceOnly {
                    LinkAdmissionDecision::RejectedLowInformationCoOccurrence
                } else {
                    LinkAdmissionDecision::Accepted
                }
            );
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

    fn associated_with_link() -> MemoryLink {
        let mut defaults = DraftDefaults::at(timestamp());
        associated_with_link_draft()
            .into_domain_with_defaults(&mut defaults)
            .unwrap()
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
