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
