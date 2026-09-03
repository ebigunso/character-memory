// Vector candidate recall contract shared by the embedded and service adapters.
use async_trait::async_trait;

use crate::api::types::retrieval::VectorRecallCompleteness;
use crate::domain::MemoryId;
use crate::errors::CustomError;
use crate::models::vector::{CanonicalCandidates, VectorCandidateSearch, VectorRecordEmbedding};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorCandidateRecall {
    pub(crate) candidates: CanonicalCandidates,
    pub(crate) completeness: VectorRecallCompleteness,
}

#[async_trait]
pub(crate) trait VectorCandidateStore: Send + Sync {
    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError>;

    /// Returns at most `query.limit` unique object/surface matches in canonical
    /// score-descending, object-type, object-id, surface order.
    ///
    /// The shared fetch loop closes every score tie that crosses the requested
    /// limit. `Exhaustive` is reported only for a closed cohort from an unindexed
    /// shard; indexed recall reports whether the returned prefix closed its
    /// boundary tie or remained open at the fetch bound.
    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<VectorCandidateRecall, CustomError>;

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError>;
}

#[async_trait]
impl<T: VectorCandidateStore + ?Sized> VectorCandidateStore for Box<T> {
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

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
        (**self).delete_candidates(object_ids).await
    }
}
