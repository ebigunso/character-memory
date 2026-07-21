// Graph authority contract: port trait plus its query/result value types.
// The bounded-expansion algorithm lives in crate::policy::graph_expansion.
// Embedded persistent Oxigraph is the application default; in-memory and fake
// stores keep tests and explicit fixture runs deterministic.
use async_trait::async_trait;

use crate::domain::{
    DerivedMemory, GraphFailureMode, MemoryId, MemoryLink, MemoryObject, MemoryObjectRef,
    ObjectType, RelationType,
};
use crate::errors::CustomError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphObjectQuery {
    pub(crate) object_refs: Vec<MemoryObjectRef>,
    pub(crate) object_ids: Vec<MemoryId>,
    pub(crate) object_types: Vec<ObjectType>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphDerivedMemoryProvenanceQuery {
    pub(crate) episode_ids: Vec<MemoryId>,
    pub(crate) observation_ids: Vec<MemoryId>,
    pub(crate) lifecycle_policy: GraphExpansionLifecyclePolicy,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphDerivedMemoryThreadQuery {
    pub(crate) thread_ids: Vec<MemoryId>,
    pub(crate) lifecycle_policy: GraphExpansionLifecyclePolicy,
    pub(crate) limit: Option<usize>,
}

impl GraphDerivedMemoryProvenanceQuery {
    pub(crate) fn by_sources(episode_ids: Vec<MemoryId>, observation_ids: Vec<MemoryId>) -> Self {
        Self {
            episode_ids,
            observation_ids,
            lifecycle_policy: GraphExpansionLifecyclePolicy::default(),
            limit: None,
        }
    }

    pub(crate) fn with_lifecycle_policy(
        mut self,
        lifecycle_policy: GraphExpansionLifecyclePolicy,
    ) -> Self {
        self.lifecycle_policy = lifecycle_policy;
        self
    }

    // Provenance queries reserve limit support for governance diagnostics; remove if that surface drops limits.
    #[allow(dead_code)]
    pub(crate) fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl GraphDerivedMemoryThreadQuery {
    pub(crate) fn by_threads(thread_ids: Vec<MemoryId>) -> Self {
        Self {
            thread_ids,
            lifecycle_policy: GraphExpansionLifecyclePolicy::default(),
            limit: None,
        }
    }

    pub(crate) fn with_lifecycle_policy(
        mut self,
        lifecycle_policy: GraphExpansionLifecyclePolicy,
    ) -> Self {
        self.lifecycle_policy = lifecycle_policy;
        self
    }

    // Thread queries reserve limit support for governance diagnostics; remove if that surface drops limits.
    #[allow(dead_code)]
    pub(crate) fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl GraphObjectQuery {
    // Diagnostics and fakes need ID-only graph lookup; remove when all callers use typed refs.
    #[allow(dead_code)]
    pub(crate) fn by_ids(object_ids: Vec<MemoryId>) -> Self {
        Self {
            object_refs: Vec::new(),
            object_ids,
            object_types: Vec::new(),
            limit: None,
        }
    }

    pub(crate) fn by_refs(object_refs: Vec<MemoryObjectRef>) -> Self {
        Self {
            object_refs,
            object_ids: Vec::new(),
            object_types: Vec::new(),
            limit: None,
        }
    }

    pub(crate) fn by_types(object_types: Vec<ObjectType>, limit: Option<usize>) -> Self {
        Self {
            object_refs: Vec::new(),
            object_ids: Vec::new(),
            object_types,
            limit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphExpansionFailurePolicy {
    pub(crate) timeout_ms: Option<u64>,
    pub(crate) mode: GraphFailureMode,
}

impl Default for GraphExpansionFailurePolicy {
    fn default() -> Self {
        Self {
            timeout_ms: Some(250),
            mode: GraphFailureMode::AllowPartialResults,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum TraceMode {
    #[default]
    Disabled,
    Enabled,
}

impl TraceMode {
    pub(crate) const fn from_enabled(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }

    pub(crate) const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct GraphExpansionLifecyclePolicy {
    pub(crate) include_archived: bool,
    pub(crate) include_suppressed: bool,
    pub(crate) include_deleted: bool,
    pub(crate) include_non_current: bool,
    pub(crate) include_superseded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphExpansionQuery {
    pub(crate) root_id: MemoryId,
    pub(crate) root_type: ObjectType,
    pub(crate) max_depth: u8,
    pub(crate) max_nodes: usize,
    pub(crate) max_fanout_per_node: usize,
    pub(crate) max_hub_edges: usize,
    pub(crate) allowed_object_types: Vec<ObjectType>,
    pub(crate) allowed_relation_types: Vec<RelationType>,
    pub(crate) fanout_overrides: Vec<GraphExpansionFanoutOverride>,
    pub(crate) trace_mode: TraceMode,
    pub(crate) lifecycle_policy: GraphExpansionLifecyclePolicy,
    pub(crate) failure_policy: GraphExpansionFailurePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphExpansionFanoutOverride {
    pub(crate) relation: RelationType,
    pub(crate) object_type: ObjectType,
    pub(crate) max_fanout: usize,
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
            max_fanout_per_node: usize::MAX,
            max_hub_edges: usize::MAX,
            allowed_object_types: Vec::new(),
            allowed_relation_types: Vec::new(),
            fanout_overrides: Vec::new(),
            trace_mode: TraceMode::Disabled,
            lifecycle_policy: GraphExpansionLifecyclePolicy::default(),
            failure_policy: GraphExpansionFailurePolicy::default(),
        }
    }

    pub(crate) fn with_allowed_object_types(mut self, object_types: Vec<ObjectType>) -> Self {
        self.allowed_object_types = object_types;
        self
    }

    pub(crate) fn with_allowed_relation_types(mut self, relation_types: Vec<RelationType>) -> Self {
        self.allowed_relation_types = relation_types;
        self
    }

    pub(crate) fn with_fanout_overrides(
        mut self,
        fanout_overrides: Vec<GraphExpansionFanoutOverride>,
    ) -> Self {
        self.fanout_overrides = fanout_overrides;
        self
    }

    pub(crate) fn with_fanout_utilization_recording(mut self, trace_mode: TraceMode) -> Self {
        self.trace_mode = trace_mode;
        self
    }

    pub(crate) fn with_max_fanout_per_node(mut self, max_fanout_per_node: usize) -> Self {
        self.max_fanout_per_node = max_fanout_per_node;
        self
    }

    pub(crate) fn with_max_hub_edges(mut self, max_hub_edges: usize) -> Self {
        self.max_hub_edges = max_hub_edges;
        self
    }

    pub(crate) fn with_lifecycle_policy(
        mut self,
        lifecycle_policy: GraphExpansionLifecyclePolicy,
    ) -> Self {
        self.lifecycle_policy = lifecycle_policy;
        self
    }

    pub(crate) fn with_failure_policy(
        mut self,
        failure_policy: GraphExpansionFailurePolicy,
    ) -> Self {
        self.failure_policy = failure_policy;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphExpansionFilteredReason {
    Archived,
    Suppressed,
    Deleted,
    NonCurrent,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphExpansionFilteredNode {
    pub(crate) object_ref: MemoryObjectRef,
    pub(crate) reason: GraphExpansionFilteredReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphExpansionBoundedFailureReason {
    NodeLimit,
    Timeout,
    HubLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphExpansionBoundedFailure {
    pub(crate) reason: GraphExpansionBoundedFailureReason,
    pub(crate) at: Option<MemoryObjectRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphExpansionRelation {
    pub(crate) link_id: MemoryId,
    pub(crate) from: MemoryObjectRef,
    pub(crate) to: MemoryObjectRef,
    pub(crate) relation: RelationType,
    pub(crate) proximity: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GraphExpansion {
    pub(crate) objects: Vec<MemoryObject>,
    pub(crate) links: Vec<MemoryLink>,
    pub(crate) relations: Vec<GraphExpansionRelation>,
    pub(crate) filtered_nodes: Vec<GraphExpansionFilteredNode>,
    pub(crate) expanded_nodes: std::collections::HashSet<MemoryObjectRef>,
    pub(crate) fanout_utilization: Vec<GraphExpansionFanoutUtilization>,
    pub(crate) bounded_failure: Option<GraphExpansionBoundedFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphExpansionFanoutUtilization {
    pub(crate) root: MemoryObjectRef,
    pub(crate) relation: RelationType,
    pub(crate) object_type: ObjectType,
    pub(crate) configured_cap: usize,
    pub(crate) selected_cap: usize,
    pub(crate) retained_count: usize,
    pub(crate) omitted_by_fanout_count: usize,
}

impl GraphExpansion {
    // Tests and future diagnostics need a minimal expansion constructor; remove when from_plan covers all callers.
    #[allow(dead_code)]
    pub(crate) fn new(objects: Vec<MemoryObject>, links: Vec<MemoryLink>) -> Self {
        Self {
            objects,
            links,
            relations: Vec::new(),
            filtered_nodes: Vec::new(),
            expanded_nodes: std::collections::HashSet::new(),
            fanout_utilization: Vec::new(),
            bounded_failure: None,
        }
    }

    pub(crate) fn from_plan(
        objects: Vec<MemoryObject>,
        links: Vec<MemoryLink>,
        relations: Vec<GraphExpansionRelation>,
        filtered_nodes: Vec<GraphExpansionFilteredNode>,
        expanded_nodes: std::collections::HashSet<MemoryObjectRef>,
        fanout_utilization: Vec<GraphExpansionFanoutUtilization>,
        bounded_failure: Option<GraphExpansionBoundedFailure>,
    ) -> Self {
        Self {
            objects,
            links,
            relations,
            filtered_nodes,
            expanded_nodes,
            fanout_utilization,
            bounded_failure,
        }
    }
}

#[async_trait]
pub(crate) trait GraphAuthorityStore: Send + Sync {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError>;

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError>;

    async fn upsert_objects_and_links(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), CustomError>;

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, CustomError>;

    async fn query_links_by_ids(
        &self,
        link_ids: &[MemoryId],
    ) -> Result<Vec<MemoryLink>, CustomError>;

    async fn query_derived_memories_by_provenance(
        &self,
        query: &GraphDerivedMemoryProvenanceQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError>;

    async fn query_derived_memories_by_thread(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError>;

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError>;

    // Governance reconciliation is dormant; remove when diagnostic object listing is no longer part of the port.
    #[allow(dead_code)]
    async fn list_diagnostic_objects(&self) -> Result<Vec<MemoryObject>, CustomError>;

    async fn list_diagnostic_links(&self) -> Result<Vec<MemoryLink>, CustomError>;
}

#[async_trait]
impl<T: GraphAuthorityStore + ?Sized> GraphAuthorityStore for Box<T> {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        (**self).upsert_objects(objects).await
    }

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
        (**self).upsert_links(links).await
    }

    async fn upsert_objects_and_links(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), CustomError> {
        (**self).upsert_objects_and_links(objects, links).await
    }

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, CustomError> {
        (**self).query_objects(query).await
    }

    async fn query_links_by_ids(
        &self,
        link_ids: &[MemoryId],
    ) -> Result<Vec<MemoryLink>, CustomError> {
        (**self).query_links_by_ids(link_ids).await
    }

    async fn query_derived_memories_by_provenance(
        &self,
        query: &GraphDerivedMemoryProvenanceQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        (**self).query_derived_memories_by_provenance(query).await
    }

    async fn query_derived_memories_by_thread(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        (**self).query_derived_memories_by_thread(query).await
    }

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError> {
        (**self).expand_bounded(query).await
    }

    async fn list_diagnostic_objects(&self) -> Result<Vec<MemoryObject>, CustomError> {
        (**self).list_diagnostic_objects().await
    }

    async fn list_diagnostic_links(&self) -> Result<Vec<MemoryLink>, CustomError> {
        (**self).list_diagnostic_links().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_queries_use_domain_ids_and_object_types() {
        let episode_id = MemoryId::new_v4();
        let by_ids = GraphObjectQuery::by_ids(vec![episode_id]);
        let by_refs = GraphObjectQuery::by_refs(vec![MemoryObjectRef::from_id_type(
            episode_id,
            ObjectType::Episode,
        )]);
        let by_types = GraphObjectQuery::by_types(vec![ObjectType::Episode], Some(5));

        assert_eq!(by_ids.object_ids, vec![episode_id]);
        assert_eq!(by_ids.object_types, Vec::<ObjectType>::new());
        assert_eq!(by_refs.object_refs[0].id, episode_id);
        assert_eq!(by_refs.object_refs[0].object_type, ObjectType::Episode);
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
        assert_eq!(query.max_fanout_per_node, usize::MAX);
        assert_eq!(query.max_hub_edges, usize::MAX);
        assert_eq!(query.trace_mode, TraceMode::Disabled);
        assert_eq!(
            query.allowed_object_types,
            vec![ObjectType::Observation, ObjectType::DerivedMemory]
        );
        assert_eq!(
            query.lifecycle_policy,
            GraphExpansionLifecyclePolicy::default()
        );
        assert_eq!(query.failure_policy, GraphExpansionFailurePolicy::default());
    }

    #[test]
    fn graph_expansion_groups_objects_and_links_without_store_behavior() {
        let expansion = GraphExpansion::new(Vec::new(), Vec::new());

        assert!(expansion.objects.is_empty());
        assert!(expansion.links.is_empty());
    }
}
