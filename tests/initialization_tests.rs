mod test_utils;

#[tokio::test]
async fn test_agent_memory_initialization() {
    // Setup
    let (_agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    // The setup function already calls init_storage, so if we got here without errors,
    // it means initialization was successful

    // No explicit assertions needed as the test would fail if initialization failed

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}
