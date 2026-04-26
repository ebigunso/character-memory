#![allow(dead_code)]

use async_trait::async_trait;

use crate::api::types::{MemoryId, MemoryLink, MemoryObject, ObjectType};
use crate::errors::CustomError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphObjectQuery {
    pub(crate) object_ids: Vec<MemoryId>,
    pub(crate) object_types: Vec<ObjectType>,
    pub(crate) limit: Option<usize>,
}

impl GraphObjectQuery {
    pub(crate) fn by_ids(object_ids: Vec<MemoryId>) -> Self {
        Self {
            object_ids,
            object_types: Vec::new(),
            limit: None,
        }
    }

    pub(crate) fn by_types(object_types: Vec<ObjectType>, limit: Option<usize>) -> Self {
        Self {
            object_ids: Vec::new(),
            object_types,
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphExpansionQuery {
    pub(crate) root_id: MemoryId,
    pub(crate) root_type: ObjectType,
    pub(crate) max_depth: u8,
    pub(crate) max_nodes: usize,
    pub(crate) allowed_object_types: Vec<ObjectType>,
}

impl GraphExpansionQuery {
    pub(crate) fn new(
        root_id: MemoryId,
        root_type: ObjectType,
        max_depth: u8,
        max_nodes: usize,
    ) -> Self {
        Self {
            root_id,
            root_type,
            max_depth,
            max_nodes,
            allowed_object_types: Vec::new(),
        }
    }

    pub(crate) fn with_allowed_object_types(mut self, object_types: Vec<ObjectType>) -> Self {
        self.allowed_object_types = object_types;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GraphExpansion {
    pub(crate) objects: Vec<MemoryObject>,
    pub(crate) links: Vec<MemoryLink>,
}

impl GraphExpansion {
    pub(crate) fn new(objects: Vec<MemoryObject>, links: Vec<MemoryLink>) -> Self {
        Self { objects, links }
    }
}

#[async_trait]
pub(crate) trait GraphAuthorityStore: Send + Sync {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError>;

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError>;

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, CustomError>;

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError>;
}

#[async_trait]
impl<T: GraphAuthorityStore + ?Sized> GraphAuthorityStore for Box<T> {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        (**self).upsert_objects(objects).await
    }

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
        (**self).upsert_links(links).await
    }

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, CustomError> {
        (**self).query_objects(query).await
    }

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError> {
        (**self).expand_bounded(query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_queries_use_domain_ids_and_object_types() {
        let episode_id = MemoryId::new_v4();
        let by_ids = GraphObjectQuery::by_ids(vec![episode_id]);
        let by_types = GraphObjectQuery::by_types(vec![ObjectType::Episode], Some(5));

        assert_eq!(by_ids.object_ids, vec![episode_id]);
        assert_eq!(by_ids.object_types, Vec::<ObjectType>::new());
        assert_eq!(by_types.object_types, vec![ObjectType::Episode]);
        assert_eq!(by_types.limit, Some(5));
    }

    #[test]
    fn bounded_expansion_query_carries_explicit_limits() {
        let root_id = MemoryId::new_v4();
        let query = GraphExpansionQuery::new(root_id, ObjectType::Entity, 2, 25)
            .with_allowed_object_types(vec![ObjectType::Observation, ObjectType::DerivedMemory]);

        assert_eq!(query.root_id, root_id);
        assert_eq!(query.root_type, ObjectType::Entity);
        assert_eq!(query.max_depth, 2);
        assert_eq!(query.max_nodes, 25);
        assert_eq!(
            query.allowed_object_types,
            vec![ObjectType::Observation, ObjectType::DerivedMemory]
        );
    }

    #[test]
    fn graph_expansion_groups_objects_and_links_without_store_behavior() {
        let expansion = GraphExpansion::new(Vec::new(), Vec::new());

        assert!(expansion.objects.is_empty());
        assert!(expansion.links.is_empty());
    }
}
