use std::collections::{BTreeSet, HashMap, HashSet};

use crate::domain::{MemoryObject, MemoryObjectRef, ObjectType};
use crate::errors::CustomError;
use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectQuery};

/// Called inside the write turn, before collision checks and graph persistence.
/// Sources in the current plan take precedence over sources already in the graph.
pub(crate) async fn derive_scope_keys(
    graph: &(impl GraphAuthorityStore + ?Sized),
    objects: &mut [MemoryObject],
) -> Result<(), CustomError> {
    let refs = objects
        .iter()
        .filter_map(|object| match object {
            MemoryObject::DerivedMemory(memory) => Some(memory),
            _ => None,
        })
        .flat_map(|memory| {
            memory
                .derived_from_episode_ids
                .iter()
                .map(|id| MemoryObjectRef::new(ObjectType::Episode, *id))
                .chain(
                    memory
                        .derived_from_observation_ids
                        .iter()
                        .map(|id| MemoryObjectRef::new(ObjectType::Observation, *id)),
                )
        })
        .collect::<HashSet<_>>();
    let mut sources = objects
        .iter()
        .filter(|object| {
            matches!(
                object,
                MemoryObject::Episode(_) | MemoryObject::Observation(_)
            )
        })
        .map(|object| (object.object_ref(), object.clone()))
        .collect::<HashMap<_, _>>();
    let missing = refs
        .iter()
        .copied()
        .filter(|reference| !sources.contains_key(reference))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        for object in graph
            .query_objects(&GraphObjectQuery::by_refs(missing))
            .await?
        {
            sources.insert(object.object_ref(), object);
        }
    }
    let episode_refs = sources
        .values()
        .filter_map(|object| match object {
            MemoryObject::Observation(observation) if refs.contains(&object.object_ref()) => Some(
                MemoryObjectRef::new(ObjectType::Episode, observation.episode_id),
            ),
            _ => None,
        })
        .filter(|reference| !sources.contains_key(reference))
        .collect::<HashSet<_>>();
    if !episode_refs.is_empty() {
        for object in graph
            .query_objects(&GraphObjectQuery::by_refs(
                episode_refs.into_iter().collect(),
            ))
            .await?
        {
            sources.insert(object.object_ref(), object);
        }
    }
    for object in objects {
        let MemoryObject::DerivedMemory(memory) = object else {
            continue;
        };
        let episode_ids = memory.derived_from_episode_ids.iter().copied().chain(
            memory.derived_from_observation_ids.iter().filter_map(|id| {
                match sources.get(&MemoryObjectRef::new(ObjectType::Observation, *id)) {
                    Some(MemoryObject::Observation(observation)) => Some(observation.episode_id),
                    _ => None,
                }
            }),
        );
        memory.scope_keys = episode_ids
            .filter_map(|id| sources.get(&MemoryObjectRef::new(ObjectType::Episode, id)))
            .filter_map(|object| match object {
                MemoryObject::Episode(episode) => Some(&episode.scene),
                _ => None,
            })
            .flat_map(|scene| scene.scope_keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    }
    Ok(())
}
