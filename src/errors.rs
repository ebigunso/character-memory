use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    CandidateValidation, DomainValidationError, LifecycleDtoValidationError, MemoryId,
    MemoryObjectRef, ObjectType, SourceReferenceKind,
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
    #[error("graph read for stats projection failed: {error}")]
    GraphRead { error: GraphQueryError },
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
    #[error("retrieval stats filesystem operation failed ({io_kind}): {detail}")]
    Filesystem { io_kind: String, detail: String },
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
    #[error("graph read for stats projection failed: {error}")]
    GraphRead { error: GraphQueryError },
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
            Self::GraphRead { error } => Some(RetrievalStatsHealthCause::GraphRead {
                error: error.clone(),
            }),
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
    #[error("Configuration parse error: {0}")]
    ConfigParseError(String),

    #[error(transparent)]
    ConfigValidation(#[from] ConfigValidationError),

    #[error(transparent)]
    DomainValidation(#[from] DomainValidationError),

    #[error(
        "source-object correction requires an original raw reference or setting key: {target:?}"
    )]
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
}
