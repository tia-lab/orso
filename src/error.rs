// Error handling

use std::fmt;

// Custom error type
#[derive(Debug)]
pub enum Error {
    /// Database connection error
    Connection(libsql::Error),
    /// SQL execution error
    Sql(String),
    /// Serialization/deserialization error
    Serialization(String),
    /// Validation error
    Validation(String),
    /// Not found error
    NotFound(String),
    /// Pagination error
    Pagination(String),
    /// Query building error
    Query(String),
    /// Worker environment error
    AnyhowError(String),
    /// Database error
    DatabaseError(String),
    /// Generic error
    Generic(String),
    /// Configuration error
    Config(String),
    /// Operations error
    Operations(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "Connection error: {msg}"),
            Error::Sql(msg) => write!(f, "SQL error: {msg}"),
            Error::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            Error::Validation(msg) => write!(f, "Validation error: {msg}"),
            Error::NotFound(msg) => write!(f, "Not found: {msg}"),
            Error::Pagination(msg) => write!(f, "Pagination error: {msg}"),
            Error::Query(msg) => write!(f, "Query error: {msg}"),
            Error::AnyhowError(msg) => write!(f, "Anyhow error: {msg}"),
            Error::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            Error::Generic(msg) => write!(f, "Error: {msg}"),
            Error::Config(msg) => write!(f, "Configuration error: {}", msg),
            Error::Operations(msg) => write!(f, "Operations error: {}", msg),
        }
    }
}

impl From<libsql::Error> for Error {
    fn from(err: libsql::Error) -> Self {
        Error::Sql(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Generic(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::Generic(err.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::AnyhowError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
