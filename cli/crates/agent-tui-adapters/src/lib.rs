#![deny(clippy::all)]
#![allow(dead_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

//! Interface adapters crate.

pub mod adapters;
pub use adapters::*;

pub mod common {
    pub use agent_tui_common::common::*;
}

pub mod domain {
    pub use agent_tui_domain::domain::*;
}

pub mod usecases {
    pub use agent_tui_usecases::usecases::*;
}
