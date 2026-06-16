//! Error type for the hub daemon.

use std::fmt;

/// A hub error. Carries a human-readable message that maps to a JSON-RPC error
/// (code -32000) or a CLI diagnostic.
#[derive(Debug, Clone)]
pub struct HubError(pub String);

impl HubError {
    pub fn new(msg: impl Into<String>) -> Self {
        HubError(msg.into())
    }
}

impl fmt::Display for HubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HubError {}

impl From<std::io::Error> for HubError {
    fn from(e: std::io::Error) -> Self {
        HubError(e.to_string())
    }
}

impl From<serde_json::Error> for HubError {
    fn from(e: serde_json::Error) -> Self {
        HubError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, HubError>;
