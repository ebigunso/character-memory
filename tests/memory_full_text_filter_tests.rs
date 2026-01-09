use agent_memory::{MemoryFilters, MemoryInput, MemoryType};
use chrono::Utc;
mod test_utils;

#[tokio::test]
async fn test_participants_full_text_token_match() {
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

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

    agent_memory
        .bulk_create_memories(&memory_inputs)
        .await
        .unwrap();

    let filters = MemoryFilters {
        memory_type: Some("episodic".to_string()),
        start_date: None,
        end_date: None,
        location_text: None,
        participants: Some(vec!["Alice".to_string()]),
    };

    let results = agent_memory
        .search_memories("Alice", 10, Some(filters))
        .await
        .unwrap();

    assert!(
        results.iter().any(|m| m.content.contains("Alice Johnson")),
        "Expected to find memory containing 'Alice Johnson' via token match on participants"
    );

    // Non-goal: within-word substring matching is not required.
    let filters_substring = MemoryFilters {
        memory_type: Some("episodic".to_string()),
        start_date: None,
        end_date: None,
        location_text: None,
        participants: Some(vec!["Ali".to_string()]),
    };

    let results_substring = agent_memory
        .search_memories("Alice", 10, Some(filters_substring))
        .await
        .unwrap();

    assert!(
        results_substring
            .iter()
            .all(|m| !m.content.contains("Alice Johnson")),
        "Did not expect within-word substring match (Ali -> Alice)"
    );

    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_location_text_full_text_token_match() {
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

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

    agent_memory
        .bulk_create_memories(&memory_inputs)
        .await
        .unwrap();

    let filters = MemoryFilters {
        memory_type: Some("episodic".to_string()),
        start_date: None,
        end_date: None,
        location_text: Some("New York".to_string()),
        participants: None,
    };

    let results = agent_memory
        .search_memories("Manhattan", 10, Some(filters))
        .await
        .unwrap();

    assert!(
        results.iter().any(|m| m.content.contains("Manhattan")),
        "Expected to find memory in 'New York City' when filtering by 'New York'"
    );

    test_utils::cleanup_collection(&collection_name).await;
}
