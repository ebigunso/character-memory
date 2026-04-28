// Transitional v0.1 link pipeline: later facade wiring will consume this
// internal service directly. Remove this allow once the public link surface
// exercises the service, or prune unused outcome helpers then.
#![allow(dead_code)]

use crate::api::types::{DraftDefaults, MemoryLink, MemoryLinkDraft};
use crate::errors::CustomError;
use crate::internal::repositories::GraphAuthorityStore;

pub(crate) struct LinkPipeline<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    graph_store: &'a G,
}

impl<'a, G> LinkPipeline<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    pub(crate) fn new(graph_store: &'a G) -> Self {
        Self { graph_store }
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
        let link = draft
            .into_domain_with_defaults(defaults)
            .map_err(validation_error)?;
        self.graph_store
            .upsert_links(std::slice::from_ref(&link))
            .await?;
        Ok(link)
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
        MemoryId, MemoryObject, ObjectType, RelationType, DEFAULT_SCHEMA_VERSION,
    };
    use crate::internal::repositories::test_support::{
        representative_fixtures, FakeGraphAuthorityStore,
    };
    use crate::internal::repositories::{GraphAuthorityStore, GraphExpansionQuery};

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

    fn id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-04-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }
}
