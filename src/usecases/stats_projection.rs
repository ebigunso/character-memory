use crate::api::types::StatsUpdateStatus;
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
    pub(crate) causes: Vec<StatsUpdateCause>,
}

impl StatsProjectionOutcome {
    pub(crate) fn into_status(self) -> StatsUpdateStatus {
        if self.causes.is_empty() {
            StatsUpdateStatus::succeeded(self.attempted_object_ids)
        } else {
            StatsUpdateStatus::failed([], self.attempted_object_ids, self.causes)
        }
    }
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
        let attempted_object_ids = attempted_stats_object_ids(objects, &endpoint_refs);
        let (stats_objects, mut causes) = if endpoint_refs.is_empty() {
            (objects.to_vec(), Vec::new())
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
                        Vec::new(),
                    )
                }
                Err(error) => (
                    objects.to_vec(),
                    vec![StatsUpdateCause::EndpointHydration { error }],
                ),
            }
        };

        causes.extend(self.write_projection(&stats_objects, links).await);

        match self.stats_store.health().await {
            Ok(health) if health.state == RetrievalStatsHealthState::Unhealthy => {
                causes.push(StatsUpdateCause::StoreUnhealthy {
                    health_cause: health.last_error_cause,
                });
            }
            Err(error) => {
                causes.push(StatsUpdateCause::HealthCheck { error });
            }
            _ => {}
        }

        if let Some(primary_cause) = causes.first() {
            if let Some(health_cause) = primary_cause.health_cause() {
                if let Err(error) = self.stats_store.mark_unhealthy(health_cause).await {
                    causes.push(StatsUpdateCause::HealthMark { error });
                }
            }
        }

        StatsProjectionOutcome {
            attempted_object_ids,
            causes,
        }
    }

    async fn write_projection(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Vec<StatsUpdateCause> {
        let states = retrieval_stats_object_states(objects);
        let edges = retrieval_stats_edges(objects, links);
        let mut causes = Vec::new();
        if let Err(error) = self.stats_store.record_edges(&edges).await {
            causes.push(StatsUpdateCause::EdgeWrite { error });
        }

        if !states.is_empty() {
            if let Err(error) = self.stats_store.record_object_states(&states).await {
                causes.push(StatsUpdateCause::ObjectStateWrite { error });
            }
        }
        causes
    }
}

