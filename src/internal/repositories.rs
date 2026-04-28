mod embedder;
mod graph_authority_store;
mod link_pipeline;
mod memory_repository;
mod raw_reference_resolver;
mod remember_pipeline;
#[cfg(test)]
pub(crate) mod test_support;
mod vector_candidate_store;
mod vector_memory_repository;

// Transitional contract surface: remove this allow once adapter or
// pipeline code consumes the embedder contract directly, or prune the re-export.
#[allow(unused_imports)]
pub(crate) use embedder::MemoryEmbedder;

// Transitional contract surface: remove this allow once adapter or
// pipeline code consumes the graph authority contract directly, or prune unused
// exports.
#[allow(unused_imports)]
pub(crate) use graph_authority_store::{
    bounded_expansion_node_set, GraphAuthorityStore, GraphExpansion, GraphExpansionQuery,
    GraphObjectQuery,
};

// Transitional contract surface: remove this allow once facade wiring
// consumes the link pipeline directly, or prune unused outcome types.
#[allow(unused_imports)]
pub(crate) use link_pipeline::LinkPipeline;
pub(crate) use memory_repository::MemoryRepository;

// Transitional contract surface: remove this allow once adapter or
// pipeline code consumes raw-reference resolution directly, or prune unused
// exports.
#[allow(unused_imports)]
pub(crate) use raw_reference_resolver::{RawReference, RawReferenceResolver};

// Transitional contract surface: remove this allow once facade wiring
// consumes the remember pipeline directly, or prune unused outcome types.
#[allow(unused_imports)]
pub(crate) use remember_pipeline::{
    RememberPipeline, RememberPipelineDraft, RememberPipelineOutcome,
    VectorIndexingFailure as InternalVectorIndexingFailure,
};

// Transitional contract surface: remove this allow once adapter or
// pipeline code consumes vector candidate storage directly, or prune the re-export.
#[allow(unused_imports)]
pub(crate) use vector_candidate_store::VectorCandidateStore;
pub(crate) use vector_memory_repository::VectorMemoryRepository;
