use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::RwLock; // Read-Write Lock: Allows many readers or one writer at a time.
use uuid::Uuid;

use crate::error::{MnemonicError, Result};
use crate::types::concept::{ConceptId, ConceptVersion};
use crate::types::relationship::{RelationshipId, RelationshipVersion};

/// VersionStore manages all versions of concepts and relationships for MVCC.
#[derive(Debug, Default)] // Default trait lets use create a new one easily.
pub struct VersionStore {
    // A map form a Concept's ID to a list of all its historical versions.
    // Wrapped in a RwLock to make it thread-safe.
    concept_versions: RwLock<HashMap<ConceptId, Vec<ConceptVersion>>>,

    // Same for relationships.
    relationship_versions: RwLock<HashMap<RelationshipId, Vec<RelationshipVersion>>>,
}

impl VersionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// The core of "Time Travel". Finds the correct version of the concept
    /// that was "live" at a specific timestamp.
    pub fn get_concept_version_at_timestamp(
        &self,
        concept_id: &ConceptId,
        timestamp: DateTime<Utc>,
    ) -> Result<Option<ConceptVersion>> {
        // We need to `read` the data, which requires a lock.
        let versions_map = self
            .concept_versions
            .read()
            .map_err(|e| MnemonicError::Transaction(format!("Read lock failed: {}", e)))?;

        // Find the list of versions for this specific concept ID.
        if let Some(versions_vec) = versions_map.get(concept_id) {
            // Search backwards from the newest version to the oldest.
            for version in versions_vec.iter().rev() {
                // Find the first version that was created at or before our query time.
                if version.created_at <= timestamp {
                    // NOW, check if `this specific version` was active at that time.

                    // Use our handy helper methhod to see if this version was active at the time.
                    if version.is_active_at(timestamp) {
                        return Ok(Some(version.clone()));
                    } else {
                        // We found the correct historical record, but it was inactive (deleted).
                        // So the state at that time was `nothing`. Stop searching.
                        return Ok(None);
                    }
                }
            }
        }

        Ok(None) // No active version found for that time
    }

    /// Finds the correct version of a relationship that was "live" at a specific timestamp.
    pub fn get_relationship_version_at_timestamp(
        &self,
        relationship_id: &RelationshipId,
        timestamp: DateTime<Utc>,
    ) -> Result<Option<RelationshipVersion>> {
        let versions_map = self
            .relationship_versions
            .read()
            .map_err(|e| MnemonicError::Transaction(format!("Read lock failed: {}", e)))?;

        if let Some(versions_vec) = versions_map.get(relationship_id) {
            // Search backwards from the newest version to the oldest.
            for version in versions_vec.iter().rev() {
                if version.created_at <= timestamp {
                    // Now we just use the single, correct source of truth.
                    if version.is_active_at(timestamp) {
                        return Ok(Some(version.clone()));
                    } else {
                        return Ok(None);
                    }
                }
            }
        }

        Ok(None)
    }

    /// Adds a new version to a concept's history chain
    pub fn add_concept_version(&self, version: ConceptVersion) -> Result<()> {
        // We need to `write` to the data, which requires a write lock.
        let mut versions_map = self
            .concept_versions
            .write()
            .map_err(|e| MnemonicError::Transaction(format!("Write lock failed: {}", e)))?;

        // Find the vector for this concept ID, or create a new empty one if it's the first version.
        versions_map
            .entry(version.concept_id)
            .or_default()
            .push(version);
        Ok(())
    }

    /// Adds a new version to a relationship's history chain.
    pub fn add_relationship_version(&self, version: RelationshipVersion) -> Result<()> {
        let mut versions_map = self
            .relationship_versions
            .write()
            .map_err(|e| MnemonicError::Transaction(format!("Write lock failed: {}", e)))?;

        // Find the vector for this relationship ID, or create a new empty one.
        versions_map
            .entry(version.relationship_id)
            .or_default()
            .push(version);

        Ok(())
    }

    /// Adds a simple check for conflict detection
    pub fn has_concept_been_modified_since(
        &self,
        concept_id: &ConceptId,
        timestamp: DateTime<Utc>,
    ) -> Result<bool> {
        let versions_map = self
            .concept_versions
            .read()
            .map_err(|e| MnemonicError::Transaction(format!("Read lock failed: {}", e)))?;

        if let Some(versions_vec) = versions_map.get(concept_id) {
            // If the latest version was created after our timestamp, there is a conflict.
            if let Some(latest_version) = versions_vec.last() {
                return Ok(latest_version.created_at > timestamp);
            }
        }
        Ok(false)
    }

    /// Checks if a relationship has been modified since a given timestamp.
    pub fn has_relationship_been_modified_since(
        &self,
        relationship_id: &RelationshipId,
        timestamp: DateTime<Utc>,
    ) -> Result<bool> {
        let versions_map = self
            .relationship_versions
            .read()
            .map_err(|e| MnemonicError::Transaction(format!("Read lock failed: {}", e)))?;

        if let Some(versions_vec) = versions_map.get(relationship_id) {
            // If the latest version's timestamp is after our check time, there is a conflict.
            if let Some(latest_version) = versions_vec.last() {
                // A modification can be a creation or a deletion.
                let last_mod_time = latest_version
                    .deleted_at
                    .unwrap_or(latest_version.created_at);
                return Ok(last_mod_time > timestamp);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::concept::ConceptData;
    use crate::types::relationship::RelationType;
    use crate::types::relationship::{Relationship, RelationshipVersion};

    #[test]
    fn test_version_time_travel() {
        let store = VersionStore::new();
        let concept_id = Uuid::new_v4();
        let txn_id = Uuid::new_v4();

        let t1 = Utc::now();
        std::thread::sleep(std::time::Duration::from_millis(10)); // wait a bit

        // Version 1 created at time T1
        let version1 = ConceptVersion {
            concept_id,
            version: 1,
            data: ConceptData::Structured("v1".to_string()),
            created_at: t1,
            created_by: txn_id,
            deleted_at: None,
            deleted_by: None,
        };
        store.add_concept_version(version1.clone()).unwrap();

        let t2 = Utc::now();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Version 2 created at time T2
        let version2 = ConceptVersion {
            concept_id,
            version: 2,
            data: ConceptData::Structured("v2".to_string()),
            created_at: t2,
            created_by: txn_id,
            deleted_at: None,
            deleted_by: None,
        };
        store.add_concept_version(version2.clone()).unwrap();

        // Check 1: Query for time T1 should give version 1
        let retrieved_v1 = store
            .get_concept_version_at_timestamp(&concept_id, t1)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved_v1, version1);

        // Check 2: Query for time T2 should give version 2
        let retrieved_v2 = store
            .get_concept_version_at_timestamp(&concept_id, t2)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved_v2, version2);

        // Check 3: Query for a time before anything existed should None
        let before_time = t1 - chrono::Duration::seconds(1);
        let nothing = store
            .get_concept_version_at_timestamp(&concept_id, before_time)
            .unwrap();
        assert!(nothing.is_none());
    }

    #[test]
    fn test_relationship_version_time_travel() {
        let store = VersionStore::new();

        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let txn_id_1 = Uuid::new_v4();
        let txn_id_2 = Uuid::new_v4();

        // --- 1. Create a relationship object ---
        let original_rel = Relationship::new(source_id, "knows".to_string(), target_id);
        let rel_id = original_rel.id;
        let t1 = original_rel.metadata.created_at;

        // --- 2. Create the FIRST version from it and store it ---
        let version1 = RelationshipVersion::from_relationship(&original_rel, txn_id_1);
        store.add_relationship_version(version1.clone()).unwrap();

        // --- 3. Create the SECOND (deleted) version ---
        // Let's pretend some time has passed
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = Utc::now();

        let version2 = RelationshipVersion {
            relationship_id: rel_id,
            version: 2, // It's a new version
            source: source_id,
            target: target_id,
            relationship_type: "knows".to_string(),
            created_at: t2,
            created_by: txn_id_2,
            deleted_at: Some(t2),
            deleted_by: Some(txn_id_2),
        };
        store.add_relationship_version(version2.clone()).unwrap();

        // --- 4. Now, assert our time-travel queries ---

        // At time T1, we should find the original, non-deleted version 1
        let retrieved_at_t1 = store
            .get_relationship_version_at_timestamp(&rel_id, t1)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved_at_t1.version, 1);
        assert!(retrieved_at_t1.deleted_at.is_none());

        // At time T2 (the moment of deletion), we should find NOTHING, because it's no longer active.
        let retrieved_at_t2 = store
            .get_relationship_version_at_timestamp(&rel_id, t2)
            .unwrap();
        assert!(retrieved_at_t2.is_none());
    }
}
