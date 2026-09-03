use crate::api::types::VectorIndexingFailure;
use crate::domain::MemoryObjectRef;
use crate::errors::{CustomError, VectorIndexingCause};
use crate::models::vector::{VectorRecord, VectorRecordEmbedding};
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::vector_candidate::VectorCandidateStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VectorIndexingOutcome {
    pub(crate) indexed_objects: Vec<MemoryObjectRef>,
    pub(crate) failure: Option<VectorIndexingFailure>,
}

pub(crate) struct VectorIndexingService<'a, V, E>
where
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    vector_store: &'a V,
    embedder: &'a E,
}

impl<'a, V, E> VectorIndexingService<'a, V, E>
where
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    pub(crate) fn new(vector_store: &'a V, embedder: &'a E) -> Self {
        Self {
            vector_store,
            embedder,
        }
    }

    pub(crate) async fn index(
        &self,
        records: Vec<VectorRecord>,
    ) -> Result<VectorIndexingOutcome, CustomError> {
        if records.is_empty() {
            return Ok(VectorIndexingOutcome {
                indexed_objects: Vec::new(),
                failure: None,
            });
        }

        let objects = records
            .iter()
            .map(|record| MemoryObjectRef::new(record.object_type, record.object_id))
            .collect::<Vec<_>>();
        let embedding_inputs = records
            .iter()
            .map(VectorRecord::embedding_input)
            .collect::<Vec<_>>();
        let embeddings = match self.embedder.embed_batch(&embedding_inputs).await {
            Ok(embeddings) => embeddings,
            Err(CustomError::Embedding(error)) => {
                return Ok(failed(objects, VectorIndexingCause::Embedding(error)));
            }
            Err(error) => return Err(error),
        };

        if embeddings.len() != records.len() {
            let expected = records.len();
            let actual = embeddings.len();
            return Ok(failed(
                objects,
                VectorIndexingCause::CardinalityMismatch { expected, actual },
            ));
        }

        if let Some(index) = embeddings
            .iter()
            .position(|embedding| embedding.iter().all(|value| *value == 0.0))
        {
            let object = objects[index];
            return Ok(failed(
                objects,
                VectorIndexingCause::ZeroNormEmbedding { object },
            ));
        }

        let record_embeddings = records
            .iter()
            .zip(embeddings.iter())
            .map(|(record, embedding)| VectorRecordEmbedding::new(record, embedding))
            .collect::<Vec<_>>();
        match self
            .vector_store
            .upsert_vector_records(&record_embeddings)
            .await
        {
            Ok(()) => Ok(VectorIndexingOutcome {
                indexed_objects: objects,
                failure: None,
            }),
            Err(CustomError::VectorDatabaseError(error)) => {
                Ok(failed(objects, VectorIndexingCause::VectorDatabase(error)))
            }
            Err(error) => Err(error),
        }
    }
}

fn failed(objects: Vec<MemoryObjectRef>, cause: VectorIndexingCause) -> VectorIndexingOutcome {
    VectorIndexingOutcome {
        indexed_objects: Vec::new(),
        failure: Some(VectorIndexingFailure {
            unindexed_objects: objects,
            cause,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    use crate::domain::MemoryId;
    use crate::models::vector::{zero_norm_record_fixture, EmbeddingInput, VectorCandidateSearch};
    use crate::ports::vector_candidate::VectorCandidateRecall;

    struct FixedEmbedder(Vec<f32>);

    #[async_trait]
    impl MemoryEmbedder for FixedEmbedder {
        async fn embed(&self, _input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
            Ok(self.0.clone())
        }

        async fn embed_batch(
            &self,
            inputs: &[EmbeddingInput],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            Ok(vec![self.0.clone(); inputs.len()])
        }
    }

    struct AdapterMustNotRun;

    #[async_trait]
    impl VectorCandidateStore for AdapterMustNotRun {
        async fn upsert_vector_records(
            &self,
            _records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            panic!("zero-norm embeddings must be rejected before the adapter")
        }

        async fn search_candidates(
            &self,
            _query: &VectorCandidateSearch,
        ) -> Result<VectorCandidateRecall, CustomError> {
            unreachable!("search is not part of this test")
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            unreachable!("deletion is not part of this test")
        }
    }

    #[tokio::test]
    async fn zero_norm_record_embedding_is_typed_failure_before_adapter() {
        let (record, embedding) = zero_norm_record_fixture();
        let object = MemoryObjectRef::new(record.object_type, record.object_id);
        let store = AdapterMustNotRun;
        let embedder = FixedEmbedder(embedding);
        let service = VectorIndexingService::new(&store, &embedder);

        let outcome = service.index(vec![record]).await.expect("typed outcome");

        assert!(outcome.indexed_objects.is_empty());
        assert_eq!(
            outcome.failure,
            Some(VectorIndexingFailure {
                unindexed_objects: vec![object],
                cause: VectorIndexingCause::ZeroNormEmbedding { object },
            })
        );
    }
}
