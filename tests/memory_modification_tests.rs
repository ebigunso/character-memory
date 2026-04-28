#![allow(deprecated)]

use character_memory::{MemoryInput, MemoryType};
use chrono::Utc;
use uuid::Uuid;
mod test_utils;

#[tokio::test]
async fn test_update_memory() {
    // Setup
    let (character_memory, collection_name) = test_utils::setup_character_memory().await;

    // Create a test memory
    let memory_input = MemoryInput {
        id: None,
        content: "Original content".to_string(),
        memory_type: MemoryType::Episodic,
        timestamp: Some(Utc::now()),
        location_text: Some("Original location".to_string()),
        participants: Some(vec!["Original participant".to_string()]),
    };

    // Create the memory and get its ID
    let created_memory = character_memory.create_memory(memory_input).await.unwrap();
    let memory_id = created_memory.id;
    test_utils::wait_for_memory(&character_memory, memory_id).await;

    // Create an updated memory input with the same ID
    let updated_input = MemoryInput {
        id: Some(memory_id),
        content: "Updated content".to_string(),
        memory_type: MemoryType::Episodic,
        timestamp: Some(Utc::now()),
        location_text: Some("Updated location".to_string()),
        participants: Some(vec!["Updated participant".to_string()]),
    };

    // Update the memory
    let result = character_memory.update_memory(updated_input).await;

    // Verify the result
    assert!(result.is_ok(), "Failed to update memory");

    let updated_memory = result.unwrap();
    assert_eq!(updated_memory.id, memory_id);
    assert_eq!(updated_memory.content, "Updated content");
    assert_eq!(updated_memory.memory_type, MemoryType::Episodic);
    assert!(updated_memory.location_text.is_some());
    assert_eq!(updated_memory.location_text.unwrap(), "Updated location");
    assert!(updated_memory.participants.is_some());
    assert_eq!(
        updated_memory.participants.unwrap(),
        vec!["Updated participant".to_string()]
    );

    // Verify the update by retrieving the memory
    let retrieved = character_memory.get_memory_by_id(memory_id).await.unwrap();
    assert_eq!(retrieved.content, "Updated content");

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_update_memory_type() {
    // Setup
    let (character_memory, collection_name) = test_utils::setup_character_memory().await;

    // Create a test memory with Episodic type
    let memory_input = MemoryInput {
        id: None,
        content: "Memory content".to_string(),
        memory_type: MemoryType::Episodic,
        timestamp: Some(Utc::now()),
        location_text: Some("Location".to_string()),
        participants: Some(vec!["Participant".to_string()]),
    };

    // Create the memory and get its ID
    let created_memory = character_memory.create_memory(memory_input).await.unwrap();
    let memory_id = created_memory.id;
    test_utils::wait_for_memory(&character_memory, memory_id).await;

    // Create an updated memory input with Semantic type
    let updated_input = MemoryInput {
        id: Some(memory_id),
        content: "Memory content".to_string(),
        memory_type: MemoryType::Semantic,
        timestamp: None,
        location_text: None,
        participants: None,
    };

    // Update the memory
    let result = character_memory.update_memory(updated_input).await;

    // Verify the result
    assert!(result.is_ok(), "Failed to update memory type");

    let updated_memory = result.unwrap();
    assert_eq!(updated_memory.id, memory_id);
    assert_eq!(updated_memory.memory_type, MemoryType::Semantic);
    assert!(updated_memory.timestamp.is_none());
    assert!(updated_memory.location_text.is_none());
    assert!(updated_memory.participants.is_none());

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_update_nonexistent_memory() {
    // Setup
    let (character_memory, collection_name) = test_utils::setup_character_memory().await;

    // Generate a random UUID that doesn't exist in the database
    let nonexistent_id = Uuid::new_v4();

    // Create an update input with the nonexistent ID
    let update_input = MemoryInput {
        id: Some(nonexistent_id),
        content: "This memory doesn't exist".to_string(),
        memory_type: MemoryType::Episodic,
        timestamp: Some(Utc::now()),
        location_text: None,
        participants: None,
    };

    // Try to update a nonexistent memory
    let result = character_memory.update_memory(update_input).await;

    // Verify the result is an error
    assert!(
        result.is_err(),
        "Expected error when updating nonexistent memory"
    );

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_delete_memory() {
    // Setup
    let (character_memory, collection_name) = test_utils::setup_character_memory().await;

    // Create a test memory
    let memory_input = MemoryInput {
        id: None,
        content: "Memory to delete".to_string(),
        memory_type: MemoryType::Episodic,
        timestamp: Some(Utc::now()),
        location_text: Some("Test location".to_string()),
        participants: Some(vec!["Test participant".to_string()]),
    };

    // Create the memory and get its ID
    let created_memory = character_memory.create_memory(memory_input).await.unwrap();
    let memory_id = created_memory.id;
    test_utils::wait_for_memory(&character_memory, memory_id).await;

    // Delete the memory
    let result = character_memory.delete_memory(memory_id).await;

    // Verify the deletion was successful
    assert!(result.is_ok(), "Failed to delete memory");

    // Try to retrieve the deleted memory
    let retrieve_result = character_memory.get_memory_by_id(memory_id).await;

    // Verify the memory no longer exists
    assert!(
        retrieve_result.is_err(),
        "Memory should no longer exist after deletion"
    );

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_delete_nonexistent_memory() {
    // Setup
    let (character_memory, collection_name) = test_utils::setup_character_memory().await;

    // Generate a random UUID that doesn't exist in the database
    let nonexistent_id = Uuid::new_v4();

    // Try to delete a nonexistent memory
    let result = character_memory.delete_memory(nonexistent_id).await;

    // Verify the result is an error
    assert!(
        result.is_err(),
        "Expected error when deleting nonexistent memory"
    );

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}
