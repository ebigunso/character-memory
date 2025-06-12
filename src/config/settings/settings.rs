use config::Config;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::env;

use crate::errors::CustomError;
use crate::models::vector::EmbeddingModel;

#[derive(Debug, Deserialize)]
pub struct Settings {
    qdrant_connection_string: SecretString,
    oxigraph_connection_string: SecretString,
    openai_api_key: SecretString,
    embedding_model: SecretString,
}

impl Settings {
    /// Creates a new Settings instance.
    ///
    /// # Description
    ///
    /// Primary constructor for creating a Settings instance. Takes a pre-configured Config object that defines all required settings.
    /// This allows for flexible configuration sourcing while maintaining a clean initialization interface.
    ///
    /// # Parameters
    ///
    /// - `config`: A `config::Config` instance containing all required settings:
    ///     - `qdrant_connection_string`: Connection string for Qdrant database
    ///     - `oxigraph_connection_string`: Connection string for Oxigraph database
    ///     - `openai_api_key`: API key for OpenAI services
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `Settings` instance with the provided configuration
    /// - `Err`: A `CustomError` if any required settings are missing or invalid
    pub fn new(config: Config) -> Result<Self, CustomError> {
        config.try_deserialize().map_err(|e| {
            CustomError::ConfigParseError(format!("Failed to parse external configuration: {}", e))
        })
    }

    /// Loads settings from environment variables and configuration files using default loaders.
    ///
    /// # Description
    ///
    /// This function provides a convenient way to load settings using the default environment and configuration loaders.
    /// If a `.env` file exists in the project root it will be loaded automatically.
    /// When the file is absent the function relies solely on the current environment variables.
    ///
    /// # Important
    ///
    /// This function is intended ONLY for use in integration tests and should not be used anywhere else in the codebase.
    /// For production code, use the `Settings::new()` constructor with an explicitly configured `Config` instance.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `Settings` instance with configuration loaded from environment and config files
    /// - `Err`: A `CustomError` if:
    ///     - Environment variables are missing or invalid
    ///     - There are errors parsing the configuration
    ///
    pub(crate) fn load() -> Result<Self, CustomError> {
        dotenvy::dotenv().ok();

        let qdrant_connection_string = env::var("QDRANT_CONNECTION_STRING").map_err(|e| {
            CustomError::ConfigParseError(format!("QDRANT_CONNECTION_STRING: {}", e))
        })?;
        let oxigraph_connection_string = env::var("OXIGRAPH_CONNECTION_STRING").map_err(|e| {
            CustomError::ConfigParseError(format!("OXIGRAPH_CONNECTION_STRING: {}", e))
        })?;
        let openai_api_key = env::var("OPENAI_API_KEY")
            .map_err(|e| CustomError::ConfigParseError(format!("OPENAI_API_KEY: {}", e)))?;
        let embedding_model = env::var("EMBEDDING_MODEL")
            .map_err(|e| CustomError::ConfigParseError(format!("EMBEDDING_MODEL: {}", e)))?;

        Ok(Self {
            qdrant_connection_string: SecretString::new(qdrant_connection_string.into()),
            oxigraph_connection_string: SecretString::new(oxigraph_connection_string.into()),
            openai_api_key: SecretString::new(openai_api_key.into()),
            embedding_model: SecretString::new(embedding_model.into()),
        })
    }

    pub fn get_qdrant_connection(&self) -> &str {
        self.qdrant_connection_string.expose_secret()
    }

    #[allow(dead_code)] // Remove after implementing Oxigraph
    pub fn get_oxigraph_connection(&self) -> &str {
        self.oxigraph_connection_string.expose_secret()
    }

    pub fn get_openai_api_key(&self) -> &str {
        self.openai_api_key.expose_secret()
    }

    pub(crate) fn get_embedding_model(&self) -> Result<EmbeddingModel, CustomError> {
        self.embedding_model.expose_secret().parse()
    }
}

#[cfg(test)]
impl Settings {
    pub fn new_for_tests(
        qdrant_connection_string: SecretString,
        oxigraph_connection_string: SecretString,
        openai_api_key: SecretString,
        embedding_model: SecretString,
    ) -> Self {
        Settings {
            qdrant_connection_string,
            oxigraph_connection_string,
            openai_api_key,
            embedding_model,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::CustomError;

    #[test]
    fn test_settings_new_success() {
        let external_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_connection_string", "external_oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .set_override("embedding_model", "TextEmbedding3Small")
            .unwrap()
            .build()
            .unwrap();

        let result = Settings::new(external_config);
        assert!(result.is_ok());

        let settings = result.unwrap();
        assert_eq!(settings.get_qdrant_connection(), "external_qdrant");
        assert_eq!(settings.get_oxigraph_connection(), "external_oxigraph");
    }

    #[test]
    fn test_settings_new_error() {
        let incomplete_config = Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .build()
            .unwrap();

        let result = Settings::new(incomplete_config);
        assert!(matches!(result, Err(CustomError::ConfigParseError(_))));
    }
}
