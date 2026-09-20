// Vector candidate recall contract shared by the embedded and service adapters.
use async_trait::async_trait;

use crate::api::types::retrieval::VectorRecallCompleteness;
use crate::domain::MemoryObjectRef;
use crate::errors::CustomError;
use crate::models::vector::{CanonicalCandidates, VectorCandidateSearch, VectorRecordEmbedding};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorCandidateRecall {
    pub(crate) candidates: CanonicalCandidates,
    pub(crate) completeness: VectorRecallCompleteness,
}

#[async_trait]
pub(crate) trait VectorCandidateStore: Send + Sync {
    /// Releases local resources; stores without a local owner need no shutdown.
    async fn close(&self) -> Result<(), CustomError> {
        Ok(())
    }

    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError>;

    /// Returns at most `query.limit` unique object/surface matches in canonical
    /// score-descending, object-type, object-id, surface order.
    ///
    /// The shared fetch loop closes every score tie that crosses the requested
    /// limit. `Exhaustive` is reported only when every record in the requested
    /// scope was scored through a path the adapter knows to be exhaustive and the
    /// cutoff cohort closed; `scanned` is the number of scoped records actually scored
    /// by that path. An index-produced result prefix reports whether its boundary tie
    /// closed or remained open at the fetch bound.
    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<VectorCandidateRecall, CustomError>;

    /// Deletes every surface of each typed object, preserving other object types with the same ID.
    async fn delete_candidates(&self, objects: &[MemoryObjectRef]) -> Result<(), CustomError>;
}

#[async_trait]
impl<T: VectorCandidateStore + ?Sized> VectorCandidateStore for Box<T> {
    async fn close(&self) -> Result<(), CustomError> {
        (**self).close().await
    }

    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        (**self).upsert_vector_records(records).await
    }

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<VectorCandidateRecall, CustomError> {
        (**self).search_candidates(query).await
    }

    async fn delete_candidates(&self, objects: &[MemoryObjectRef]) -> Result<(), CustomError> {
        (**self).delete_candidates(objects).await
    }
}
