use agent_memory::{AgentMemory, MemoryInput, MemoryType};
use agent_memory::test_utils::load_test_settings;
use chrono::Utc;
use std::sync::Once;
use uuid::Uuid;
use qdrant_client::Qdrant;

static INIT: Once = Once::new();

// Initialize environment once for all tests
pub fn initialize() {
    INIT.call_once(|| {
        // Any global setup can go here
    });
}

// Create a unique collection name for parallel test execution
pub fn unique_collection_name() -> String {
    format!("test_collection_{}", Uuid::new_v4())
}

// Setup AgentMemory instance with a unique collection
pub async fn setup_agent_memory() -> (AgentMemory, String) {
    initialize();

    let collection_name = unique_collection_name();

    // Use the load_test_settings function from the test_utils module
    let settings = load_test_settings()
        .expect("Failed to load settings from environment");

    let agent_memory = AgentMemory::new(settings, collection_name.clone())
        .await
        .expect("Failed to create AgentMemory");

    agent_memory.init_storage().await.expect("Failed to initialize storage");

    (agent_memory, collection_name)
}

// Cleanup after tests by deleting the collection
pub async fn cleanup_collection(collection_name: &str) {
    let settings = load_test_settings()
        .expect("Failed to load settings from environment");

    let qdrant_url = settings.get_qdrant_connection().to_string();
    let client = Qdrant::from_url(&qdrant_url)
        .build()
        .expect("Failed to create Qdrant client");

    // Delete the collection
    let _ = client.delete_collection(collection_name).await;

    // We don't assert on the result because the collection might not exist
    // if the test failed before creating it
}
