use agent_memory::{MemoryInput, MemoryType};
use chrono::Utc;
mod test_utils;

#[tokio::test]
async fn test_create_episodic_memory() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    let memory_input = MemoryInput {
        id: None,
        content: "Test episodic memory content".to_string(),
        memory_type: MemoryType::Episodic,
        timestamp: Some(Utc::now()),
        location_text: Some("Test Location".to_string()),
        participants: Some(vec!["Test User".to_string()]),
    };

    // Create the memory
    let result = agent_memory.create_memory(memory_input).await;

    // Verify the result
    assert!(result.is_ok(), "Failed to create episodic memory");

    let memory = result.unwrap();
    assert_eq!(memory.content, "Test episodic memory content");
    assert_eq!(memory.memory_type, MemoryType::Episodic);
    assert!(memory.location_text.is_some());
    assert_eq!(memory.location_text.unwrap(), "Test Location");
    assert!(memory.participants.is_some());
    assert_eq!(memory.participants.unwrap(), vec!["Test User".to_string()]);

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_create_semantic_memory() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    let memory_input = MemoryInput {
        id: None,
        content: "Test semantic memory content".to_string(),
        memory_type: MemoryType::Semantic,
        timestamp: None,
        location_text: None,
        participants: None,
    };

    // Create the memory
    let result = agent_memory.create_memory(memory_input).await;

    // Verify the result
    assert!(result.is_ok(), "Failed to create semantic memory");

    let memory = result.unwrap();
    assert_eq!(memory.content, "Test semantic memory content");
    assert_eq!(memory.memory_type, MemoryType::Semantic);

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_bulk_create_memories() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    let memory_inputs = vec![
        MemoryInput {
            id: None,
            content: "Bulk episodic memory 1".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(Utc::now()),
            location_text: Some("Test Location 1".to_string()),
            participants: Some(vec!["User 1".to_string()]),
        },
        MemoryInput {
            id: None,
            content: "Bulk semantic memory 1".to_string(),
            memory_type: MemoryType::Semantic,
            timestamp: None,
            location_text: None,
            participants: None,
        },
        MemoryInput {
            id: None,
            content: "Bulk episodic memory 2".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(Utc::now()),
            location_text: Some("Test Location 3".to_string()),
            participants: Some(vec!["User 3".to_string()]),
        },
    ];

    // Bulk create memories
    let result = agent_memory.bulk_create_memories(&memory_inputs).await;

    // Verify the result
    assert!(result.is_ok(), "Failed to bulk create memories");

    let memories = result.unwrap();
    assert_eq!(memories.len(), 3, "Should have created 3 memories");

    // Verify each memory
    assert_eq!(memories[0].content, "Bulk episodic memory 1");
    assert_eq!(memories[0].memory_type, MemoryType::Episodic);

    assert_eq!(memories[1].content, "Bulk semantic memory 1");
    assert_eq!(memories[1].memory_type, MemoryType::Semantic);

    assert_eq!(memories[2].content, "Bulk episodic memory 2");
    assert_eq!(memories[2].memory_type, MemoryType::Episodic);

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}
