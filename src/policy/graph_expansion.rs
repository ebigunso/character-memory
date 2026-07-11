// Bounded graph expansion policy (ADR-I-0006). Single home for the
// expansion algorithm previously split between the GraphAuthorityStore
// contract file and the Oxigraph adapter.
//
// Two flavors are deliberately colocated here because they differ
// semantically, not just in types:
// - `bounded_expansion` (and its private `bounded_expansion_plan`) computes a
//   complete expansion over fully materialized `MemoryObject`/`MemoryLink`
//   collections, including lifecycle filtering, relation records, and final
//   deterministic ordering.
// - `bounded_incident_link_refs` is a per-node pre-hydration pruning step for
//   adapter-side traversal over lightweight link refs (no objects available
//   yet, so no lifecycle filtering and no object-existence checks); adapters
//   use it to bound which named graphs to hydrate before running the full
//   `bounded_expansion` over the hydrated data.
// Both flavors share the same hub/fanout limiting primitives
// (`apply_fanout_limits_by_pair`, `bounded_hub_retention_limit`), filters
// (`relation_allowed`, `object_type_allowed`), and bounded-failure error
// construction (`graph_expansion_bounded_error`).
use std::collections::{HashMap, HashSet, VecDeque};

use crate::api::types::{
    DerivedMemory, MemoryId, MemoryLink, MemoryObject, ObjectType, RelationType, RetentionState,
    ThreadStatus,
};
use crate::errors::CustomError;
use crate::ports::graph_authority::{
    GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery, GraphExpansion,
    GraphExpansionBoundedFailure, GraphExpansionBoundedFailureReason, GraphExpansionFailurePolicy,
    GraphExpansionFanoutUtilization, GraphExpansionFilteredNode, GraphExpansionFilteredReason,
    GraphExpansionLifecyclePolicy, GraphExpansionQuery, GraphExpansionRelation, GraphObjectRef,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoundedExpansionPlan {
    pub(crate) visited: HashSet<GraphObjectRef>,
    pub(crate) relations: Vec<GraphExpansionRelation>,
    pub(crate) filtered_nodes: Vec<GraphExpansionFilteredNode>,
    pub(crate) fanout_utilization: Vec<GraphExpansionFanoutUtilization>,
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
        plan.fanout_utilization,
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

// Node-set expansion is reserved for diagnostics/tests; remove if callers never need object IDs only.
#[allow(dead_code)]
pub(crate) fn bounded_expansion_node_set(
    query: &GraphExpansionQuery,
    root_exists: bool,
    links: impl IntoIterator<Item = MemoryLink>,
) -> Result<HashSet<(MemoryId, ObjectType)>, CustomError> {
    if query.max_fanout_per_node != usize::MAX
        || query.max_hub_edges != usize::MAX
        || !query.allowed_relation_types.is_empty()
        || !query.fanout_overrides.is_empty()
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
                fanout_utilization: Vec::new(),
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
            fanout_utilization: Vec::new(),
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
    let mut fanout_utilization = Vec::new();
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

        let apply_selectivity_overrides = depth == 0
            && object_ref.object_id == query.root_id
            && object_ref.object_type == query.root_type;
        let pre_limit_counts = query.record_fanout_utilization.then(|| {
            fanout_counts_by_pair(&incident_links, &|item| {
                (item.0.relation, item.1.object_type)
            })
        });

        if incident_links.len() > query.max_hub_edges {
            let failure = GraphExpansionBoundedFailure {
                reason: GraphExpansionBoundedFailureReason::HubLimit,
                at: Some(object_ref),
            };
            if !query.failure_policy.allow_partial_results {
                return Err(graph_expansion_bounded_error(failure));
            }
            bounded_failure.get_or_insert(failure);
            incident_links.truncate(bounded_hub_retention_limit(
                query,
                apply_selectivity_overrides,
            ));
        }
        if let Some(pre_limit_counts) = pre_limit_counts {
            let (limited_incident_links, utilization) = apply_fanout_limits_with_utilization(
                query,
                object_ref,
                incident_links,
                pre_limit_counts,
                apply_selectivity_overrides,
            );
            incident_links = limited_incident_links;
            fanout_utilization.extend(utilization);
        } else {
            incident_links =
                apply_fanout_limits(query, incident_links, apply_selectivity_overrides);
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
        fanout_utilization,
        bounded_failure,
    })
}

fn apply_fanout_limits<'a>(
    query: &GraphExpansionQuery,
    incident_links: Vec<(&'a MemoryLink, GraphObjectRef)>,
    apply_selectivity_overrides: bool,
) -> Vec<(&'a MemoryLink, GraphObjectRef)> {
    apply_fanout_limits_by_pair(
        query,
        incident_links,
        apply_selectivity_overrides,
        |(link, neighbor)| (link.relation, neighbor.object_type),
    )
}

fn apply_fanout_limits_with_utilization<'a>(
    query: &GraphExpansionQuery,
    root: GraphObjectRef,
    incident_links: Vec<(&'a MemoryLink, GraphObjectRef)>,
    pre_limit_counts: FanoutCounts,
    apply_selectivity_overrides: bool,
) -> (
    Vec<(&'a MemoryLink, GraphObjectRef)>,
    Vec<GraphExpansionFanoutUtilization>,
) {
    apply_fanout_limits_with_utilization_by_pair(
        query,
        root,
        incident_links,
        pre_limit_counts,
        apply_selectivity_overrides,
        |(link, neighbor)| (link.relation, neighbor.object_type),
    )
}

fn apply_fanout_limits_with_utilization_by_pair<T>(
    query: &GraphExpansionQuery,
    root: GraphObjectRef,
    incident_items: Vec<T>,
    pre_limit_counts: FanoutCounts,
    apply_selectivity_overrides: bool,
    pair_for_item: impl Fn(&T) -> (RelationType, ObjectType),
) -> (Vec<T>, Vec<GraphExpansionFanoutUtilization>) {
    let retained =
        apply_fanout_limits_by_pair(query, incident_items, apply_selectivity_overrides, |item| {
            pair_for_item(item)
        });
    let retained_counts = fanout_counts_by_pair(&retained, &pair_for_item);
    let mut utilization = pre_limit_counts
        .into_iter()
        .map(|((relation, object_type), before_count)| {
            let retained_count = retained_counts
                .get(&(relation, object_type))
                .copied()
                .unwrap_or_default();
            GraphExpansionFanoutUtilization {
                root,
                relation,
                object_type,
                configured_cap: query.max_fanout_per_node,
                selected_cap: fanout_limit_for_pair_with_override_mode(
                    query,
                    relation,
                    object_type,
                    apply_selectivity_overrides,
                ),
                retained_count,
                omitted_by_fanout_count: before_count.saturating_sub(retained_count),
            }
        })
        .collect::<Vec<_>>();
    utilization.sort_by_key(|entry| {
        (
            relation_type_rank(entry.relation),
            object_type_rank(entry.object_type),
        )
    });
    utilization.retain(|entry| entry.retained_count > 0 || entry.omitted_by_fanout_count > 0);
    (retained, utilization)
}

type FanoutCounts = HashMap<(RelationType, ObjectType), usize>;

fn fanout_counts_by_pair<T>(
    incident_items: &[T],
    pair_for_item: &impl Fn(&T) -> (RelationType, ObjectType),
) -> FanoutCounts {
    let mut counts = HashMap::new();
    for item in incident_items {
        *counts.entry(pair_for_item(item)).or_default() += 1;
    }
    counts
}

pub(crate) fn apply_fanout_limits_by_pair<T>(
    query: &GraphExpansionQuery,
    mut incident_items: Vec<T>,
    apply_selectivity_overrides: bool,
    pair_for_item: impl Fn(&T) -> (RelationType, ObjectType),
) -> Vec<T> {
    if query.fanout_overrides.is_empty() || !apply_selectivity_overrides {
        incident_items.truncate(query.max_fanout_per_node);
        return incident_items;
    }

    let mut retained = Vec::new();
    let mut per_pair_counts = std::collections::HashMap::<(RelationType, ObjectType), usize>::new();
    for item in incident_items {
        if retained.len() >= query.max_fanout_per_node {
            break;
        }
        let (relation, object_type) = pair_for_item(&item);
        let max_for_pair = fanout_limit_for_pair(query, relation, object_type);
        let count = per_pair_counts.entry((relation, object_type)).or_default();
        if *count >= max_for_pair {
            continue;
        }
        *count += 1;
        retained.push(item);
    }
    retained
}

pub(crate) fn bounded_hub_retention_limit(
    query: &GraphExpansionQuery,
    apply_selectivity_overrides: bool,
) -> usize {
    if apply_selectivity_overrides && !query.fanout_overrides.is_empty() {
        query.max_hub_edges
    } else {
        query.max_hub_edges.min(query.max_fanout_per_node)
    }
}

fn fanout_limit_for_pair(
    query: &GraphExpansionQuery,
    relation: RelationType,
    object_type: ObjectType,
) -> usize {
    fanout_limit_for_pair_with_override_mode(query, relation, object_type, true)
}

fn fanout_limit_for_pair_with_override_mode(
    query: &GraphExpansionQuery,
    relation: RelationType,
    object_type: ObjectType,
    apply_selectivity_overrides: bool,
) -> usize {
    if !apply_selectivity_overrides {
        return query.max_fanout_per_node;
    }
    query
        .fanout_overrides
        .iter()
        .find(|override_| override_.relation == relation && override_.object_type == object_type)
        .map(|override_| override_.max_fanout)
        .unwrap_or(query.max_fanout_per_node)
        .min(query.max_fanout_per_node)
}

pub(crate) fn graph_expansion_bounded_error(failure: GraphExpansionBoundedFailure) -> CustomError {
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

// Pre-hydration link-ref flavor (see module comment). Adapters implement
// `BoundedExpansionLinkRef` for their lightweight link-ref types and use
// `bounded_incident_link_refs` to bound traversal before hydrating objects.
pub(crate) trait BoundedExpansionLinkRef: Copy {
    fn link_id(self) -> MemoryId;
    fn from(self) -> GraphObjectRef;
    fn to(self) -> GraphObjectRef;
    fn relation(self) -> RelationType;

    fn other_endpoint(self, object_ref: GraphObjectRef) -> GraphObjectRef {
        if self.from() == object_ref {
            self.to()
        } else {
            self.from()
        }
    }
}

pub(crate) fn bounded_incident_link_refs<T: BoundedExpansionLinkRef>(
    query: &GraphExpansionQuery,
    root_ref: GraphObjectRef,
    object_ref: GraphObjectRef,
    depth: u8,
    link_refs: &[T],
    bounded_failure: &mut Option<GraphExpansionBoundedFailure>,
) -> Result<(Vec<T>, Vec<GraphExpansionFanoutUtilization>), CustomError> {
    let mut incident_links = link_refs
        .iter()
        .copied()
        .filter(|link_ref| relation_allowed(query, link_ref.relation()))
        .filter(|link_ref| {
            object_type_allowed(query, link_ref.other_endpoint(object_ref).object_type)
        })
        .collect::<Vec<_>>();

    let apply_selectivity_overrides = depth == 0 && object_ref == root_ref;
    let pre_limit_counts = query.record_fanout_utilization.then(|| {
        fanout_counts_by_pair(&incident_links, &|link_ref| {
            let neighbor = link_ref.other_endpoint(object_ref);
            (link_ref.relation(), neighbor.object_type)
        })
    });

    if incident_links.len() > query.max_hub_edges {
        let failure = GraphExpansionBoundedFailure {
            reason: GraphExpansionBoundedFailureReason::HubLimit,
            at: Some(object_ref),
        };
        if !query.failure_policy.allow_partial_results {
            return Err(graph_expansion_bounded_error(failure));
        }
        bounded_failure.get_or_insert(failure);
        incident_links.truncate(bounded_hub_retention_limit(
            query,
            apply_selectivity_overrides,
        ));
    }
    if let Some(pre_limit_counts) = pre_limit_counts {
        Ok(apply_fanout_limits_with_utilization_by_pair(
            query,
            object_ref,
            incident_links,
            pre_limit_counts,
            apply_selectivity_overrides,
            |link_ref| {
                let neighbor = link_ref.other_endpoint(object_ref);
                (link_ref.relation(), neighbor.object_type)
            },
        ))
    } else {
        Ok((
            apply_link_ref_fanout_limits(
                query,
                object_ref,
                incident_links,
                apply_selectivity_overrides,
            ),
            Vec::new(),
        ))
    }
}

pub(crate) fn apply_link_ref_fanout_limits<T: BoundedExpansionLinkRef>(
    query: &GraphExpansionQuery,
    object_ref: GraphObjectRef,
    incident_links: Vec<T>,
    apply_selectivity_overrides: bool,
) -> Vec<T> {
    apply_fanout_limits_by_pair(
        query,
        incident_links,
        apply_selectivity_overrides,
        |link_ref| {
            let neighbor = link_ref.other_endpoint(object_ref);
            (link_ref.relation(), neighbor.object_type)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::ports::graph_authority::GraphExpansionFanoutOverride;

    use crate::test_support::{high_fanout_graph_fixture, representative_fixtures};

    #[test]
    fn hub_retention_limit_keeps_large_window_only_for_root_selectivity_overrides() {
        let root_id = MemoryId::new_v4();
        let query = GraphExpansionQuery::new(root_id, ObjectType::Entity, 1, 20)
            .with_max_hub_edges(64)
            .with_max_fanout_per_node(16);

        assert_eq!(bounded_hub_retention_limit(&query, false), 16);
        assert_eq!(bounded_hub_retention_limit(&query, true), 16);

        let query = query.with_fanout_overrides(vec![GraphExpansionFanoutOverride {
            relation: RelationType::About,
            object_type: ObjectType::DerivedMemory,
            max_fanout: 4,
        }]);

        assert_eq!(bounded_hub_retention_limit(&query, false), 16);
        assert_eq!(bounded_hub_retention_limit(&query, true), 64);
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
    fn fanout_utilization_counts_links_removed_by_hub_truncation() {
        let fixture = high_fanout_graph_fixture();
        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory])
            .with_max_hub_edges(8)
            .with_max_fanout_per_node(4)
            .with_fanout_utilization_recording(true)
            .with_failure_policy(GraphExpansionFailurePolicy {
                timeout_ms: Some(250),
                allow_partial_results: true,
            });

        let without_utilization = bounded_expansion(
            &query.clone().with_fanout_utilization_recording(false),
            fixture.objects(),
            fixture.links.clone(),
        )
        .unwrap();
        let expansion = bounded_expansion(&query, fixture.objects(), fixture.links).unwrap();
        let utilization = expansion
            .fanout_utilization
            .iter()
            .find(|entry| {
                entry.root.object_id == fixture.hub_entity.id
                    && entry.relation == RelationType::About
                    && entry.object_type == ObjectType::DerivedMemory
            })
            .unwrap();

        assert_eq!(expansion.links.len(), 4);
        assert_eq!(without_utilization.objects, expansion.objects);
        assert_eq!(without_utilization.links, expansion.links);
        assert_eq!(without_utilization.relations, expansion.relations);
        assert_eq!(without_utilization.filtered_nodes, expansion.filtered_nodes);
        assert_eq!(
            without_utilization.bounded_failure,
            expansion.bounded_failure
        );
        assert_eq!(utilization.retained_count, 4);
        assert_eq!(utilization.omitted_by_fanout_count, 8);
        assert_eq!(
            expansion.bounded_failure,
            Some(GraphExpansionBoundedFailure {
                reason: GraphExpansionBoundedFailureReason::HubLimit,
                at: Some(GraphObjectRef::new(
                    fixture.hub_entity.id,
                    ObjectType::Entity
                )),
            })
        );
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

    #[test]
    fn bounded_expansion_applies_selectivity_fanout_overrides_only_at_root() {
        let fixture = representative_fixtures();
        let root_expanded = &fixture.derived_reflection;
        let root_pruned = &fixture.user_preference;
        let downstream_a = &fixture.open_loop;
        let downstream_b = &fixture.commitment;
        let links = vec![
            test_link_with_id(
                uuid::Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0300),
                fixture.hub_entity.id,
                ObjectType::Entity,
                root_expanded.id,
                ObjectType::DerivedMemory,
                RelationType::About,
            ),
            test_link_with_id(
                uuid::Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0301),
                fixture.hub_entity.id,
                ObjectType::Entity,
                root_pruned.id,
                ObjectType::DerivedMemory,
                RelationType::About,
            ),
            test_link_with_id(
                uuid::Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0302),
                root_expanded.id,
                ObjectType::DerivedMemory,
                downstream_a.id,
                ObjectType::DerivedMemory,
                RelationType::About,
            ),
            test_link_with_id(
                uuid::Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0303),
                root_expanded.id,
                ObjectType::DerivedMemory,
                downstream_b.id,
                ObjectType::DerivedMemory,
                RelationType::About,
            ),
        ];
        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 2, 10)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory])
            .with_max_fanout_per_node(10)
            .with_fanout_overrides(vec![GraphExpansionFanoutOverride {
                relation: RelationType::About,
                object_type: ObjectType::DerivedMemory,
                max_fanout: 1,
            }]);

        let expansion = bounded_expansion(&query, fixture.objects(), links).unwrap();

        assert!(expansion.objects.iter().any(|object| {
            matches!(object, MemoryObject::DerivedMemory(memory) if memory.id == root_expanded.id)
        }));
        assert!(!expansion.objects.iter().any(|object| {
            matches!(object, MemoryObject::DerivedMemory(memory) if memory.id == root_pruned.id)
        }));
        assert!(expansion.objects.iter().any(|object| {
            matches!(object, MemoryObject::DerivedMemory(memory) if memory.id == downstream_a.id)
        }));
        assert!(expansion.objects.iter().any(|object| {
            matches!(object, MemoryObject::DerivedMemory(memory) if memory.id == downstream_b.id)
        }));
    }

    #[test]
    fn fanout_utilization_recording_does_not_change_pruned_expansion() {
        let fixture = high_fanout_graph_fixture();
        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 10)
            .with_max_fanout_per_node(2);

        let without_utilization =
            bounded_expansion(&query, fixture.objects(), fixture.links.clone()).unwrap();
        let with_utilization = bounded_expansion(
            &query.with_fanout_utilization_recording(true),
            fixture.objects(),
            fixture.links,
        )
        .unwrap();

        assert_eq!(without_utilization.objects, with_utilization.objects);
        assert_eq!(without_utilization.links, with_utilization.links);
        assert_eq!(without_utilization.relations, with_utilization.relations);
        assert_eq!(
            without_utilization.filtered_nodes,
            with_utilization.filtered_nodes
        );
        assert_eq!(
            without_utilization.bounded_failure,
            with_utilization.bounded_failure
        );
        assert!(without_utilization.fanout_utilization.is_empty());
        assert!(!with_utilization.fanout_utilization.is_empty());
    }

    #[test]
    fn fanout_utilization_is_attributed_to_each_expanded_node() {
        let fixture = representative_fixtures();
        let links = vec![
            test_link_with_id(
                uuid::Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0310),
                fixture.hub_entity.id,
                ObjectType::Entity,
                fixture.derived_reflection.id,
                ObjectType::DerivedMemory,
                RelationType::About,
            ),
            test_link_with_id(
                uuid::Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0311),
                fixture.derived_reflection.id,
                ObjectType::DerivedMemory,
                fixture.user_preference.id,
                ObjectType::DerivedMemory,
                RelationType::About,
            ),
        ];
        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 2, 10)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory])
            .with_max_fanout_per_node(10)
            .with_fanout_utilization_recording(true);

        let expansion = bounded_expansion(&query, fixture.objects(), links).unwrap();
        let about_derived_roots = expansion
            .fanout_utilization
            .iter()
            .filter(|entry| {
                entry.relation == RelationType::About
                    && entry.object_type == ObjectType::DerivedMemory
            })
            .map(|entry| entry.root)
            .collect::<HashSet<_>>();

        assert_eq!(
            about_derived_roots,
            HashSet::from([
                GraphObjectRef::new(fixture.hub_entity.id, ObjectType::Entity),
                GraphObjectRef::new(fixture.derived_reflection.id, ObjectType::DerivedMemory,),
            ])
        );
    }

    fn test_link(
        from_id: MemoryId,
        from_type: ObjectType,
        to_id: MemoryId,
        to_type: ObjectType,
        relation: RelationType,
    ) -> MemoryLink {
        test_link_with_id(
            MemoryId::new_v4(),
            from_id,
            from_type,
            to_id,
            to_type,
            relation,
        )
    }

    fn test_link_with_id(
        id: MemoryId,
        from_id: MemoryId,
        from_type: ObjectType,
        to_id: MemoryId,
        to_type: ObjectType,
        relation: RelationType,
    ) -> MemoryLink {
        MemoryLink {
            id,
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
