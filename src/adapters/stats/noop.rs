#[cfg(test)]
use std::sync::OnceLock;

#[cfg(test)]
use async_trait::async_trait;

#[cfg(test)]
use crate::domain::{ObjectType, RelationType};
#[cfg(test)]
use crate::errors::{RetrievalStatsHealthCause, RetrievalStatsStoreError};
#[cfg(test)]
use crate::ports::retrieval_stats::{
    RetrievalStatsCounter, RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth,
    RetrievalStatsObjectState, RetrievalStatsStore,
};

#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct NoopRetrievalStatsStore;

#[cfg(test)]
#[async_trait]
impl RetrievalStatsStore for NoopRetrievalStatsStore {
    async fn record_edges(
        &self,
        _edges: &[RetrievalStatsEdge],
    ) -> Result<(), RetrievalStatsStoreError> {
        Ok(())
    }

    async fn record_object_states(
        &self,
        _states: &[RetrievalStatsObjectState],
    ) -> Result<(), RetrievalStatsStoreError> {
        Ok(())
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
        Ok(RetrievalStatsHealth::default())
    }

    async fn mark_unhealthy(
        &self,
        _cause: RetrievalStatsHealthCause,
    ) -> Result<(), RetrievalStatsStoreError> {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn noop_retrieval_stats_store() -> &'static dyn RetrievalStatsStore {
    static STORE: OnceLock<NoopRetrievalStatsStore> = OnceLock::new();
    STORE.get_or_init(NoopRetrievalStatsStore::default)
}
