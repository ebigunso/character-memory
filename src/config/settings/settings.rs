use std::path::PathBuf;
use config::Config;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::config::enums::EmbeddingModel;
use crate::config::loaders::{ConfigLoader, DefaultConfigLoader};
use crate::config::loaders::{EnvLoader, DefaultEnvLoader};
use crate::errors::CustomError;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
    /// It expects a `.env` file in the project root directory and will attempt to load configuration values from both environment variables and the configuration system.
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
    ///     - The `.env` file is missing
    ///     - There are errors loading the environment variables
    ///     - There are errors parsing the configuration
    ///     - Required settings are missing or invalid
    ///
    pub(crate) fn load() -> Result<Self, CustomError> {
        Self::load_with_loaders(DefaultEnvLoader, DefaultConfigLoader)
    }

    fn load_with_loaders<E: EnvLoader, C: ConfigLoader>(
        env_loader: E,
        config_loader: C,
    ) -> Result<Self, CustomError> {
        // Get project root directory from CARGO_MANIFEST_DIR
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let env_path = project_root.join(".env");

        // Check if .env exists in project root
        if !env_loader.exists(env_path.clone()) {
            return Err(CustomError::EnvFileNotFound(
                "Please create .env file with required secrets in the project root.".to_string(),
            ));
        }

        // Load .env file from project root
        env_loader.load_from_path(env_path).map_err(|e| {
            CustomError::EnvLoadError(format!("Failed to load .env file: {}", e))
        })?;

        // Build configuration
        let settings = config_loader.build_config().map_err(|e| {
            CustomError::ConfigParseError(format!("Failed to build configuration: {}", e))
        })?;

        // Try to convert into Settings struct
        settings.try_deserialize().map_err(|e| {
            CustomError::ConfigParseError(format!("Failed to parse configuration: {}", e))
        })
    }

    #[allow(dead_code)]
    pub(crate) fn get_qdrant_connection(&self) -> &str {
        self.qdrant_connection_string.expose_secret()
    }

    #[allow(dead_code)]
    pub(crate) fn get_oxigraph_connection(&self) -> &str {
        self.oxigraph_connection_string.expose_secret()
    }

    pub(crate) fn get_openai_api_key(&self) -> &str {
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
    use crate::errors::custom::CustomError;
    use config::ConfigError;
    use crate::config::loaders::config_loader::MockConfigLoader;
    use crate::config::loaders::env_loader::MockEnvLoader;

    #[test]
    fn test_settings_load_success() {
        let mut mock_env = MockEnvLoader::new();
        mock_env.expect_exists().return_const(true);
        mock_env.expect_load_from_path().returning(|_| Ok(()));

        let mut mock_config = MockConfigLoader::new();
        mock_config.expect_build_config().returning(|| {
            Ok(Config::builder()
                .set_override("qdrant_connection_string", "test_qdrant")
                .unwrap()
                .set_override("oxigraph_connection_string", "test_oxigraph")
                .unwrap()
                .set_override("openai_api_key", "test_openai")
                .unwrap()
                .set_override("embedding_model", "TextEmbedding3Small")
                .unwrap()
                .build()
                .unwrap())
        });

        let result = Settings::load_with_loaders(mock_env, mock_config);
        assert!(result.is_ok());

        let settings = result.unwrap();
        assert_eq!(settings.get_qdrant_connection(), "test_qdrant");
        assert_eq!(settings.get_oxigraph_connection(), "test_oxigraph");
    }

    #[test]
    fn test_settings_load_env_missing() {
        let mut mock_env = MockEnvLoader::new();
        mock_env.expect_exists().return_const(false);
        let mock_config = MockConfigLoader::new();
        let result = Settings::load_with_loaders(mock_env, mock_config);
        assert!(matches!(result, Err(CustomError::EnvFileNotFound(_))));
    }

    #[test]
    fn test_settings_load_env_error() {
        let mut mock_env = MockEnvLoader::new();
        mock_env.expect_exists().return_const(true);
        mock_env.expect_load_from_path().returning(|_| {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Mock env load error"))
        });
        let mock_config = MockConfigLoader::new();
        let result = Settings::load_with_loaders(mock_env, mock_config);
        assert!(matches!(result, Err(CustomError::EnvLoadError(_))));
    }

    #[test]
    fn test_settings_load_config_error() {
        let mut mock_env = MockEnvLoader::new();
        mock_env.expect_exists().return_const(true);
        mock_env.expect_load_from_path().returning(|_| Ok(()));

        let mut mock_config = MockConfigLoader::new();
        mock_config.expect_build_config().returning(|| Err(ConfigError::NotFound("test".to_string())));

        let result = Settings::load_with_loaders(mock_env, mock_config);
        assert!(matches!(result, Err(CustomError::ConfigParseError(_))));
    }

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
