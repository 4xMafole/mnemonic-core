use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Core concept identifier type. It's just a unique ID.
pub type ConceptId = Uuid;
pub type TransactionId = Uuid;

/// Data for tracking when a concept was created/changed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,
    pub transaction_id: TransactionId,
}

impl Default for ConceptMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            version: 1,
            transaction_id: Uuid::nil(),
        }
    }
}

/// The actual data stored in the concept.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConceptData {
    // For pure structural nodes, like a group.
    Empty,
    // For storing structured info, like a user profile.
    Structured(String),
}

/// The complete Concept struct. This is a node in our graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Concept {
    pub id: ConceptId,
    pub data: ConceptData,
    pub metadata: ConceptMetadata,
}

// These are "constructors" - easy ways to make a new Concept.
impl Concept {
    /// Create a new concept with structured data (e.g a JSON object).
    pub fn new(data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            data: ConceptData::Structured(data.to_string()),
            metadata: ConceptMetadata::default(),
        }
    }

    /// Create a new empty concept.
    pub fn empty() -> Self {
        Self {
            id: Uuid::new_v4(),
            data: ConceptData::Empty,
            metadata: ConceptMetadata::default(),
        }
    }
}

/// A versioned snapshot of a concept's state for MVCC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptVersion {
    pub concept_id: ConceptId,
    pub version: u64,
    pub data: ConceptData,
    pub created_at: DateTime<Utc>,
    pub created_by: TransactionId,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<TransactionId>,
}

impl ConceptVersion {
    pub fn from_concept(concept: &Concept, transaction_id: TransactionId) -> Self {
        Self {
            concept_id: concept.id,
            version: concept.metadata.version,
            data: concept.data.clone(),
            created_at: concept.metadata.updated_at,
            created_by: transaction_id,
            deleted_at: None,
            deleted_by: None,
        }
    }

    /// Checks if this version was "live" at a given timestamp
    pub fn is_active_at(&self, timestamp: DateTime<Utc>) -> bool {
        self.created_at <= timestamp && self.deleted_at.map_or(true, |deleted| deleted > timestamp)
    }
}
