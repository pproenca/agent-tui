//! Signal handling for graceful daemon shutdown.
//!
//! This module sets up signal handlers for SIGINT and SIGTERM to trigger
//! graceful shutdown of the daemon.

use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use tracing::info;

use crate::error::DaemonError;

/// A signal handler that sets a shutdown flag when SIGINT or SIGTERM is received.
pub struct SignalHandler {
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

impl SignalHandler {
    /// Set up signal handling for graceful shutdown.
    ///
    /// When SIGINT or SIGTERM is received, sets `shutdown` to true.
    /// Returns a handle that can be used for cleanup (though typically
    /// the daemon exits before needing to join the thread).
    pub fn setup(shutdown: Arc<AtomicBool>) -> Result<Self, DaemonError> {
        let mut signals =
            Signals::new([SIGINT, SIGTERM]).map_err(|e| DaemonError::SignalSetup(e.to_string()))?;

        let handle = thread::Builder::new()
            .name("signal-handler".to_string())
            .spawn(move || {
                if let Some(sig) = signals.forever().next() {
                    info!(
                        signal = sig,
                        "Received signal, initiating graceful shutdown"
                    );
                    shutdown.store(true, Ordering::SeqCst);
                }
            })
            .map_err(|e| {
                DaemonError::SignalSetup(format!("failed to spawn signal handler: {}", e))
            })?;

        Ok(Self { handle })
    }
}
