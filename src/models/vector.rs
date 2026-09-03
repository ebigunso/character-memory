mod candidate_record;
mod embedding_model;
mod record;

#[cfg(test)]
pub(crate) use crate::domain::VectorSurface;
#[cfg(test)]
use crate::domain::{MemoryId, ObjectType, DEFAULT_SCHEMA_VERSION};
#[cfg(test)]
pub(crate) use candidate_record::VectorCandidateRecord;
pub(crate) use candidate_record::{
    CanonicalCandidates, EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch,
};
pub(crate) use embedding_model::EmbeddingModel;
pub(crate) use record::{VectorRecord, VectorRecordEmbedding};

#[cfg(test)]
pub(crate) fn zero_norm_record_fixture() -> (VectorRecord, Vec<f32>) {
    (
        VectorRecord::new(
            MemoryId::from_u128(1),
            ObjectType::Episode,
            VectorSurface::Summary,
            DEFAULT_SCHEMA_VERSION,
            "Episode summary",
        ),
        vec![0.0, 0.0],
    )
}
