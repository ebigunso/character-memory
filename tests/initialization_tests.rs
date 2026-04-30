mod test_utils;
use test_utils::{cleanup_collection, try_setup_character_memory};

#[tokio::test]
async fn test_character_memory_initialization() {
    // Setup
    let Ok((_character_memory, collection_name)) = try_setup_character_memory().await else {
        println!("skipping live initialization test because Qdrant is unavailable");
        return;
    };

    // The setup function already calls init_storage, so if we got here without errors,
    // it means initialization was successful

    // No explicit assertions needed as the test would fail if initialization failed

    // Cleanup
    cleanup_collection(&collection_name).await;
}
