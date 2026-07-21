use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    CandidateValidation, GraphExpansionBoundedFailureTrace, LifecycleDtoValidationError,
    LifecyclePolicyKnob, MemoryId, ObjectType,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VectorDatabaseErrorKind {
    Response,
    ResourceExhausted,
    Conversion,
    InvalidUri,
    NoSnapshotFound,
    Io { io_kind: IoErrorKind },
    HttpTimeout,
    HttpConnect,
    HttpStatus,
    Http,
    JsonToPayload,
    PayloadDeserialization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum IoErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    QuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    InvalidFilename,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
    Unrecognized,
}

impl From<std::io::ErrorKind> for IoErrorKind {
    fn from(kind: std::io::ErrorKind) -> Self {
        match kind {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            std::io::ErrorKind::ConnectionRefused => Self::ConnectionRefused,
            std::io::ErrorKind::ConnectionReset => Self::ConnectionReset,
            std::io::ErrorKind::HostUnreachable => Self::HostUnreachable,
            std::io::ErrorKind::NetworkUnreachable => Self::NetworkUnreachable,
            std::io::ErrorKind::ConnectionAborted => Self::ConnectionAborted,
            std::io::ErrorKind::NotConnected => Self::NotConnected,
            std::io::ErrorKind::AddrInUse => Self::AddrInUse,
            std::io::ErrorKind::AddrNotAvailable => Self::AddrNotAvailable,
            std::io::ErrorKind::NetworkDown => Self::NetworkDown,
            std::io::ErrorKind::BrokenPipe => Self::BrokenPipe,
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            std::io::ErrorKind::WouldBlock => Self::WouldBlock,
            std::io::ErrorKind::NotADirectory => Self::NotADirectory,
            std::io::ErrorKind::IsADirectory => Self::IsADirectory,
            std::io::ErrorKind::DirectoryNotEmpty => Self::DirectoryNotEmpty,
            std::io::ErrorKind::ReadOnlyFilesystem => Self::ReadOnlyFilesystem,
            std::io::ErrorKind::StaleNetworkFileHandle => Self::StaleNetworkFileHandle,
            std::io::ErrorKind::InvalidInput => Self::InvalidInput,
            std::io::ErrorKind::InvalidData => Self::InvalidData,
            std::io::ErrorKind::TimedOut => Self::TimedOut,
            std::io::ErrorKind::WriteZero => Self::WriteZero,
            std::io::ErrorKind::StorageFull => Self::StorageFull,
            std::io::ErrorKind::NotSeekable => Self::NotSeekable,
            std::io::ErrorKind::QuotaExceeded => Self::QuotaExceeded,
            std::io::ErrorKind::FileTooLarge => Self::FileTooLarge,
            std::io::ErrorKind::ResourceBusy => Self::ResourceBusy,
            std::io::ErrorKind::ExecutableFileBusy => Self::ExecutableFileBusy,
            std::io::ErrorKind::Deadlock => Self::Deadlock,
            std::io::ErrorKind::CrossesDevices => Self::CrossesDevices,
            std::io::ErrorKind::TooManyLinks => Self::TooManyLinks,
            std::io::ErrorKind::InvalidFilename => Self::InvalidFilename,
            std::io::ErrorKind::ArgumentListTooLong => Self::ArgumentListTooLong,
            std::io::ErrorKind::Interrupted => Self::Interrupted,
            std::io::ErrorKind::Unsupported => Self::Unsupported,
            std::io::ErrorKind::UnexpectedEof => Self::UnexpectedEof,
            std::io::ErrorKind::OutOfMemory => Self::OutOfMemory,
            std::io::ErrorKind::Other => Self::Other,
            _ => Self::Unrecognized,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
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
    #[error("unrecognized external embedding-provider error: {detail}")]
    Unrecognized { detail: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingTransportErrorKind {
    Timeout,
    Connect,
    Request,
    Body,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "cause", content = "detail", rename_all = "snake_case")]
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

    macro_rules! exhaustive_embedding_error_fixtures {
        ($( $pattern:pat => $fixture:expr ),+ $(,)?) => {{
            fn assert_exhaustive(error: &EmbeddingError) {
                match error {
                    $( $pattern => {} ),+
                }
            }

            let fixtures = vec![$($fixture),+];
            for fixture in &fixtures {
                assert_exhaustive(fixture);
            }
            fixtures
        }};
    }

    #[test]
    fn every_embedding_error_variant_round_trips_through_serde() {
        let errors = exhaustive_embedding_error_fixtures![
            EmbeddingError::MissingApiKey => EmbeddingError::MissingApiKey,
            EmbeddingError::ProviderVectorSizeMismatch { .. } => EmbeddingError::ProviderVectorSizeMismatch {
                expected: 3,
                actual: 2,
            },
            EmbeddingError::BlankInput { .. } => EmbeddingError::BlankInput { index: Some(1) },
            EmbeddingError::Transport { .. } => EmbeddingError::Transport {
                transport_kind: EmbeddingTransportErrorKind::Connect,
                detail: "connection refused".to_owned(),
            },
            EmbeddingError::HttpStatus { .. } => EmbeddingError::HttpStatus {
                status: 429,
                body: "rate limited".to_owned(),
            },
            EmbeddingError::InvalidJson { .. } => EmbeddingError::InvalidJson {
                detail: "unexpected token".to_owned(),
            },
            EmbeddingError::MissingData => EmbeddingError::MissingData,
            EmbeddingError::CountMismatch { .. } => EmbeddingError::CountMismatch {
                expected: 2,
                actual: 1,
            },
            EmbeddingError::MissingIndex { .. } => EmbeddingError::MissingIndex { item: 0 },
            EmbeddingError::IndexOutOfRange { .. } => EmbeddingError::IndexOutOfRange {
                index: 2,
                expected_count: 2,
            },
            EmbeddingError::DuplicateIndex { .. } => EmbeddingError::DuplicateIndex { index: 0 },
            EmbeddingError::MissingEmbedding { .. } => EmbeddingError::MissingEmbedding { item: 0 },
            EmbeddingError::DimensionMismatch { .. } => EmbeddingError::DimensionMismatch {
                index: 0,
                expected: 3,
                actual: 2,
            },
            EmbeddingError::NonNumericValue { .. } => EmbeddingError::NonNumericValue {
                index: 0,
                component: 1,
            },
            EmbeddingError::MissingResponseIndex { .. } => EmbeddingError::MissingResponseIndex { index: 1 },
            EmbeddingError::Unrecognized { .. } => EmbeddingError::Unrecognized {
                detail: "custom provider failure".to_owned(),
            },
        ];

        for error in errors {
            let serialized = serde_json::to_value(&error).unwrap();
            let deserialized = serde_json::from_value(serialized.clone()).unwrap();
            assert_eq!(error, deserialized);

            if matches!(error, EmbeddingError::Unrecognized { .. }) {
                assert_eq!(serialized["kind"], "unrecognized");
                assert_eq!(serialized["detail"], "custom provider failure");
            }
        }
    }

    #[test]
    fn io_error_kind_preserves_transport_classification_and_round_trips() {
        let kind = IoErrorKind::from(std::io::ErrorKind::ConnectionRefused);

        assert_eq!(kind, IoErrorKind::ConnectionRefused);
        let serialized = serde_json::to_string(&kind).unwrap();
        assert_eq!(
            serde_json::from_str::<IoErrorKind>(&serialized).unwrap(),
            kind
        );
    }

    #[test]
    fn unrecognized_io_error_kind_is_an_opaque_marker() {
        let serialized = serde_json::to_value(IoErrorKind::Unrecognized).unwrap();

        assert_eq!(serialized, serde_json::json!({ "kind": "unrecognized" }));
        assert!(
            serialized.get("value").is_none(),
            "the fallback must not expose a Debug-derived carrier"
        );
    }

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
