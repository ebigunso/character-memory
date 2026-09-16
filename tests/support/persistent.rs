use character_memory::{CharacterMemory, CustomError};
use std::path::Path;

use super::base;

/// Opens a facade whose vector, graph and stats stores all live under `root`
/// (`vectors`, `graph`, `stats.sqlite3`), so a second call with the same root and
/// collection name reopens the same persisted state.
pub async fn try_setup_persistent_character_memory(
    collection_name: String,
    root: &Path,
    about_derived_memory_fanout: Option<(usize, usize)>,
) -> Result<CharacterMemory, CustomError> {
    let mut builder = base::embedded_settings(root)
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
        .unwrap();

    if let Some((min, max)) = about_derived_memory_fanout {
        builder = builder
            .set_override(
                "retrieval.fanout.about_entity.derived_memory.min",
                min as i64,
            )
            .unwrap()
            .set_override(
                "retrieval.fanout.about_entity.derived_memory.max",
                max as i64,
            )
            .unwrap();
    }

    base::open(builder, collection_name).await
}
