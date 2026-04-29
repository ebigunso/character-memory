mod candidate_record;
mod embedding_model;
mod embedding_surface;
mod record;
mod vector_metadata;

// Transitional contract surface: these re-exports are consumed by test
// support before production adapters use all of them. Remove once adapter or
// pipeline code consumes the surface directly, or prune unused exports.
#[allow(unused_imports)]
pub(crate) use candidate_record::{
    default_vector_candidate_object_types, EmbeddingInput, VectorCandidateFilters,
    VectorCandidateMatch, VectorCandidateRecord, VectorCandidateSearch, VectorSurface,
    VectorTimeField, VectorTimeRangeFilter,
};
pub(crate) use embedding_model::EmbeddingModel;
#[allow(unused_imports)]
pub(crate) use embedding_surface::{
    derived_memory_vector_record, entity_vector_record, episode_vector_record,
    memory_object_vector_record, memory_thread_vector_record, observation_vector_record,
};
#[allow(unused_imports)]
pub(crate) use record::{
    VectorPayloadHints, VectorRecord, VectorRecordEmbedding, VectorRelationshipHints,
};
pub(crate) use vector_metadata::VectorMetadata;
