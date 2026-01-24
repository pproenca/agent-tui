use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use tracing::info;

use crate::daemon::error::DaemonError;

pub struct SignalHandler {
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

impl SignalHandler {
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
