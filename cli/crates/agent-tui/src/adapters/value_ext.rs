//! serde_json::Value extensions.

use serde_json::Value;

pub trait ValueExt {
    fn str_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str;
    fn u64_or(&self, key: &str, default: u64) -> u64;
    fn bool_or(&self, key: &str, default: bool) -> bool;
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
