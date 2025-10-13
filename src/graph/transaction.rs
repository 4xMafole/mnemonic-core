use super::versioning::VersionStore;
use crate::storage::RocksBackend;
use crate::types::concept::{Concept, ConceptId, ConceptVersion};
use crate::types::relationship::RelationshipId;
use crate::{MnemonicError, Result};
use chrono::{DateTime, Utc};
use rocksdb::WriteBatch;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// A unique ID for a transaction.
pub type TransactionId = Uuid;

/// Defines how much a transaciton is isolated from other concurrent transactions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsolationLevel {
    Snapshot, // For now, we will only implement the strongest level.
}

/// A Transaction is a "workspace" for a set of atomic changes to the graph.
#[derive(Debug, Clone)]
pub struct Transaction {
    /// The unique ID for this transaction.
    pub id: TransactionId,

    /// The exact moment in time this transaction started. Crucial for MVCC.
    pub start_timestamp: DateTime<Utc>,

    /// The isolation level for this transaction
    pub isolation_level: IsolationLevel,

    /// A list of ConceptIDs this transaction has read. Used for conflict detection.
    pub read_set: HashSet<ConceptId>,

    /// A list of ConceptIDs this transaction has written to. Used for conflict detection.
    pub write_set: HashSet<ConceptId>,

    // NOTE: We'll add sets for relationships later to keep this simple for now.
    /// A private "scratchpad" for new or updated concepts for this transaction.
    pub pending_writes: HashMap<ConceptId, Concept>,

    /// A list of relationships marked for deletion in this transaction.
    pub pending_deletes: HashSet<RelationshipId>,
}

impl Transaction {
    /// Creates a new, empty transaction.
    pub fn new(isolation_level: IsolationLevel) -> Self {
        Self {
            id: Uuid::new_v4(),
            start_timestamp: Utc::now(),
            isolation_level,
            read_set: HashSet::new(),
            write_set: HashSet::new(),
            pending_writes: HashMap::new(),
            pending_deletes: HashSet::new(),
        }
    }
}

/// TransactionManager orchestrates all transactions and handles MVCC.
#[derive(Debug)]
pub struct TransactionManager {
    // It holds a reference to the VersionStore to read history and write new versions.
    version_store: Arc<VersionStore>,
    // Stores data to rocksdb
    backend: Arc<RocksBackend>,
    // A thread-safe map of all currently active, uncommitted transactions.
    active_transactions: RwLock<HashMap<TransactionId, Transaction>>,
}

impl TransactionManager {
    /// Creates a new, empty TransactionManager.
    pub fn new(backend: Arc<RocksBackend>) -> Result<Self> {
        // Note: It now returns a Result
        // 1. Create a new, empty VersionStore.
        let version_store = VersionStore::new();

        // 2. Load all historical versions from the disk.
        let all_versions = backend.load_all_concept_versions()?;

        // 3. "Hydrate" the in-memory VersionStore by re-inserting all the historical data.
        for version in all_versions {
            version_store.add_concept_version(version)?;
        }

        // We would also hydrate relationship versions here in a full implementation.

        // 4. Create the manager with the now-hydrated VersionStore.
        Ok(Self {
            version_store: Arc::new(version_store),
            backend,
            active_transactions: RwLock::new(HashMap::new()),
        })
    }

    /// Begins a new transaction and registers it as active.
    pub fn begin_transaction(&self, isolation_level: IsolationLevel) -> Result<Transaction> {
        //1. Create a new transaction "shopping cart".
        let transaction = Transaction::new(isolation_level);

        //2. Lock the active transaction list for writing.
        let mut active_txs = self
            .active_transactions
            .write()
            .map_err(|e| MnemonicError::Transaction(format!("Lock failed: {}", e)))?;

        //3. Add the new transaction to the list of active ones.
        active_txs.insert(transaction.id, transaction.clone());

        Ok(transaction)
    }

