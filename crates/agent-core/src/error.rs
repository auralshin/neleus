//! Error types for agent-core.

use thiserror::Error;

/// Result type for agent operations.
pub type AgentResult<T> = Result<T, AgentError>;

/// Errors that can occur in agent operations.
#[derive(Debug, Error)]
pub enum AgentError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool execution failed
    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    /// Formatting error
    #[error("Formatting error: {0}")]
    Formatting(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Memory error
    #[error("Memory error: {0}")]
    Memory(#[from] agent_memory::MemoryError),

    /// Communication error
    #[error("Communication error: {0}")]
    Communication(#[from] agent_comm::CommError),
}
