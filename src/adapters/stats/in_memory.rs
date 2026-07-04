use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::api::types::{ObjectType, RelationType};
use crate::errors::CustomError;
use crate::ports::retrieval_stats::{
    insert_edge, recomputed_counters, recomputed_global_counters, RetrievalStatsCounter,
    RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth, RetrievalStatsHealthState,
    RetrievalStatsObjectState, RetrievalStatsStore,
};
#[derive(Debug, Default)]
pub(crate) struct InMemoryRetrievalStatsStore {
    pub(crate) state: Mutex<InMemoryState>,
}

impl InMemoryRetrievalStatsStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn unhealthy(message: String) -> Self {
        Self {
            state: Mutex::new(InMemoryState {
                health: RetrievalStatsHealth {
                    state: RetrievalStatsHealthState::Unhealthy,
                    last_error_message: Some(message),
                },
                ..InMemoryState::default()
            }),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct InMemoryState {
    pub(crate) edges: HashMap<String, RetrievalStatsEdge>,
    pub(crate) counters: HashMap<RetrievalStatsCounterKey, RetrievalStatsCounter>,
    pub(crate) global_counters: HashMap<(RelationType, ObjectType), RetrievalStatsCounter>,
    pub(crate) counters_dirty: bool,
    pub(crate) health: RetrievalStatsHealth,
    pub(crate) rejected_low_information_link_count: u64,
}

#[async_trait]
impl RetrievalStatsStore for InMemoryRetrievalStatsStore {
    async fn record_edges(&self, edges: &[RetrievalStatsEdge]) -> Result<(), CustomError> {
        let mut state = self.state.lock().await;
        for edge in edges {
            insert_edge(&mut state.edges, edge.clone());
        }
        state.counters_dirty = true;
        Ok(())
    }

    async fn record_object_states(
        &self,
        states: &[RetrievalStatsObjectState],
    ) -> Result<(), CustomError> {
        let mut state = self.state.lock().await;
        for object_state in states {
            for edge in state.edges.values_mut() {
                if edge.object_id == object_state.object_id
                    && edge.object_type == object_state.object_type
                {
                    edge.retention_state = object_state.retention_state;
                    edge.is_current = object_state.is_current;
                    edge.last_seen_at = edge.last_seen_at.max(object_state.observed_at);
                }
            }
        }
        state.counters_dirty = true;
        Ok(())
    }

    async fn counter(
        &self,
        key: &RetrievalStatsCounterKey,
    ) -> Result<Option<RetrievalStatsCounter>, CustomError> {
        let mut state = self.state.lock().await;
        state.refresh_counters_if_dirty();
        Ok(state.counters.get(key).copied())
    }

    async fn global_counter(
        &self,
        relation_kind: RelationType,
        object_type: ObjectType,
    ) -> Result<Option<RetrievalStatsCounter>, CustomError> {
        let mut state = self.state.lock().await;
        state.refresh_counters_if_dirty();
        Ok(state
            .global_counters
            .get(&(relation_kind, object_type))
            .copied())
    }

    async fn health(&self) -> Result<RetrievalStatsHealth, CustomError> {
        Ok(self.state.lock().await.health.clone())
    }

    async fn mark_unhealthy(&self, message: String) -> Result<(), CustomError> {
        let mut state = self.state.lock().await;
        state.health = RetrievalStatsHealth {
            state: RetrievalStatsHealthState::Unhealthy,
            last_error_message: Some(message),
        };
        Ok(())
    }
    async fn record_rejected_low_information_link(&self) -> Result<(), CustomError> {
        let mut state = self.state.lock().await;
        state.rejected_low_information_link_count += 1;
        Ok(())
    }

    async fn rejected_low_information_link_count(&self) -> Result<u64, CustomError> {
        Ok(self.state.lock().await.rejected_low_information_link_count)
    }
}

impl InMemoryState {
    fn refresh_counters_if_dirty(&mut self) {
        if !self.counters_dirty {
            return;
        }

        self.counters = recomputed_counters(&self.edges);
        self.global_counters = recomputed_global_counters(&self.edges);
        self.counters_dirty = false;
    }
}
