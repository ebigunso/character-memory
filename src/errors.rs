use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    CandidateValidation, DomainValidationError, GraphExpansionBoundedFailureTrace,
    LifecycleDtoValidationError, MemoryId, MemoryObjectRef, ObjectType, SourceReferenceKind,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VectorDatabaseErrorKind {
    Engine,
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
    #[error("embedding provider vector size must be positive, got {actual}")]
    InvalidVectorSize { actual: usize },
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
    #[error("graph currency lookup failed: {0}")]
    GraphQuery(#[source] GraphQueryError),
    #[error("embedding failed: {0}")]
    Embedding(#[source] EmbeddingError),
    #[error("embedding cardinality mismatch: expected {expected}, got {actual}")]
    CardinalityMismatch { expected: usize, actual: usize },
    #[error("zero-norm embedding for {object:?}")]
    ZeroNormEmbedding { object: MemoryObjectRef },
    #[error("vector database failed: {0}")]
    VectorDatabase(#[source] VectorDatabaseError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "cause", rename_all = "snake_case")]
pub enum StatsUpdateCause {
    #[error("stats endpoint hydration failed: {error}")]
    EndpointHydration { error: GraphQueryError },
    #[error("stats edge write failed: {error}")]
    EdgeWrite { error: RetrievalStatsStoreError },
    #[error("stats object-state write failed: {error}")]
    ObjectStateWrite { error: RetrievalStatsStoreError },
    #[error("stats health check failed: {error}")]
    HealthCheck { error: RetrievalStatsStoreError },
    #[error("stats unhealthy-state write failed: {error}")]
    HealthMark { error: RetrievalStatsStoreError },
    #[error("stats store is unhealthy: {health_cause:?}")]
    StoreUnhealthy {
        health_cause: Option<RetrievalStatsHealthCause>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphQueryError {
    #[error("graph object selection failed: {detail}")]
    Selection { detail: String },
    #[error("graph object hydration failed: {detail}")]
    Hydration { detail: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RetrievalStatsStoreError {
    #[error("retrieval stats counter was negative: {value}")]
    NegativeCounter { value: i64 },
    #[error("retrieval stats sqlite operation failed: {detail}")]
    Sqlite { detail: String },
    #[error("retrieval stats filesystem operation failed ({io_kind:?}): {detail}")]
    Filesystem {
        io_kind: IoErrorKind,
        detail: String,
    },
    #[error("retrieval stats store lock is poisoned")]
    LockPoisoned,
    #[error("retrieval stats health serialization failed: {detail}")]
    HealthSerialization { detail: String },
    #[error("retrieval stats health deserialization failed: {detail}")]
    HealthDeserialization { detail: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum RetrievalStatsHealthCause {
    #[error("stats store initialization failed: {error}")]
    StoreInitialization { error: RetrievalStatsStoreError },
    #[error("stats endpoint hydration failed: {error}")]
    EndpointHydration { error: GraphQueryError },
    #[error("stats edge write failed: {error}")]
    EdgeWrite { error: RetrievalStatsStoreError },
    #[error("stats object-state write failed: {error}")]
    ObjectStateWrite { error: RetrievalStatsStoreError },
    #[error("stats health check failed: {error}")]
    HealthCheck { error: RetrievalStatsStoreError },
    #[error("stats counter read failed: {error}")]
    CounterRead { error: RetrievalStatsStoreError },
    #[error("stats global-counter read failed: {error}")]
    GlobalCounterRead { error: RetrievalStatsStoreError },
}

impl StatsUpdateCause {
    pub(crate) fn health_cause(&self) -> Option<RetrievalStatsHealthCause> {
        match self {
            Self::EndpointHydration { error } => {
                Some(RetrievalStatsHealthCause::EndpointHydration {
                    error: error.clone(),
                })
            }
            Self::EdgeWrite { error } => Some(RetrievalStatsHealthCause::EdgeWrite {
                error: error.clone(),
            }),
            Self::ObjectStateWrite { error } => Some(RetrievalStatsHealthCause::ObjectStateWrite {
                error: error.clone(),
            }),
            Self::HealthCheck { error } => Some(RetrievalStatsHealthCause::HealthCheck {
                error: error.clone(),
            }),
            Self::HealthMark { .. } => None,
            Self::StoreUnhealthy { .. } => None,
        }
    }
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
    #[error("collection name mismatch: expected {expected}, got {actual}")]
    CollectionName { expected: String, actual: String },
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
    #[error("required value is missing for {mode_key}={mode}")]
    MissingForMode {
        mode_key: &'static str,
        mode: &'static str,
    },
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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("replacement {replacement_id} identity conflict: {conflict}")]
pub struct ReplacementIdentityConflictError {
    pub replacement_id: MemoryId,
    pub conflict: ReplacementIdentityConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ReplacementIdentityConflict {
    #[error("duplicate replacement ID in correction plan")]
    DuplicateInPlan,
    #[error("existing replacement has divergent content")]
    DivergentExisting,
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

    #[error(transparent)]
    DomainValidation(#[from] DomainValidationError),

    /// Rejected by the production low-information co-occurrence guard.
    ///
    /// Its only rejecting evidence class is currently constructible under `cfg(test)`;
    /// the public link path supplies `ExplicitCallerIntent`. This variant is declared
    /// ahead of a production producer for that rejecting evidence.
    #[error("low-information co-occurrence link rejected: {link_id}")]
    LowInformationCoOccurrence { link_id: MemoryId },

    #[error("source-object correction requires an original raw or source reference: {target:?}")]
    MissingOriginalSourceReference { target: MemoryObjectRef },

    #[error("original {kind:?} reference does not match source object {target:?}")]
    OriginalSourceReferenceMismatch {
        target: MemoryObjectRef,
        kind: SourceReferenceKind,
        provided: String,
        stored: Option<String>,
    },

    #[error("write plan deterministic ID collided with existing divergent content: {object:?}")]
    DeterministicIdCollision { object: MemoryObjectRef },

    #[error(transparent)]
    ReplacementIdentityConflict(#[from] ReplacementIdentityConflictError),

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
    GraphQuery(#[from] GraphQueryError),

    #[error(transparent)]
    RetrievalStatsStore(#[from] RetrievalStatsStoreError),

    #[error(transparent)]
    CollectionIncompatible(#[from] CollectionCompatibilityError),

    #[error("Unsupported schema version for {context}: expected {expected}, got {actual}")]
    UnsupportedSchemaVersion {
        context: &'static str,
        expected: &'static str,
        actual: String,
    },

    #[error("Unsupported graph expansion root: {object:?}")]
    UnsupportedExpansionRoot { object: MemoryObjectRef },

    #[error("Graph expansion root not found: {object_type:?} {object_id}")]
    GraphExpansionRootNotFound {
        object_type: ObjectType,
        object_id: MemoryId,
    },

    #[error("graph expansion bounded by retrieval policy: {0}")]
    GraphExpansionBounded(GraphExpansionBoundedFailureTrace),

    #[error(transparent)]
    LifecycleDraftInvalid(#[from] LifecycleDtoValidationError),

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

    #[test]
    fn unrecognized_embedding_error_preserves_persisted_kind_and_detail() {
        // CharacterMemoryEvals/crates/cmem-eval/src/results.rs write_jsonl/read_jsonl
        // persists this via VectorIndexingCause::Embedding in RememberOutcome.vector_indexing_failure.
        let detail = "opaque provider detail";
        let serialized = serde_json::to_value(EmbeddingError::Unrecognized {
            detail: detail.to_owned(),
        })
        .unwrap();

        assert_eq!(serialized["kind"], "unrecognized");
        assert_eq!(serialized["detail"], detail);
    }

    #[test]
    fn io_error_kind_preserves_transport_classification() {
        let kind = IoErrorKind::from(std::io::ErrorKind::ConnectionRefused);

        assert_eq!(kind, IoErrorKind::ConnectionRefused);
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
}
