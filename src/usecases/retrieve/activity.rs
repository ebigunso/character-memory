use super::*;
use crate::api::types::{ActivityRef, ActivityResolution, ActivityResult};
use crate::domain::RetentionState;
use crate::ports::graph_authority::{
    GraphDerivedMemoryThreadQuery, GraphExpansionFilteredNode, GraphObjectQuery,
};

impl<G, V, E> RetrievePipeline<'_, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    pub(super) async fn activity_roots(
        &self,
        context: &RetrievalContext,
    ) -> Result<
        (
            Option<ActivityResult>,
            Vec<CandidateRoot>,
            Vec<GraphExpansionFilteredNode>,
        ),
        CustomError,
    > {
        let Some(activity) = context.activity else {
            return Ok((None, Vec::new(), Vec::new()));
        };
        let reference = match activity {
            ActivityRef::Thread(id) => MemoryObjectRef::new(ObjectType::MemoryThread, id),
            ActivityRef::OpenLoop(id) => MemoryObjectRef::new(ObjectType::DerivedMemory, id),
        };
        let objects = self
            .graph_store
            .query_objects(&GraphObjectQuery::by_refs(vec![reference]))
            .await?;
        let object = objects.into_iter().find(|object| match (activity, object) {
            (ActivityRef::Thread(_), MemoryObject::MemoryThread(_)) => true,
            (ActivityRef::OpenLoop(_), MemoryObject::DerivedMemory(memory)) => {
                memory.derived_type == DerivedType::OpenLoop
            }
            _ => false,
        });
        let Some(object) = object else {
            return Ok((
                Some(ActivityResult {
                    activity,
                    resolution: ActivityResolution::Unknown,
                }),
                Vec::new(),
                Vec::new(),
            ));
        };
        let result = Some(ActivityResult {
            activity,
            resolution: ActivityResolution::Found,
        });
        let root = |(position, reference)| {
            CandidateRoot::new(reference, RecallRoad::Activity, 1.0, position)
        };
        let mut filtered = Vec::new();
        let mut roots = vec![root((0, reference))];
        // Resolution reports existence even when the activity cannot cue its members.
        if !context.graph_limits.allowed_object_types.is_empty()
            && !context
                .graph_limits
                .allowed_object_types
                .contains(&reference.object_type)
        {
            return Ok((result, roots, filtered));
        }
        let mut members = Vec::new();
        match object {
            MemoryObject::MemoryThread(thread) => {
                let query = GraphDerivedMemoryThreadQuery::by_threads(vec![thread.id])
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy::from(
                        context.lifecycle_policy,
                    ));
                let (memories, omitted) = self
                    .graph_store
                    .query_thread_state(&query, RecallRoad::Activity.contribution(context))
                    .await?;
                filtered.extend(omitted);
                members.extend(
                    memories
                        .into_iter()
                        .map(|memory| MemoryObjectRef::new(ObjectType::DerivedMemory, memory.id)),
                );
            }
            MemoryObject::DerivedMemory(memory) => {
                if (!context.lifecycle_policy.include_suppressed
                    && memory.retention_state == RetentionState::Suppressed)
                    || (!context.lifecycle_policy.include_superseded
                        && !self
                            .graph_store
                            .query_superseded_derived_memory_ids(&[memory.id])
                            .await?
                            .is_empty())
                {
                    return Ok((result, roots, filtered));
                }
                members.extend(
                    memory
                        .thread_ids
                        .into_iter()
                        .map(|id| MemoryObjectRef::new(ObjectType::MemoryThread, id)),
                );
                members.extend(
                    memory
                        .derived_from_episode_ids
                        .into_iter()
                        .map(|id| MemoryObjectRef::new(ObjectType::Episode, id)),
                );
                members.extend(
                    memory
                        .derived_from_observation_ids
                        .into_iter()
                        .map(|id| MemoryObjectRef::new(ObjectType::Observation, id)),
                );
                let sources = members
                    .iter()
                    .copied()
                    .filter(|reference| {
                        matches!(
                            reference.object_type,
                            ObjectType::Episode | ObjectType::Observation
                        )
                    })
                    .collect::<Vec<_>>();
                let occasions = self.graph_store.query_episode_occasions(&sources).await?;
                let policy = GraphExpansionLifecyclePolicy::from(context.lifecycle_policy);
                members.retain(|reference| {
                    if let Some((object_ref, reason)) = occasions
                        .get(reference)
                        .and_then(|occasion| occasion.filtered_reason(*reference, policy))
                    {
                        filtered.push(GraphExpansionFilteredNode {
                            object_ref,
                            reason,
                            superseded_by: Vec::new(),
                        });
                        false
                    } else {
                        true
                    }
                });
                members.sort_by_key(|reference| {
                    (
                        reference.object_type.stable_rank(),
                        std::cmp::Reverse(
                            occasions
                                .get(reference)
                                .filter(|_| reference.object_type == ObjectType::Episode)
                                .map(|occasion| occasion.time),
                        ),
                        reference.id,
                    )
                });
            }
            _ => return Ok((result, roots, filtered)),
        }
        members.dedup();
        roots.extend(
            members
                .into_iter()
                .enumerate()
                .map(|(position, reference)| root((position + 1, reference))),
        );
        Ok((result, roots, filtered))
    }
}
