#[path = "base.rs"]
mod base;

use character_memory::test_utils::load_test_settings;
use character_memory::{CharacterMemory, CustomError, Settings};
use config::Config;
use std::path::Path;

pub use base::{cleanup_collection, is_qdrant_unavailable_error, unique_collection_name};

pub async fn try_setup_persistent_character_memory(
    collection_name: String,
    graph_path: &Path,
    stats_path: &Path,
    about_derived_memory_fanout: Option<(usize, usize)>,
) -> Result<CharacterMemory, CustomError> {
    base::initialize();

    let base_settings = load_test_settings()?;
    let embedding_model = std::env::var("EMBEDDING_MODEL")
        .map_err(|error| CustomError::ConfigParseError(format!("EMBEDDING_MODEL: {error}")))?;

    let mut builder = Config::builder()
        .set_override(
            "qdrant_connection_string",
            base_settings.get_qdrant_connection(),
        )
        .map_err(config_error)?
        .set_override("oxigraph_connection_string", path_string(graph_path))
        .map_err(config_error)?
        .set_override("openai_api_key", base_settings.get_openai_api_key())
        .map_err(config_error)?
        .set_override("embedding_model", embedding_model)
        .map_err(config_error)?
        .set_override("graph_store_mode", "persistent")
        .map_err(config_error)?
        .set_override("retrieval_stats_store_mode", "sqlite")
        .map_err(config_error)?
        .set_override("retrieval_stats_path", path_string(stats_path))
        .map_err(config_error)?;

    if let Some((min, max)) = about_derived_memory_fanout {
        builder = builder
            .set_override(
                "retrieval.fanout.about_entity.derived_memory.min",
                min as i64,
            )
            .map_err(config_error)?
            .set_override(
                "retrieval.fanout.about_entity.derived_memory.max",
                max as i64,
            )
            .map_err(config_error)?;
    }

    let settings = Settings::new(builder.build().map_err(config_error)?)?;
    let embed_provider = Box::new(base::DeterministicEmbeddingProvider::new(
        settings.get_embedding_vector_size()?,
    ));

    CharacterMemory::new_with_embedding_provider(settings, collection_name, embed_provider).await
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn config_error(error: config::ConfigError) -> CustomError {
    CustomError::ConfigParseError(error.to_string())
}
