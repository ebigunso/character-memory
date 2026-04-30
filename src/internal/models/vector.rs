mod candidate_record;
mod embedding_model;
mod embedding_surface;
mod record;

// Provider-neutral vector model surface. Adapters, pipelines, and test
// support intentionally consume different subsets of these helpers.
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
