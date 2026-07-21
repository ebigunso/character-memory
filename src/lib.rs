pub(crate) mod adapters;
pub mod api;
mod composition;
mod config;
pub mod domain;
mod errors;
mod memory;
pub(crate) mod models;
pub(crate) mod policy;
pub(crate) mod ports;
#[cfg(test)]
pub(crate) mod test_support;
pub(crate) mod usecases;

// Re-export types for public use
pub use crate::api::embedding::EmbeddingProvider;
pub use crate::api::types::{
    default_retrieval_object_types, ArchivePolicy, CandidateCount, CandidateProducerKind,
    CandidateProvenance, CandidateRationale, CommitOptions, ContextPackSection,
    ContinuityContextPack, ContinuitySectionLimits, CorrectMemoryDraft, CorrectionCascadePolicy,
    CorrectionLifecyclePolicy, CorrectionTarget, DeferredDestructiveLifecyclePolicy,
    DeferredLifecycleAction, DerivedMemoryCandidate, DerivedMemoryDraft, DiagnosticSeverity,
    DraftDefaults, EntityCandidate, EntityDraft, EpisodeCandidate, EpisodeDraft,
    ExternalSourceReference, FanoutUtilizationTrace, ForgetCascadePolicy, ForgetLifecyclePolicy,
    ForgetMemoryDraft, GraphExpansionBoundedFailureSummary, GraphExpansionOutcome,
    GraphExpansionTelemetry, GraphExpansionTrace, GraphRelationTrace, IncludedDerivedMemory,
    LifecycleFilterAction, LifecycleFilterDecision, LifecycleFilterReason,
    LifecycleMutationDiagnostics, LifecycleMutationOutcome, LifecycleMutationTrace,
    LifecycleMutationWarning, LifecycleMutationWarningReason, LifecycleOmissionSummary,
    LifecycleTargetRef, MemoryCandidate, MemoryLinkCandidate, MemoryLinkDraft, MemoryObjectDraft,
    MemoryThreadCandidate, MemoryThreadDraft, ObservationCandidate, ObservationDraft,
    PrepareOptions, RationaleCategory, RationaleOrigin, RememberDiagnostic, RememberDiagnosticCode,
    RememberDiagnostics, RememberInput, RememberOptions, RememberOutcome, RememberWritePlan,
    RepairMarker, ReplacementDerivedMemoryDraft, RetrievalCandidateLimits, RetrievalContext,
    RetrievalGraphLimits, RetrievalLifecyclePolicy, RetrievalRationale, RetrievalTelemetry,
    RetrievalTrace, RetrieveOutcome, SectionAssignment, SectionAssignmentReason,
    SectionPressureSummary, SectionScoreComponents, SelectivityCountScope, SelectivityDecision,
    SelectivityTelemetry, SelectivityTrace, SourceObjectCorrectionTarget, SourceProvenance,
    SourceProvenanceReference, SourceSpan, SourceSpanRange, SourceSpanValidationError,
    StaleCandidateOmission, StaleCandidateOmissionSummary, StaleCandidateReason,
    StatsUpdateCandidate, StatsUpdateFailure, StatsUpdateStatus, SupersededByEvidence,
    SuppressionPolicy, VectorCandidateTrace, VectorIndexCandidate, VectorIndexingFailure,
    VectorMaintenanceFailure, VectorMaintenanceFailureItem, VectorMaintenanceOperation,
    VectorSurface,
};
pub use crate::config::{
    GraphStoreMode, RetrievalStatsHealthFailMode, RetrievalStatsStoreMode, Settings,
};
pub use crate::domain::{
    graph_uri, CandidateProvenanceIssue, CandidateReferenceRole, CandidateScoreField,
    CandidateSourceSpanIssue, CandidateTimestampField, CandidateValidation,
    CandidateValidationIssue, CandidateValidationStatus, DerivedMemory, DerivedType,
    DomainValidationError, Entity, EntityType, Episode, GraphExpansionBoundedFailureTrace,
    GraphExpansionBoundedReason, GraphFailureMode, LifecycleDtoValidationError,
    LifecyclePolicyKnob, MemoryCandidateKind, MemoryId, MemoryLink, MemoryLinkEndpoint,
    MemoryObject, MemoryObjectRef, MemoryThread, Modality, ObjectType, Observation,
    PlanIdentityField, RelationType, RetentionState, Stability, ThreadStatus,
    CURRENT_SCHEMA_VERSION, DEFAULT_SCHEMA_VERSION, EPISODIC_MEMORY_SCHEMA_VERSION,
};
pub use crate::errors::{
    CollectionCompatibilityError, CollectionMismatch, ConfigValidationError,
    ConfigValidationReason, CustomError, EmbeddingError, EmbeddingTransportErrorKind,
    StatsUpdateCause, TransportStatus, VectorDatabaseError, VectorDatabaseErrorKind,
    VectorIndexingCause,
};
pub use crate::memory::CharacterMemory;
pub use crate::usecases::write_planning::{PreparedCandidateRefs, RememberPlanDefaults};
