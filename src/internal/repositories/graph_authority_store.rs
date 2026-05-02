// Graph authority contract and provider-neutral expansion helpers.
// Oxigraph service mode is the application default; embedded and fake stores
// keep tests and explicit fixture runs deterministic.
#![allow(dead_code)]

use async_trait::async_trait;
use std::collections::{HashSet, VecDeque};

use crate::api::types::{
    DerivedMemory, MemoryId, MemoryLink, MemoryObject, ObjectType, RelationType, RetentionState,
    ThreadStatus,
};
use crate::errors::CustomError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GraphObjectRef {
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
}

impl GraphObjectRef {
    pub(crate) const fn new(object_id: MemoryId, object_type: ObjectType) -> Self {
        Self {
            object_id,
            object_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphObjectQuery {
    pub(crate) object_refs: Vec<GraphObjectRef>,
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

    pub(crate) fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl GraphObjectQuery {
    pub(crate) fn by_ids(object_ids: Vec<MemoryId>) -> Self {
        Self {
            object_refs: Vec::new(),
            object_ids,
            object_types: Vec::new(),
            limit: None,
        }
    }

    pub(crate) fn by_refs(object_refs: Vec<GraphObjectRef>) -> Self {
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
    pub(crate) allow_partial_results: bool,
}

impl Default for GraphExpansionFailurePolicy {
    fn default() -> Self {
        Self {
            timeout_ms: Some(250),
            allow_partial_results: true,
        }
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
    pub(crate) lifecycle_policy: GraphExpansionLifecyclePolicy,
    pub(crate) failure_policy: GraphExpansionFailurePolicy,
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
    pub(crate) object_ref: GraphObjectRef,
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
    pub(crate) at: Option<GraphObjectRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphExpansionRelation {
    pub(crate) link_id: MemoryId,
    pub(crate) from: GraphObjectRef,
    pub(crate) to: GraphObjectRef,
    pub(crate) relation: RelationType,
    pub(crate) proximity: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GraphExpansion {
    pub(crate) objects: Vec<MemoryObject>,
    pub(crate) links: Vec<MemoryLink>,
    pub(crate) relations: Vec<GraphExpansionRelation>,
    pub(crate) filtered_nodes: Vec<GraphExpansionFilteredNode>,
    pub(crate) bounded_failure: Option<GraphExpansionBoundedFailure>,
}

impl GraphExpansion {
    pub(crate) fn new(objects: Vec<MemoryObject>, links: Vec<MemoryLink>) -> Self {
        Self {
            objects,
            links,
            relations: Vec::new(),
            filtered_nodes: Vec::new(),
            bounded_failure: None,
        }
    }

    fn from_plan(
        objects: Vec<MemoryObject>,
        links: Vec<MemoryLink>,
        relations: Vec<GraphExpansionRelation>,
        filtered_nodes: Vec<GraphExpansionFilteredNode>,
        bounded_failure: Option<GraphExpansionBoundedFailure>,
    ) -> Self {
        Self {
            objects,
            links,
            relations,
            filtered_nodes,
            bounded_failure,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoundedExpansionPlan {
    pub(crate) visited: HashSet<GraphObjectRef>,
    pub(crate) relations: Vec<GraphExpansionRelation>,
    pub(crate) filtered_nodes: Vec<GraphExpansionFilteredNode>,
    pub(crate) bounded_failure: Option<GraphExpansionBoundedFailure>,
}

pub(crate) fn bounded_expansion(
    query: &GraphExpansionQuery,
    objects: impl IntoIterator<Item = MemoryObject>,
    links: impl IntoIterator<Item = MemoryLink>,
) -> Result<GraphExpansion, CustomError> {
    let objects = objects.into_iter().collect::<Vec<_>>();
    let links = links.into_iter().collect::<Vec<_>>();
    let plan = bounded_expansion_plan(query, objects.iter(), links.iter())?;

    let mut expanded_objects: Vec<_> = objects
        .into_iter()
        .filter(|object| plan.visited.contains(&graph_object_ref(object)))
        .collect();
    sort_objects(&mut expanded_objects);

    let traversed_link_ids = plan
        .relations
        .iter()
        .map(|relation| relation.link_id)
        .collect::<HashSet<_>>();
    let mut expanded_links: Vec<_> = links
        .into_iter()
        .filter(|link| {
            traversed_link_ids.contains(&link.id)
                && plan
                    .visited
                    .contains(&GraphObjectRef::new(link.from_id, link.from_type))
                && plan
                    .visited
                    .contains(&GraphObjectRef::new(link.to_id, link.to_type))
        })
        .collect();
    expanded_links.sort_by_key(|link| link.id);

    Ok(GraphExpansion::from_plan(
        expanded_objects,
        expanded_links,
        plan.relations,
        plan.filtered_nodes,
        plan.bounded_failure,
    ))
}

pub(crate) fn derived_memories_by_provenance(
    query: &GraphDerivedMemoryProvenanceQuery,
    objects: impl IntoIterator<Item = MemoryObject>,
    links: impl IntoIterator<Item = MemoryLink>,
) -> Vec<DerivedMemory> {
    let episode_ids = query.episode_ids.iter().copied().collect::<HashSet<_>>();
    let observation_ids = query
        .observation_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let links = links.into_iter().collect::<Vec<_>>();
    let link_refs = links.iter().collect::<Vec<_>>();
    let superseded = superseded_derived_memory_ids(&link_refs);
    let provenance_linked =
        provenance_linked_derived_memory_ids(&episode_ids, &observation_ids, &links);
    let mut memories = objects
        .into_iter()
        .filter_map(|object| match object {
            MemoryObject::DerivedMemory(memory) => Some(memory),
            _ => None,
        })
        .filter(|memory| {
            derived_memory_matches_provenance(
                &episode_ids,
                &observation_ids,
                memory,
                &provenance_linked,
            )
        })
        .filter(|memory| {
            derived_memory_lifecycle_filter_reason(memory, &superseded, query.lifecycle_policy)
                .is_none()
        })
        .collect::<Vec<_>>();

    memories.sort_by_key(|memory| memory.id);
    if let Some(limit) = query.limit {
        memories.truncate(limit);
    }
    memories
}

pub(crate) fn derived_memories_by_thread(
    query: &GraphDerivedMemoryThreadQuery,
    objects: impl IntoIterator<Item = MemoryObject>,
    links: impl IntoIterator<Item = MemoryLink>,
) -> Vec<DerivedMemory> {
    let thread_ids = query.thread_ids.iter().copied().collect::<HashSet<_>>();
    let links = links.into_iter().collect::<Vec<_>>();
    let link_refs = links.iter().collect::<Vec<_>>();
    let superseded = superseded_derived_memory_ids(&link_refs);
    let mut memories = objects
        .into_iter()
        .filter_map(|object| match object {
            MemoryObject::DerivedMemory(memory) => Some(memory),
            _ => None,
        })
        .filter(|memory| {
            !thread_ids.is_empty()
                && memory
                    .thread_ids
                    .iter()
                    .any(|thread_id| thread_ids.contains(thread_id))
        })
        .filter(|memory| {
            derived_memory_lifecycle_filter_reason(memory, &superseded, query.lifecycle_policy)
                .is_none()
        })
        .collect::<Vec<_>>();

    memories.sort_by_key(|memory| memory.id);
    if let Some(limit) = query.limit {
        memories.truncate(limit);
    }
    memories
}

fn derived_memory_matches_provenance(
    episode_ids: &HashSet<MemoryId>,
    observation_ids: &HashSet<MemoryId>,
    memory: &DerivedMemory,
    provenance_linked: &HashSet<MemoryId>,
) -> bool {
    (!episode_ids.is_empty()
        && memory
            .derived_from_episode_ids
            .iter()
            .any(|episode_id| episode_ids.contains(episode_id)))
        || (!observation_ids.is_empty()
            && memory
                .derived_from_observation_ids
                .iter()
                .any(|observation_id| observation_ids.contains(observation_id)))
        || provenance_linked.contains(&memory.id)
}

fn provenance_linked_derived_memory_ids(
    episode_ids: &HashSet<MemoryId>,
    observation_ids: &HashSet<MemoryId>,
    links: &[MemoryLink],
) -> HashSet<MemoryId> {
    links
        .iter()
        .filter(|link| link.relation == RelationType::DerivedFrom)
        .filter_map(|link| provenance_linked_derived_memory_id(episode_ids, observation_ids, link))
        .collect()
}

fn provenance_linked_derived_memory_id(
    episode_ids: &HashSet<MemoryId>,
    observation_ids: &HashSet<MemoryId>,
    link: &MemoryLink,
) -> Option<MemoryId> {
    let from = GraphObjectRef::new(link.from_id, link.from_type);
    let to = GraphObjectRef::new(link.to_id, link.to_type);
    match (from.object_type, to.object_type) {
        (ObjectType::DerivedMemory, ObjectType::Episode) if episode_ids.contains(&to.object_id) => {
            Some(from.object_id)
        }
        (ObjectType::Episode, ObjectType::DerivedMemory)
            if episode_ids.contains(&from.object_id) =>
        {
            Some(to.object_id)
        }
        (ObjectType::DerivedMemory, ObjectType::Observation)
            if observation_ids.contains(&to.object_id) =>
        {
            Some(from.object_id)
        }
        (ObjectType::Observation, ObjectType::DerivedMemory)
            if observation_ids.contains(&from.object_id) =>
        {
            Some(to.object_id)
        }
        _ => None,
    }
}

pub(crate) fn bounded_expansion_node_set(
    query: &GraphExpansionQuery,
    root_exists: bool,
    links: impl IntoIterator<Item = MemoryLink>,
) -> Result<HashSet<(MemoryId, ObjectType)>, CustomError> {
    if query.max_fanout_per_node != usize::MAX
        || query.max_hub_edges != usize::MAX
        || !query.allowed_relation_types.is_empty()
        || query.lifecycle_policy != GraphExpansionLifecyclePolicy::default()
        || query.failure_policy != GraphExpansionFailurePolicy::default()
    {
        return Err(CustomError::MemoryValidation(
            "bounded_expansion_node_set only supports basic depth/node/object-type bounds"
                .to_owned(),
        ));
    }

    if query.root_type == ObjectType::MemoryLink {
        return Err(CustomError::MemoryValidation(
            "bounded graph expansion does not support MemoryLink roots".to_owned(),
        ));
    }

    if !root_exists {
        return Err(CustomError::GraphExpansionRootNotFound {
            object_type: query.root_type,
            object_id: query.root_id,
        });
    }

    if query.max_nodes == 0 {
        return Ok(HashSet::new());
    }

    let links = links.into_iter().collect::<Vec<_>>();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::from([(query.root_id, query.root_type, 0_u8)]);

    while let Some((object_id, object_type, depth)) = queue.pop_front() {
        if visited.len() >= query.max_nodes || !visited.insert((object_id, object_type)) {
            continue;
        }

        if depth >= query.max_depth {
            continue;
        }

        let mut neighbors: Vec<_> = links
            .iter()
            .filter_map(|link| {
                if link.from_id == object_id && link.from_type == object_type {
                    Some((link.to_id, link.to_type))
                } else if link.to_id == object_id && link.to_type == object_type {
                    Some((link.from_id, link.from_type))
                } else {
                    None
                }
            })
            .filter(|(_, neighbor_type)| {
                query.allowed_object_types.is_empty()
                    || query.allowed_object_types.contains(neighbor_type)
            })
            .collect();
        neighbors.sort_by_key(|node| stable_node_key(*node));

        for neighbor in neighbors {
            if visited.len() + queue.len() >= query.max_nodes && !visited.contains(&neighbor) {
                continue;
            }
            queue.push_back((neighbor.0, neighbor.1, depth + 1));
        }
    }

    Ok(visited)
}

fn bounded_expansion_plan<'a>(
    query: &GraphExpansionQuery,
    objects: impl IntoIterator<Item = &'a MemoryObject>,
    links: impl IntoIterator<Item = &'a MemoryLink>,
) -> Result<BoundedExpansionPlan, CustomError> {
    if query.root_type == ObjectType::MemoryLink {
        return Err(CustomError::MemoryValidation(
            "bounded graph expansion does not support MemoryLink roots".to_owned(),
        ));
    }

    let objects = objects.into_iter().collect::<Vec<_>>();
    let links = links.into_iter().collect::<Vec<_>>();
    let object_refs = objects
        .iter()
        .map(|object| graph_object_ref(object))
        .collect::<HashSet<_>>();
    let root = GraphObjectRef::new(query.root_id, query.root_type);

    if !object_refs.contains(&root) {
        return Err(CustomError::GraphExpansionRootNotFound {
            object_type: query.root_type,
            object_id: query.root_id,
        });
    }

    if let Some(0) = query.failure_policy.timeout_ms {
        let bounded_failure = GraphExpansionBoundedFailure {
            reason: GraphExpansionBoundedFailureReason::Timeout,
            at: Some(root),
        };
        if query.failure_policy.allow_partial_results {
            return Ok(BoundedExpansionPlan {
                visited: HashSet::new(),
                relations: Vec::new(),
                filtered_nodes: Vec::new(),
                bounded_failure: Some(bounded_failure),
            });
        }
        return Err(graph_expansion_bounded_error(bounded_failure));
    }

    if query.max_nodes == 0 {
        let bounded_failure = GraphExpansionBoundedFailure {
            reason: GraphExpansionBoundedFailureReason::NodeLimit,
            at: Some(root),
        };
        if !query.failure_policy.allow_partial_results {
            return Err(graph_expansion_bounded_error(bounded_failure));
        }
        return Ok(BoundedExpansionPlan {
            visited: HashSet::new(),
            relations: Vec::new(),
            filtered_nodes: Vec::new(),
            bounded_failure: Some(bounded_failure),
        });
    }

    let superseded = superseded_derived_memory_ids(&links);
    let object_lifecycle = objects
        .iter()
        .map(|object| {
            let object_ref = graph_object_ref(object);
            (
                object_ref,
                lifecycle_filter_reason(object, &superseded, query.lifecycle_policy),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let mut visited = HashSet::new();
    let mut filtered_nodes = Vec::new();
    let mut relations = Vec::new();
    let mut bounded_failure = None;
    let mut relation_link_ids = HashSet::new();
    let mut queued = HashSet::from([root]);
    let mut queue = VecDeque::from([(root, 0_u8)]);

    while let Some((object_ref, depth)) = queue.pop_front() {
        queued.remove(&object_ref);
        if visited.contains(&object_ref) {
            continue;
        }
        if visited.len() >= query.max_nodes {
            let failure = GraphExpansionBoundedFailure {
                reason: GraphExpansionBoundedFailureReason::NodeLimit,
                at: Some(object_ref),
            };
            if !query.failure_policy.allow_partial_results {
                return Err(graph_expansion_bounded_error(failure));
            }
            bounded_failure.get_or_insert(failure);
            continue;
        }

        if let Some(reason) = object_lifecycle.get(&object_ref).copied().flatten() {
            push_filtered_node(&mut filtered_nodes, object_ref, reason);
            continue;
        }

        visited.insert(object_ref);

        if depth >= query.max_depth {
            continue;
        }

        let mut incident_links = links
            .iter()
            .filter(|link| relation_allowed(query, link.relation))
            .filter(|link| link_touches_ref(link, object_ref))
            .filter_map(|link| {
                let neighbor = other_endpoint(link, object_ref);
                if object_refs.contains(&neighbor)
                    && object_type_allowed(query, neighbor.object_type)
                {
                    Some((*link, neighbor))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        incident_links.sort_by_key(|(link, _)| stable_link_key(link));

        if incident_links.len() > query.max_hub_edges {
            let failure = GraphExpansionBoundedFailure {
                reason: GraphExpansionBoundedFailureReason::HubLimit,
                at: Some(object_ref),
            };
            if !query.failure_policy.allow_partial_results {
                return Err(graph_expansion_bounded_error(failure));
            }
            bounded_failure.get_or_insert(failure);
            incident_links.truncate(query.max_fanout_per_node.min(query.max_hub_edges));
        } else {
            incident_links.truncate(query.max_fanout_per_node);
        }

        for (link, neighbor) in incident_links {
            if relation_link_ids.insert(link.id) {
                relations.push(GraphExpansionRelation {
                    link_id: link.id,
                    from: GraphObjectRef::new(link.from_id, link.from_type),
                    to: GraphObjectRef::new(link.to_id, link.to_type),
                    relation: link.relation,
                    proximity: depth.saturating_add(1),
                });
            }

            if let Some(reason) = object_lifecycle.get(&neighbor).copied().flatten() {
                push_filtered_node(&mut filtered_nodes, neighbor, reason);
                continue;
            }

            if visited.len() + queued.len() >= query.max_nodes
                && !visited.contains(&neighbor)
                && !queued.contains(&neighbor)
            {
                let failure = GraphExpansionBoundedFailure {
                    reason: GraphExpansionBoundedFailureReason::NodeLimit,
                    at: Some(neighbor),
                };
                if !query.failure_policy.allow_partial_results {
                    return Err(graph_expansion_bounded_error(failure));
                }
                bounded_failure.get_or_insert(failure);
                continue;
            }
            if queued.insert(neighbor) {
                queue.push_back((neighbor, depth + 1));
            }
        }
    }

    relations.sort_by_key(|relation| {
        (
            relation.proximity,
            relation.link_id,
            stable_node_key((relation.to.object_id, relation.to.object_type)),
        )
    });
    filtered_nodes.sort_by_key(|filtered| {
        stable_node_key((
            filtered.object_ref.object_id,
            filtered.object_ref.object_type,
        ))
    });
    filtered_nodes.dedup_by_key(|filtered| filtered.object_ref);

    Ok(BoundedExpansionPlan {
        visited,
        relations,
        filtered_nodes,
        bounded_failure,
    })
}

fn graph_expansion_bounded_error(failure: GraphExpansionBoundedFailure) -> CustomError {
    CustomError::GraphExpansionBounded {
        reason: bounded_failure_reason_name(failure.reason).to_owned(),
        location: failure
            .at
            .map(|object_ref| {
                format!(
                    " at object_type={} object_id={}",
                    graph_object_type_name(object_ref.object_type),
                    object_ref.object_id
                )
            })
            .unwrap_or_default(),
    }
}

fn bounded_failure_reason_name(reason: GraphExpansionBoundedFailureReason) -> &'static str {
    match reason {
        GraphExpansionBoundedFailureReason::NodeLimit => "node_limit",
        GraphExpansionBoundedFailureReason::Timeout => "timeout",
        GraphExpansionBoundedFailureReason::HubLimit => "hub_limit",
    }
}

fn graph_object_type_name(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Episode => "episode",
        ObjectType::Observation => "observation",
        ObjectType::Entity => "entity",
        ObjectType::MemoryThread => "memory_thread",
        ObjectType::DerivedMemory => "derived_memory",
        ObjectType::MemoryLink => "memory_link",
    }
}

fn push_filtered_node(
    filtered_nodes: &mut Vec<GraphExpansionFilteredNode>,
    object_ref: GraphObjectRef,
    reason: GraphExpansionFilteredReason,
) {
    if !filtered_nodes
        .iter()
        .any(|filtered| filtered.object_ref == object_ref)
    {
        filtered_nodes.push(GraphExpansionFilteredNode { object_ref, reason });
    }
}

fn lifecycle_filter_reason(
    object: &MemoryObject,
    superseded: &HashSet<MemoryId>,
    policy: GraphExpansionLifecyclePolicy,
) -> Option<GraphExpansionFilteredReason> {
    match object {
        MemoryObject::Episode(object) => retention_filter_reason(object.retention_state, policy),
        MemoryObject::Observation(object) => {
            retention_filter_reason(object.retention_state, policy)
        }
        MemoryObject::MemoryThread(object) => {
            if object.status == ThreadStatus::Archived && !policy.include_archived {
                Some(GraphExpansionFilteredReason::Archived)
            } else {
                None
            }
        }
        MemoryObject::DerivedMemory(object) => {
            derived_memory_lifecycle_filter_reason(object, superseded, policy)
        }
        MemoryObject::Entity(_) | MemoryObject::MemoryLink(_) => None,
    }
}

fn derived_memory_lifecycle_filter_reason(
    object: &DerivedMemory,
    superseded: &HashSet<MemoryId>,
    policy: GraphExpansionLifecyclePolicy,
) -> Option<GraphExpansionFilteredReason> {
    retention_filter_reason(object.retention_state, policy)
        .or(if !object.is_current && !policy.include_non_current {
            Some(GraphExpansionFilteredReason::NonCurrent)
        } else {
            None
        })
        .or(
            if superseded.contains(&object.id) && !policy.include_superseded {
                Some(GraphExpansionFilteredReason::Superseded)
            } else {
                None
            },
        )
}

fn retention_filter_reason(
    retention_state: RetentionState,
    policy: GraphExpansionLifecyclePolicy,
) -> Option<GraphExpansionFilteredReason> {
    match retention_state {
        RetentionState::Active => None,
        RetentionState::Archived if !policy.include_archived => {
            Some(GraphExpansionFilteredReason::Archived)
        }
        RetentionState::Suppressed if !policy.include_suppressed => {
            Some(GraphExpansionFilteredReason::Suppressed)
        }
        RetentionState::Deleted if !policy.include_deleted => {
            Some(GraphExpansionFilteredReason::Deleted)
        }
        RetentionState::Archived | RetentionState::Suppressed | RetentionState::Deleted => None,
    }
}

fn superseded_derived_memory_ids(links: &[&MemoryLink]) -> HashSet<MemoryId> {
    links
        .iter()
        .filter(|link| {
            link.relation == RelationType::Supersedes
                && link.from_type == ObjectType::DerivedMemory
                && link.to_type == ObjectType::DerivedMemory
        })
        .map(|link| link.to_id)
        .collect()
}

fn relation_allowed(query: &GraphExpansionQuery, relation: RelationType) -> bool {
    query.allowed_relation_types.is_empty() || query.allowed_relation_types.contains(&relation)
}

fn object_type_allowed(query: &GraphExpansionQuery, object_type: ObjectType) -> bool {
    query.allowed_object_types.is_empty() || query.allowed_object_types.contains(&object_type)
}

fn link_touches_ref(link: &MemoryLink, object_ref: GraphObjectRef) -> bool {
    (link.from_id == object_ref.object_id && link.from_type == object_ref.object_type)
        || (link.to_id == object_ref.object_id && link.to_type == object_ref.object_type)
}

fn other_endpoint(link: &MemoryLink, object_ref: GraphObjectRef) -> GraphObjectRef {
    if link.from_id == object_ref.object_id && link.from_type == object_ref.object_type {
        GraphObjectRef::new(link.to_id, link.to_type)
    } else {
        GraphObjectRef::new(link.from_id, link.from_type)
    }
}

fn graph_object_ref(object: &MemoryObject) -> GraphObjectRef {
    let (object_id, object_type) = object_identity(object);
    GraphObjectRef::new(object_id, object_type)
}

fn object_identity(object: &MemoryObject) -> (MemoryId, ObjectType) {
    match object {
        MemoryObject::Episode(object) => (object.id, object.object_type),
        MemoryObject::Observation(object) => (object.id, object.object_type),
        MemoryObject::Entity(object) => (object.id, object.object_type),
        MemoryObject::MemoryThread(object) => (object.id, object.object_type),
        MemoryObject::DerivedMemory(object) => (object.id, object.object_type),
        MemoryObject::MemoryLink(object) => (object.id, object.object_type),
    }
}

fn sort_objects(objects: &mut [MemoryObject]) {
    objects.sort_by_key(|object| stable_node_key(object_identity(object)));
}

fn stable_link_key(link: &MemoryLink) -> (MemoryId, MemoryId, MemoryId, u8, u8, u8) {
    (
        link.to_id,
        link.from_id,
        link.id,
        object_type_rank(link.to_type),
        object_type_rank(link.from_type),
        relation_type_rank(link.relation),
    )
}

fn stable_node_key(node: (MemoryId, ObjectType)) -> (MemoryId, u8) {
    (node.0, object_type_rank(node.1))
}

fn object_type_rank(object_type: ObjectType) -> u8 {
    match object_type {
        ObjectType::Episode => 0,
        ObjectType::Observation => 1,
        ObjectType::Entity => 2,
        ObjectType::MemoryThread => 3,
        ObjectType::DerivedMemory => 4,
        ObjectType::MemoryLink => 5,
    }
}

fn relation_type_rank(relation_type: RelationType) -> u8 {
    match relation_type {
        RelationType::HasObservation => 0,
        RelationType::ObservedIn => 1,
        RelationType::Mentions => 2,
        RelationType::Involves => 3,
        RelationType::About => 4,
        RelationType::DerivedFrom => 5,
        RelationType::PartOfThread => 6,
        RelationType::Supports => 7,
        RelationType::Contradicts => 8,
        RelationType::Supersedes => 9,
        RelationType::Resolves => 10,
        RelationType::CreatesOpenLoop => 11,
        RelationType::FulfillsCommitment => 12,
        RelationType::AssociatedWith => 13,
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

    async fn list_diagnostic_objects(&self) -> Result<Vec<MemoryObject>, CustomError> {
        self.query_objects(&GraphObjectQuery::by_types(
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
    }

    async fn list_diagnostic_links(&self) -> Result<Vec<MemoryLink>, CustomError> {
        Ok(Vec::new())
    }
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
    use chrono::Utc;

    use crate::internal::repositories::test_support::{
        high_fanout_graph_fixture, representative_fixtures,
    };

    #[test]
    fn graph_queries_use_domain_ids_and_object_types() {
        let episode_id = MemoryId::new_v4();
        let by_ids = GraphObjectQuery::by_ids(vec![episode_id]);
        let by_refs =
            GraphObjectQuery::by_refs(vec![GraphObjectRef::new(episode_id, ObjectType::Episode)]);
        let by_types = GraphObjectQuery::by_types(vec![ObjectType::Episode], Some(5));

        assert_eq!(by_ids.object_ids, vec![episode_id]);
        assert_eq!(by_ids.object_types, Vec::<ObjectType>::new());
        assert_eq!(by_refs.object_refs[0].object_id, episode_id);
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

    #[test]
    fn bounded_expansion_validates_missing_root_before_zero_node_limit() {
        let query = GraphExpansionQuery::new(MemoryId::new_v4(), ObjectType::Entity, 1, 0);

        let error = bounded_expansion_node_set(&query, false, Vec::new()).unwrap_err();

        assert!(matches!(
            error,
            CustomError::GraphExpansionRootNotFound { .. }
        ));
        assert!(error.to_string().contains("root not found"));
    }

    #[test]
    fn bounded_expansion_reports_node_limit_when_traversal_truncates_results() {
        let fixtures = representative_fixtures();
        let query = GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 2, 1)
            .with_failure_policy(GraphExpansionFailurePolicy {
                timeout_ms: Some(250),
                allow_partial_results: true,
            });

        let expansion = bounded_expansion(&query, fixtures.objects(), fixtures.links()).unwrap();

        assert_eq!(expansion.objects.len(), 1);
        assert!(matches!(
            expansion.bounded_failure,
            Some(GraphExpansionBoundedFailure {
                reason: GraphExpansionBoundedFailureReason::NodeLimit,
                at: Some(_),
            })
        ));
    }

    #[test]
    fn bounded_expansion_fails_closed_when_node_limit_truncates_results() {
        let fixtures = representative_fixtures();
        let query = GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 2, 1)
            .with_failure_policy(GraphExpansionFailurePolicy {
                timeout_ms: Some(250),
                allow_partial_results: false,
            });

        let error = bounded_expansion(&query, fixtures.objects(), fixtures.links()).unwrap_err();

        assert!(matches!(
            error,
            CustomError::GraphExpansionBounded { reason, .. } if reason == "node_limit"
        ));
    }

    #[test]
    fn bounded_expansion_applies_hub_limits_after_traversable_filtering() {
        let fixture = high_fanout_graph_fixture();
        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 5)
            .with_allowed_object_types(vec![ObjectType::Episode])
            .with_max_hub_edges(1)
            .with_failure_policy(GraphExpansionFailurePolicy {
                timeout_ms: Some(250),
                allow_partial_results: false,
            });

        let expansion = bounded_expansion(&query, fixture.objects(), fixture.links).unwrap();

        assert!(expansion.bounded_failure.is_none());
        assert!(expansion.objects.iter().any(|object| {
            matches!(object, MemoryObject::Episode(episode) if episode.id == fixture.episode.id)
        }));
        assert_eq!(expansion.links.len(), 1);
    }

    #[test]
    fn bounded_expansion_node_limit_counts_unique_queued_nodes() {
        let fixtures = representative_fixtures();
        let links = vec![
            test_link(
                fixtures.hub_entity.id,
                ObjectType::Entity,
                fixtures.episode.id,
                ObjectType::Episode,
                RelationType::Involves,
            ),
            test_link(
                fixtures.hub_entity.id,
                ObjectType::Entity,
                fixtures.salient_observation.id,
                ObjectType::Observation,
                RelationType::Mentions,
            ),
            test_link(
                fixtures.episode.id,
                ObjectType::Episode,
                fixtures.derived_reflection.id,
                ObjectType::DerivedMemory,
                RelationType::DerivedFrom,
            ),
            test_link(
                fixtures.salient_observation.id,
                ObjectType::Observation,
                fixtures.derived_reflection.id,
                ObjectType::DerivedMemory,
                RelationType::DerivedFrom,
            ),
        ];
        let query = GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 2, 4)
            .with_allowed_object_types(vec![
                ObjectType::Episode,
                ObjectType::Observation,
                ObjectType::DerivedMemory,
            ])
            .with_max_fanout_per_node(2);

        let expansion = bounded_expansion(&query, fixtures.objects(), links).unwrap();

        assert!(expansion.bounded_failure.is_none());
        assert!(expansion.objects.iter().any(|object| {
            matches!(object, MemoryObject::DerivedMemory(memory) if memory.id == fixtures.derived_reflection.id)
        }));
    }

    fn test_link(
        from_id: MemoryId,
        from_type: ObjectType,
        to_id: MemoryId,
        to_type: ObjectType,
        relation: RelationType,
    ) -> MemoryLink {
        MemoryLink {
            id: MemoryId::new_v4(),
            object_type: ObjectType::MemoryLink,
            from_id,
            from_type,
            to_id,
            to_type,
            relation,
            confidence: 1.0,
            rationale: None,
            created_at: Utc::now(),
            schema_version: "test_schema".to_owned(),
        }
    }
}
