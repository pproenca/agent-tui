//! Extension traits for serde_json::Value to reduce boilerplate in handlers.

use serde_json::Value;

/// Extension trait for convenient JSON value extraction with defaults.
pub trait ValueExt {
    /// Get a string field or return default.
    fn str_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str;

    /// Get a u64 field or return default.
    fn u64_or(&self, key: &str, default: u64) -> u64;

    /// Get a bool field or return default.
    fn bool_or(&self, key: &str, default: bool) -> bool;

    /// Get an array field's string elements joined by a separator.
    fn str_array_join(&self, key: &str, sep: &str) -> String;
}

impl ValueExt for Value {
    fn str_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).and_then(|v| v.as_str()).unwrap_or(default)
    }

    fn u64_or(&self, key: &str, default: u64) -> u64 {
        self.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
    }

    fn bool_or(&self, key: &str, default: bool) -> bool {
        self.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    fn str_array_join(&self, key: &str, sep: &str) -> String {
        self.get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(sep)
            })
            .unwrap_or_default()
    }
}
