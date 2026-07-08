//! JSON-RPC request identifier shared by transports and protocol adapters.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize
)]
#[serde(untagged)]
pub enum RpcId {
    String(String),
    Integer(i64),
}

impl RpcId {
    pub fn integer(value: u64) -> Self {
        i64::try_from(value).map_or_else(|_| Self::String(value.to_string()), Self::Integer)
    }
}

impl From<u64> for RpcId {
    fn from(value: u64) -> Self {
        Self::integer(value)
    }
}

impl From<u32> for RpcId {
    fn from(value: u32) -> Self {
        Self::Integer(i64::from(value))
    }
}

impl From<i64> for RpcId {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<i32> for RpcId {
    fn from(value: i32) -> Self {
        Self::Integer(i64::from(value))
    }
}

impl From<String> for RpcId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for RpcId {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<&RpcId> for RpcId {
    fn from(value: &RpcId) -> Self {
        value.clone()
    }
}

impl fmt::Display for RpcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(value) => f.write_str(value),
            Self::Integer(value) => write!(f, "{value}"),
        }
    }
}
