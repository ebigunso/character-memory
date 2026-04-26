use agent_memory::{MemoryInput, MemoryType};
use chrono::Utc;
use uuid::Uuid;
mod test_utils;

#[tokio::test]
async fn test_get_memory_by_id() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    // Create a test memory
    let memory_input = MemoryInput {
        id: None,
        content: "Test memory for retrieval".to_string(),
        memory_type: MemoryType::Episodic,
        timestamp: Some(Utc::now()),
        location_text: Some("Test Location".to_string()),
        participants: Some(vec!["Test User".to_string()]),
    };

    // Create the memory and get its ID
    let created_memory = agent_memory.create_memory(memory_input).await.unwrap();
    let memory_id = created_memory.id;

    // Retrieve the memory by ID
    let retrieved_memory = test_utils::wait_for_memory(&agent_memory, memory_id).await;
    assert_eq!(retrieved_memory.id, memory_id);
    assert_eq!(retrieved_memory.content, "Test memory for retrieval");
    assert_eq!(retrieved_memory.memory_type, MemoryType::Episodic);

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_get_memory_by_nonexistent_id() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    // Generate a random UUID that doesn't exist in the database
    let nonexistent_id = Uuid::new_v4();

    // Try to retrieve a memory with a nonexistent ID
    let result = agent_memory.get_memory_by_id(nonexistent_id).await;

    // Verify the result is an error
    assert!(
        result.is_err(),
        "Expected error when retrieving nonexistent memory"
    );

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_get_memories_by_ids() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    // Create multiple test memories
    let memory_inputs = vec![
        MemoryInput {
            id: None,
            content: "First test memory".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(Utc::now()),
            location_text: Some("Location 1".to_string()),
            participants: Some(vec!["User 1".to_string()]),
        },
        MemoryInput {
            id: None,
            content: "Second test memory".to_string(),
            memory_type: MemoryType::Semantic,
            timestamp: None,
            location_text: None,
            participants: None,
        },
    ];

    // Create the memories and collect their IDs
    let created_memories = agent_memory
        .bulk_create_memories(&memory_inputs)
        .await
        .unwrap();
    let memory_ids: Vec<Uuid> = created_memories.iter().map(|m| m.id).collect();
    for memory_id in &memory_ids {
        test_utils::wait_for_memory(&agent_memory, *memory_id).await;
    }

    // Retrieve the memories by IDs
    let result = agent_memory.get_memories_by_ids(&memory_ids).await;

    // Verify the result
    assert!(result.is_ok(), "Failed to retrieve memories by IDs");

    let retrieved_memories = result.unwrap();
    assert_eq!(
        retrieved_memories.len(),
        2,
        "Should have retrieved 2 memories"
    );

    // Verify the memories were retrieved in the same order as the IDs
    assert_eq!(retrieved_memories[0].id, memory_ids[0]);
    assert_eq!(retrieved_memories[0].content, "First test memory");
    assert_eq!(retrieved_memories[0].memory_type, MemoryType::Episodic);

    assert_eq!(retrieved_memories[1].id, memory_ids[1]);
    assert_eq!(retrieved_memories[1].content, "Second test memory");
    assert_eq!(retrieved_memories[1].memory_type, MemoryType::Semantic);

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}
