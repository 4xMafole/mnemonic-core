use serde_json;
use std::path::Path;
use std::sync::Arc;
use tokio::task;

use crate::error::{MnemonicError, Result};
use crate::storage::RocksBackend;
use crate::types::{
    concept::{Concept, ConceptId},
    relationship::{RelationType, Relationship, RelationshipId},
};

/// High-level graph engine that provides the core Mnemoninc Computing primities
pub struct GraphEngine {
    // We hold the backend inside an Arc so we can share it safely
    // across multiple concurrent operations.
    backend: Arc<RocksBackend>,
}

impl GraphEngine {
    /// Create a new GraphEngine instance with the specified storage path.
    pub fn new(storage_path: &Path) -> Result<Self> {
        // Initialize the low-level backend.
        let backend = RocksBackend::new(storage_path)?;
        // Wrap it in an Arc and store it.
        Ok(Self {
            backend: Arc::new(backend),
        })
    }

    /// STORE primitive: Add a new concept to the graph (asynchronously).
    pub async fn store(&self, data: serde_json::Value) -> Result<ConceptId> {
        // Create a thread-safe copy of the pointer to our backend.
        let backend = Arc::clone(&self.backend);

        // This is the new pattern. We move the blocking work to another thread.
        task::spawn_blocking(move || {
            // This closure contains the "blocking" work.
            let concept = Concept::new(data);
            let concept_id = concept.id;

            // This is the call to RocksDB, which is slow and blocking.
            backend.store_concept(&concept)?;

            // Returns the ID if successful.
            Ok(concept_id)
        })
        .await // Wait for the blocking task to finish...
        .unwrap() // ...handle potential panic from the other thread...
    }

    /// RELATE primitive: Create a relationship between two concepts.
    pub async fn relate(
        &self,
        source: ConceptId,
        relationship_type: RelationType,
        target: ConceptId,
    ) -> Result<RelationshipId> {
        let backend = Arc::clone(&self.backend);

        task::spawn_blocking(move || {
            // A good practice: check that the concepts you're connecting actually exist.
            if backend.get_concept(&source)?.is_none() {
                return Err(MnemonicError::ConceptNotFound(source));
            }
            if backend.get_concept(&target)?.is_none() {
                return Err(MnemonicError::ConceptNotFound(target));
            }

            let relationship = Relationship::new(source, relationship_type, target);
            let id = relationship.id;
            backend.store_relationship(&relationship)?;
            Ok(id)
        })
        .await
        .unwrap()
    }

    /// UNRELATE primitive: Remove a relationship from the graph.
    pub async fn unrelate(&self, rel_id: RelationshipId) -> Result<()> {
        let backend = Arc::clone(&self.backend);

        task::spawn_blocking(move || {
            // This now calls the new function we added to the backend.
            backend.delete_ralationship(&rel_id)
        })
        .await
        .unwrap()
    }

    /// Basic RETRIEVE: Get all relationships originating from a concept.
    pub async fn retrieve_by_source(&self, source_id: ConceptId) -> Result<Vec<Relationship>> {
        let backend = Arc::clone(&self.backend);

        task::spawn_blocking(move || backend.get_relationships_by_source(&source_id))
            .await
            .unwrap()
    }
}
