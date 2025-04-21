use std::io;

use thiserror::Error;

#[derive(Debug, Error, uniffi::Error)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("CBOR serialization error: {0}")]
    Cbor(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

impl From<io::Error> for DatabaseError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<redb::Error> for DatabaseError {
    fn from(error: redb::Error) -> Self {
        Self::Database(error.to_string())
    }
}

impl From<serde_json::Error> for DatabaseError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error.to_string())
    }
}