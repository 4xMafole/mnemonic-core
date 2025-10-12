use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::fmt::format;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use super::versioning::VersionStore;
use crate::types::concept::{Concept, ConceptId, ConceptVersion};
use crate::types::relationship::{Relationship, RelationshipId};
use crate::{MnemonicError, Result};

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
#[derive(Debug, Default)]
pub struct TransactionManager {
    // It holds a reference to the VersionStore to read history and write new versions.
    version_store: Arc<VersionStore>,

    // A thread-safe map of all currently active, uncommitted transactions.
    active_transactions: RwLock<HashMap<TransactionId, Transaction>>,
}

impl TransactionManager {
    /// Creates a new, empty TransactionManager.
    pub fn new() -> Self {
        Self {
            version_store: Arc::new(VersionStore::new()),
            active_transactions: RwLock::new(HashMap::new()),
        }
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

        // --- PHASE 2: APPLY CHANGES ---
        // If validation passes, we can safely apply the changes.
        // We are going to "fake" the writes for now. We will write to the in-memory
        // VersionStore but NOT to RocksDB. That's our next big step.

        // Loop through all the "pending writes" in our transaction's shopping cart.
        for (_concept_id, concept) in transaction.pending_writes {
            let new_version = ConceptVersion::from_concept(&concept, transaction.id);
            self.version_store.add_concept_version(new_version);
        }

        // We would also apply pending deletes and relationship changes here...

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::concept::ConceptData;
    use serde_json::json;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_transaction_lifecycle() {
        //1. Setup
        let manager = TransactionManager::new();

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

        //5. Verify About
        // Perform our final check in another separate block.
        {
            let active_ids_after_abort = manager.active_transactions.read().unwrap();
            assert!(active_ids_after_abort.is_empty());
        } // The second read lock is released here.
    }

    #[test]
    fn test_first_committer_wins_conflict() {
        //1. Setup
        let manager = TransactionManager::new();

        // Let's create one initial concept to fight over.
        let concept_to_update = Concept::new(json!({"value": "initial"}));
        let concept_id = concept_to_update.id;
    }
}
