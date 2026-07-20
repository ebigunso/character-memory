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
    ForgetMemoryDraft, GraphExpansionBoundedFailureSummary, GraphExpansionBoundedFailureTrace,
    GraphExpansionBoundedReason, GraphExpansionOutcome, GraphExpansionTelemetry,
    GraphExpansionTrace, GraphRelationTrace, IncludedDerivedMemory, LifecycleDtoValidationError,
    LifecycleFilterAction, LifecycleFilterDecision, LifecycleFilterReason,
    LifecycleMutationDiagnostics, LifecycleMutationOutcome, LifecycleMutationTrace,
    LifecycleMutationWarning, LifecycleMutationWarningReason, LifecycleOmissionSummary,
    LifecycleTargetRef, MemoryCandidate, MemoryLinkCandidate, MemoryLinkDraft, MemoryObjectDraft,
    MemoryObjectRef, MemoryThreadCandidate, MemoryThreadDraft, ObservationCandidate,
    ObservationDraft, PrepareOptions, RationaleCategory, RationaleOrigin, RememberDiagnostic,
    RememberDiagnostics, RememberInput, RememberOptions, RememberOutcome, RememberWritePlan,
    RepairMarker, ReplacementDerivedMemoryDraft, RetrievalCandidateLimits, RetrievalContext,
    RetrievalGraphLimits, RetrievalLifecyclePolicy, RetrievalRationale, RetrievalTelemetry,
    RetrievalTrace, RetrieveOutcome, SectionAssignment, SectionPressureSummary,
    SelectivityCountScope, SelectivityDecision, SelectivityTelemetry, SelectivityTrace,
    SourceObjectCorrectionTarget, SourceProvenance, SourceProvenanceReference, SourceSpan,
    SourceSpanRange, SourceSpanValidationError, StaleCandidateOmission,
    StaleCandidateOmissionSummary, StaleCandidateReason, StatsUpdateCandidate, StatsUpdateFailure,
    StatsUpdateStatus, SupersededByEvidence, SuppressionPolicy, VectorCandidateTrace,
    VectorIndexCandidate, VectorIndexingFailure, VectorMaintenanceFailure,
};
pub use crate::config::{
    GraphStoreMode, RetrievalStatsHealthFailMode, RetrievalStatsStoreMode, Settings,
};
pub use crate::domain::{
    graph_uri, CandidateValidation, CandidateValidationStatus, DerivedMemory, DerivedType,
    DomainValidationError, Entity, EntityType, Episode, MemoryCandidateKind, MemoryId, MemoryLink,
    MemoryObject, MemoryThread, Modality, ObjectType, Observation, RelationType, RetentionState,
    Stability, ThreadStatus, CURRENT_SCHEMA_VERSION, DEFAULT_SCHEMA_VERSION,
    EPISODIC_MEMORY_SCHEMA_VERSION,
};
pub use crate::errors::{CustomError, VectorDatabaseError};
pub use crate::memory::CharacterMemory;
pub use crate::usecases::write_planning::{PreparedCandidateRefs, RememberPlanDefaults};

// Re-export for integration tests
pub mod test_utils {
    use crate::config::Settings;
    use crate::errors::CustomError;

    /// Loads settings from environment variables for integration tests.
    ///
    /// # Important
    ///
    /// This function is intended ONLY for use in integration tests and should not be used in production code.
    /// A `.env` file in the project root directory will be loaded if present,
    /// otherwise existing environment variables are used.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `Settings` instance with configuration loaded from environment
    /// - `Err`: A `CustomError` if loading fails
    pub fn load_test_settings() -> Result<Settings, CustomError> {
        Settings::load()
    }
}
