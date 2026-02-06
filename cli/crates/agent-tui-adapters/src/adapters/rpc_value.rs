//! RPC value wrappers and helpers.

use serde::Serialize;
use serde_json::Value;
use tracing::debug;

use crate::adapters::ValueExt;

#[derive(Clone, Debug)]
pub struct RpcValue(Value);

#[derive(Clone, Copy, Debug)]
pub struct RpcValueRef<'a>(&'a Value);

#[derive(Clone, Copy, Debug)]
pub struct RpcArrayRef<'a>(&'a [Value]);

impl RpcValue {
    pub fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn as_ref(&self) -> RpcValueRef<'_> {
        RpcValueRef(&self.0)
    }

    pub fn get(&self, key: &str) -> Option<RpcValueRef<'_>> {
        self.0.get(key).map(RpcValueRef)
    }

    pub fn str_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.0.str_or(key, default)
    }

    pub fn u64_or(&self, key: &str, default: u64) -> u64 {
        self.0.u64_or(key, default)
    }

    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        self.0.bool_or(key, default)
    }

    pub fn to_pretty_json(&self) -> String {
        serde_json::to_string_pretty(&self.0).unwrap_or_else(|err| {
            debug!(error = %err, "Failed to serialize RPC value to JSON");
            String::new()
        })
    }
}

impl Serialize for RpcValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'a> Serialize for RpcValueRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'a> RpcValueRef<'a> {
    pub fn get(&self, key: &str) -> Option<RpcValueRef<'a>> {
        self.0.get(key).map(RpcValueRef)
    }

    pub fn as_str(&self) -> Option<&'a str> {
        self.0.as_str()
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.0.as_u64()
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.0.as_bool()
    }

    pub fn as_array(&self) -> Option<RpcArrayRef<'a>> {
        self.0.as_array().map(|arr| RpcArrayRef(arr))
    }

    pub fn str_or(&self, key: &str, default: &'a str) -> &'a str {
        self.0.str_or(key, default)
    }

    pub fn u64_or(&self, key: &str, default: u64) -> u64 {
        self.0.u64_or(key, default)
    }

    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        self.0.bool_or(key, default)
    }
}

impl<'a> RpcArrayRef<'a> {
    pub fn iter(self) -> impl Iterator<Item = RpcValueRef<'a>> {
        self.0.iter().map(RpcValueRef)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
