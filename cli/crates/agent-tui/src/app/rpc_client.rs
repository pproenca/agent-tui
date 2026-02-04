//! RPC client helpers.

use serde::Serialize;

use crate::adapters::RpcValue;
use crate::adapters::rpc::to_value;
use crate::adapters::rpc::to_value_opt;
use crate::infra::ipc::ClientError;
use crate::infra::ipc::client::DaemonClient;
use crate::infra::ipc::client::StreamAbortHandle;
use crate::infra::ipc::client::StreamResponse;

pub(crate) struct RpcStream {
    inner: StreamResponse,
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

    pub fn abort_handle(&self) -> Option<StreamAbortHandle> {
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
    let value = to_value(params)?;
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
    let value = to_value_opt(params)?;
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
    let value = to_value(params)?;
    client.call_stream(method, Some(value)).map(RpcStream::new)
}
