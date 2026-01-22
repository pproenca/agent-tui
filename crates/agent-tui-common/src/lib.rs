#![deny(clippy::all)]

mod color;
mod json_ext;
mod sync;

pub use color::Colors;
pub use color::init as color_init;
pub use color::is_disabled as color_is_disabled;
pub use json_ext::ValueExt;
pub use sync::mutex_lock_or_recover;
pub use sync::poison_recovery_count;
pub use sync::rwlock_read_or_recover;
pub use sync::rwlock_write_or_recover;
