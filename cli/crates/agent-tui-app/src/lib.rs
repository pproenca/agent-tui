#![deny(clippy::all)]
#![allow(dead_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

//! Application composition and command handling crate.

pub mod app;
pub use app::*;

pub mod common {
    pub use agent_tui_common::common::*;
}

pub mod domain {
    pub use agent_tui_domain::domain::*;
}

pub mod usecases {
    pub use agent_tui_usecases::usecases::*;
}

pub mod adapters {
    pub use agent_tui_adapters::adapters::*;
}

pub mod infra {
    pub use agent_tui_infra::infra::*;
}

#[cfg(test)]
pub(crate) mod test_support;
