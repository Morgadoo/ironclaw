//! Error types for Project E core operations.

use thiserror::Error;

/// Top-level error type for Project E operations.
#[derive(Debug, Error)]
pub enum ProjectEError {
    #[error("configuration error: {reason}")]
    Config { reason: String },

    #[error("module error: {reason}")]
    Module { reason: String },

    #[error("event bus error: {reason}")]
    EventBus { reason: String },

    #[error("circuit breaker open for provider '{provider}'")]
    CircuitOpen { provider: String },

    #[error("LLM provider error: {reason}")]
    Llm { reason: String },

    #[error("database error: {reason}")]
    Database { reason: String },

    #[error("forge error: {reason}")]
    Forge { reason: String },

    #[error("perception error: {reason}")]
    Perception { reason: String },

    #[error("analysis error: {reason}")]
    Analysis { reason: String },

    #[error("metacognition error: {reason}")]
    Metacognition { reason: String },

    #[error("registry error: {reason}")]
    Registry { reason: String },

    #[error("serialization error: {reason}")]
    Serialization { reason: String },

    #[error("I/O error: {reason}")]
    Io { reason: String },

    #[error("timeout after {duration_secs}s")]
    Timeout { duration_secs: u64 },
}

impl From<serde_json::Error> for ProjectEError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization {
            reason: e.to_string(),
        }
    }
}

impl From<std::io::Error> for ProjectEError {
    fn from(e: std::io::Error) -> Self {
        Self::Io {
            reason: e.to_string(),
        }
    }
}

impl From<toml::de::Error> for ProjectEError {
    fn from(e: toml::de::Error) -> Self {
        Self::Config {
            reason: e.to_string(),
        }
    }
}

/// Convenience result type for Project E operations.
pub type Result<T> = std::result::Result<T, ProjectEError>;
