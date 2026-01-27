use serde::Serialize;
use serde_json::Value;

use crate::adapters::ValueExt;
use crate::adapters::ipc::client::{DaemonClient, StreamResponse};
use crate::adapters::ipc::error::ClientError;

#[derive(Clone, Debug)]
pub struct RpcValue(Value);

#[derive(Clone, Copy, Debug)]
pub struct RpcValueRef<'a>(&'a Value);

#[derive(Clone, Copy, Debug)]
pub struct RpcArrayRef<'a>(&'a [Value]);

pub struct RpcStream {
    inner: StreamResponse,
}

impl RpcValue {
    pub fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> Value {
        self.0
    }

    pub fn as_ref(&self) -> RpcValueRef<'_> {
        RpcValueRef(&self.0)
    }

    pub fn get(&self, key: &str) -> Option<RpcValueRef<'_>> {
        self.0.get(key).map(RpcValueRef)
    }

    pub fn as_array(&self) -> Option<RpcArrayRef<'_>> {
        self.0.as_array().map(|arr| RpcArrayRef(arr))
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
        serde_json::to_string_pretty(&self.0).unwrap_or_default()
    }

    pub fn str_array_join(&self, key: &str, sep: &str) -> String {
        self.0
            .get(key)
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

    pub fn str_array_join(&self, key: &str, sep: &str) -> String {
        self.0
            .get(key)
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

impl<'a> RpcArrayRef<'a> {
    pub fn iter(self) -> impl Iterator<Item = RpcValueRef<'a>> {
        self.0.iter().map(RpcValueRef)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn first(self) -> Option<RpcValueRef<'a>> {
        self.0.first().map(RpcValueRef)
    }
}

impl RpcStream {
    pub fn new(inner: StreamResponse) -> Self {
        Self { inner }
    }

    pub fn next_result(&mut self) -> Result<Option<RpcValue>, ClientError> {
        self.inner
            .next_result()
            .map(|value| value.map(RpcValue::new))
    }

    pub fn abort_handle(&self) -> Option<crate::adapters::ipc::client::StreamAbortHandle> {
        self.inner.abort_handle()
    }
}

pub fn call_with_params<C, P>(
    client: &mut C,
    method: &str,
    params: P,
) -> Result<RpcValue, ClientError>
where
    C: DaemonClient,
    P: Serialize,
{
    let value = serde_json::to_value(params)?;
    client.call(method, Some(value)).map(RpcValue::new)
}

pub fn call_with_optional_params<C, P>(
    client: &mut C,
    method: &str,
    params: Option<P>,
) -> Result<RpcValue, ClientError>
where
    C: DaemonClient,
    P: Serialize,
{
    let value = params.map(serde_json::to_value).transpose()?;
    client.call(method, value).map(RpcValue::new)
}

pub fn call_no_params<C>(client: &mut C, method: &str) -> Result<RpcValue, ClientError>
where
    C: DaemonClient,
{
    client.call(method, None).map(RpcValue::new)
}

pub fn call_stream_with_params<C, P>(
    client: &mut C,
    method: &str,
    params: P,
) -> Result<RpcStream, ClientError>
where
    C: DaemonClient,
    P: Serialize,
{
    let value = serde_json::to_value(params)?;
    client.call_stream(method, Some(value)).map(RpcStream::new)
}
