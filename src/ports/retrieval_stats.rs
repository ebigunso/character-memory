use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{MemoryId, MemoryLink, MemoryObject, ObjectType, RelationType, RetentionState};
use crate::errors::CustomError;

#[async_trait]
pub(crate) trait RetrievalStatsStore: Send + Sync {
    async fn record_edges(&self, edges: &[RetrievalStatsEdge]) -> Result<(), CustomError>;

    async fn record_object_states(
        &self,
        states: &[RetrievalStatsObjectState],
    ) -> Result<(), CustomError>;

    async fn counter(
        &self,
        key: &RetrievalStatsCounterKey,
    ) -> Result<Option<RetrievalStatsCounter>, CustomError>;

    async fn global_counter(
        &self,
        relation_kind: RelationType,
        object_type: ObjectType,
    ) -> Result<Option<RetrievalStatsCounter>, CustomError>;

    async fn health(&self) -> Result<RetrievalStatsHealth, CustomError>;

    async fn mark_unhealthy(&self, message: String) -> Result<(), CustomError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetrievalStatsEdge {
    pub(crate) edge_key: String,
    pub(crate) entity_id: MemoryId,
    pub(crate) relation_kind: RelationType,
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
    pub(crate) retention_state: RetentionState,
    pub(crate) is_current: bool,
    pub(crate) first_seen_at: DateTime<Utc>,
    pub(crate) last_seen_at: DateTime<Utc>,
}

impl RetrievalStatsEdge {
    pub(crate) fn is_active(&self) -> bool {
        self.retention_state == RetentionState::Active
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetrievalStatsObjectState {
    pub(crate) object_id: MemoryId,
    pub(crate) object_type: ObjectType,
    pub(crate) retention_state: RetentionState,
    pub(crate) is_current: bool,
    pub(crate) observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RetrievalStatsCounterKey {
    pub(crate) entity_id: MemoryId,
    pub(crate) relation_kind: RelationType,
    pub(crate) object_type: ObjectType,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RetrievalStatsCounter {
    pub(crate) total_count: u64,
    pub(crate) active_count: u64,
    pub(crate) current_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetrievalStatsHealth {
    pub(crate) state: RetrievalStatsHealthState,
    pub(crate) last_error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetrievalStatsHealthState {
    Healthy,
    Unhealthy,
}

impl Default for RetrievalStatsHealth {
    fn default() -> Self {
        Self {
            state: RetrievalStatsHealthState::Healthy,
            last_error_message: None,
        }
    }
}

pub(crate) fn retrieval_stats_edges(
    objects: &[MemoryObject],
    links: &[MemoryLink],
) -> Vec<RetrievalStatsEdge> {
    let states = retrieval_stats_object_states(objects);
    retrieval_stats_edges_with_states(objects, links, &states)
}

fn retrieval_stats_edges_with_states(
    objects: &[MemoryObject],
    links: &[MemoryLink],
    object_states: &[RetrievalStatsObjectState],
) -> Vec<RetrievalStatsEdge> {
    let object_state_lookup = object_state_lookup(object_states);
    let mut edges: HashMap<String, RetrievalStatsEdge> = HashMap::new();
    for object in objects {
        append_intrinsic_edges(&mut edges, object);
    }
    for link in links {
        append_link_edges(&mut edges, link, &object_state_lookup);
    }
    let mut edges = edges.into_values().collect::<Vec<_>>();
    edges.sort_by(|left, right| left.edge_key.cmp(&right.edge_key));
    edges
}

fn object_state_lookup(
    object_states: &[RetrievalStatsObjectState],
) -> HashMap<(MemoryId, ObjectType), RetrievalStatsObjectState> {
    let mut object_state_lookup = HashMap::new();
    for state in object_states {
        object_state_lookup.insert((state.object_id, state.object_type), state.clone());
    }
    object_state_lookup
}

pub(crate) fn retrieval_stats_object_states(
    objects: &[MemoryObject],
) -> Vec<RetrievalStatsObjectState> {
    objects.iter().filter_map(object_state).collect()
}

pub(crate) fn relation_type_key(relation: RelationType) -> &'static str {
    match relation {
        RelationType::HasObservation => "has_observation",
        RelationType::ObservedIn => "observed_in",
        RelationType::Mentions => "mentions",
        RelationType::Involves => "involves",
        RelationType::About => "about",
        RelationType::DerivedFrom => "derived_from",
        RelationType::PartOfThread => "part_of_thread",
        RelationType::Supports => "supports",
        RelationType::Contradicts => "contradicts",
        RelationType::Supersedes => "supersedes",
        RelationType::Resolves => "resolves",
        RelationType::CreatesOpenLoop => "creates_open_loop",
        RelationType::FulfillsCommitment => "fulfills_commitment",
        RelationType::AssociatedWith => "associated_with",
    }
}

pub(crate) fn object_type_key(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Episode => "episode",
        ObjectType::Observation => "observation",
        ObjectType::Entity => "entity",
        ObjectType::MemoryThread => "memory_thread",
        ObjectType::DerivedMemory => "derived_memory",
        ObjectType::MemoryLink => "memory_link",
    }
}

pub(crate) fn retention_state_key(retention_state: RetentionState) -> &'static str {
    match retention_state {
        RetentionState::Active => "active",
        RetentionState::Suppressed => "suppressed",
        RetentionState::Archived => "archived",
        RetentionState::Deleted => "deleted",
    }
}

fn append_intrinsic_edges(edges: &mut HashMap<String, RetrievalStatsEdge>, object: &MemoryObject) {
    match object {
        MemoryObject::Episode(episode) => {
            for entity_id in &episode.participant_entity_ids {
                insert_edge(
                    edges,
                    edge(
                        *entity_id,
                        RelationType::Involves,
                        episode.id,
                        ObjectType::Episode,
                        episode.retention_state,
                        true,
                        episode.created_at,
                    ),
                );
            }
        }
        MemoryObject::Observation(_) => {}
        MemoryObject::DerivedMemory(memory) => {
            for entity_id in &memory.entity_ids {
                insert_edge(
                    edges,
                    edge(
                        *entity_id,
                        RelationType::About,
                        memory.id,
                        ObjectType::DerivedMemory,
                        memory.retention_state,
                        memory.is_current,
                        memory.created_at,
                    ),
                );
            }
        }
        MemoryObject::Entity(_) | MemoryObject::MemoryThread(_) | MemoryObject::MemoryLink(_) => {}
    }
}

fn append_link_edges(
    edges: &mut HashMap<String, RetrievalStatsEdge>,
    link: &MemoryLink,
    object_states: &HashMap<(MemoryId, ObjectType), RetrievalStatsObjectState>,
) {
    if link.from_type == ObjectType::Entity && link.to_type != ObjectType::MemoryLink {
        let (retention_state, is_current) = edge_lifecycle(link.to_id, link.to_type, object_states);
        insert_edge(
            edges,
            edge(
                link.from_id,
                link.relation,
                link.to_id,
                link.to_type,
                retention_state,
                is_current,
                link.created_at,
            ),
        );
    }
    if link.to_type == ObjectType::Entity && link.from_type != ObjectType::MemoryLink {
        let (retention_state, is_current) =
            edge_lifecycle(link.from_id, link.from_type, object_states);
        insert_edge(
            edges,
            edge(
                link.to_id,
                link.relation,
                link.from_id,
                link.from_type,
                retention_state,
                is_current,
                link.created_at,
            ),
        );
    }
}

fn edge_lifecycle(
    object_id: MemoryId,
    object_type: ObjectType,
    object_states: &HashMap<(MemoryId, ObjectType), RetrievalStatsObjectState>,
) -> (RetentionState, bool) {
    object_states
        .get(&(object_id, object_type))
        .map(|state| (state.retention_state, state.is_current))
        .unwrap_or((RetentionState::Active, true))
}

pub(crate) fn insert_edge(
    edges: &mut HashMap<String, RetrievalStatsEdge>,
    edge: RetrievalStatsEdge,
) {
    edges
        .entry(edge.edge_key.clone())
        .and_modify(|existing| merge_edge(existing, &edge))
        .or_insert(edge);
}

fn merge_edge(existing: &mut RetrievalStatsEdge, incoming: &RetrievalStatsEdge) {
    existing.first_seen_at = existing.first_seen_at.min(incoming.first_seen_at);
    existing.last_seen_at = existing.last_seen_at.max(incoming.last_seen_at);
    existing.retention_state =
        more_restrictive_retention(existing.retention_state, incoming.retention_state);
    existing.is_current = existing.is_current && incoming.is_current;
}

fn more_restrictive_retention(left: RetentionState, right: RetentionState) -> RetentionState {
    if right.restrictiveness_rank() > left.restrictiveness_rank() {
        right
    } else {
        left
    }
}

fn edge(
    entity_id: MemoryId,
    relation_kind: RelationType,
    object_id: MemoryId,
    object_type: ObjectType,
    retention_state: RetentionState,
    is_current: bool,
    observed_at: DateTime<Utc>,
) -> RetrievalStatsEdge {
    RetrievalStatsEdge {
        edge_key: format!(
            "{}:{}:{}:{}",
            entity_id,
            relation_type_key(relation_kind),
            object_type_key(object_type),
            object_id
        ),
        entity_id,
        relation_kind,
        object_id,
        object_type,
        retention_state,
        is_current,
        first_seen_at: observed_at,
        last_seen_at: observed_at,
    }
}

fn object_state(object: &MemoryObject) -> Option<RetrievalStatsObjectState> {
    match object {
        MemoryObject::Episode(object) => Some(RetrievalStatsObjectState {
            object_id: object.id,
            object_type: ObjectType::Episode,
            retention_state: object.retention_state,
            is_current: true,
            observed_at: object.created_at,
        }),
        MemoryObject::Observation(object) => Some(RetrievalStatsObjectState {
            object_id: object.id,
            object_type: ObjectType::Observation,
            retention_state: object.retention_state,
            is_current: true,
            observed_at: object.created_at,
        }),
        MemoryObject::DerivedMemory(object) => Some(RetrievalStatsObjectState {
            object_id: object.id,
            object_type: ObjectType::DerivedMemory,
            retention_state: object.retention_state,
            is_current: object.is_current,
            observed_at: object.updated_at,
        }),
        MemoryObject::Entity(_) | MemoryObject::MemoryThread(_) | MemoryObject::MemoryLink(_) => {
            None
        }
    }
}

pub(crate) fn recomputed_counters(
    edges: &HashMap<String, RetrievalStatsEdge>,
) -> HashMap<RetrievalStatsCounterKey, RetrievalStatsCounter> {
    let mut counters = HashMap::new();
    for edge in edges.values() {
        let key = RetrievalStatsCounterKey {
            entity_id: edge.entity_id,
            relation_kind: edge.relation_kind,
            object_type: edge.object_type,
        };
        let counter = counters
            .entry(key)
            .or_insert_with(RetrievalStatsCounter::default);
        counter.total_count += 1;
        if edge.is_active() {
            counter.active_count += 1;
        }
        if edge.is_active() && edge.is_current {
            counter.current_count += 1;
        }
    }
    counters
}

pub(crate) fn recomputed_global_counters(
    edges: &HashMap<String, RetrievalStatsEdge>,
) -> HashMap<(RelationType, ObjectType), RetrievalStatsCounter> {
    let mut counters = HashMap::new();
    for edge in edges.values() {
        let counter = counters
            .entry((edge.relation_kind, edge.object_type))
            .or_insert_with(RetrievalStatsCounter::default);
        counter.total_count += 1;
        if edge.is_active() {
            counter.active_count += 1;
        }
        if edge.is_active() && edge.is_current {
            counter.current_count += 1;
        }
    }
    counters
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::stats::InMemoryRetrievalStatsStore;
    use crate::domain::{
        DerivedMemory, DerivedType, Episode, Modality, Stability, DEFAULT_SCHEMA_VERSION,
    };

    #[tokio::test]
    async fn in_memory_store_counts_edges_idempotently() {
        let store = InMemoryRetrievalStatsStore::new();
        let entity_id = id("550e8400-e29b-41d4-a716-446655460001");
        let episode_id = id("550e8400-e29b-41d4-a716-446655460002");
        let edge = edge(
            entity_id,
            RelationType::Involves,
            episode_id,
            ObjectType::Episode,
            RetentionState::Active,
            true,
            timestamp(),
        );

        store
            .record_edges(std::slice::from_ref(&edge))
            .await
            .unwrap();
        store
            .record_edges(std::slice::from_ref(&edge))
            .await
            .unwrap();

        let counter = store
            .counter(&RetrievalStatsCounterKey {
                entity_id,
                relation_kind: RelationType::Involves,
                object_type: ObjectType::Episode,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 1);
        assert_eq!(counter.current_count, 1);
        let global = store
            .global_counter(RelationType::Involves, ObjectType::Episode)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(global.total_count, 1);
    }

    #[tokio::test]
    async fn in_memory_store_counts_global_relation_object_pairs() {
        let store = InMemoryRetrievalStatsStore::new();
        let first_entity_id = id("550e8400-e29b-41d4-a716-446655460031");
        let second_entity_id = id("550e8400-e29b-41d4-a716-446655460032");
        let first_episode_id = id("550e8400-e29b-41d4-a716-446655460033");
        let second_episode_id = id("550e8400-e29b-41d4-a716-446655460034");

        store
            .record_edges(&[
                edge(
                    first_entity_id,
                    RelationType::Involves,
                    first_episode_id,
                    ObjectType::Episode,
                    RetentionState::Active,
                    true,
                    timestamp(),
                ),
                edge(
                    second_entity_id,
                    RelationType::Involves,
                    second_episode_id,
                    ObjectType::Episode,
                    RetentionState::Suppressed,
                    false,
                    timestamp(),
                ),
            ])
            .await
            .unwrap();

        let counter = store
            .global_counter(RelationType::Involves, ObjectType::Episode)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 2);
        assert_eq!(counter.active_count, 1);
        assert_eq!(counter.current_count, 1);
    }

    #[tokio::test]
    async fn in_memory_store_merges_duplicate_edge_timestamps() {
        let store = InMemoryRetrievalStatsStore::new();
        let entity_id = id("550e8400-e29b-41d4-a716-446655460041");
        let episode_id = id("550e8400-e29b-41d4-a716-446655460042");
        let later_edge = edge(
            entity_id,
            RelationType::Involves,
            episode_id,
            ObjectType::Episode,
            RetentionState::Active,
            true,
            timestamp_at("2026-04-28T13:00:00Z"),
        );
        let earlier_edge = edge(
            entity_id,
            RelationType::Involves,
            episode_id,
            ObjectType::Episode,
            RetentionState::Active,
            true,
            timestamp_at("2026-04-28T11:00:00Z"),
        );

        store.record_edges(&[later_edge]).await.unwrap();
        store.record_edges(&[earlier_edge]).await.unwrap();

        let state = store.state.lock().await;
        let stored = state
            .edges
            .get(&format!("{}:involves:episode:{}", entity_id, episode_id))
            .unwrap();
        assert_eq!(stored.first_seen_at, timestamp_at("2026-04-28T11:00:00Z"));
        assert_eq!(stored.last_seen_at, timestamp_at("2026-04-28T13:00:00Z"));
    }

    #[tokio::test]
    async fn in_memory_store_updates_lifecycle_counts() {
        let store = InMemoryRetrievalStatsStore::new();
        let entity_id = id("550e8400-e29b-41d4-a716-446655460011");
        let memory_id = id("550e8400-e29b-41d4-a716-446655460012");
        store
            .record_edges(&[edge(
                entity_id,
                RelationType::About,
                memory_id,
                ObjectType::DerivedMemory,
                RetentionState::Active,
                true,
                timestamp(),
            )])
            .await
            .unwrap();
        store
            .record_object_states(&[RetrievalStatsObjectState {
                object_id: memory_id,
                object_type: ObjectType::DerivedMemory,
                retention_state: RetentionState::Suppressed,
                is_current: false,
                observed_at: timestamp(),
            }])
            .await
            .unwrap();

        let counter = store
            .counter(&RetrievalStatsCounterKey {
                entity_id,
                relation_kind: RelationType::About,
                object_type: ObjectType::DerivedMemory,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 0);
        assert_eq!(counter.current_count, 0);
    }

    #[tokio::test]
    async fn in_memory_object_state_updates_do_not_regress_last_seen_at() {
        let store = InMemoryRetrievalStatsStore::new();
        let entity_id = id("550e8400-e29b-41d4-a716-446655460051");
        let memory_id = id("550e8400-e29b-41d4-a716-446655460052");
        store
            .record_edges(&[edge(
                entity_id,
                RelationType::About,
                memory_id,
                ObjectType::DerivedMemory,
                RetentionState::Active,
                true,
                timestamp_at("2026-04-28T13:00:00Z"),
            )])
            .await
            .unwrap();

        store
            .record_object_states(&[RetrievalStatsObjectState {
                object_id: memory_id,
                object_type: ObjectType::DerivedMemory,
                retention_state: RetentionState::Suppressed,
                is_current: false,
                observed_at: timestamp_at("2026-04-28T11:00:00Z"),
            }])
            .await
            .unwrap();

        let state = store.state.lock().await;
        let stored = state
            .edges
            .get(&format!("{}:about:derived_memory:{}", entity_id, memory_id))
            .unwrap();
        assert_eq!(stored.last_seen_at, timestamp_at("2026-04-28T13:00:00Z"));
    }

    #[test]
    fn derives_entity_edges_from_objects_and_links() {
        let entity_id = id("550e8400-e29b-41d4-a716-446655460021");
        let episode_id = id("550e8400-e29b-41d4-a716-446655460022");
        let memory_id = id("550e8400-e29b-41d4-a716-446655460023");
        let objects = vec![
            MemoryObject::Episode(Episode {
                id: episode_id,
                object_type: ObjectType::Episode,
                modality: Modality::Chat,
                source_conversation_id: None,
                started_at: None,
                ended_at: None,
                participant_entity_ids: vec![entity_id],
                summary: "episode".to_owned(),
                raw_ref: None,
                salience_score: 0.5,
                retention_state: RetentionState::Active,
                created_at: timestamp(),
                schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
            }),
            MemoryObject::DerivedMemory(DerivedMemory {
                id: memory_id,
                object_type: ObjectType::DerivedMemory,
                derived_type: DerivedType::Reflection,
                text: "memory".to_owned(),
                derived_from_episode_ids: vec![episode_id],
                derived_from_observation_ids: Vec::new(),
                thread_ids: Vec::new(),
                entity_ids: vec![entity_id],
                confidence: 0.7,
                salience_score: 0.7,
                stability: Stability::Medium,
                is_current: true,
                supersedes: Vec::new(),
                retention_state: RetentionState::Active,
                created_at: timestamp(),
                updated_at: timestamp(),
                schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
            }),
        ];

        let edges = retrieval_stats_edges(&objects, &[]);

        assert_eq!(edges.len(), 2);
        assert!(edges.iter().any(|edge| {
            edge.entity_id == entity_id
                && edge.relation_kind == RelationType::Involves
                && edge.object_id == episode_id
        }));
        assert!(edges.iter().any(|edge| {
            edge.entity_id == entity_id
                && edge.relation_kind == RelationType::About
                && edge.object_id == memory_id
        }));
    }

    #[test]
    fn link_edges_use_available_object_lifecycle() {
        let entity_id = id("550e8400-e29b-41d4-a716-446655460031");
        let memory_id = id("550e8400-e29b-41d4-a716-446655460032");
        let link_id = id("550e8400-e29b-41d4-a716-446655460033");
        let objects = vec![MemoryObject::DerivedMemory(DerivedMemory {
            id: memory_id,
            object_type: ObjectType::DerivedMemory,
            derived_type: DerivedType::Reflection,
            text: "memory".to_owned(),
            derived_from_episode_ids: Vec::new(),
            derived_from_observation_ids: Vec::new(),
            thread_ids: Vec::new(),
            entity_ids: Vec::new(),
            confidence: 0.7,
            salience_score: 0.7,
            stability: Stability::Medium,
            is_current: false,
            supersedes: Vec::new(),
            retention_state: RetentionState::Suppressed,
            created_at: timestamp(),
            updated_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        })];
        let link = MemoryLink {
            id: link_id,
            object_type: ObjectType::MemoryLink,
            from_id: entity_id,
            from_type: ObjectType::Entity,
            to_id: memory_id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::About,
            confidence: 1.0,
            rationale: None,
            created_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        };

        let edges = retrieval_stats_edges(&objects, &[link]);

        let edge = edges
            .iter()
            .find(|edge| edge.entity_id == entity_id && edge.object_id == memory_id)
            .unwrap();
        assert_eq!(edge.retention_state, RetentionState::Suppressed);
        assert!(!edge.is_current);
    }

    #[test]
    fn speaker_entity_id_does_not_count_as_mentions() {
        let entity_id = id("550e8400-e29b-41d4-a716-446655460061");
        let observation_id = id("550e8400-e29b-41d4-a716-446655460062");
        let objects = vec![MemoryObject::Observation(crate::domain::Observation {
            id: observation_id,
            object_type: ObjectType::Observation,
            episode_id: id("550e8400-e29b-41d4-a716-446655460063"),
            speaker_entity_id: Some(entity_id),
            observed_at: None,
            modality: Modality::Chat,
            text: "speaker relationship is not a mentions edge".to_owned(),
            raw_ref: None,
            salience_score: 0.5,
            retention_state: RetentionState::Active,
            created_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        })];

        let edges = retrieval_stats_edges(&objects, &[]);

        assert!(edges.is_empty());
    }

    #[test]
    fn duplicate_edge_derivations_merge_lifecycle_deterministically() {
        let entity_id = id("550e8400-e29b-41d4-a716-446655460041");
        let memory_id = id("550e8400-e29b-41d4-a716-446655460042");
        let link_id = id("550e8400-e29b-41d4-a716-446655460043");
        let memory = MemoryObject::DerivedMemory(DerivedMemory {
            id: memory_id,
            object_type: ObjectType::DerivedMemory,
            derived_type: DerivedType::Reflection,
            text: "memory".to_owned(),
            derived_from_episode_ids: Vec::new(),
            derived_from_observation_ids: Vec::new(),
            thread_ids: Vec::new(),
            entity_ids: vec![entity_id],
            confidence: 0.7,
            salience_score: 0.7,
            stability: Stability::Medium,
            is_current: false,
            supersedes: Vec::new(),
            retention_state: RetentionState::Suppressed,
            created_at: timestamp(),
            updated_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        });
        let link = MemoryLink {
            id: link_id,
            object_type: ObjectType::MemoryLink,
            from_id: entity_id,
            from_type: ObjectType::Entity,
            to_id: memory_id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::About,
            confidence: 1.0,
            rationale: None,
            created_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        };

        let edges = retrieval_stats_edges(&[memory], &[link]);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].retention_state, RetentionState::Suppressed);
        assert!(!edges[0].is_current);
    }

    #[tokio::test]
    async fn health_tracks_internal_failure_markers() {
        let store = InMemoryRetrievalStatsStore::new();
        store
            .mark_unhealthy("stats write failed".to_owned())
            .await
            .unwrap();
        store
            .record_edges(&[edge(
                id("550e8400-e29b-41d4-a716-446655460071"),
                RelationType::Involves,
                id("550e8400-e29b-41d4-a716-446655460072"),
                ObjectType::Episode,
                RetentionState::Active,
                true,
                timestamp(),
            )])
            .await
            .unwrap();

        let health = store.health().await.unwrap();
        assert_eq!(health.state, RetrievalStatsHealthState::Unhealthy);
        assert_eq!(
            health.last_error_message.as_deref(),
            Some("stats write failed")
        );
    }

    #[tokio::test]
    async fn fallback_health_marker_survives_successful_writes() {
        let store = InMemoryRetrievalStatsStore::unhealthy(
            "sqlite retrieval stats unavailable; using in-memory fallback".to_owned(),
        );
        let entity_id = id("550e8400-e29b-41d4-a716-446655460051");
        let episode_id = id("550e8400-e29b-41d4-a716-446655460052");

        store
            .record_edges(&[edge(
                entity_id,
                RelationType::Involves,
                episode_id,
                ObjectType::Episode,
                RetentionState::Active,
                true,
                timestamp(),
            )])
            .await
            .unwrap();

        let health = store.health().await.unwrap();
        assert_eq!(health.state, RetrievalStatsHealthState::Unhealthy);
        assert!(health
            .last_error_message
            .as_deref()
            .unwrap()
            .contains("in-memory fallback"));
    }

    fn id(value: &str) -> MemoryId {
        uuid::Uuid::parse_str(value).unwrap()
    }

    fn timestamp() -> DateTime<Utc> {
        timestamp_at("2026-04-28T12:00:00Z")
    }

    fn timestamp_at(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }
}
