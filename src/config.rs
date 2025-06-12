// Module declarations
pub mod settings {
    mod embedding_repository_settings;
    mod settings;
    mod vector_memory_repository_settings;

    pub(crate) use embedding_repository_settings::EmbeddingRepositorySettings;
    pub use settings::Settings;
    pub(crate) use vector_memory_repository_settings::VectorMemoryRepositorySettings;
}

