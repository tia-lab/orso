//! Utility functions for ORSO

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Utility functions for ORSO
#[derive(Debug, Clone)]
pub struct Utils;

impl Utils {
    pub fn generate_id() -> Option<String> {
        Some(Uuid::new_v4().to_string())
    }

    pub fn current_timestamp() -> Option<DateTime<Utc>> {
        Some(Utc::now())
    }

    pub fn create_timestamp(timestamp: DateTime<Utc>) -> String {
        timestamp.to_rfc3339()
    }

    pub fn parse_timestamp(timestamp: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
        DateTime::parse_from_rfc3339(timestamp).map(|dt| dt.with_timezone(&Utc))
    }

    /// Convert DateTime to Unix timestamp (seconds since epoch)
    pub fn datetime_to_unix_timestamp(dt: &DateTime<Utc>) -> i64 {
        dt.timestamp()
    }

    /// Convert Unix timestamp (seconds since epoch) to DateTime
    pub fn unix_timestamp_to_datetime(timestamp: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(timestamp, 0).unwrap_or_else(|| Utc::now())
    }

    /// Convert DateTime to Unix timestamp with milliseconds
    pub fn datetime_to_unix_timestamp_millis(dt: &DateTime<Utc>) -> i64 {
        dt.timestamp_millis()
    }

    /// Convert Unix timestamp with milliseconds to DateTime
    pub fn unix_timestamp_millis_to_datetime(timestamp: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(timestamp).unwrap_or_else(|| Utc::now())
    }
}
impl Utils {
    pub fn value_to_libsql_value(value: &crate::Value) -> libsql::Value {
        match value {
            crate::Value::Null => libsql::Value::Null,
            crate::Value::Integer(i) => libsql::Value::Integer(*i),
            crate::Value::Real(f) => libsql::Value::Real(*f),
            crate::Value::Text(s) => libsql::Value::Text(s.clone()),
            crate::Value::Blob(b) => libsql::Value::Blob(b.clone()),
            crate::Value::Boolean(b) => libsql::Value::Integer(if *b { 1 } else { 0 }),
        }
    }

    /// Convert libsql::Value to our Value type
    pub fn libsql_value_to_value(value: &libsql::Value) -> crate::Value {
        match value {
            libsql::Value::Null => crate::Value::Null,
            libsql::Value::Integer(i) => crate::Value::Integer(*i),
            libsql::Value::Real(f) => crate::Value::Real(*f),
            libsql::Value::Text(s) => crate::Value::Text(s.clone()),
            libsql::Value::Blob(b) => crate::Value::Blob(b.clone()),
        }
    }
}
