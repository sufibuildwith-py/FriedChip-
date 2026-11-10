use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Binary parsing error: {0}")]
    Binary(String),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("File already imported: {0}")]
    Duplicate(String),

    #[error("Incompatible platform: {0}")]
    IncompatiblePlatform(String),

    #[error("Operation cancelled: {0}")]
    Cancelled(String),

    #[error("System error: {0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
