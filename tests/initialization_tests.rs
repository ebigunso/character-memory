use character_memory::{CustomError, VectorDatabaseError};

#[path = "support/mod.rs"]
pub mod test_support;
use test_support::{cleanup_collection, is_qdrant_unavailable_error, try_setup_character_memory};

#[tokio::test]
async fn test_character_memory_initialization() {
    // Setup
    let (_character_memory, collection_name) = match try_setup_character_memory().await {
        Ok(setup) => setup,
        Err(CustomError::VectorDatabaseError(error)) if is_qdrant_unavailable_error(&error) => {
            println!("skipping live initialization test because Qdrant is unavailable: {error}");
            return;
        }
        Err(error) => panic!("unexpected live initialization setup failure: {error}"),
    };

    // Construction initializes the Qdrant candidate collection, so reaching this point
    // means the public constructor completed live storage setup.
    let test_result: Result<(), String> = async { Ok(()) }.await;

    // Cleanup
    cleanup_collection(&collection_name).await;
    test_result.expect("live initialization test should pass");
}

#[test]
fn qdrant_skip_gate_uses_typed_transport_classification() {
    let typed_unavailable: VectorDatabaseError = serde_json::from_value(serde_json::json!({
        "backend": "qdrant",
        "kind": { "kind": "response" },
        "status": { "kind": "unavailable" },
        "message": "opaque transport detail",
        "retry_after_seconds": null
    }))
    .unwrap();
    let prose_only: VectorDatabaseError = serde_json::from_value(serde_json::json!({
        "backend": "qdrant",
        "kind": { "kind": "response" },
        "status": null,
        "message": "failed to connect: tcp connect error",
        "retry_after_seconds": null
    }))
    .unwrap();

    assert!(is_qdrant_unavailable_error(&typed_unavailable));
    assert!(!is_qdrant_unavailable_error(&prose_only));
}
