use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{MemoryId, MemoryObjectRef, ObjectType};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCandidateKind {
    Episode,
    Observation,
    Entity,
    MemoryThread,
    DerivedMemory,
    MemoryLink,
    VectorIndex,
    StatsUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateValidation {
    pub candidate_index: usize,
    pub candidate_kind: MemoryCandidateKind,
    pub status: CandidateValidationStatus,
    pub errors: Vec<CandidateValidationIssue>,
    pub warnings: Vec<CandidateValidationIssue>,
}

impl CandidateValidation {
    pub fn valid(candidate_index: usize, candidate_kind: MemoryCandidateKind) -> Self {
        Self {
            candidate_index,
            candidate_kind,
            status: CandidateValidationStatus::Valid,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(
        candidate_index: usize,
        candidate_kind: MemoryCandidateKind,
        error: CandidateValidationIssue,
    ) -> Self {
        Self {
            candidate_index,
            candidate_kind,
            status: CandidateValidationStatus::Invalid,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: CandidateValidationIssue) -> Self {
        self.warnings.push(warning);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CandidateValidationIssue {
    #[error("write plan is missing {field:?}")]
    MissingPlanIdentity { field: PlanIdentityField },
    #[error("candidate id must be present")]
    MissingCandidateId,
    #[error("candidate schema_version must be present")]
    MissingCandidateSchemaVersion,
    #[error("candidate timestamp {field:?} must be present for deterministic commit")]
    MissingTimestamp { field: CandidateTimestampField },
    #[error("candidate object_type must be {expected:?}, got {actual:?}")]
    ObjectTypeMismatch {
        expected: ObjectType,
        actual: ObjectType,
    },
    #[error("episode summary must not be empty")]
    EmptyEpisodeSummary,
    #[error("observation episode_id must reference an episode")]
    MissingEpisodeReference,
    #[error("derived memory must reference at least one source episode or observation")]
    MissingDerivedSource,
    #[error("candidate score {field:?} must be finite and in 0.0..=1.0, got {actual}")]
    InvalidScore {
        field: CandidateScoreField,
        actual: String,
    },
    #[error("memory link endpoint {endpoint:?} cannot reference a memory link")]
    UnsupportedMemoryLinkEndpoint { endpoint: MemoryLinkEndpoint },
    #[error("memory link cannot point from an object to itself: {referenced:?}")]
    SelfLink { referenced: MemoryObjectRef },
    #[error("materialized memory object schema_version must be present")]
    MissingObjectSchemaVersion,
    #[error("memory link was rejected by the link admission policy")]
    MemoryLinkRejectedByAdmissionPolicy,
    #[error("suppressed memory cannot be current")]
    SuppressedMemoryMarkedCurrent,
    #[error("superseding memory cannot be current unless explicitly historical")]
    SupersedingMemoryMarkedCurrent,
    #[error("candidate provenance is invalid: {reason:?}")]
    InvalidProvenance { reason: CandidateProvenanceIssue },
    #[error("candidate source span is invalid: {reason:?}")]
    InvalidSourceSpan { reason: CandidateSourceSpanIssue },
    #[error("vector index candidate embedding_text must not be empty")]
    EmptyVectorEmbeddingText,
    #[error("stats update candidate relation and object must be supplied together")]
    IncompleteStatsRelationObjectPair,
    #[error("candidate reference {role:?} does not exist: {referenced:?}")]
    UnknownObjectRef {
        role: CandidateReferenceRole,
        referenced: MemoryObjectRef,
    },
    #[error(
        "candidate reference {role:?} must target an object in the write plan: {referenced:?}"
    )]
    ReferenceNotInPlan {
        role: CandidateReferenceRole,
        referenced: MemoryObjectRef,
    },
    #[error(
        "candidate content echoes episode content {echo_surface:?}; matching episodes: {matching_episode_ids:?}"
    )]
    DuplicateObservationEcho {
        echo_surface: String,
        matching_episode_ids: Vec<MemoryId>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanIdentityField {
    OperationId,
    IdempotencyKey,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateTimestampField {
    CreatedAt,
    UpdatedAt,
    LastTouchedAt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateScoreField {
    EpisodeSalience,
    ObservationSalience,
    MemoryThreadSalience,
    DerivedMemoryConfidence,
    DerivedMemorySalience,
    MemoryLinkConfidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLinkEndpoint {
    From,
    To,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateProvenanceIssue {
    NonCallerClaimedCallerRationale,
    EmptyRationaleText,
    EmptyExternalReference,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSourceSpanIssue {
    EmptySourceRef,
    EmptyRawRef,
    EmptyMessageId,
    EmptyTranscriptSegmentId,
    InvalidTurnRange,
    InvalidCharRange,
    InvalidByteRange,
    InvalidTimestampRange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateReferenceRole {
    DerivedSourceEpisode,
    DerivedSourceObservation,
    MemoryLinkFrom,
    MemoryLinkTo,
    VectorIndexTarget,
    StatsUpdateSubject,
    StatsUpdateObject,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateValidationStatus {
    Valid,
    Invalid,
}
