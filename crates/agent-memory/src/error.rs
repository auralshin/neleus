//! Error types for the memory system.

use thiserror::Error;

/// Result type for memory operations.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Errors that can occur in the memory system.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Entry not found
    #[error("Memory entry not found: {0}")]
    NotFound(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// SQLite error
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Vector operation error
    #[error("Vector operation error: {0}")]
    Vector(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Memory expired
    #[error("Memory entry has expired: {0}")]
    Expired(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
