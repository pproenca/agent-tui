#![deny(clippy::all)]
#![allow(dead_code)]
#![cfg_attr(test, allow(clippy::expect_used))]

//! Use-case orchestration crate.

#[cfg(not(unix))]
compile_error!(
    "agent-tui is Unix-only. Supported environments are Linux, macOS, and other Unix-like systems with PTYs, Unix domain sockets, and POSIX signals."
);

pub mod usecases;
pub use usecases::*;

pub mod common {
    pub use agent_tui_common::common::*;
}

pub mod domain {
    pub use agent_tui_domain::domain::*;
}

#[cfg(test)]
pub(crate) mod test_support {
    pub use crate::usecases::ports::test_support::MockError;
    pub use crate::usecases::ports::test_support::MockSession;
    pub use crate::usecases::ports::test_support::MockSessionRepository;
}