    /// Aborts a transaction, discarding all its changes.
    pub fn abort_transaction(&self, transaction_id: TransactionId) -> Result<()> {
        let mut active_txs = self
            .active_transactions
            .write()
            .map_err(|e| MnemonicError::Transaction(format!("Lock failed: {}", e)))?;

        // Simply remove the transaction from the active list. Its changes are never saved.
        if active_txs.remove(&transaction_id).is_some() {
            Ok(())
        } else {
            Err(MnemonicError::Transaction(format!(
                "Transaction {} not found to abort",
                transaction_id
            )))
        }
    }

    /// Commits a transaction, applying its changes if there are no conflicts.
    pub fn commit_transaction(&self, transaction: Transaction) -> Result<()> {
        // --- PHASE 1: VALIDATION ---
        // Before we do anything, check for conflicts with other committed changes.
        self.validate_transaction(&transaction)?;

        // --- PHASE 2: PERSISTENCE & APPLY CHANGES ---
        let mut batch = WriteBatch::default(); //1. Create a new atomic batch

        // Loop through all the "pending writes" in our transaction's shopping cart.
        for (concept_id, pending_concept) in transaction.pending_writes {
            // 1. Get the last known version from the in-memory store.
            let last_version = self
                .version_store
                .get_concept_version_at_timestamp(&concept_id, transaction.start_timestamp)?;

            // 2. Calculate the next version number
            let next_version_num = last_version.map_or(1, |v| v.version + 1);

            // 3. Create the new version with the correct number.
            let new_version =
                ConceptVersion::from_concept(&pending_concept, transaction.id, next_version_num);

            // 4. Prepare for durable write and update in-memory store.
            self.backend
                .store_concept_version(&new_version, &mut batch)?;
            self.version_store.add_concept_version(new_version)?;
        }

        // We would also apply pending deletes and relationship changes here...

        // write the entire batch to disk, atomically.
        self.backend.db.write(batch)?;

        // --- PHASE 3: CLEANUP ---
        // The commit was successful. Remove the transaction from the active list.
        let mut active_txs = self
            .active_transactions
            .write()
            .map_err(|e| MnemonicError::Transaction(format!("Lock failed: {}", e)))?;
        active_txs.remove(&transaction.id);

        Ok(())
    }

    /// The "First Committer Wins" conflict detection logic.
    fn validate_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Go through every concept ID that our transaction tried to change
        for concept_id in &transaction.write_set {
            // Ask the VersionStore: "Has this concept been modified by anyone else
            // since our transaction started?"

            //If YES, we have a conflict! Abort the commit.
            if self
                .version_store
                .has_concept_been_modified_since(concept_id, transaction.start_timestamp)?
            {
                return Err(MnemonicError::TransactionConflict(format!(
                    "Conflict detected on concept {}",
                    concept_id
                )));
            }
        }

