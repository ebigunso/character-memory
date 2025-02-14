use super::settings::{Settings, MockEnvLoader, MockConfigLoader};
use crate::errors::custom::CustomError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_load_success() {
        let mut mock_env = MockEnvLoader::new();
        mock_env.expect_exists()
            .return_const(true);
        mock_env.expect_load_from_path()
            .returning(|_| Ok(()));

        let mut mock_config = MockConfigLoader::new();
        mock_config.expect_build_config()
            .returning(|| {
                Ok(config::Config::builder()
                    .set_override("qdrant_connection_string", "test_qdrant").unwrap()
                    .set_override("oxigraph_connection_string", "test_oxigraph").unwrap()
                    .set_override("openai_api_key", "test_openai").unwrap()
                    .build().unwrap())
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
        mock_env.expect_exists()
            .return_const(false);
        let mock_config = MockConfigLoader::new();
        let result = Settings::load_with_loaders(mock_env, mock_config);
        assert!(matches!(result, Err(CustomError::EnvFileNotFound(_))));
    }

    #[test]
    fn test_settings_load_env_error() {
        let mut mock_env = MockEnvLoader::new();
        mock_env.expect_exists()
            .return_const(true);
        mock_env.expect_load_from_path()
            .returning(|_| Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Mock env load error"
            )));
        let mock_config = MockConfigLoader::new();
        let result = Settings::load_with_loaders(mock_env, mock_config);
        assert!(matches!(result, Err(CustomError::EnvLoadError(_))));
    }

    #[test]
    fn test_settings_load_config_error() {
        let mut mock_env = MockEnvLoader::new();
        mock_env.expect_exists()
            .return_const(true);
        mock_env.expect_load_from_path()
            .returning(|_| Ok(()));

        let mut mock_config = MockConfigLoader::new();
        mock_config.expect_build_config()
            .returning(|| Err(config::ConfigError::NotFound("test".to_string())));

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
            .set_override("openai_api_key", "external_openai").unwrap()
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
        // Missing required fields
        let incomplete_config = config::Config::builder()
            .set_override("qdrant_connection_string", "external_qdrant")
            .unwrap()
            // oxigraph_connection_string is missing
            .build()
            .unwrap();

        let result = Settings::from_config(incomplete_config);
        assert!(matches!(result, Err(CustomError::ConfigParseError(_))));
    }
}
