#![deny(clippy::all)]

//! Domain layer crate.

#[cfg(not(unix))]
compile_error!(
    "agent-tui is Unix-only. Supported environments are Linux, macOS, and other Unix-like systems with PTYs, Unix domain sockets, and POSIX signals."
);

pub mod domain;
pub use domain::*;
