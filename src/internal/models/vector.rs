mod candidate_record;
mod embedding_model;
mod vector_metadata;

// Transitional v0.1 contract surface: these re-exports are consumed by test
// support before production adapters use all of them. Remove once adapter or
// pipeline code consumes the surface directly, or prune unused exports.
#[allow(unused_imports)]
pub(crate) use candidate_record::{
    EmbeddingInput, VectorCandidateMatch, VectorCandidateRecord, VectorCandidateSearch,
    VectorSurface,
};
pub(crate) use embedding_model::EmbeddingModel;
pub(crate) use vector_metadata::VectorMetadata;
