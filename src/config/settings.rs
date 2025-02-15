use std::path::PathBuf;
use config::Config;
use mockall::automock;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use crate::errors::custom::CustomError;

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
        dotenvy::from_path(&path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    fn exists(&self, path: PathBuf) -> bool {
        path.exists()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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

    /// Creates a Settings instance from an externally provided `config::Config`.
    /// This method is intended for consumers who wish to build their configuration using custom sources (files, command-line, etc.) and inject it.
    pub fn from_config(config: Config) -> Result<Self, CustomError> {
        config.try_deserialize().map_err(|e| {
            CustomError::ConfigParseError(format!("Failed to parse external configuration: {}", e))
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

    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::custom::CustomError;

    #[test]
    fn test_settings_load_success() {
        let mut mock_env = MockEnvLoader::new();
        mock_env.expect_exists().return_const(true);
        mock_env.expect_load_from_path().returning(|_| Ok(()));

        let mut mock_config = MockConfigLoader::new();
        mock_config.expect_build_config().returning(|| {
            Ok(config::Config::builder()
                .set_override("qdrant_connection_string", "test_qdrant")
                .unwrap()
                .set_override("oxigraph_connection_string", "test_oxigraph")
                .unwrap()
                .set_override("openai_api_key", "test_openai")
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
        mock_config.expect_build_config().returning(|| Err(config::ConfigError::NotFound("test".to_string())));

        let result = Settings::load_with_loaders(mock_env, mock_config);
        assert!(matches!(result, Err(CustomError::ConfigParseError(_))));
    }

    #[test]
    fn test_settings_from_config_success() {
        let external_config = config::Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .set_override("oxigraph_connection_string", "external_oxigraph")
            .unwrap()
            .set_override("openai_api_key", "external_openai")
            .unwrap()
            .build()
            .unwrap();

        let result = Settings::from_config(external_config);
        assert!(result.is_ok());

        let settings = result.unwrap();
        assert_eq!(settings.get_qdrant_connection(), "external_qdrant");
        assert_eq!(settings.get_oxigraph_connection(), "external_oxigraph");
    }

    #[test]
    fn test_settings_from_config_error() {
        let incomplete_config = config::Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            .build()
            .unwrap();

        let result = Settings::from_config(incomplete_config);
        assert!(matches!(result, Err(CustomError::ConfigParseError(_))));
    }
}
