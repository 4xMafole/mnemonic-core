use crate::error::{MnemonicError, Result};
use crate::types::concept::ConceptVersion;
use crate::types::concept::*; //Import everything from the concept file
use crate::types::relationship::*;
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options, WriteBatch};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid; //Import everything from relationship file

// These are the names of our "filing cabinets" inside the database.
// This separates different kinds of data for better performance.

const CF_CONCEPTS: &str = "concepts";
const CF_RELATIONSHIPS: &str = "relationships";
const CF_INDICES: &str = "indices";
const CF_VERSIONS: &str = "versions";

/// RocksDB-based storage backend for Mnemonic
#[derive(Debug)]
pub struct RocksBackend {
    pub db: Arc<DB>, // Arc stands for 'Atomically Reference Counted'.
                     // It's a safe way to share the database connection across many threads.
}

impl RocksBackend {
    /// Create a new or open an existing RocksDB database with optimized settings.
    pub fn new(path: &Path) -> Result<Self> {
        // --- General Settings ---
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.increase_parallelism(num_cpus::get() as i32); // Use all available CPU cores

        // --- Our Filing Cabinets ---
        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_CONCEPTS, Options::default()),
            ColumnFamilyDescriptor::new(CF_RELATIONSHIPS, Options::default()),
            ColumnFamilyDescriptor::new(CF_INDICES, Options::default()),
            ColumnFamilyDescriptor::new(CF_VERSIONS, Options::default()),
        ];

        // --- Open the Database ---
        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Saves a concept to the database.
    pub fn store_concept(&self, concept: &Concept) -> Result<()> {
        //1. Get a "handle" to the 'concepts' filing cabinet.
        let cf = self.db.cf_handle(CF_CONCEPTS).unwrap();

        //2. Create a unique key for this concept. We'll use "concept:[UUID]".
        let key = format!("concept:{}", concept.id);

        //3. Convert our Rust struct into a sequence of bytes.
        let value = bincode::serialize(concept)?;

        //4. Put the key and value into the database.
        self.db.put_cf(cf, key, value)?;
        Ok(())
    }

    /// Retrieves a concept from the database by its ID.
    pub fn get_concept(&self, id: &ConceptId) -> Result<Option<Concept>> {
        let cf = self.db.cf_handle(CF_CONCEPTS).unwrap();
        let key = format!("concept:{}", id);

        //1. Ask the database for the value associated with our key.
        let result = self.db.get_cf(cf, key)?;

        //2. The result might be nothing (None) if the key wasn't found.
        match result {
            Some(data) => {
                //3. If we found data, convert the bytes back into a Concept struct.
                let concept = bincode::deserialize(&data)?;
                Ok(Some(concept))
            }
            None => {
                //4. If nothing was found, return Ok(None) to signal that.
                Ok(None)
            }
        }
    }

    /// Saves a relationship AND its index entries atomically.
    pub fn store_relationship(&self, relationship: &Relationship) -> Result<()> {
        let cf_rels = self.db.cf_handle(CF_RELATIONSHIPS).unwrap();
        let cf_indices = self.db.cf_handle(CF_INDICES).unwrap();

        let key = format!("rel:{}", relationship.id);
        let value = bincode::serialize(relationship)?;

        //We use a WriteBatch to make sure everything saves at once, or nothing does.
        let mut batch = WriteBatch::default();

        //Put the main relationship data in its cabinet.
        batch.put_cf(&cf_rels, key, &value);

        // We need to serialize the ID to store it as bytes in the value.
        let rel_id_bytes = bincode::serialize(&relationship.id)?;

        //Now, put the index entries in the 'indices' cabinet.

        // Index by source: key = "idx_src:[source_id]:[rel_id]" -> value = empty
        let source_key = format!("idx_src:{}:{}", relationship.source, relationship.id);
        batch.put_cf(&cf_indices, source_key, &rel_id_bytes);

        //Index by target: key = "idx_tgt:[target_id]:[rel_id]" -> value = empty
        let target_key = format!("idx_tgt:{}:{}", relationship.target, relationship.id);
        batch.put_cf(&cf_indices, target_key, &rel_id_bytes);

        //Now, write the entire batch to the database.
        self.db.write(batch)?;

        Ok(())
    }

    /// Retrieves a single relationship by its unique ID.
    pub fn get_relationship(&self, id: &RelationshipId) -> Result<Option<Relationship>> {
        let cf = self.db.cf_handle(CF_RELATIONSHIPS).unwrap();
        let key = format!("rel:{}", id);

        match self.db.get_cf(&cf, key)? {
            Some(data) => Ok(Some(bincode::deserialize(&data)?)),
            None => Ok(None),
        }
    }

    /// Finds all relationships that start from a given concept ID.
    // In src/storage/rocks_backend.rs

    pub fn get_relationships_by_source(&self, source_id: &ConceptId) -> Result<Vec<Relationship>> {
        let cf_indices = self.db.cf_handle(CF_INDICES).unwrap();
        let mut relationships = Vec::new();

        // The prefix to search for, e.g., "idx_src:[source_uuid]:"
        let prefix_str = format!("idx_src:{}:", source_id);
        let prefix_bytes = prefix_str.as_bytes();

        // Start an iterator at the beginning of our key range.
        let iter = self.db.iterator_cf(
            &cf_indices,
            rocksdb::IteratorMode::From(prefix_bytes, rocksdb::Direction::Forward),
        );

        for item in iter {
            let (key, value) = item?;

            // This is the CRUCIAL check. If the key no longer starts
            // with our prefix, we have processed all relevant records and must stop.
            if !key.starts_with(prefix_bytes) {
                break;
            }

            // The rest of the logic is the same: deserialize the value and fetch the full relationship.
            if let Ok(rel_id) = bincode::deserialize::<Uuid>(&value) {
                if let Some(rel) = self.get_relationship(&rel_id)? {
                    relationships.push(rel);
                }
            }
        }

        Ok(relationships)
    }

    /// Delete a relationship AND its index entries atomically.
    pub fn delete_ralationship(&self, id: &RelationshipId) -> Result<()> {
        let cf_rels = self.db.cf_handle(CF_RELATIONSHIPS).unwrap();
        let cf_indices = self.db.cf_handle(CF_INDICES).unwrap();

        // First, we need to get the relationship to know its source/target for index deletion.
        if let Some(rel) = self.get_relationship(id)? {
            let mut batch = WriteBatch::default();

            // Delete the main relationship data.
            batch.delete_cf(&cf_rels, format!("rel:{}", id));

            // Delete the index entries.
            batch.delete_cf(&cf_indices, format!("idx_src::{}:{}", rel.source, rel.id));
            batch.delete_cf(&cf_indices, format!("idx_tgt::{}:{}", rel.target, rel.id));

            self.db.write(batch)?;
        }

        Ok(())
    }

    /// Adds a `put` operation for a ConceptVersion to a WriteBatch.
    /// This is used by the TransactionManager to commit changes atomically.
    pub fn store_concept_version(
        &self,
        version: &ConceptVersion,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        let cf = self.db.cf_handle(CF_VERSIONS).unwrap();

        // We'll create a key like: "cv:{concept_id}:{version_number}"
        // This lets us easily look up all versions for a concept
        let key = format!("cv:{}:{}", version.concept_id, version.version);
        let value = bincode::serialize(version)?;

        batch.put_cf(&cf, key, value);
        Ok(())
    }

    /// Adds a 'put' operation for a RelationshipVersion to a WriteBatch.
    pub fn store_relationship_version(
        &self,
        version: &RelationshipVersion,
        batch: &mut WriteBatch,
    ) -> Result<()> {
        let cf = self.db.cf_handle(CF_VERSIONS).unwrap();

        // Key: "rv:{relationship_id}:{version_number}" (rv for Relationship Version)
        let key = format!("rv:{}:{}", version.relationship_id, version.version);
        let value = bincode::serialize(version)?;

        batch.put_cf(&cf, key, value);

        Ok(())
    }

    /// Loads all concept versions from the database.
    /// This is used to "hydrate" the in-memory VersionStore on startup.
    pub fn load_all_concept_versions(&self) -> Result<Vec<ConceptVersion>> {
        let cf = self.db.cf_handle(CF_VERSIONS).unwrap();

        // Create an iterator that scans the entire 'versions' column family.
        let mut iter = self.db.iterator_cf(&cf, IteratorMode::Start);
        let mut versions = Vec::new();

        while let Some(result) = iter.next() {
            match result {
                Ok((_key, value)) => {
                    // For each record found, deserialize the value back into a ConceptVersion.
                    if let Ok(version) = bincode::deserialize(&value) {
                        versions.push(version);
                    }
                    // In real code, we'd log deserialization errors. For now, we just skip them.
                }
                Err(e) => return Err(MnemonicError::Storage(e)),
            }
        }

        Ok(versions)
    }
    /// Loads all relationship versions from the database.
    /// This is used to "hydrate" the in-memory VersionStore on startup.
pub fn load_all_relationship_versions(&self) -> Result<Vec<RelationshipVersion>> {
    let cf = self.db.cf_handle(CF_VERSIONS).unwrap();
    let mut versions = Vec::new();
    // Use a prefix iterator to only scan for "rv:" (Relationship Version) keys
    let iter = self.db.prefix_iterator_cf(&cf, "rv:");

    for item in iter {
        let (_key, value) = item?;
        if let Ok(version) = bincode::deserialize::<RelationshipVersion>(&value) {
            versions.push(version);
        }
    }
    Ok(versions)
}
}
