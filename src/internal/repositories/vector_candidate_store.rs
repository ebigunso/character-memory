// Vector candidate recall contract. Qdrant is the default adapter, while
// tests use deterministic fake stores.
#![allow(dead_code)]

use async_trait::async_trait;

use crate::api::types::MemoryId;
use crate::errors::CustomError;
use crate::internal::models::vector::{
    VectorCandidateDiagnosticRecord, VectorCandidateMatch, VectorCandidateSearch,
    VectorRecordEmbedding,
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

    async fn list_candidate_diagnostics(
        &self,
    ) -> Result<Vec<VectorCandidateDiagnosticRecord>, CustomError> {
        Ok(Vec::new())
    }

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

    async fn list_candidate_diagnostics(
        &self,
    ) -> Result<Vec<VectorCandidateDiagnosticRecord>, CustomError> {
        (**self).list_candidate_diagnostics().await
    }

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
        (**self).delete_candidates(object_ids).await
    }
}
