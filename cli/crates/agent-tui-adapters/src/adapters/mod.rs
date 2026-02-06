//! Interface adapters that translate external formats into use-case inputs.

pub mod daemon;
pub mod presenter;
pub mod rpc;
mod rpc_value;
mod snapshot_adapters;
mod value_ext;

pub use rpc::*;
pub use rpc_value::RpcValue;
pub use rpc_value::RpcValueRef;
pub use value_ext::ValueExt;
