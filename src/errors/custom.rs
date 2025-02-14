use thiserror::Error;
use qdrant_client::QdrantError;

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

    #[error("Database operation failed: {0}")]
    DatabaseError(String),

    #[error("Vector database error: {0}")]
    QdrantError(#[from] QdrantError),

    #[error("Embedding initialization error: {0}")]
    EmbeddingInitializationError(String),

    #[error("Embedding generation error: {0}")]
    EmbeddingGenerationError(String),
}
