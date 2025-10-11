use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::types::concept::{Concept, ConceptId};
use crate::types::relationship::{Relationship, RelationshipId};

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