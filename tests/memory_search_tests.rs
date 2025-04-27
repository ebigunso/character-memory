use agent_memory::{MemoryFilters, MemoryInput, MemoryType};
use chrono::{Duration, Utc};
mod test_utils;

#[tokio::test]
async fn test_basic_search() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    // Create test memories with different content
    let memory_inputs = vec![
        MemoryInput {
            id: None,
            content: "The quick brown fox jumps over the lazy dog".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(Utc::now()),
            location_text: Some("Forest".to_string()),
            participants: Some(vec!["Fox".to_string(), "Dog".to_string()]),
        },
        MemoryInput {
            id: None,
            content: "The early bird catches the worm".to_string(),
            memory_type: MemoryType::Semantic,
            timestamp: None,
            location_text: None,
            participants: None,
        },
        MemoryInput {
            id: None,
            content: "All that glitters is not gold".to_string(),
            memory_type: MemoryType::Semantic,
            timestamp: None,
            location_text: None,
            participants: None,
        },
    ];

    // Create the memories
    agent_memory
        .bulk_create_memories(&memory_inputs)
        .await
        .unwrap();

    // Search for memories related to "fox"
    let result = agent_memory.search_memories("fox", 10, None).await;

    // Verify the result
    assert!(result.is_ok(), "Failed to search memories");

    let memories = result.unwrap();
    assert!(
        !memories.is_empty(),
        "Should have found at least one memory"
    );

    // The first result should be the one with "fox" in it
    assert!(
        memories[0].content.contains("fox"),
        "First result should contain 'fox'"
    );

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_search_with_memory_type_filter() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    // Create test memories with different types
    let memory_inputs = vec![
        MemoryInput {
            id: None,
            content: "Episodic memory about a vacation".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(Utc::now()),
            location_text: Some("Beach".to_string()),
            participants: Some(vec!["Family".to_string()]),
        },
        MemoryInput {
            id: None,
            content: "Semantic memory about vacation destinations".to_string(),
            memory_type: MemoryType::Semantic,
            timestamp: None,
            location_text: None,
            participants: None,
        },
        MemoryInput {
            id: None,
            content: "Another episodic memory about a trip".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(Utc::now()),
            location_text: Some("Mountain".to_string()),
            participants: Some(vec!["Friends".to_string()]),
        },
    ];

    // Create the memories
    agent_memory
        .bulk_create_memories(&memory_inputs)
        .await
        .unwrap();

    // Create a filter for episodic memories
    let filters = MemoryFilters {
        memory_type: Some("episodic".to_string()),
        start_date: None,
        end_date: None,
        location_text: None,
        participants: None,
    };

    // Search for memories related to "vacation" with episodic filter
    let result = agent_memory
        .search_memories("vacation", 10, Some(filters))
        .await;

    // Verify the result
    assert!(result.is_ok(), "Failed to search memories with filter");

    let memories = result.unwrap();
    assert!(
        !memories.is_empty(),
        "Should have found at least one memory"
    );

    // All results should be episodic memories
    for memory in &memories {
        assert_eq!(
            memory.memory_type,
            MemoryType::Episodic,
            "All results should be episodic memories"
        );
    }

    // The result should contain the episodic memory about vacation, not the semantic one
    let has_episodic_vacation = memories.iter().any(|m| m.content.contains("vacation"));
    assert!(
        has_episodic_vacation,
        "Results should include episodic memory about vacation"
    );

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn test_search_with_date_filter() {
    // Setup
    let (agent_memory, collection_name) = test_utils::setup_agent_memory().await;

    // Create test memories with different timestamps
    let now = Utc::now();
    let yesterday = now - Duration::days(1);
    let last_week = now - Duration::days(7);

    let memory_inputs = vec![
        MemoryInput {
            id: None,
            content: "Recent memory from today".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(now),
            location_text: Some("Home".to_string()),
            participants: Some(vec!["Me".to_string()]),
        },
        MemoryInput {
            id: None,
            content: "Memory from yesterday".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(yesterday),
            location_text: Some("Office".to_string()),
            participants: Some(vec!["Colleagues".to_string()]),
        },
        MemoryInput {
            id: None,
            content: "Old memory from last week".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(last_week),
            location_text: Some("Park".to_string()),
            participants: Some(vec!["Friends".to_string()]),
        },
    ];

    // Create the memories
    agent_memory
        .bulk_create_memories(&memory_inputs)
        .await
        .unwrap();

    // Create a filter for memories from the last 2 days
    let two_days_ago = now - Duration::days(2);
    let filters = MemoryFilters {
        memory_type: None,
        start_date: Some(two_days_ago),
        end_date: None,
        location_text: None,
        participants: None,
    };

    // Search for memories with date filter
    let result = agent_memory
        .search_memories("memory", 10, Some(filters))
        .await;

    // Verify the result
    assert!(result.is_ok(), "Failed to search memories with date filter");

    let memories = result.unwrap();
    assert!(
        !memories.is_empty(),
        "Should have found at least one memory"
    );

    // Results should only include memories from the last 2 days
    let has_recent = memories.iter().any(|m| m.content.contains("today"));
    let has_yesterday = memories.iter().any(|m| m.content.contains("yesterday"));
    let has_old = memories.iter().any(|m| m.content.contains("last week"));

    assert!(has_recent, "Results should include today's memory");
    assert!(has_yesterday, "Results should include yesterday's memory");
    assert!(!has_old, "Results should not include last week's memory");

    // Cleanup
    test_utils::cleanup_collection(&collection_name).await;
}
