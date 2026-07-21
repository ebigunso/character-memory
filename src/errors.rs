use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    CandidateValidation, GraphExpansionBoundedFailureTrace, LifecycleDtoValidationError,
    LifecyclePolicyKnob, MemoryId, ObjectType,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum VectorDatabaseErrorKind {
    Response,
    ResourceExhausted,
    Conversion,
    InvalidUri,
    NoSnapshotFound,
    Io { io_kind: String },
    HttpTimeout,
    HttpConnect,
    HttpStatus,
    Http,
    JsonToPayload,
    PayloadDeserialization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TransportStatus {
    Ok,
    Cancelled,
    Unknown,
    InvalidArgument,
    DeadlineExceeded,
    NotFound,
    AlreadyExists,
    PermissionDenied,
    ResourceExhausted,
    FailedPrecondition,
    Aborted,
    OutOfRange,
    Unimplemented,
    Internal,
    Unavailable,
    DataLoss,
    Unauthenticated,
    Unrecognized(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[error(
    "{backend} error: kind={kind:?} status={status:?} message={message} retry_after_seconds={retry_after_seconds:?}"
)]
#[non_exhaustive]
pub struct VectorDatabaseError {
    pub backend: String,
    pub kind: VectorDatabaseErrorKind,
    pub status: Option<TransportStatus>,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum EmbeddingError {
    #[error("embedding API key is missing")]
    MissingApiKey,
    #[error("embedding provider vector size mismatch: expected {expected}, got {actual}")]
    ProviderVectorSizeMismatch { expected: usize, actual: usize },
    #[error("embedding input is blank at index {index:?}")]
    BlankInput { index: Option<usize> },
    #[error("embedding transport failed ({transport_kind:?}): {detail}")]
    Transport {
        transport_kind: EmbeddingTransportErrorKind,
        detail: String,
    },
    #[error("embedding service returned HTTP {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("embedding response JSON is invalid: {detail}")]
    InvalidJson { detail: String },
    #[error("embedding response is missing data")]
    MissingData,
    #[error("embedding count mismatch: expected {expected}, got {actual}")]
    CountMismatch { expected: usize, actual: usize },
    #[error("embedding response item {item} is missing its index")]
    MissingIndex { item: usize },
    #[error("embedding index {index} is outside expected count {expected_count}")]
    IndexOutOfRange { index: usize, expected_count: usize },
    #[error("embedding response contains duplicate index {index}")]
    DuplicateIndex { index: usize },
    #[error("embedding response item {item} is missing its vector")]
    MissingEmbedding { item: usize },
    #[error("embedding dimension mismatch at index {index}: expected {expected}, got {actual}")]
    DimensionMismatch {
        index: usize,
        expected: usize,
        actual: usize,
    },
    #[error("embedding value at index {index}, component {component} is not numeric")]
    NonNumericValue { index: usize, component: usize },
    #[error("embedding response is missing index {index}")]
    MissingResponseIndex { index: usize },
    #[error("unrecognized external embedding-provider error: {0}")]
    Unrecognized(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EmbeddingTransportErrorKind {
    Timeout,
    Connect,
    Request,
    Body,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "cause", content = "detail", rename_all = "snake_case")]
#[non_exhaustive]
pub enum VectorIndexingCause {
    #[error("embedding failed: {0}")]
    Embedding(#[source] EmbeddingError),
    #[error("embedding cardinality mismatch: expected {expected}, got {actual}")]
    CardinalityMismatch { expected: usize, actual: usize },
    #[error("vector database failed: {0}")]
    VectorDatabase(#[source] VectorDatabaseError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "cause", rename_all = "snake_case")]
#[non_exhaustive]
pub enum StatsUpdateCause {
    #[error("stats endpoint hydration failed: {detail}")]
    EndpointHydration { detail: String },
    #[error("stats edge write failed: {detail}")]
    EdgeWrite { detail: String },
    #[error("stats object-state write failed: {detail}")]
    ObjectStateWrite { detail: String },
    #[error("stats health check failed: {detail}")]
    HealthCheck { detail: String },
    #[error("stats store is unhealthy: {detail:?}")]
    StoreUnhealthy { detail: Option<String> },
}

impl VectorDatabaseError {
    pub(crate) fn new(
        backend: &'static str,
        kind: VectorDatabaseErrorKind,
        status: Option<TransportStatus>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            backend: backend.to_owned(),
            kind,
            status,
            message: message.into(),
            retry_after_seconds: None,
        }
    }

    pub(crate) fn with_retry_after_seconds(mut self, retry_after_seconds: u64) -> Self {
        self.retry_after_seconds = Some(retry_after_seconds);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("collection {collection:?} is incompatible: {mismatch}")]
pub struct CollectionCompatibilityError {
    pub collection: String,
    pub mismatch: CollectionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CollectionMismatch {
    #[error("missing vector configuration")]
    MissingVectorConfiguration,
    #[error("vector size mismatch: expected {expected}, got {actual}")]
    VectorSize { expected: u64, actual: u64 },
    #[error("distance mismatch: expected {expected}, got {actual}")]
    Distance {
        expected: &'static str,
        actual: String,
    },
    #[error("named vectors are unsupported: {names:?}")]
    NamedVectors { names: Vec<String> },
    #[error("vector configuration is empty")]
    EmptyVectorConfiguration,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("configuration validation failed for {keys:?}: {reason}")]
pub struct ConfigValidationError {
    pub keys: Vec<&'static str>,
    pub reason: ConfigValidationReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ConfigValidationReason {
    #[error("required value is missing")]
    MissingValue,
    #[error("expected {expected}, got {actual:?}")]
    OutOfDomain {
        expected: &'static str,
        actual: String,
    },
    #[error("keys {first} and {second} must be provided together")]
    PairedKeyViolation {
        first: &'static str,
        second: &'static str,
    },
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CustomError {
    #[error("Environment file not found: {0}")]
    EnvFileNotFound(String),

    #[error("Failed to load environment file: {0}")]
    EnvLoadError(String),

    #[error("Configuration parse error: {0}")]
    ConfigParseError(String),

    #[error(transparent)]
    ConfigValidation(#[from] ConfigValidationError),

    #[error("Memory validation error: {0}")]
    MemoryValidation(String),

    #[error(
        "Write plan validation rejected: {}",
        write_plan_validation_errors(.validations)
    )]
    WritePlanValidationRejected {
        validations: Vec<CandidateValidation>,
    },

    #[error("Missing required field for episodic memory: {0}")]
    MissingEpisodicField(&'static str),

    #[error("Invalid semantic memory: semantic memories should not include episodic fields")]
    InvalidSemanticMemory,

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Database operation failed: {0}")]
    DatabaseError(String),

    #[error(transparent)]
    CollectionIncompatible(#[from] CollectionCompatibilityError),

    #[error("Unsupported schema version for {context}: expected {expected}, got {actual}")]
    UnsupportedSchemaVersion {
        context: &'static str,
        expected: &'static str,
        actual: String,
    },

    #[error("Graph expansion root not found: {object_type:?} {object_id}")]
    GraphExpansionRootNotFound {
        object_type: ObjectType,
        object_id: MemoryId,
    },

    #[error("graph expansion bounded by retrieval policy: {0}")]
    GraphExpansionBounded(GraphExpansionBoundedFailureTrace),

    #[error(transparent)]
    LifecycleDraftInvalid(#[from] LifecycleDtoValidationError),

    #[error("lifecycle policy knob is unsupported in this release: {knob:?}")]
    LifecyclePolicyUnsupported { knob: LifecyclePolicyKnob },

    #[error("Vector database error: {0}")]
    VectorDatabaseError(#[source] VectorDatabaseError),

    #[error(transparent)]
    Embedding(#[from] EmbeddingError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

fn write_plan_validation_errors(validations: &[CandidateValidation]) -> String {
    validations
        .iter()
        .flat_map(|validation| validation.errors.iter())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CandidateValidationIssue, CandidateValidationStatus, MemoryCandidateKind};

    #[test]
    fn write_plan_rejection_preserves_validation_issues() {
        let error = CustomError::WritePlanValidationRejected {
            validations: vec![CandidateValidation {
                candidate_index: 2,
                candidate_kind: MemoryCandidateKind::DerivedMemory,
                status: CandidateValidationStatus::Invalid,
                errors: vec![CandidateValidationIssue::MissingDerivedSource],
                warnings: Vec::new(),
            }],
        };

        let CustomError::WritePlanValidationRejected { validations } = error else {
            panic!("expected write-plan validation rejection");
        };
        assert_eq!(
            validations[0].errors,
            vec![CandidateValidationIssue::MissingDerivedSource]
        );
    }
}
