// Transitional v0.1 repository contract: vector adapters and pipelines will
// consume this after the contract chunk. Remove once production consumers use
// the full surface, or prune unused methods.
#![allow(dead_code)]

use async_trait::async_trait;

use crate::api::types::MemoryId;
use crate::errors::CustomError;
use crate::internal::models::vector::{
    VectorCandidateMatch, VectorCandidateSearch, VectorRecordEmbedding,
};

#[async_trait]
pub(crate) trait VectorCandidateStore: Send + Sync {
    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError>;

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<Vec<VectorCandidateMatch>, CustomError>;

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
    ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
        (**self).search_candidates(query).await
    }

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
        (**self).delete_candidates(object_ids).await
    }
}
