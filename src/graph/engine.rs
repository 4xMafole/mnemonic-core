use serde_json;
use std::path::Path;
use std::sync::Arc;
use tokio::task;
use uuid::Uuid;

use super::transaction::{IsolationLevel, Transaction, TransactionManager};
use crate::error::{MnemonicError, Result};
use crate::storage::RocksBackend;
use crate::types::{
    concept::{Concept, ConceptId},
    relationship::{RelationType, Relationship, RelationshipId, RelationshipMetadata},
};

/// High-level graph engine that provides the core Mnemoninc Computing primities
#[derive(Debug)]
pub struct GraphEngine {
    // We hold the backend inside an Arc so we can share it safely
    // across multiple concurrent operations.
    transaction_manager: Arc<TransactionManager>,
    backend: Arc<RocksBackend>,
}

impl GraphEngine {
    /// Create a new GraphEngine instance with the specified storage path.
    pub fn new(storage_path: &Path) -> Result<Self> {
        // Initialize the low-level backend.
        let backend = Arc::new(RocksBackend::new(storage_path)?);
        let transaction_manager = TransactionManager::new(Arc::clone(&backend))?;
        // Wrap it in an Arc and store it.
        Ok(Self {
            transaction_manager: Arc::new(transaction_manager),
            backend,
        })
    }

    /// STORE primitive: Creates and commits a concept in a single, atomic transaction.
    pub async fn store(&self, data: serde_json::Value) -> Result<ConceptId> {
        let manager = Arc::clone(&self.transaction_manager);

        task::spawn_blocking(move || {
            let mut txn = manager.begin_transaction(IsolationLevel::Snapshot)?;
            let new_concept = Concept::new(data);
            let concept_id = new_concept.id;
            txn.write_set.insert(concept_id);
            txn.pending_writes.insert(concept_id, new_concept);
            manager.commit_transaction(txn)?;
            Ok(concept_id)
        })
        .await
        .unwrap()
    }

    /// RELATE primitive: Creates and commits a relationship in a single transaction.
    pub async fn relate(
        &self,
        source: ConceptId,
        relationship_type: RelationType,
        target: ConceptId,
    ) -> Result<RelationshipId> {
        // 1. Begin a new transaction for this single operation.
        let manager = Arc::clone(&self.transaction_manager);

        task::spawn_blocking(move || {
            let mut txn = manager.begin_transaction(IsolationLevel::Snapshot)?;

            // For a 'relate', we should check that the source and target concepts exist
            // This is a 'read', so we should add them to our read_set.
            let source_version = manager
                .version_store()
                .get_concept_version_at_timestamp(&source, txn.start_timestamp)?;
            let target_version = manager
                .version_store()
                .get_concept_version_at_timestamp(&target, txn.start_timestamp)?;

            if source_version.is_none() {
                return Err(MnemonicError::ConceptNotFound(source));
            }
            if target_version.is_none() {
                return Err(MnemonicError::ConceptNotFound(target));
            }
            txn.read_set.insert(source);
            txn.read_set.insert(target);

            // 2. Perform the work inside the transaction.
            let new_rel = Relationship::new(source, relationship_type, target);
            let rel_id = new_rel.id;

            // Add the new relationship to the transaction's "shopping cart".
            txn.relationship_write_set.insert(rel_id);
            txn.pending_relationship_writes.insert(rel_id, new_rel);

            // 3. Commit the transaction atomically.
            manager.commit_transaction(txn)?;

            Ok(rel_id)
        })
        .await
        .unwrap()
    }

    /// UNRELATE primitive: Remove a relationship from the graph.
    pub async fn unrelate(&self, rel_id: RelationshipId) -> Result<()> {
        let manager = Arc::clone(&self.transaction_manager);

        task::spawn_blocking(move || {
            // 1. Begin a new transaction.
            let mut txn = manager.begin_transaction(IsolationLevel::Snapshot)?;

            // 2. Check if the relationship exists to be deleted.
            if manager
                .version_store()
                .get_relationship_version_at_timestamp(&rel_id, txn.start_timestamp)?
                .is_none()
            {
                return Err(MnemonicError::RelationshipNotFound(rel_id));
            }

            // 3. Add the delete operation to our "shopping cart".
            txn.pending_deletes.insert(rel_id);

            // Mark this as a "write" operation for conflict detection purposes.
            txn.relationship_write_set.insert(rel_id);

            // 4. Commit the transaction.
            manager.commit_transaction(txn)?;

            Ok(())
        })
        .await
        .unwrap()
    }

    /// Basic RETRIEVE: Get all relationships originating from a concept.
    pub async fn retrieve_by_source(&self, source_id: ConceptId) -> Result<Vec<Relationship>> {
        let manager = Arc::clone(&self.transaction_manager);

        task::spawn_blocking(move || {
            let now = chrono::Utc::now();
            let version_store = manager.version_store();

            // 1. Get ALL relationships from the version store's memory.
            let all_active_rels = version_store.get_all_active_relationships()?;

            // 2. Filter them down to find the ones that match our source_id.
            let matching_rels: Vec<Relationship> = all_active_rels
                .into_iter()
                .filter(|version| version.source == source_id)
                // 3. Convert them back to the simple 'Relationship' type for the API.
                .map(|version| Relationship {
                    id: version.relationship_id,
                    source: version.source,
                    relationship_type: version.relationship_type.clone(),
                    target: version.target,
                    metadata: RelationshipMetadata {
                        created_at: version.created_at,
                        version: version.version,
                        transaction_id: version.created_by,
                    },
                })
                .collect();

            Ok(matching_rels)
        })
        .await
        .unwrap()
    }
    /// Begin a new transaction
    pub async fn begin_transaction(&self, isolation_level: IsolationLevel) -> Result<Transaction> {
        let manager = Arc::clone(&self.transaction_manager);
        task::spawn_blocking(move || manager.begin_transaction(isolation_level))
            .await
            .unwrap() // This unwrap can be improved later
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, transaction: Transaction) -> Result<()> {
        let manager = Arc::clone(&self.transaction_manager);
        task::spawn_blocking(move || manager.commit_transaction(transaction))
            .await
            .unwrap()
    }

    /// Abort a transaction
    pub async fn abort_transaction(&self, transaction_id: Uuid) -> Result<()> {
        let manager = Arc::clone(&self.transaction_manager);
        task::spawn_blocking(move || manager.abort_transaction(transaction_id))
            .await
            .unwrap()
    }

    /// Returns a thread-safe handle to the internal TransactionManager.
    /// This is useful for advanced operations or for testing and debugging.
    pub fn transaction_manager(&self) -> Arc<TransactionManager> {
        Arc::clone(&self.transaction_manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_engine_transactional_store_is_visible() {
        let dir = tempdir().unwrap();
        let engine = GraphEngine::new(dir.path()).unwrap();

        // Store a concept using the engine's public API
        let concept_id = engine
            .store(serde_json::json!({"name": "Test"}))
            .await
            .unwrap();

        // Use the internal manager to check if the data is visible
        let manager = engine.transaction_manager();
        let version_store = manager.version_store();

        let retrieved_version = version_store
            .get_concept_version_at_timestamp(&concept_id, chrono::Utc::now())
            .unwrap();

        // Assert that the commit was successful and the data is now in the version store
        assert!(retrieved_version.is_some());
        assert_eq!(retrieved_version.unwrap().concept_id, concept_id);
    }
}
