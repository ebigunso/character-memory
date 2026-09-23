use std::collections::{HashMap, HashSet};

use crate::api::types::VectorIndexingFailure;
use crate::domain::{MemoryObjectRef, ObjectType};
use crate::errors::{CustomError, VectorIndexingCause};
use crate::models::vector::{EmbeddingInput, VectorRecord, VectorRecordEmbedding};
use crate::ports::graph_authority::GraphAuthorityStore;
use crate::ports::vector_candidate::VectorCandidateStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VectorIndexingOutcome {
    pub(crate) indexed_objects: Vec<MemoryObjectRef>,
    pub(crate) failure: Option<VectorIndexingFailure>,
}

pub(crate) struct VectorIndexingService<'a, V>
where
    V: VectorCandidateStore + ?Sized,
{
    vector_store: &'a V,
}

impl<'a, V> VectorIndexingService<'a, V>
where
    V: VectorCandidateStore + ?Sized,
{
    pub(crate) fn new(vector_store: &'a V) -> Self {
        Self { vector_store }
    }

    pub(crate) async fn index<G: GraphAuthorityStore + ?Sized>(
        &self,
        graph_store: &G,
        mut records: Vec<VectorRecord>,
        inputs: &[EmbeddingInput],
        embeddings: Result<Vec<Vec<f32>>, CustomError>,
    ) -> Result<VectorIndexingOutcome, CustomError> {
        let expected = inputs.len();
        let derived_ids = records
            .iter()
            .filter(|record| record.object_type == ObjectType::DerivedMemory)
            .map(|record| record.object_id)
            .collect::<Vec<_>>();
        if !derived_ids.is_empty() {
            let superseded = match graph_store
                .query_superseded_derived_memory_ids(&derived_ids)
                .await
            {
                Ok(ids) => ids,
                Err(error) => {
                    return Ok(failed(
                        record_objects(&records),
                        VectorIndexingCause::GraphQuery(error),
                    ))
                }
            };
            records.retain(|record| {
                record.object_type != ObjectType::DerivedMemory
                    || !superseded.contains(&record.object_id)
            });
        }
        if records.is_empty() {
            return Ok(VectorIndexingOutcome {
                indexed_objects: Vec::new(),
                failure: None,
            });
        }

        let objects = record_objects(&records);
        let embeddings = match embeddings {
            Ok(embeddings) => embeddings,
            Err(CustomError::Embedding(error)) => {
                return Ok(failed(objects, VectorIndexingCause::Embedding(error)));
            }
            Err(error) => return Err(error),
        };

        if embeddings.len() != expected {
            let actual = embeddings.len();
            return Ok(failed(
                objects,
                VectorIndexingCause::CardinalityMismatch { expected, actual },
            ));
        }

        let embeddings = inputs
            .iter()
            .zip(embeddings)
            .map(|(input, embedding)| {
                (
                    (input.object_type, input.object_id, input.surface),
                    (input.text.as_str(), embedding),
                )
            })
            .collect::<HashMap<_, _>>();
        let record_key = |record: &VectorRecord| {
            (
                Some(record.object_type),
                Some(record.object_id),
                record.surface,
            )
        };
        let matched = records
            .iter()
            .filter(|record| embeddings.contains_key(&record_key(record)))
            .count();
        if matched != records.len() {
            return Ok(failed(
                objects,
                VectorIndexingCause::CardinalityMismatch {
                    expected: records.len(),
                    actual: matched,
                },
            ));
        }
        if let Some(record) = records.iter().find(|record| {
            embeddings[&record_key(record)]
                .1
                .iter()
                .all(|value| *value == 0.0)
        }) {
            let object = MemoryObjectRef::new(record.object_type, record.object_id);
            return Ok(failed(
                objects,
                VectorIndexingCause::ZeroNormEmbedding { object },
            ));
        }

        let record_embeddings = records
            .iter()
            .map(|record| {
                let (text, embedding) = &embeddings[&record_key(record)];
                debug_assert_eq!(
                    *text, record.embedding_text,
                    "paired embedding text changed"
                );
                VectorRecordEmbedding::new(record, embedding)
            })
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

pub(crate) async fn delete_vectors<V: VectorCandidateStore + ?Sized>(
    vector_store: &V,
    objects: &[MemoryObjectRef],
) -> Result<Option<crate::api::types::VectorMaintenanceFailureItem>, CustomError> {
    if objects.is_empty() {
        return Ok(None);
    }
    match vector_store.delete_candidates(objects).await {
        Ok(()) => Ok(None),
        Err(CustomError::VectorDatabaseError(error)) => {
            Ok(Some(crate::api::types::VectorMaintenanceFailureItem {
                operation: crate::api::types::VectorMaintenanceOperation::Delete,
                objects: objects.to_vec(),
                cause: VectorIndexingCause::VectorDatabase(error),
            }))
        }
        Err(error) => Err(error),
    }
}

fn record_objects(records: &[VectorRecord]) -> Vec<MemoryObjectRef> {
    let mut seen = HashSet::new();
    records
        .iter()
        .map(|record| MemoryObjectRef::new(record.object_type, record.object_id))
        .filter(|object| seen.insert(*object))
        .collect()
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
    use crate::ports::embedder::MemoryEmbedder;
    use async_trait::async_trait;

    use crate::domain::{MemoryId, VectorSurface, DEFAULT_SCHEMA_VERSION};
    use crate::models::vector::VectorCandidateSearch;
    use crate::ports::vector_candidate::VectorCandidateRecall;

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

        async fn delete_candidates(&self, _objects: &[MemoryObjectRef]) -> Result<(), CustomError> {
            unreachable!("deletion is not part of this test")
        }
    }

    #[tokio::test]
    async fn zero_norm_record_embedding_is_typed_failure_before_adapter() {
        let object = MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(1));
        let record = VectorRecord::new(
            object.id,
            object.object_type,
            VectorSurface::Summary,
            DEFAULT_SCHEMA_VERSION,
            "Episode summary",
        );
        let store = AdapterMustNotRun;
        let embedder = crate::test_support::TestEmbedder(|_: &EmbeddingInput| vec![0.0, 0.0]);
        let service = VectorIndexingService::new(&store);
        let inputs = [record.embedding_input()];
        let embeddings = embedder.embed_batch(&inputs).await;

        let outcome = service
            .index(
                &crate::test_support::in_memory_graph_store(),
                vec![record],
                &inputs,
                embeddings,
            )
            .await
            .expect("typed outcome");

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
