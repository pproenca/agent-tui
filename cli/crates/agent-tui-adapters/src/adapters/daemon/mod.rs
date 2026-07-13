//! Daemon-facing adapters (HTTP/RPC routing and controller wiring).

pub mod error;
pub mod handlers;
pub mod router;

pub use error::DomainError;
pub use router::Router;
