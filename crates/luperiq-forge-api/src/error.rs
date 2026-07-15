use std::fmt;

/// Unified error type for all platform abstractions.
#[derive(Debug)]
pub enum PlatformError {
    /// Underlying journal / storage error.
    Storage(String),
    /// Serialization or deserialization failure.
    Serialization(String),
    /// Requested entity was not found.
    NotFound(String),
    /// Caller lacks required authorization.
    Unauthorized(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::Storage(e) => write!(f, "storage error: {e}"),
            PlatformError::Serialization(msg) => write!(f, "serialization error: {msg}"),
            PlatformError::NotFound(msg) => write!(f, "not found: {msg}"),
            PlatformError::Unauthorized(msg) => write!(f, "unauthorized: {msg}"),
        }
    }
}

impl std::error::Error for PlatformError {}
