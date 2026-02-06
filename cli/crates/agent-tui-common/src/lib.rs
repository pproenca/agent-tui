#![deny(clippy::all)]
#![allow(dead_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

//! Shared utilities used across architecture layers.

pub mod common;
pub use common::*;
