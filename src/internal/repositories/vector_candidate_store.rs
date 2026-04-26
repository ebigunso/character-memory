#![allow(dead_code)]

use async_trait::async_trait;

use crate::api::types::MemoryId;
use crate::errors::CustomError;
use crate::internal::models::vector::{
    VectorCandidateMatch, VectorCandidateRecord, VectorCandidateSearch,
};

#[async_trait]
pub(crate) trait VectorCandidateStore: Send + Sync {
    async fn upsert_candidates(
        &self,
        candidates: &[VectorCandidateRecord],
    ) -> Result<(), CustomError>;

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<Vec<VectorCandidateMatch>, CustomError>;

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError>;
}

#[async_trait]
impl<T: VectorCandidateStore + ?Sized> VectorCandidateStore for Box<T> {
    async fn upsert_candidates(
        &self,
        candidates: &[VectorCandidateRecord],
    ) -> Result<(), CustomError> {
        (**self).upsert_candidates(candidates).await
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
