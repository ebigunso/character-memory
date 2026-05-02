mod correction_forget_pipeline;
mod embedder;
mod graph_authority_store;
mod link_pipeline;
mod raw_reference_resolver;
mod reconciliation;
mod remember_pipeline;
mod retrieve_pipeline;
#[cfg(test)]
pub(crate) mod test_support;
mod vector_candidate_store;

// Internal contract surface. Pipelines, adapters, and test support consume
// different subsets, so keep the module boundary stable.
#[allow(unused_imports)]
pub(crate) use embedder::MemoryEmbedder;

// Lifecycle pipeline surface.
#[allow(unused_imports)]
pub(crate) use correction_forget_pipeline::CorrectionForgetPipeline;

// Graph-authority contract surface. Retrieval and lifecycle code use
// different subsets of the query/expansion helpers.
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

// Raw-reference contracts remain internal until production raw storage lands.
#[allow(unused_imports)]
pub(crate) use raw_reference_resolver::{RawReference, RawReferenceResolver};

// Internal/admin reconciliation diagnostics. These remain out of the public
// CharacterMemory facade until a governance surface is planned.
#[allow(unused_imports)]
pub(crate) use reconciliation::{
    reconcile_graph_vector_stores, ReconciliationDiagnostic, ReconciliationDriftKind,
    ReconciliationReport,
};

// Remember pipeline surface; outcome types are converted at the public
// facade boundary.
#[allow(unused_imports)]
pub(crate) use remember_pipeline::{
    RememberPipeline, RememberPipelineDraft, RememberPipelineOutcome,
    VectorIndexingFailure as InternalVectorIndexingFailure,
};

// Continuity retrieval pipeline surface.
#[allow(unused_imports)]
pub(crate) use retrieve_pipeline::RetrievePipeline;

// Vector candidate recall contract surface.
#[allow(unused_imports)]
pub(crate) use vector_candidate_store::VectorCandidateStore;
