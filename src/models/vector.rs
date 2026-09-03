mod candidate_record;
mod embedding_model;
mod record;

#[cfg(test)]
pub(crate) use crate::domain::VectorSurface;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::domain::{MemoryId, MemoryObjectRef, ObjectType, DEFAULT_SCHEMA_VERSION};
#[cfg(test)]
pub(crate) use candidate_record::VectorCandidateRecord;
pub(crate) use candidate_record::{
    CanonicalCandidates, EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch,
};
pub(crate) use embedding_model::EmbeddingModel;
pub(crate) use record::{VectorRecord, VectorRecordEmbedding};

#[cfg(any(test, feature = "test-fixtures"))]
#[doc(hidden)]
pub fn zero_norm_record_fixture() -> (
    MemoryObjectRef,
    crate::domain::VectorSurface,
    &'static str,
    &'static str,
    Vec<f32>,
) {
    (
        MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(1)),
        crate::domain::VectorSurface::Summary,
        DEFAULT_SCHEMA_VERSION,
        "Episode summary",
        vec![0.0, 0.0],
    )
}
