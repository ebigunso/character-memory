use super::*;
use crate::api::types::{ActivityRef, ActivityResolution, ActivityResult};
use crate::domain::RetentionState;
use crate::ports::graph_authority::{GraphDerivedMemoryThreadQuery, GraphObjectQuery};

impl<G, V, E> RetrievePipeline<'_, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    pub(super) async fn activity_roots(
        &self,
        context: &RetrievalContext,
    ) -> Result<(Option<ActivityResult>, Vec<CandidateRoot>), CustomError> {
        let Some(activity) = context.activity else {
            return Ok((None, Vec::new()));
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
            ));
        };
        let result = Some(ActivityResult {
            activity,
            resolution: ActivityResolution::Found,
        });
        let root = |reference: MemoryObjectRef| CandidateRoot {
            object_id: reference.id,
            object_type: reference.object_type,
            score: 1.0,
            source: GraphRootSource::Activity,
            vector_score: None,
            cue_kinds: BTreeSet::from([CueKind::Activity]),
        };
        let mut roots = vec![root(reference)];
        // Resolution reports existence even when the activity cannot cue its members.
        if !context.graph_limits.allowed_object_types.is_empty()
            && !context
                .graph_limits
                .allowed_object_types
                .contains(&reference.object_type)
        {
            return Ok((result, roots));
        }
        let mut members = Vec::new();
        match object {
            MemoryObject::MemoryThread(thread) => {
                let query = GraphDerivedMemoryThreadQuery::by_threads(vec![thread.id])
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy {
                        include_suppressed: context.lifecycle_policy.include_suppressed,
                        include_superseded: context.lifecycle_policy.include_superseded,
                    });
                let mut memories = self
                    .graph_store
                    .query_derived_memories_by_thread(&query)
                    .await?;
                memories.sort_by(|left, right| {
                    right
                        .created_at
                        .cmp(&left.created_at)
                        .then_with(|| left.id.cmp(&right.id))
                });
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
                    return Ok((result, roots));
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
                members
                    .sort_by_key(|reference| (reference.object_type.stable_rank(), reference.id));
            }
            _ => return Ok((result, roots)),
        }
        members.dedup();
        roots.extend(members.into_iter().map(root));
        Ok((result, roots))
    }
}
