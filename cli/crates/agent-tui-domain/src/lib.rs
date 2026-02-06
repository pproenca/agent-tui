#![deny(clippy::all)]
#![allow(dead_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

//! Domain layer crate.

pub mod domain;
pub use domain::*;
