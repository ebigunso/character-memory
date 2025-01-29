mod config;
mod errors;

use config::settings::Settings;

/// Initialize the library with configuration from environment
pub fn init() -> Result<(), errors::custom::CustomError> {
    let _settings = Settings::load()?;
    Ok(())
}

// Public API functions will be added here as needed
// They will use the internal settings but won't expose them
