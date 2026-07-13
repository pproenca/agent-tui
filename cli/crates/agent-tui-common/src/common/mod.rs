#![deny(clippy::all)]

//! Shared utilities used across layers without domain logic.

pub mod color;
pub mod daemon_error;
pub mod error_codes;
mod rpc_id;
mod string_utils;
mod sync;
pub mod telemetry;
mod thread_join;

pub use color::init as color_init;
pub use daemon_error::DaemonError;
pub use rpc_id::RpcId;
pub use string_utils::strip_ansi_codes;
pub use sync::mutex_lock_or_recover;
pub use sync::rwlock_read_or_recover;
pub use sync::rwlock_write_or_recover;
pub use thread_join::ThreadJoinOutcome;
pub use thread_join::join_thread_and_warn_on_panic;
pub use thread_join::join_thread_with_timeout_or_reap;
pub use thread_join::join_thread_with_timeout_or_reap_with_poll_interval;
