use super::concept::ConceptId; // This means "import ConceptId from the concept.rs file in this same folder"
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An ID for a relationship, which is an edge in our graph.
pub type RelationshipId = Uuid;

// For now, a relationship type is just a simple string, like "works_for".
pub type RelationType = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelationshipMetadata {
    pub created_at: DateTime<Utc>,
    pub version: u64,
}

impl Default for RelationshipMetadata {
    fn default() -> Self {
        Self {
            created_at: Utc::now(),
            version: 1,
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
