// Graph authority contract: port trait plus its query/result value types.
// The bounded-expansion algorithm lives in crate::policy::graph_expansion.
// Embedded persistent Oxigraph is the application default; the in-memory
// store keeps tests and explicit fixture runs deterministic.
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    DerivedMemory, GraphFailureMode, MemoryId, MemoryLink, MemoryObject, MemoryObjectRef,
    ObjectType, RelationType, ScopeKey,
};
use crate::errors::{CustomError, GraphQueryError};

/// Compact selector metadata; content is hydrated only after admission.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GraphMemoryRank {
    pub(crate) id: MemoryId,
    pub(crate) time: DateTime<Utc>,
    pub(crate) salience: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::enum_variant_names,
    reason = "the design names the three closed query forms ByRefs, ByIds, and ByTypes"
)]
pub(crate) enum GraphObjectQuery {
    ByRefs(Vec<MemoryObjectRef>),
    ByIds(Vec<MemoryId>),
    ByTypes {
        object_types: Vec<ObjectType>,
        limit: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphDerivedMemoryProvenanceQuery {
    pub(crate) episode_ids: Vec<MemoryId>,
    pub(crate) observation_ids: Vec<MemoryId>,
    pub(crate) lifecycle_policy: GraphExpansionLifecyclePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphDerivedMemoryThreadQuery {
    pub(crate) thread_ids: Vec<MemoryId>,
    pub(crate) lifecycle_policy: GraphExpansionLifecyclePolicy,
}

impl GraphDerivedMemoryProvenanceQuery {
    pub(crate) fn by_sources(episode_ids: Vec<MemoryId>, observation_ids: Vec<MemoryId>) -> Self {
        Self {
            episode_ids,
            observation_ids,
            lifecycle_policy: GraphExpansionLifecyclePolicy::default(),
        }
    }

    pub(crate) fn with_lifecycle_policy(
        mut self,
        lifecycle_policy: GraphExpansionLifecyclePolicy,
    ) -> Self {
        self.lifecycle_policy = lifecycle_policy;
        self
    }
}

impl GraphDerivedMemoryThreadQuery {
    pub(crate) fn by_threads(thread_ids: Vec<MemoryId>) -> Self {
        Self {
            thread_ids,
            lifecycle_policy: GraphExpansionLifecyclePolicy::default(),
        }
    }

    pub(crate) fn with_lifecycle_policy(
        mut self,
        lifecycle_policy: GraphExpansionLifecyclePolicy,
    ) -> Self {
        self.lifecycle_policy = lifecycle_policy;
        self
    }
}

impl GraphObjectQuery {
    pub(crate) fn by_ids(object_ids: Vec<MemoryId>) -> Self {
        Self::ByIds(object_ids)
    }

    pub(crate) fn by_refs(object_refs: Vec<MemoryObjectRef>) -> Self {
        Self::ByRefs(object_refs)
    }

    pub(crate) fn by_types(object_types: Vec<ObjectType>, limit: Option<usize>) -> Self {
        Self::ByTypes {
            object_types,
            limit,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::ByRefs(object_refs) => object_refs.is_empty(),
            Self::ByIds(object_ids) => object_ids.is_empty(),
            Self::ByTypes { object_types, .. } => object_types.is_empty(),
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
    pub(crate) include_suppressed: bool,
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
    pub(crate) current_subject_state: bool,
    pub(crate) reminder_only: bool,
    pub(crate) participant_reference_time: DateTime<Utc>,
    // Only an occasion directly contributed by the caller's range can be future.
    pub(crate) allow_future_root: bool,
    pub(crate) current_thread_state: bool,
    // Hydrated lifecycle evidence may lie outside the adapter's selected traversal.
    pub(crate) traversal_link_ids: Option<std::collections::HashSet<MemoryId>>,
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
            current_subject_state: false,
            reminder_only: false,
            participant_reference_time: DateTime::<Utc>::MAX_UTC,
            allow_future_root: false,
            current_thread_state: false,
            traversal_link_ids: None,
            trace_mode: TraceMode::Disabled,
            lifecycle_policy: GraphExpansionLifecyclePolicy::default(),
            failure_policy: GraphExpansionFailurePolicy::default(),
        }
    }

    // Every reminder stays on its occasion; notions, threads and interpreted
    // work are visible leaves, never routes into their other occasions.
    pub(crate) fn may_continue_from(&self, kind: ObjectType) -> bool {
        !self.reminder_only
            || !matches!(
                kind,
                ObjectType::Entity | ObjectType::MemoryThread | ObjectType::DerivedMemory
            )
    }

    pub(crate) fn allows_object(&self, object: MemoryObjectRef) -> bool {
        (self.allowed_object_types.is_empty()
            || self.allowed_object_types.contains(&object.object_type))
            && (!self.reminder_only
                || object.object_type != ObjectType::Episode
                || object.id == self.root_id)
    }

    pub(crate) fn allows_incident_link(
        &self,
        from: MemoryObjectRef,
        relation: RelationType,
        to: MemoryObjectRef,
    ) -> bool {
        self.allows_object(to)
            && (!self.reminder_only
                || to.object_type != ObjectType::Observation
                || (relation == RelationType::ObservedIn
                    && from == MemoryObjectRef::new(ObjectType::Episode, self.root_id)))
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
    Suppressed,
    Superseded,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphExpansionFilteredNode {
    pub(crate) object_ref: MemoryObjectRef,
    pub(crate) reason: GraphExpansionFilteredReason,
    pub(crate) superseded_by: Vec<MemoryId>,
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
    /// Admitted objects in traversal order, before stable output sorting.
    pub(crate) selection_order: Vec<MemoryObjectRef>,
    pub(crate) links: Vec<MemoryLink>,
    pub(crate) relations: Vec<GraphExpansionRelation>,
    pub(crate) filtered_nodes: Vec<GraphExpansionFilteredNode>,
    pub(crate) resolved_by: std::collections::HashMap<MemoryId, Vec<MemoryId>>,
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
    #[cfg(test)]
    pub(crate) fn new(objects: Vec<MemoryObject>, links: Vec<MemoryLink>) -> Self {
        Self {
            selection_order: objects.iter().map(MemoryObject::object_ref).collect(),
            objects,
            links,
            relations: Vec::new(),
            filtered_nodes: Vec::new(),
            resolved_by: std::collections::HashMap::new(),
            expanded_nodes: std::collections::HashSet::new(),
            fanout_utilization: Vec::new(),
            bounded_failure: None,
        }
    }
}

#[async_trait]
pub(crate) trait GraphAuthorityStore: Send + Sync {
    /// Return bounded eligible episode rank rows, newest recorded time then ID.
    /// Both time bounds are inclusive; an absent start leaves that end open.
    async fn query_episodes_by_time(
        &self,
        start: Option<DateTime<Utc>>,
        end: DateTime<Utc>,
        limit: usize,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Vec<crate::ports::graph_authority::GraphMemoryRank>, CustomError>;

    /// Matching local calendar dates: each shared/unshared list has its own limit, newest instant then ID.
    async fn query_anniversaries(
        &self,
        date: chrono::NaiveDate,
        participants: &[MemoryId],
        limit: usize,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Vec<(crate::ports::graph_authority::GraphMemoryRank, bool)>, CustomError>;

    /// Read recorded occasion time and retention for episode or observation references.
    async fn query_episode_occasions(
        &self,
        episodes: &[MemoryObjectRef],
    ) -> Result<crate::policy::graph_expansion::ParticipantOccasions, CustomError>;

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
    ) -> Result<Vec<MemoryObject>, GraphQueryError>;

    /// Returns every notion with a current exact-name assertion; shared names remain ambiguous.
    #[allow(
        dead_code,
        reason = "the scene slice will consume exact-name notion cues"
    )]
    async fn query_notions_known_as(&self, name: &str) -> Result<Vec<MemoryId>, GraphQueryError>;

    /// Returns requested predecessors with an incoming interpreted-memory Supersedes link,
    /// including links from suppressed successors.
    async fn query_superseded_derived_memory_ids(
        &self,
        memory_ids: &[MemoryId],
    ) -> Result<Vec<MemoryId>, GraphQueryError>;

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
    ) -> Result<(Vec<DerivedMemory>, Vec<GraphExpansionFilteredNode>), CustomError>;

    /// Retrieval state has its own bounded read; lifecycle queries still see all members.
    async fn query_thread_state(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
        limit: usize,
    ) -> Result<(Vec<GraphMemoryRank>, Vec<GraphExpansionFilteredNode>), CustomError>;

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
    >;

    async fn query_last_interaction(
        &self,
        participant: MemoryId,
        reference_time: DateTime<Utc>,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Option<(MemoryId, DateTime<Utc>)>, CustomError>;

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError>;
}
