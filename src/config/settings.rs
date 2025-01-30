use crate::errors::custom::CustomError;
use config::{Config};
use mockall::automock;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::path::{PathBuf};

#[automock]
pub trait ConfigLoader {
    fn build_config(&self) -> Result<Config, config::ConfigError>;
}

pub struct DefaultConfigLoader;

impl ConfigLoader for DefaultConfigLoader {
    fn build_config(&self) -> Result<Config, config::ConfigError> {
        Config::builder()
            .add_source(config::Environment::default())
            .build()
    }
}

#[automock]
pub trait EnvLoader {
    fn load_from_path(&self, path: PathBuf) -> Result<(), std::io::Error>;
    fn exists(&self, path: PathBuf) -> bool;
}

#[derive(Default)]
pub struct DefaultEnvLoader;

impl EnvLoader for DefaultEnvLoader {
    fn load_from_path(&self, path: PathBuf) -> Result<(), std::io::Error> {
        dotenvy::from_path(&path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    fn exists(&self, path: PathBuf) -> bool {
        path.exists()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Settings {
    qdrant_connection_string: SecretString,
    oxigraph_connection_string: SecretString,
}

impl Settings {
    pub(crate) fn load() -> Result<Self, CustomError> {
        Self::load_with_loaders(DefaultEnvLoader, DefaultConfigLoader)
    }

    pub(crate) fn load_with_env_loader<E: EnvLoader>(env_loader: E) -> Result<Self, CustomError> {
        Self::load_with_loaders(env_loader, DefaultConfigLoader)
    }

    pub(crate) fn load_with_loaders<E: EnvLoader, C: ConfigLoader>(
        env_loader: E,
        config_loader: C
    ) -> Result<Self, CustomError> {
        // Get project root directory from CARGO_MANIFEST_DIR
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let env_path = project_root.join(".env");

        // Check if .env exists in project root
        if !env_loader.exists(env_path.clone()) {
            return Err(CustomError::EnvFileNotFound(
                "Please create .env file with required secrets in the project root.".to_string()
            ));
        }

        // Load .env file from project root
        env_loader.load_from_path(env_path)
            .map_err(|e| CustomError::EnvLoadError(
                format!("Failed to load .env file: {}", e)
            ))?;

        // Build configuration
        let settings = config_loader.build_config()
            .map_err(|e| CustomError::ConfigParseError(
                format!("Failed to build configuration: {}", e)
            ))?;

        // Try to convert into Settings struct
        settings.try_deserialize()
            .map_err(|e| CustomError::ConfigParseError(
                format!("Failed to parse configuration: {}", e)
            ))
    }

    pub(crate) fn get_qdrant_connection(&self) -> &str {
        self.qdrant_connection_string.expose_secret()
    }

    pub(crate) fn get_oxigraph_connection(&self) -> &str {
        self.oxigraph_connection_string.expose_secret()
    }
}
