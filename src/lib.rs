mod config;
mod errors;

use config::settings::Settings;

/// Initialize the library with externally supplied settings.
#[allow(unused_variables)] // ToDo: Remove this when implementing the function
pub fn init(settings: Settings) -> Result<(), errors::custom::CustomError> {
    // Use `settings` to initialize internal components,
    // such as establishing database connections or configuring clients.
    Ok(())
}

/// Initialize the library by loading settings from the environment.
/// This function is primarily intended for integration tests.
#[allow(dead_code)] // ToDo: Remove this when implementing integration tests
pub(crate) fn init_from_env() -> Result<(), errors::custom::CustomError> {
    let settings = Settings::load()?;
    init(settings)
}

// Public API functions will be added here as needed
// They will use the internal settings but won't expose them
