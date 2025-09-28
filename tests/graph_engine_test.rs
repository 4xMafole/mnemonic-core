use mnemonic_core::graph::GraphEngine;
use serde_json::json;
use tempfile::tempdir;

// The `#[tokio::test]` attribute tells Rust to use the Tokio async runtime to run this test.
// This is necessary because our GraphEngine functions are all async.

#[tokio::test]
async fn test_full_engine_lifecycle() {
    // ---1. SETUP ---
    // The setup is the same: createa a temporary directory and initialize our engine.
    let dir = tempdir().unwrap();
    let engine = GraphEngine::new(dir.path()).unwrap();

    // --2. ACTION: STORE ---
    // Call our new async `store` function on the engine.
    // The `.await` keywork tells Rust to wait for the async operation to complete.
    println!("Storing concepts...");
    let person_id = engine.store(json!({"name": "Carol"})).await.unwrap();
    let project_id = engine.store(json!({"name": "Mnemonic"})).await.unwrap();

    // --3. ACTION: RELATE ---
    // Call our new async `relate` function.
    println!("Relating concepts...");
    let relationship_id = engine
        .relate(person_id, "leads_project".to_string(), project_id)
        .await
        .unwrap();

    // --4. VERIFICATION: RETRIEVE --
    // Call our async `retrieve_by_source` function to check our work.
    println!("Retrieving relationships...");
    let relationships = engine.retrieve_by_source(person_id).await.unwrap();

    // The assertions check that the full lifecycle worked correctly.
    assert_eq!(relationships.len(), 1); // We should find exactly one relationship.
    let rel = &relationships[0];
    assert_eq!(rel.id, relationship_id);
    assert_eq!(rel.target, project_id);
    assert_eq!(rel.relationship_type, "leads_project");
    println!("Retrieve verification PASSED!");

    //--5. ACTION & VERIFICATION: UNRELATE ---
    println!("Unrelating concepts...");
    engine.unrelate(relationship_id).await.unwrap();

    // Retrieve again and verify that the relationship is now gone.
    let relationships_after_unrelate = engine.retrieve_by_source(person_id).await.unwrap();
    assert_eq!(relationships_after_unrelate.len(), 0);
    println!("Unrelate verification PASSED!");
}
