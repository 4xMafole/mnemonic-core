use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Core concept identifier type. It's just a unique ID.
pub type ConceptId = Uuid;

/// Data for tracking when a concept was created/changed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,
}

impl Default for ConceptMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            version: 1,
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
