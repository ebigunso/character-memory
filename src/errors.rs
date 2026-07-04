use qdrant_client::QdrantError;
use thiserror::Error;

use crate::api::types::{MemoryId, ObjectType};

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("Environment file not found: {0}")]
    EnvFileNotFound(String),

    #[error("Failed to load environment file: {0}")]
    EnvLoadError(String),

    #[error("Configuration parse error: {0}")]
    ConfigParseError(String),

    #[error("Memory validation error: {0}")]
    MemoryValidation(String),

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
    QdrantError(#[source] Box<QdrantError>),

    #[error("Embedding initialization error: {0}")]
    EmbeddingInitializationError(String),

    #[error("Embedding generation error: {0}")]
    EmbeddingGenerationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl From<QdrantError> for CustomError {
    fn from(err: QdrantError) -> Self {
        CustomError::QdrantError(Box::new(err))
    }
}
