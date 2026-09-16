use character_memory::{CharacterMemory, CustomError};
use tempfile::TempDir;

use super::base;

/// Opens a facade over an embedded vector store under a fresh temporary root with
/// in-memory graph and stats. Finish with `close_and_remove_root(memory, root)`.
pub async fn try_setup_character_memory() -> Result<(CharacterMemory, TempDir), CustomError> {
    let root = TempDir::new().expect("store root should be created");
    let memory = base::open(
        base::embedded_settings(root.path()),
        base::unique_collection_name(),
    )
    .await?;
    Ok((memory, root))
}