fn attempted_stats_object_ids(
    objects: &[MemoryObject],
    endpoint_refs: &[MemoryObjectRef],
) -> Vec<MemoryId> {
    let mut ids = objects.iter().map(MemoryObject::id).collect::<Vec<_>>();
    for endpoint_ref in endpoint_refs {
        if !ids.contains(&endpoint_ref.id) {
            ids.push(endpoint_ref.id);
        }
    }
    ids
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
    use std::sync::Mutex;

    use super::*;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};

    use crate::domain::{RelationType, DEFAULT_SCHEMA_VERSION};
    use crate::errors::{RetrievalStatsHealthCause, RetrievalStatsStoreError};
    use crate::ports::retrieval_stats::{
        RetrievalStatsCounter, RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth,
        RetrievalStatsObjectState,
    };
    use crate::test_support::{simple_episode, FakeGraphAuthorityStore};

    #[derive(Debug)]
    struct RecordingStatsStore {
        edge_error: Option<RetrievalStatsStoreError>,
        object_state_error: Option<RetrievalStatsStoreError>,
        health_result: Result<RetrievalStatsHealth, RetrievalStatsStoreError>,
        marked_causes: Mutex<Vec<RetrievalStatsHealthCause>>,
    }

    impl Default for RecordingStatsStore {
        fn default() -> Self {
            Self {
                edge_error: None,
                object_state_error: None,
                health_result: Ok(RetrievalStatsHealth::default()),
                marked_causes: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl RetrievalStatsStore for RecordingStatsStore {
        async fn record_edges(
            &self,
            _edges: &[RetrievalStatsEdge],
        ) -> Result<(), RetrievalStatsStoreError> {
            match &self.edge_error {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        }

        async fn record_object_states(
            &self,
            _states: &[RetrievalStatsObjectState],
        ) -> Result<(), RetrievalStatsStoreError> {
            match &self.object_state_error {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        }

        async fn counter(
            &self,
            _key: &RetrievalStatsCounterKey,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(None)
        }

        async fn global_counter(
            &self,
            _relation_kind: RelationType,
            _object_type: ObjectType,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(None)
        }

        async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
            self.health_result.clone()
        }

        async fn mark_unhealthy(
            &self,
            cause: RetrievalStatsHealthCause,
        ) -> Result<(), RetrievalStatsStoreError> {
            self.marked_causes.lock().unwrap().push(cause);
            Ok(())
        }
    }

    fn health_check_error() -> RetrievalStatsStoreError {
        RetrievalStatsStoreError::Sqlite {
            detail: "health check failed".to_owned(),
        }
    }

    fn edge_write_error() -> RetrievalStatsStoreError {
        RetrievalStatsStoreError::Sqlite {
            detail: "edge write failed".to_owned(),
        }
    }

    fn object_state_write_error() -> RetrievalStatsStoreError {
        RetrievalStatsStoreError::Sqlite {
            detail: "object-state write failed".to_owned(),
        }
    }

    #[tokio::test]
    async fn successful_writes_then_health_failure_marks_store_with_retained_cause() {
        let graph_store = FakeGraphAuthorityStore::new();
        let stats_store = RecordingStatsStore {
            health_result: Err(health_check_error()),
            ..RecordingStatsStore::default()
        };

        let outcome = StatsProjectionService::new(&graph_store, &stats_store)
            .project(&[], &[])
            .await;

        assert_eq!(
            outcome.causes,
            vec![StatsUpdateCause::HealthCheck {
                error: health_check_error(),
            }]
        );
        assert_eq!(
            *stats_store.marked_causes.lock().unwrap(),
            vec![RetrievalStatsHealthCause::HealthCheck {
                error: health_check_error(),
            }]
        );
    }

    #[tokio::test]
    async fn edge_and_object_state_failures_are_both_retained() {
        let graph_store = FakeGraphAuthorityStore::new();
        let stats_store = RecordingStatsStore {
            edge_error: Some(edge_write_error()),
            object_state_error: Some(object_state_write_error()),
            ..RecordingStatsStore::default()
        };

        let outcome = StatsProjectionService::new(&graph_store, &stats_store)
            .project(&[MemoryObject::Episode(simple_episode())], &[])
            .await;

        assert_eq!(
            outcome.causes,
            vec![
                StatsUpdateCause::EdgeWrite {
                    error: edge_write_error(),
                },
                StatsUpdateCause::ObjectStateWrite {
                    error: object_state_write_error(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn preexisting_unhealthy_state_is_retained_with_write_failure() {
        let graph_store = FakeGraphAuthorityStore::new();
        let stored_health_cause = RetrievalStatsHealthCause::CounterRead {
            error: health_check_error(),
        };
        let stats_store = RecordingStatsStore {
            edge_error: Some(edge_write_error()),
            health_result: Ok(RetrievalStatsHealth {
                state: RetrievalStatsHealthState::Unhealthy,
                last_error_cause: Some(stored_health_cause.clone()),
            }),
            ..RecordingStatsStore::default()
        };

        let outcome = StatsProjectionService::new(&graph_store, &stats_store)
            .project(&[], &[])
            .await;

        assert_eq!(
            outcome.causes,
            vec![
                StatsUpdateCause::EdgeWrite {
                    error: edge_write_error(),
                },
                StatsUpdateCause::StoreUnhealthy {
                    health_cause: Some(stored_health_cause),
                },
            ]
        );
    }

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
