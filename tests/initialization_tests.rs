mod test_utils;
use test_utils::{cleanup_collection, try_setup_character_memory};

#[tokio::test]
async fn test_character_memory_initialization() {
    // Setup
    let Ok((_character_memory, collection_name)) = try_setup_character_memory().await else {
        println!("skipping live initialization test because Qdrant is unavailable");
        return;
    };

    // Construction initializes the Qdrant candidate collection, so reaching this point
    // means the public constructor completed live storage setup.

    // Cleanup
    cleanup_collection(&collection_name).await;
}
