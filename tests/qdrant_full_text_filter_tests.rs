#![allow(deprecated)]

use character_memory::{MemoryFilters, MemoryInput, MemoryType};
use chrono::Utc;
mod test_utils;

#[tokio::test]
async fn test_participants_full_text_token_match() {
    let (character_memory, collection_name) = test_utils::setup_character_memory().await;

    let now = Utc::now();
    let memory_inputs = vec![
        MemoryInput {
            id: None,
            content: "Met Alice Johnson at the cafe".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(now),
            location_text: Some("New York City".to_string()),
            participants: Some(vec!["Alice Johnson".to_string(), "Bob".to_string()]),
        },
        MemoryInput {
            id: None,
            content: "Met Charlie in London".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(now),
            location_text: Some("London".to_string()),
            participants: Some(vec!["Charlie".to_string()]),
        },
    ];

    let created_memories = character_memory
        .bulk_create_memories(&memory_inputs)
        .await
        .unwrap();
    for memory in &created_memories {
        test_utils::wait_for_memory(&character_memory, memory.id).await;
    }

    let filters = MemoryFilters {
        memory_type: Some("episodic".to_string()),
        start_date: None,
        end_date: None,
        location_text: None,
        participants: Some(vec!["Alice".to_string()]),
    };

    let results = character_memory
        .search_memories("Alice", 10, Some(filters))
        .await
        .unwrap();

    assert!(
        results.iter().all(|m| m.score.is_finite()),
        "All results should have finite scores"
    );
    assert!(
        results.windows(2).all(|w| w[0].score >= w[1].score),
        "Results should be ordered by descending score"
    );

    assert!(
        results
            .iter()
            .any(|m| m.memory.content.contains("Alice Johnson")),
        "Expected to find memory containing 'Alice Johnson' via token match on participants"
    );

    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_location_text_full_text_token_match() {
    let (character_memory, collection_name) = test_utils::setup_character_memory().await;

    let now = Utc::now();
    let memory_inputs = vec![
        MemoryInput {
            id: None,
            content: "Walked around Manhattan".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(now),
            location_text: Some("New York City".to_string()),
            participants: Some(vec!["Alice Johnson".to_string()]),
        },
        MemoryInput {
            id: None,
            content: "Visited the Tower of London".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(now),
            location_text: Some("London".to_string()),
            participants: Some(vec!["Charlie".to_string()]),
        },
    ];

    let created_memories = character_memory
        .bulk_create_memories(&memory_inputs)
        .await
        .unwrap();
    for memory in &created_memories {
        test_utils::wait_for_memory(&character_memory, memory.id).await;
    }

    let filters = MemoryFilters {
        memory_type: Some("episodic".to_string()),
        start_date: None,
        end_date: None,
        location_text: Some("New York".to_string()),
        participants: None,
    };

    let results = character_memory
        .search_memories("Manhattan", 10, Some(filters))
        .await
        .unwrap();

    assert!(
        results.iter().all(|m| m.score.is_finite()),
        "All results should have finite scores"
    );
    assert!(
        results.windows(2).all(|w| w[0].score >= w[1].score),
        "Results should be ordered by descending score"
    );

    assert!(
        results
            .iter()
            .any(|m| m.memory.content.contains("Manhattan")),
        "Expected to find memory in 'New York City' when filtering by 'New York'"
    );

    test_utils::cleanup_collection(&collection_name).await;
}
