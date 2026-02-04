#![deny(clippy::all)]
// CLI-only crate: keep internal API surfaces without dead_code noise.
#![allow(dead_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod adapters;
mod app;
mod common;
mod domain;
mod infra;
mod usecases;

pub use app::Application;
