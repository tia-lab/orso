pub struct Utils;
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
