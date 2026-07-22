use character_memory::{CharacterMemory, CustomError, Settings};
use config::Config;

use super::base;

pub async fn try_setup_character_memory() -> Result<(CharacterMemory, String), CustomError> {
    let collection_name = base::unique_collection_name();
    let settings = load_in_memory_settings()?;
    let embed_provider = Box::new(base::DeterministicEmbeddingProvider::new(
        settings.get_embedding_vector_size()?,
    ));

    let character_memory = CharacterMemory::new_with_embedding_provider(
        settings,
        collection_name.clone(),
        embed_provider,
    )
    .await?;

    Ok((character_memory, collection_name))
}

fn load_in_memory_settings() -> Result<Settings, CustomError> {
    // Builds fixture settings via explicit config overrides instead of
    // process-global env mutation. Note: unlike plain load_test_settings(),
    // this path intentionally does not honor optional env overrides such as
    // RETRIEVAL_STATS_STORE_MODE or SELECTIVITY_*/fanout vars; fixtures that
    // need those should set them as explicit overrides here.
    let base_settings = base::load_test_settings()?;
    let embedding_model = std::env::var("EMBEDDING_MODEL")
        .map_err(|error| CustomError::ConfigParseError(format!("EMBEDDING_MODEL: {error}")))?;

    let config = Config::builder()
        .set_override(
            "qdrant_connection_string",
            base_settings.get_qdrant_connection(),
        )
        .map_err(base::config_error)?
        .set_override(
            "oxigraph_path",
            base_settings
                .get_oxigraph_path()?
                .to_string_lossy()
                .into_owned(),
        )
        .map_err(base::config_error)?
        .set_override("openai_api_key", base_settings.get_openai_api_key())
        .map_err(base::config_error)?
        .set_override("embedding_model", embedding_model)
        .map_err(base::config_error)?
        .set_override("graph_store_mode", "in_memory")
        .map_err(base::config_error)?
        .build()
        .map_err(base::config_error)?;

    Settings::new(config)
}
