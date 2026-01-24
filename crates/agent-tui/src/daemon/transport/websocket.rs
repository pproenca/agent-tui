//! WebSocket transport implementation (stub for future UI integration)
//!
//! This module will provide WebSocket support for browser-based UI clients.
//! The transport abstraction allows the daemon to serve both Unix socket
//! (CLI) and WebSocket (UI) clients through the same handler infrastructure.
//!
//! # Future Implementation Requirements
//!
//! - Use `tokio-tungstenite` or similar async WebSocket library
//! - Support binary (MessagePack) and text (JSON) message formats
//! - Handle WebSocket handshake and upgrade from HTTP
//! - Implement ping/pong for connection keepalive
//! - Support TLS for secure connections (wss://)
//!
//! # Architecture Notes
//!
//! The WebSocket transport will need to bridge async WebSocket I/O with
//! the synchronous `TransportConnection` trait. Options:
//! 1. Run WebSocket listener in separate tokio runtime
//! 2. Use blocking wrapper around async operations
//! 3. Extend traits to support async (breaking change)

/// WebSocket connection placeholder.
///
/// Will implement `TransportConnection` trait when WebSocket support is added.
#[allow(dead_code)]
pub struct WebSocketConnection {
    _private: (),
}

/// WebSocket listener placeholder.
///
/// Will implement `TransportListener` trait when WebSocket support is added.
#[allow(dead_code)]
pub struct WebSocketListener {
    _private: (),
}
