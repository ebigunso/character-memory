mod candidate_record;
mod embedding_model;
mod record;

#[cfg(test)]
pub(crate) use crate::domain::VectorSurface;
#[cfg(test)]
pub(crate) use candidate_record::VectorCandidateRecord;
pub(crate) use candidate_record::{
    CanonicalCandidates, EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch,
};
pub(crate) use embedding_model::EmbeddingModel;
pub(crate) use record::{VectorRecord, VectorRecordEmbedding};
