use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::concept::{ConceptId, TransactionId}; // This means "import ConceptId & TransactionID from the concept.rs file in this same folder"

/// An ID for a relationship, which is an edge in our graph.
pub type RelationshipId = Uuid;

// For now, a relationship type is just a simple string, like "works_for".
pub type RelationType = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelationshipMetadata {
    pub created_at: DateTime<Utc>,
    pub version: u64,
    pub transaction_id: TransactionId,
}

impl Default for RelationshipMetadata {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            version: 1,
            transaction_id: Uuid::nil(),
        }
    }
}

/// The complete Relationship struct. This is an edge in our graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    pub id: RelationshipId,
    pub source: ConceptId, // The ID of the concept where the edge starts.
    pub relationship_type: RelationType,
    pub target: ConceptId, // The ID of the concept where the edge ends.
    pub metadata: RelationshipMetadata,
}

impl Relationship {
    /// A constructor to easily create a new relationship.
    pub fn new(source: ConceptId, relationship_type: RelationType, target: ConceptId) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            relationship_type,
            target,
            metadata: RelationshipMetadata::default(),
        }
    }
}

/// A versioned snapshot of a relationship's state for MVCC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelationshipVersion {
    pub relationship_id: RelationshipId,
    pub version: u64,
    pub source: ConceptId,
    pub relationship_type: RelationType,
    pub target: ConceptId,
    pub created_at: DateTime<Utc>,
    pub created_by: TransactionId,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<TransactionId>,
}

impl RelationshipVersion {
    /// Creates the First version of the relationship
    pub fn from_relationship(relationship: &Relationship, transaction_id: TransactionId) -> Self {
        Self {
            relationship_id: relationship.id,
            version: 1,
            source: relationship.source,
            relationship_type: relationship.relationship_type.clone(),
            target: relationship.target,
            created_at: relationship.metadata.created_at,
            created_by: transaction_id,
            deleted_at: None,
            deleted_by: None,
        }
    }

    /// Checks if this version was "live" at a given timestamp.
    pub fn is_active_at(&self, timestamp: DateTime<Utc>) -> bool {
        self.created_at <= timestamp && self.deleted_at.map_or(true, |deleted| deleted > timestamp)
    }
}
