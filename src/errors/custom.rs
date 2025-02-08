use thiserror::Error;

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
    QdrantError(#[from] qdrant_client::Error),
}

impl CustomError {
    pub(crate) fn missing_episodic_field(field: &'static str) -> Self {
        CustomError::MissingEpisodicField(field)
    }

    pub(crate) fn database_error(msg: impl Into<String>) -> Self {
        CustomError::DatabaseError(msg.into())
    }
}
