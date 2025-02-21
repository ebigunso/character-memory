use config::{Config, ConfigError};
use mockall::automock;

#[automock]
pub(crate) trait ConfigLoader {
    fn build_config(&self) -> Result<Config, ConfigError>;
}

pub(crate) struct DefaultConfigLoader;

impl ConfigLoader for DefaultConfigLoader {
    fn build_config(&self) -> Result<Config, ConfigError> {
        Config::builder()
            .add_source(config::Environment::default())
            .build()
    }
}
