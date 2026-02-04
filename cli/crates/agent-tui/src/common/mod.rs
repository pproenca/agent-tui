#![deny(clippy::all)]

mod color;
pub mod daemon_error;
pub mod error_codes;
mod string_utils;
mod sync;
pub mod telemetry;

pub use color::Colors;
pub use color::init as color_init;
pub use daemon_error::DaemonError;
pub use string_utils::strip_ansi_codes;
pub use sync::mutex_lock_or_recover;
pub use sync::rwlock_read_or_recover;
pub use sync::rwlock_write_or_recover;
