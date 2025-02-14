use crate::errors::custom::CustomError;
use config::Config;
use mockall::automock;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::path::PathBuf;

#[automock]
pub(crate) trait ConfigLoader {
    fn build_config(&self) -> Result<Config, config::ConfigError>;
}

pub(crate) struct DefaultConfigLoader;

impl ConfigLoader for DefaultConfigLoader {
    fn build_config(&self) -> Result<Config, config::ConfigError> {
        Config::builder()
            .add_source(config::Environment::default())
            .build()
    }
}

#[automock]
pub(crate) trait EnvLoader {
    fn load_from_path(&self, path: PathBuf) -> Result<(), std::io::Error>;
    fn exists(&self, path: PathBuf) -> bool;
}

#[derive(Default)]
pub(crate) struct DefaultEnvLoader;

impl EnvLoader for DefaultEnvLoader {
    fn load_from_path(&self, path: PathBuf) -> Result<(), std::io::Error> {
        dotenvy::from_path(&path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    fn exists(&self, path: PathBuf) -> bool {
        path.exists()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields will be used when implementing database connections
pub struct Settings {
    qdrant_connection_string: SecretString,
    oxigraph_connection_string: SecretString,
    openai_api_key: SecretString,
}

impl Settings {
    pub(crate) fn load() -> Result<Self, CustomError> {
        Self::load_with_loaders(DefaultEnvLoader, DefaultConfigLoader)
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

    /// Creates a Settings instance from an externally provided `config::Config`.
    /// This method is intended for consumers who wish to build their configuration using custom sources (files, command-line, etc.) and inject it.
    pub fn from_config(config: Config) -> Result<Self, CustomError> {
        config.try_deserialize()
            .map_err(|e| {
                CustomError::ConfigParseError(format!("Failed to parse external configuration: {}", e))
            })
    }

    #[allow(dead_code)] // Will be used when implementing Qdrant database connection
    pub(crate) fn get_qdrant_connection(&self) -> &str {
        self.qdrant_connection_string.expose_secret()
    }

    #[allow(dead_code)] // Will be used when implementing Oxigraph database connection
    pub(crate) fn get_oxigraph_connection(&self) -> &str {
        self.oxigraph_connection_string.expose_secret()
    }

    #[allow(dead_code)] // Will be used when implementing OpenAI API connection
    pub(crate) fn get_openai_api_key(&self) -> &str {
        self.openai_api_key.expose_secret()
    }
}

#[cfg(test)]
impl Settings {
    pub fn new_for_tests(
        qdrant_connection_string: SecretString,
        oxigraph_connection_string: SecretString,
        openai_api_key: SecretString,
    ) -> Self {
        Settings {
            qdrant_connection_string,
            oxigraph_connection_string,
            openai_api_key,
        }
    }
}
