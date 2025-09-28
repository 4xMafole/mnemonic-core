use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum MnemonicError {
    #[error("Storage error: {0}")]
    Storage(#[from] rocksdb::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] Box<bincode::ErrorKind>),

    #[error("Concept not found: {0}")]
    ConceptNotFound(Uuid),
    
    #[error("Relationship not found: {0}")]
    RelationshipNotFound(Uuid),

    // ... we can add more specific errors later
}

// This creates a handy shortcut for our functions.
// Instead of writing Result<String, MnemonicError>, we can just write Result<String>.
pub type Result<T> = std::result::Result<T, MnemonicError>;