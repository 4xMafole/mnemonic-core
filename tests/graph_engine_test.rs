use chrono::Utc;
use mnemonic_core::{
    graph::{GraphEngine, IsolationLevel},
    types::concept::Concept,
};
use serde_json::json;
use tempfile::tempdir;

use std::time::Duration;
use tokio::{task, time::sleep};

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

#[tokio::test]
async fn test_transaction_is_durable_across_restarts() {
    // --- 1. SETUP ---
    let dir = tempdir().unwrap();
    let db_path = dir.path().to_path_buf(); // Save the path for later.
    let concept_id; // A variable to hold the ID we create.

    // --- 2. FIRST SESSION: CREATE AND COMMIT DATA ---
    println!("Starting first engine session...");
    {
        let engine1 = GraphEngine::new(&db_path).unwrap();

        // Begin a transaction
        let mut txn = engine1
            .begin_transaction(IsolationLevel::Snapshot)
            .await
            .unwrap();

        // Create a new concept within the transaction
        let new_concept = Concept::new(json!({"handle": "4xMafole"}));
        concept_id = new_concept.id;

        txn.write_set.insert(concept_id);
        txn.pending_writes.insert(concept_id, new_concept);

        // Commit the transaction. This should write to RocksDB.
        engine1.commit_transaction(txn).await.unwrap();

        println!("Concept {} created and committed.", concept_id);
    } // `engine1` is dropped here, simulating the program shutting down.

    // Wait a moment to ensure file handles are released.
    sleep(Duration::from_millis(100)).await;

    // --- 3. SECOND SESSION: RESTART AND VERIFY ---
    println!("Starting second engine session to verify data...");
    {
        // Create a BRAND NEW engine, pointing to the SAME database file.
        // The `new()` constructor should now trigger our hydration logic.
        let engine2 = GraphEngine::new(&db_path).unwrap();

        // Now, we will check the IN-MEMORY VersionStore of this new engine.
        // We need to do this in a blocking task because the TransactionManager uses sync RwLock.
        let tm = engine2.transaction_manager();
        let concept_was_loaded = task::spawn_blocking(move || {
            let version_store = tm.version_store(); // Get access to the hydrated store

            // Try to find the concept we created in the first session, at the current time.
            let version = version_store.get_concept_version_at_timestamp(&concept_id, Utc::now());

            // Check if we found it.
            version.unwrap().is_some()
        })
        .await
        .unwrap();

        // --- 4. ASSERTION ---
        // This is the moment of truth. If this passes, our data survived the restart.
        assert!(
            concept_was_loaded,
            "Concept was not loaded from disk on engine restart!"
        );

        println!("SUCCESS: Transaction was durable and hydrated correctly!");
    }
}
