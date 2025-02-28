// Module declarations
pub mod settings {
    mod settings;
    mod vector_memory_repository_settings;
    mod embedding_repository_settings;

    pub use settings::Settings;
    pub(crate) use vector_memory_repository_settings::VectorMemoryRepositorySettings;
    pub(crate) use embedding_repository_settings::EmbeddingRepositorySettings;
}

pub(crate) mod loaders {
    // Make modules public only for testing
    #[cfg(test)]
    pub(crate) mod config_loader;
    #[cfg(test)]
    pub(crate) mod env_loader;

    #[cfg(not(test))]
    mod config_loader;
    #[cfg(not(test))]
    mod env_loader;

    pub(crate) use config_loader::ConfigLoader;
    pub(crate) use config_loader::DefaultConfigLoader;
    pub(crate) use env_loader::EnvLoader;
    pub(crate) use env_loader::DefaultEnvLoader;
}
