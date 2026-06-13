#[path = "base.rs"]
mod base;

use character_memory::test_utils::load_test_settings;
use character_memory::{CharacterMemory, CustomError};

pub use base::{cleanup_collection, is_qdrant_unavailable_error};

pub async fn try_setup_character_memory() -> Result<(CharacterMemory, String), CustomError> {
    base::initialize();

    let collection_name = base::unique_collection_name();
    let settings = load_test_settings()?;
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
