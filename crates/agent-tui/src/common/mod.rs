//! Common utilities and types shared across agent-tui crates.
//!
//! Provides error codes, color handling, JSON extensions, string utilities,
//! key name validation, and synchronization utilities.

#![deny(clippy::all)]

mod color;
pub mod error_codes;
mod json_ext;
pub mod key_names;
mod string_utils;
mod sync;

pub use color::Colors;
pub use color::init as color_init;
pub use color::is_disabled as color_is_disabled;
pub use json_ext::ValueExt;
pub use string_utils::strip_ansi_codes;
pub use sync::mutex_lock_or_recover;
pub use sync::poison_recovery_count;
pub use sync::rwlock_read_or_recover;
pub use sync::rwlock_write_or_recover;
