pub mod lifecycle;
pub mod retrieval;
pub mod write_plan;

mod draft;

pub use crate::domain;
pub use crate::domain::{
    graph_uri, DerivedMemory, DerivedType, DomainValidationError, Entity, EntityType, Episode,
    MemoryId, MemoryLink, MemoryObject, MemoryThread, Modality, ObjectType, Observation,
    RelationType, RetentionState, Stability, ThreadStatus, CURRENT_SCHEMA_VERSION,
    DEFAULT_SCHEMA_VERSION, EPISODIC_MEMORY_SCHEMA_VERSION,
};
pub use draft::{
    DerivedMemoryDraft, DraftDefaults, EntityDraft, EpisodeDraft, MemoryLinkDraft,
    MemoryObjectDraft, MemoryThreadDraft, ObservationDraft, RememberDraft, RememberOutcome,
    VectorIndexingFailure,
};
pub use lifecycle::{
    ArchivePolicy, CorrectMemoryDraft, CorrectionCascadePolicy, CorrectionLifecyclePolicy,
    CorrectionTarget, DeferredDestructiveLifecyclePolicy, DeferredLifecycleAction,
    ExternalSourceReference, ForgetCascadePolicy, ForgetLifecyclePolicy, ForgetMemoryDraft,
    LifecycleDtoValidationError, LifecycleMutationOutcome, LifecycleMutationTrace,
    LifecycleTargetRef, ReplacementDerivedMemoryDraft, SourceObjectCorrectionTarget,
    SourceProvenanceReference, SupersededByEvidence, SuppressionPolicy, VectorMaintenanceFailure,
};
pub use retrieval::{
    default_retrieval_object_types, ContextPackSection, ContinuityContextPack,
    ContinuitySectionLimits, GraphExpansionBoundedFailureSummary,
    GraphExpansionBoundedFailureTrace, GraphExpansionBoundedReason, GraphExpansionOutcome,
    GraphExpansionTelemetry, GraphExpansionTrace, GraphRelationTrace, IncludedDerivedMemory,
    LifecycleFilterAction, LifecycleFilterDecision, LifecycleFilterReason,
    LifecycleOmissionSummary, MemoryObjectRef, RetrievalCandidateLimits, RetrievalContext,
    RetrievalGraphLimits, RetrievalLifecyclePolicy, RetrievalRationale, RetrievalTelemetry,
    RetrievalTrace, RetrieveOutcome, SectionAssignment, SectionPressureSummary,
    SelectivityCountScope, SelectivityDecision, SelectivityTelemetry, SelectivityTrace,
    StaleCandidateOmission, StaleCandidateOmissionSummary, StaleCandidateReason,
    VectorCandidateTrace,
};
pub use write_plan::{
    CandidateCount, CandidateProducerKind, CandidateProvenance, CandidateRationale,
    CandidateValidation, CandidateValidationStatus, CommitOptions, DerivedMemoryCandidate,
    DiagnosticSeverity, EntityCandidate, EpisodeCandidate, MemoryCandidate, MemoryCandidateKind,
    MemoryLinkCandidate, MemoryThreadCandidate, ObservationCandidate, PrepareOptions,
    RationaleOrigin, RememberDiagnostic, RememberDiagnostics, RememberInput, RememberOptions,
    RememberWritePlan, RepairMarker, SourceProvenance, SourceSpan, SourceSpanRange,
    SourceSpanValidationError, StatsUpdateCandidate, StatsUpdateFailure, StatsUpdateStatus,
    VectorIndexCandidate,
};
