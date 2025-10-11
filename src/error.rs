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

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Transaction conflict: {0}")]
    TransactionConflict(String),

    #[error("Index error: {0}")]
    Index(String),
}

// This creates a handy shortcut for our functions.
// Instead of writing Result<String, MnemonicError>, we can just write Result<String>.
pub type Result<T> = std::result::Result<T, MnemonicError>;
