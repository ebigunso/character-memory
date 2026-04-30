mod correction_forget_pipeline;
mod embedder;
mod graph_authority_store;
mod link_pipeline;
mod raw_reference_resolver;
mod remember_pipeline;
mod retrieve_pipeline;
#[cfg(test)]
pub(crate) mod test_support;
mod vector_candidate_store;

// Transitional contract surface: remove this allow once adapter or
// pipeline code consumes the embedder contract directly, or prune the re-export.
#[allow(unused_imports)]
pub(crate) use embedder::MemoryEmbedder;

// Transitional contract surface: remove this allow once facade wiring
// consumes lifecycle mutation pipelines directly, or prune unused outcome types.
#[allow(unused_imports)]
pub(crate) use correction_forget_pipeline::{CorrectionForgetPipeline, LifecyclePipelineOutcome};

// Transitional contract surface: remove this allow once adapter or
// pipeline code consumes the graph authority contract directly, or prune unused
// exports.
#[allow(unused_imports)]
pub(crate) use graph_authority_store::{
    bounded_expansion, bounded_expansion_node_set, derived_memories_by_provenance,
    derived_memories_by_thread, GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery,
    GraphDerivedMemoryThreadQuery, GraphExpansion, GraphExpansionBoundedFailure,
    GraphExpansionBoundedFailureReason, GraphExpansionFailurePolicy, GraphExpansionFilteredNode,
    GraphExpansionFilteredReason, GraphExpansionLifecyclePolicy, GraphExpansionQuery,
    GraphExpansionRelation, GraphObjectQuery, GraphObjectRef,
};

#[allow(unused_imports)]
pub(crate) use link_pipeline::LinkPipeline;

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

// Transitional contract surface: remove this allow once facade wiring
// consumes the retrieve pipeline directly, or prune unused outcome helpers.
#[allow(unused_imports)]
pub(crate) use retrieve_pipeline::RetrievePipeline;

// Transitional contract surface: remove this allow once adapter or
// pipeline code consumes vector candidate storage directly, or prune the re-export.
#[allow(unused_imports)]
pub(crate) use vector_candidate_store::VectorCandidateStore;
