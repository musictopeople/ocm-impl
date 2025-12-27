use thiserror::Error;

#[derive(Error, Debug)]
pub enum OcmError {
    #[cfg(feature = "native")]
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[cfg(feature = "native")]
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    DatabaseGeneric(String),

    #[error("Network error: {0}")]
    NetworkGeneric(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cryptography error: {0}")]
    Cryptography(String),

    #[error("PLC error: {0}")]
    Plc(String),

    #[error("CRDT error: {0}")]
    Crdt(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl From<ed25519_dalek::SignatureError> for OcmError {
    fn from(err: ed25519_dalek::SignatureError) -> Self {
        OcmError::Cryptography(format!("Signature error: {}", err))
    }
}

impl From<base64::DecodeError> for OcmError {
    fn from(err: base64::DecodeError) -> Self {
        OcmError::Cryptography(format!("Base64 decode error: {}", err))
    }
}

impl From<Box<dyn std::error::Error>> for OcmError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        OcmError::OperationFailed(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, OcmError>;
