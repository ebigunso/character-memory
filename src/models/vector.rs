mod candidate_record;
mod embedding_model;
mod record;

// Provider-neutral vector model surface. Adapters, pipelines, and test
// support intentionally consume different subsets of these helpers.
// Candidate query builders are used by adapter/test subsets; remove when all callers import concrete modules.
#[allow(unused_imports)]
pub(crate) use candidate_record::{
    canonicalize_vector_candidates, default_vector_candidate_object_types, EmbeddingInput,
    VectorCandidateFilters, VectorCandidateMatch, VectorCandidateRecord, VectorCandidateSearch,
    VectorSurface, VectorTimeField, VectorTimeRangeFilter,
};
pub(crate) use embedding_model::EmbeddingModel;
// Diagnostic/vector-record helpers are used by adapter/test subsets; remove when all callers import concrete modules.
#[allow(unused_imports)]
pub(crate) use record::{
    VectorCandidateDiagnosticRecord, VectorPayloadHints, VectorRecord, VectorRecordEmbedding,
    VectorRelationshipHints,
};
