//! Communication error types.

use thiserror::Error;

/// Result type for communication operations.
pub type CommResult<T> = Result<T, CommError>;

/// Errors that can occur in the communication system.
#[derive(Debug, Error)]
pub enum CommError {
    /// Agent not found
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Topic not found
    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    /// Message send failed
    #[error("Failed to send message: {0}")]
    SendFailed(String),

    /// Message receive failed
    #[error("Failed to receive message: {0}")]
    ReceiveFailed(String),

    /// Timeout waiting for response
    #[error("Timeout waiting for response")]
    Timeout,

    /// Channel closed
    #[error("Channel closed")]
    ChannelClosed,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}
