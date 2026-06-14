//! Error types for the mesh daemon.
//!
//! Library paths never `unwrap`/`panic` on fallible operations; they return
//! [`MeshError`]. The MCP layer maps these to JSON-RPC error objects.

use thiserror::Error;

/// All fallible operations in mesh return this error type.
#[derive(Debug, Error)]
pub enum MeshError {
    #[error("lmdb error: {0}")]
    Lmdb(#[from] heed::Error),

    #[error("journal error: {0}")]
    Journal(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("time formatting error: {0}")]
    TimeFormat(#[from] time::error::Format),

    #[error("invalid request: {0}")]
    BadRequest(String),

    /// A verb that does not exist on the contract was requested.
    #[error("unknown verb: {0}")]
    UnknownVerb(String),
}

/// Convenience alias for `Result<T, MeshError>`.
pub type Result<T> = std::result::Result<T, MeshError>;
