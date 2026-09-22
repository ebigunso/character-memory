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

#[cfg(feature = "test-fixtures")]
#[doc(hidden)]
pub use crate::models::vector::zero_norm_record_fixture;

// Re-export types for public use
pub use crate::api::embedding::EmbeddingProvider;
pub use crate::api::types::{
    default_retrieval_object_types, ActivityRef, ActivityResolution, ActivityResult,
    CandidateCount, CandidateProducerKind, CandidateProvenance, CandidateRationale, CommitOptions,
    ContextPackSection, ContinuityContextPack, ContinuitySectionLimits, CorrectMemoryDraft,
    CorrectionCascadePolicy, CorrectionTarget, CueKind, DerivedMemoryCandidate, DerivedMemoryDraft,
    DiagnosticSeverity, DraftDefaults, EntityCandidate, EntityDraft, EpisodeCandidate,
    EpisodeDraft, ExternalSourceReference, FanoutUtilizationTrace, ForgetCascadePolicy,
    ForgetLifecyclePolicy, ForgetMemoryDraft, GraphExpansionBoundedFailureSummary,
    GraphExpansionOutcome, GraphExpansionTelemetry, GraphExpansionTrace, GraphRelationTrace,
    GraphRootSource, IncludedDerivedMemory, LastInteraction, LifecycleFilterAction,
    LifecycleFilterDecision, LifecycleFilterReason, LifecycleMutationDiagnostics,
    LifecycleMutationOutcome, LifecycleMutationTrace, LifecycleMutationWarning,
    LifecycleMutationWarningReason, LifecycleOmissionSummary, LifecycleTargetRef, LinkOutcome,
    MemoryCandidate, MemoryLinkCandidate, MemoryLinkDraft, MemoryObjectDraft, MemoryScenes,
    MemoryThreadCandidate, MemoryThreadDraft, ObservationCandidate, ObservationDraft,
    PrepareOptions, RememberDiagnostic, RememberDiagnosticCode, RememberDiagnostics, RememberInput,
    RememberOptions, RememberOutcome, RememberWritePlan, RepairMarker,
    ReplacementDerivedMemoryDraft, RetrievalCandidateLimits, RetrievalContext,
    RetrievalGraphLimits, RetrievalLifecyclePolicy, RetrievalRationale, RetrievalTelemetry,
    RetrievalTrace, RetrieveOutcome, SceneCueSearchTrace, SceneReference, SceneReferenceResolution,
    SceneReferenceResult, SectionAssignment, SectionAssignmentReason, SectionPressureSummary,
    SectionScoreComponents, SelectivityCountScope, SelectivityDecision, SelectivityTelemetry,
    SelectivityTrace, SourceObjectCorrectionTarget, SourceProvenance, SourceProvenanceReference,
    SourceScene, SourceSceneUnavailableReason, SourceSpan, SourceSpanRange,
    SourceSpanValidationError, StaleCandidateOmission, StaleCandidateOmissionSummary,
    StaleCandidateReason, StatsUpdateCandidate, StatsUpdateFailure, StatsUpdateStatus,
    SupersededByEvidence, SuppressionPolicy, TimeRange, VectorCandidateTrace, VectorIndexCandidate,
    VectorIndexingFailure, VectorMaintenanceFailure, VectorMaintenanceFailureItem,
    VectorMaintenanceOperation, VectorRecallCompleteness,
};
pub use crate::config::{
    GraphStoreMode, RetrievalStatsHealthFailMode, RetrievalStatsStoreMode, Settings,
};
pub use crate::domain::{
    graph_uri, BeliefAssertion, BeliefPredicate, BeliefValidationError, CandidateProvenanceIssue,
    CandidateReferenceRole, CandidateScoreField, CandidateSourceSpanIssue, CandidateTimestampField,
    CandidateValidation, CandidateValidationIssue, CandidateValidationStatus, DerivedMemory,
    DerivedType, DomainValidationError, Entity, Episode, GraphExpansionBoundedFailureTrace,
    GraphExpansionBoundedReason, GraphFailureMode, LifecycleDtoValidationError,
    MemoryCandidateKind, MemoryId, MemoryLink, MemoryLinkEndpoint, MemoryObject, MemoryObjectRef,
    MemoryThread, Modality, ObjectType, Observation, RelationType, RetentionState, Scene,
    SceneParticipant, SceneSetting, SourceReferenceKind, ThreadStatus, VectorSurface,
    CURRENT_SCHEMA_VERSION, DEFAULT_SCHEMA_VERSION, EPISODIC_MEMORY_SCHEMA_VERSION,
};
pub use crate::errors::{
    CollectionCompatibilityError, CollectionMismatch, ConfigValidationError,
    ConfigValidationReason, CustomError, EmbeddingError, EmbeddingTransportErrorKind,
    GraphQueryError, IoErrorKind, ReplacementIdentityConflict, ReplacementIdentityConflictError,
    RetrievalStatsHealthCause, RetrievalStatsStoreError, StatsUpdateCause, TransportStatus,
    VectorDatabaseError, VectorDatabaseErrorKind, VectorIndexingCause,
};
pub use crate::memory::CharacterMemory;
pub use crate::policy::embedding_surface::max_embedding_surfaces;
pub use crate::usecases::write_planning::{PreparedCandidateRefs, RememberPlanDefaults};
