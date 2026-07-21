use crate::domain::{MemoryId, MemoryLink, MemoryObject, MemoryObjectRef, ObjectType};
use crate::errors::StatsUpdateCause;
use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectQuery};
use crate::ports::retrieval_stats::{
    retrieval_stats_edges, retrieval_stats_object_states, RetrievalStatsHealthState,
    RetrievalStatsStore,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatsProjectionOutcome {
    pub(crate) attempted_object_ids: Vec<MemoryId>,
    pub(crate) cause: Option<StatsUpdateCause>,
}

pub(crate) struct StatsProjectionService<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    graph_store: &'a G,
    stats_store: &'a dyn RetrievalStatsStore,
}

impl<'a, G> StatsProjectionService<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    pub(crate) fn new(graph_store: &'a G, stats_store: &'a dyn RetrievalStatsStore) -> Self {
        Self {
            graph_store,
            stats_store,
        }
    }

    pub(crate) async fn project(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> StatsProjectionOutcome {
        let endpoint_refs = stats_endpoint_refs(objects, links);
        let (stats_objects, mut cause) = if endpoint_refs.is_empty() {
            (objects.to_vec(), None)
        } else {
            let endpoint_ids = endpoint_refs
                .iter()
                .map(|object_ref| object_ref.id)
                .collect();
            match self
                .graph_store
                .query_objects(&GraphObjectQuery::by_ids(endpoint_ids))
                .await
            {
                Ok(endpoint_objects) => {
                    let endpoint_objects = endpoint_objects
                        .into_iter()
                        .filter(|object| endpoint_refs.contains(&object.object_ref()))
                        .collect();
                    (
                        stats_objects_with_endpoint_lifecycle(objects, endpoint_objects),
                        None,
                    )
                }
                Err(error) => (
                    objects.to_vec(),
                    Some(StatsUpdateCause::EndpointHydration {
                        detail: error.to_string(),
                    }),
                ),
            }
        };

        let attempted_object_ids = stats_objects.iter().map(MemoryObject::id).collect();
        if let Err(write_cause) = self.write_projection(&stats_objects, links).await {
            if cause.is_none() {
                cause = Some(write_cause);
            }
        }

        if let Some(primary_cause) = &cause {
            let _ = self
                .stats_store
                .mark_unhealthy(primary_cause.to_string())
                .await;
        }

        match self.stats_store.health().await {
            Ok(health)
                if cause.is_none() && health.state == RetrievalStatsHealthState::Unhealthy =>
            {
                cause = Some(StatsUpdateCause::StoreUnhealthy {
                    detail: health.last_error_message,
                });
            }
            Err(error) if cause.is_none() => {
                cause = Some(StatsUpdateCause::HealthCheck {
                    detail: error.to_string(),
                });
            }
            _ => {}
        }

        StatsProjectionOutcome {
            attempted_object_ids,
            cause,
        }
    }

    async fn write_projection(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), StatsUpdateCause> {
        let states = retrieval_stats_object_states(objects);
        let edges = retrieval_stats_edges(objects, links);
        self.stats_store
            .record_edges(&edges)
            .await
            .map_err(|error| StatsUpdateCause::EdgeWrite {
                detail: error.to_string(),
            })?;

        if !states.is_empty() {
            self.stats_store
                .record_object_states(&states)
                .await
                .map_err(|error| StatsUpdateCause::ObjectStateWrite {
                    detail: error.to_string(),
                })?;
        }
        Ok(())
    }
}

fn stats_endpoint_refs(objects: &[MemoryObject], links: &[MemoryLink]) -> Vec<MemoryObjectRef> {
    let mut refs = Vec::new();
    for link in links {
        push_stats_endpoint_ref(&mut refs, objects, link.from_id, link.from_type);
        push_stats_endpoint_ref(&mut refs, objects, link.to_id, link.to_type);
    }
    refs
}

fn push_stats_endpoint_ref(
    refs: &mut Vec<MemoryObjectRef>,
    objects: &[MemoryObject],
    object_id: MemoryId,
    object_type: ObjectType,
) {
    let object_ref = MemoryObjectRef::from_id_type(object_id, object_type);
    if !object_type_has_stats_state(object_type)
        || objects
            .iter()
            .any(|object| object.object_ref() == object_ref)
        || refs.contains(&object_ref)
    {
        return;
    }
    refs.push(object_ref);
}

fn stats_objects_with_endpoint_lifecycle(
    objects: &[MemoryObject],
    endpoint_objects: Vec<MemoryObject>,
) -> Vec<MemoryObject> {
    let mut stats_objects = objects.to_vec();
    for endpoint_object in endpoint_objects {
        if !stats_objects
            .iter()
            .any(|object| object.object_ref() == endpoint_object.object_ref())
        {
            stats_objects.push(endpoint_object);
        }
    }
    stats_objects
}

fn object_type_has_stats_state(object_type: ObjectType) -> bool {
    matches!(
        object_type,
        ObjectType::Episode | ObjectType::Observation | ObjectType::DerivedMemory
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    use crate::domain::{RelationType, DEFAULT_SCHEMA_VERSION};

    #[test]
    fn endpoint_refs_dedupe_self_referential_stats_endpoint() {
        let object_id = MemoryId::from_u128(1);
        let link = MemoryLink {
            id: MemoryId::from_u128(2),
            object_type: ObjectType::MemoryLink,
            from_id: object_id,
            from_type: ObjectType::Observation,
            to_id: object_id,
            to_type: ObjectType::Observation,
            relation: RelationType::AssociatedWith,
            confidence: 0.8,
            rationale: None,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        };

        assert_eq!(
            stats_endpoint_refs(&[], &[link]),
            vec![MemoryObjectRef::new(ObjectType::Observation, object_id)]
        );
    }
}
