#[cfg(test)]
pub(crate) mod test_support;

// Compatibility re-exports: ports, policies, and use cases have moved to
// responsibility-named modules. Existing call sites keep importing through
// this barrel until later waves remove `internal` entirely.
#[allow(unused_imports)]
pub(crate) use crate::ports::embedder::MemoryEmbedder;

// Lifecycle pipeline surface.
#[allow(unused_imports)]
pub(crate) use crate::usecases::CorrectionForgetPipeline;

// Graph-authority contract surface. Retrieval and lifecycle code use
// different subsets of the query/expansion helpers.
#[allow(unused_imports)]
pub(crate) use crate::policy::graph_expansion::{
    apply_fanout_limits_by_pair, bounded_expansion, bounded_expansion_node_set,
    bounded_hub_retention_limit, derived_memories_by_provenance, derived_memories_by_thread,
};
#[allow(unused_imports)]
pub(crate) use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
    GraphExpansion, GraphExpansionBoundedFailure, GraphExpansionBoundedFailureReason,
    GraphExpansionFailurePolicy, GraphExpansionFanoutOverride, GraphExpansionFilteredNode,
    GraphExpansionFilteredReason, GraphExpansionLifecyclePolicy, GraphExpansionQuery,
    GraphExpansionRelation, GraphObjectQuery, GraphObjectRef,
};

#[allow(unused_imports)]
pub(crate) use crate::usecases::{
    admit_link, LinkAdmissionDecision, LinkAdmissionEvidence, LinkPipeline,
};

// Source-reference contracts remain internal; core stores opaque references,
// not caller-owned source material.
#[allow(unused_imports)]
pub(crate) use crate::ports::source_reference::{ResolvedSourceReference, SourceReferenceResolver};

// Internal/admin reconciliation diagnostics. These remain out of the public
// CharacterMemory facade until a governance surface is planned.
#[allow(unused_imports)]
pub(crate) use crate::usecases::{
    reconcile_graph_vector_stores, ReconciliationDiagnostic, ReconciliationDriftKind,
    ReconciliationReport,
};

// Remember pipeline surface; outcome types are converted at the public
// facade boundary.
#[allow(unused_imports)]
pub(crate) use crate::usecases::{
    RememberPipeline, RememberPipelineDraft, RememberPipelineOutcome,
    VectorIndexingFailure as InternalVectorIndexingFailure,
};

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use crate::adapters::stats::noop_retrieval_stats_store;
#[allow(unused_imports)]
pub(crate) use crate::adapters::stats::InMemoryRetrievalStatsStore;
#[allow(unused_imports)]
pub(crate) use crate::ports::retrieval_stats::{
    object_type_key, record_stats_after_write, relation_type_key, retention_state_key,
    retrieval_stats_edges, retrieval_stats_object_states, RetrievalStatsCounter,
    RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth, RetrievalStatsHealthState,
    RetrievalStatsObjectState, RetrievalStatsStore,
};

#[allow(unused_imports)]
pub(crate) use crate::policy::{
    selectivity_plan_for_candidate, RetrievalSelectivityPolicy, SelectivityPlan,
    SelectivityStatsContext,
};

// Continuity retrieval pipeline surface.
#[allow(unused_imports)]
pub(crate) use crate::usecases::RetrievePipeline;

// Vector candidate recall contract surface.
#[allow(unused_imports)]
pub(crate) use crate::ports::vector_candidate::VectorCandidateStore;

#[allow(unused_imports)]
pub(crate) use crate::usecases::{
    WritePlanCommitValues, WritePlanValidationDecision, WritePlanValidationVerdict,
    WritePlanValidator,
};
