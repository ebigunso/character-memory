use character_memory::{CharacterMemory, CustomError};
use std::path::Path;

use super::base;

/// Opens a facade whose vector, graph and stats stores all live under `root`
/// (`vectors`, `graph`, `stats.sqlite3`), so a second call with the same root and
/// collection name reopens the same persisted state.
pub async fn try_setup_persistent_character_memory(
    collection_name: String,
    root: &Path,
) -> Result<CharacterMemory, CustomError> {
    base::open(persistent_settings(root), collection_name).await
}

/// Persistent vector, graph and stats stores at a caller-owned test root.
pub fn persistent_settings(root: &Path) -> config::ConfigBuilder<config::builder::DefaultState> {
    base::embedded_settings(root)
        .set_override("graph_store_mode", "persistent")
        .unwrap()
        .set_override("oxigraph_path", base::path_string(&root.join("graph")))
        .unwrap()
        .set_override("retrieval_stats_store_mode", "sqlite")
        .unwrap()
        .set_override(
            "retrieval_stats_path",
            base::path_string(&root.join("stats.sqlite3")),
        )
        .unwrap()
}
