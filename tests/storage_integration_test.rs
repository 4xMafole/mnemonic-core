use mnemonic_core::storage::RocksBackend;
use mnemonic_core::types::concept::Concept;
use serde_json::json; // A handy macro for creating JSON data easily.
use tempfile::tempdir; // This will create our temporary directories.
// We need to import the Relationship type as well
use mnemonic_core::types::relationship::Relationship;

//The `#[test]` attribute tells Rust that this function is a test case.
#[test]
fn test_store_and_get_concept() {
    // --- 1. SETUP ---
    // Create a new, temporary directory for our test database.
    // It will be automatically deleted when the test finishes, even if it fails.
    let dir = tempdir().unwrap();

    //Create an instance of our storage backend, pointing it to our temporary directory.
    let backend = RocksBackend::new(dir.path()).unwrap();

    //Create the test data: a simple concept for a person named "Alice".
    let concept_to_store = Concept::new(json!({
        "type": "person",
        "name": "Alice"
    }));

    //We need to saver the ID so we can use it to retrieve the concept later.
    let concept_id = concept_to_store.id;

    // ---2. ACTION---
    // This is the line we are actually testing. Store the concept in the database.
    backend.store_concept(&concept_to_store).unwrap();

    // ---3. VERIFICATION ---
    // Retrieve the concept from the database using the ID we saved.
    // The first .unwrap() handles the database Result (in case the operation failed).
    // The second .unwrap() handles the Option (in case the concept was not found).
    let retrieved_concept = backend.get_concept(&concept_id).unwrap().unwrap();

    //This is the most important line: the assertion.
    //It checks if the retrieved concept is equal to the one we stored.
    //If they are not equal, the test will panic and fail.
    assert_eq!(retrieved_concept.id, concept_id);
    assert_eq!(retrieved_concept.data, concept_to_store.data);
    println!("SUCCESS: Concept round-trip test passed!");
}

#[test]
fn test_store_and_get_relationship_by_source() {
    // --- 1. SETUP ---
    let dir = tempdir().unwrap();
    let backend = RocksBackend::new(dir.path()).unwrap();

    // To have a relationship, we first need two concepts.
    let person_concept = Concept::new(json!({"name": "Bob"}));
    let company_concept = Concept::new(json!({"name": "TechCorp"}));

    // Save the concepts to the database first.
    backend.store_concept(&person_concept).unwrap();
    backend.store_concept(&company_concept).unwrap();

    // Now, create the relationship connecting them.
    let relationship_to_store = Relationship::new(
        person_concept.id,
        "works_for".to_string(),
        company_concept.id,
    );

    // --- 2. ACTION ---
    // Store the relationship. This should also create our index entries.
    backend.store_relationship(&relationship_to_store).unwrap();

    // --- 3. VERIFICATION ---
    // This is the key part. We are testing the index by querying
    // for all relationships that start from 'person_concept'.
    let retrieved_relationships = backend
        .get_relationships_by_source(&person_concept.id)
        .unwrap();

    // Assert that we found exactly one relationship.
    assert_eq!(retrieved_relationships.len(), 1);

    // Assert that the relationship we found is the correct one.
    let found_rel = &retrieved_relationships[0];
    assert_eq!(found_rel.id, relationship_to_store.id);
    assert_eq!(found_rel.target, company_concept.id);
    assert_eq!(found_rel.relationship_type, "works_for");
    println!("SUCCESS: Relationship indexing test passed!");
}
