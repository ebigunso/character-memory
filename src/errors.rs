use thiserror::Error;

use crate::domain::{CandidateValidation, MemoryId, ObjectType};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VectorDatabaseError {
    pub backend: &'static str,
    pub kind: String,
    pub status: Option<String>,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}

impl VectorDatabaseError {
    pub(crate) fn new(
        backend: &'static str,
        kind: impl Into<String>,
        status: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            backend,
            kind: kind.into(),
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

impl std::fmt::Display for VectorDatabaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.status, self.retry_after_seconds) {
            (Some(status), Some(retry_after_seconds)) => write!(
                formatter,
                "{} error: kind={} status={} message={} retry_after_seconds={}",
                self.backend, self.kind, status, self.message, retry_after_seconds
            ),
            (Some(status), None) => write!(
                formatter,
                "{} error: kind={} status={} message={}",
                self.backend, self.kind, status, self.message
            ),
            (None, Some(retry_after_seconds)) => write!(
                formatter,
                "{} error: kind={} message={} retry_after_seconds={}",
                self.backend, self.kind, self.message, retry_after_seconds
            ),
            (None, None) => write!(
                formatter,
                "{} error: kind={} message={}",
                self.backend, self.kind, self.message
            ),
        }
    }
}

impl std::error::Error for VectorDatabaseError {}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CustomError {
    #[error("Environment file not found: {0}")]
    EnvFileNotFound(String),

    #[error("Failed to load environment file: {0}")]
    EnvLoadError(String),

    #[error("Configuration parse error: {0}")]
    ConfigParseError(String),

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

    #[error("Graph expansion bounded by retrieval policy: reason={reason}{location}")]
    GraphExpansionBounded { reason: String, location: String },

    #[error("Vector database error: {0}")]
    VectorDatabaseError(#[source] VectorDatabaseError),

    #[error("Embedding initialization error: {0}")]
    EmbeddingInitializationError(String),

    #[error("Embedding generation error: {0}")]
    EmbeddingGenerationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

fn write_plan_validation_errors(validations: &[CandidateValidation]) -> String {
    validations
        .iter()
        .flat_map(|validation| validation.errors.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CandidateValidationStatus, MemoryCandidateKind};

    #[test]
    fn write_plan_rejection_display_projects_validation_errors() {
        let error = CustomError::WritePlanValidationRejected {
            validations: vec![CandidateValidation {
                candidate_index: 2,
                candidate_kind: MemoryCandidateKind::DerivedMemory,
                status: CandidateValidationStatus::Invalid,
                errors: vec!["derived source is missing".to_owned()],
                warnings: Vec::new(),
            }],
        };

        let message = error.to_string();
        assert!(message.contains("derived source is missing"));
    }
}
