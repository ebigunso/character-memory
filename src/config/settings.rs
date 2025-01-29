use secrecy::{SecretString, ExposeSecret};
use serde::Deserialize;
use std::path::{PathBuf};
use crate::errors::custom::CustomError;

#[derive(Debug, Deserialize)]
pub(crate) struct Settings {
    qdrant_connection_string: SecretString,
    oxigraph_connection_string: SecretString,
}

impl Settings {
    pub(crate) fn load() -> Result<Self, CustomError> {
        // Get project root directory from CARGO_MANIFEST_DIR
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let env_path = project_root.join(".env");

        // Check if .env exists in project root
        if !env_path.exists() {
            return Err(CustomError::EnvFileNotFound(
                "Please create .env file with required secrets in the project root.".to_string()
            ));
        }

        // Load .env file from project root
        dotenvy::from_path(&env_path)
            .map_err(|e| CustomError::EnvLoadError(
                format!("Failed to load .env file: {}", e)
            ))?;

        // Build configuration
        let settings = config::Config::builder()
            .add_source(config::Environment::default())
            .build()
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
