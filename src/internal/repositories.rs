mod correction_forget_pipeline;
mod embedder;
mod graph_authority_store;
mod link_pipeline;
mod reconciliation;
mod remember_pipeline;
mod retrieval_selectivity;
mod retrieval_stats_store;
mod retrieve_pipeline;
mod source_reference;
#[cfg(test)]
pub(crate) mod test_support;
mod vector_candidate_store;
mod write_planning;

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
    apply_fanout_limits_by_pair, bounded_expansion, bounded_expansion_node_set,
    bounded_hub_retention_limit, derived_memories_by_provenance, derived_memories_by_thread,
    GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
    GraphExpansion, GraphExpansionBoundedFailure, GraphExpansionBoundedFailureReason,
    GraphExpansionFailurePolicy, GraphExpansionFanoutOverride, GraphExpansionFilteredNode,
    GraphExpansionFilteredReason, GraphExpansionLifecyclePolicy, GraphExpansionQuery,
    GraphExpansionRelation, GraphObjectQuery, GraphObjectRef,
};

#[allow(unused_imports)]
pub(crate) use link_pipeline::{
    admit_link, LinkAdmissionDecision, LinkAdmissionEvidence, LinkPipeline,
};

// Source-reference contracts remain internal; core stores opaque references,
// not caller-owned source material.
#[allow(unused_imports)]
pub(crate) use source_reference::{ResolvedSourceReference, SourceReferenceResolver};

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

#[cfg(test)]
pub(crate) use retrieval_stats_store::noop_retrieval_stats_store;
#[allow(unused_imports)]
pub(crate) use retrieval_stats_store::{
    object_type_key, record_stats_after_write, relation_type_key, retention_state_key,
    retrieval_stats_edges, retrieval_stats_object_states, InMemoryRetrievalStatsStore,
    RetrievalStatsCounter, RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth,
    RetrievalStatsHealthState, RetrievalStatsObjectState, RetrievalStatsStore,
};

#[allow(unused_imports)]
pub(crate) use retrieval_selectivity::{
    selectivity_plan_for_candidate, RetrievalSelectivityPolicy, SelectivityPlan,
    SelectivityStatsContext,
};

// Continuity retrieval pipeline surface.
#[allow(unused_imports)]
pub(crate) use retrieve_pipeline::RetrievePipeline;

// Vector candidate recall contract surface.
#[allow(unused_imports)]
pub(crate) use vector_candidate_store::VectorCandidateStore;

#[allow(unused_imports)]
pub(crate) use write_planning::{
    WritePlanCommitValues, WritePlanValidationDecision, WritePlanValidationVerdict,
    WritePlanValidator,
};