        // If we get through the whole loop without finding any conflicts, we are safe.
        Ok(())
    }

    /// Returns a thread-safe handle to the internal VersionStore.
    /// This is needed for the engine to perform read operations.
    pub fn version_store(&self) -> Arc<VersionStore> {
        Arc::clone(&self.version_store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::concept::{ConceptData, ConceptMetadata};
    use serde_json::json;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_transaction_lifecycle() {
        //1. Setup
        let dir = tempdir().unwrap();
        let backend = Arc::new(RocksBackend::new(dir.path()).unwrap());
        let manager = TransactionManager::new(Arc::clone(&backend)).unwrap();

        //2. Begin transaction
        let txn = manager.begin_transaction(IsolationLevel::Snapshot).unwrap();

        //3. Verify begin
        // We will do our check inside a separate block
        // This ensures the read lock is released immediately after the check
        {
            let active_ids = manager.active_transactions.read().unwrap();
            assert_eq!(active_ids.len(), 1);
            assert!(active_ids.contains_key(&txn.id));
        } // The read lock is automatically released here as `active_txs` is destroyed.

        //4. Abort transaction
        manager.abort_transaction(txn.id).unwrap();

        //5. Verify Abort
        // Perform our final check in another separate block.
        {
            let active_ids_after_abort = manager.active_transactions.read().unwrap();
            assert!(active_ids_after_abort.is_empty());
        } // The second read lock is released here.
    }

    #[test]
    fn test_first_committer_wins_conflict() {
        // --- 1. SETUP ---
        // Create a backend directly for our test, so we can peek into it.
        let dir = tempdir().unwrap();
        let backend = Arc::new(RocksBackend::new(dir.path()).unwrap());
        let manager = TransactionManager::new(Arc::clone(&backend)).unwrap();

        // --- 2. CREATE INITIAL STATE (The Correct Way) ---
        // Let's create our initial concept inside a transaction and commit it.
        let concept_id;
        {
            let mut initial_txn = manager.begin_transaction(IsolationLevel::Snapshot).unwrap();
            let concept_to_create = Concept::new(json!({"value": "initial"}));
            concept_id = concept_to_create.id; // Save the ID

            initial_txn.write_set.insert(concept_id);
            initial_txn
                .pending_writes
                .insert(concept_id, concept_to_create);

            // This commit writes the INITIAL version (version 1) to RocksDB and in-memory store.
            manager.commit_transaction(initial_txn).unwrap();
        }

        // --- 2. THE RACE BEGINS ---

        // Alice starts her transaction.
        let mut alice_txn = manager.begin_transaction(IsolationLevel::Snapshot).unwrap();

        // Bob starts his transaction right after. His view of the world is the same as Alice's.
        let mut bob_txn = manager.begin_transaction(IsolationLevel::Snapshot).unwrap();

        // --- 4. ALICE WINS THE RACE ---
        {
            // Alice needs to read the concept first to modify it.
            let concept_for_alice = manager
                .version_store
                .get_concept_version_at_timestamp(&concept_id, alice_txn.start_timestamp)
                .unwrap()
                .unwrap();

            // Create the updated concept
            let updated_concept = Concept {
                id: concept_id,
                data: ConceptData::Structured(json!({"value": "alice was here"}).to_string()),
                metadata: ConceptMetadata {
                    created_at: concept_for_alice.created_at,
                    updated_at: Utc::now(),
                    version: concept_for_alice.version + 1,
                    transaction_id: alice_txn.id,
                },
            };

            alice_txn.write_set.insert(concept_id);
            alice_txn.pending_writes.insert(concept_id, updated_concept);

            thread::sleep(Duration::from_millis(10));
            assert!(manager.commit_transaction(alice_txn).is_ok());
        }

        // --- 5. BOB TRIES TO COMMIT (AND FAILS) ---
        {
            let updated_concept_bob = Concept {
                id: concept_id,
                data: ConceptData::Structured(json!({"value": "bob was here"}).to_string()),
                metadata: Default::default(),
            };
            bob_txn.write_set.insert(concept_id);
            bob_txn
                .pending_writes
                .insert(concept_id, updated_concept_bob);

            let bob_commit_result = manager.commit_transaction(bob_txn);
            assert!(bob_commit_result.is_err());
            assert!(matches!(
                bob_commit_result.unwrap_err(),
                MnemonicError::TransactionConflict(_)
            ));
        }

        // --- 6. DURABILITY PROOF ---
        // Let's check that ALICE's commit (version 2) is on disk.
        let cf_versions = backend.db.cf_handle("versions").unwrap();
        let expected_key_v2 = format!("cv:{}:2", concept_id);
        let version_data_v2 = backend.db.get_cf(&cf_versions, expected_key_v2).unwrap();
        assert!(version_data_v2.is_some());

        // And check that the INITIAL commit (version 1) is ALSO on disk.
        let expected_key_v1 = format!("cv:{}:1", concept_id);
        let version_data_v1 = backend.db.get_cf(&cf_versions, expected_key_v1).unwrap();
        assert!(version_data_v1.is_some());
    }
}
