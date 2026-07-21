// Vector candidate recall contract. Qdrant is the default adapter, while
// tests use deterministic fake stores.
use async_trait::async_trait;

use crate::domain::MemoryId;
use crate::errors::CustomError;
use crate::models::vector::{
    CanonicalCandidates, VectorCandidateDiagnosticRecord, VectorCandidateSearch,
    VectorRecordEmbedding,
};

#[async_trait]
pub(crate) trait VectorCandidateStore: Send + Sync {
    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError>;

    /// Returns at most `query.limit` unique object/surface matches in canonical
    /// score-descending, object-type, object-id, surface order. Adapters close
    /// equal-score cutoff cohorts before canonical truncation, subject to their
    /// documented bounded-overfetch degradation policy.
    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<CanonicalCandidates, CustomError>;

    // Reconciliation diagnostics are dormant; remove when vector candidate diagnostics are retired.
    #[allow(dead_code)]
    async fn list_candidate_diagnostics(
        &self,
    ) -> Result<Vec<VectorCandidateDiagnosticRecord>, CustomError>;

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
    ) -> Result<CanonicalCandidates, CustomError> {
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
