//! Daemon signal handling.

use signal_hook::consts::SIGINT;
use signal_hook::consts::SIGTERM;
use signal_hook::iterator::Signals;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::JoinHandle;
use tracing::info;

use crate::common::DaemonError;
use crate::usecases::ports::ShutdownNotifierHandle;

pub struct SignalHandler {
    _handle: JoinHandle<()>,
}

impl SignalHandler {
    pub fn setup(
        shutdown: Arc<AtomicBool>,
        notifier: Option<ShutdownNotifierHandle>,
    ) -> Result<Self, DaemonError> {
        let mut signals =
            Signals::new([SIGINT, SIGTERM]).map_err(|e| DaemonError::SignalSetup(e.to_string()))?;

        let handle = thread::Builder::new()
            .name("signal-handler".to_string())
            .spawn(move || {
                let notifier = notifier;
                let mut shutdown_initiated = false;
                for sig in signals.forever() {
                    if shutdown_initiated {
                        info!(signal = sig, "Received second signal, forcing shutdown");
                        shutdown.store(true, Ordering::SeqCst);
                        if let Some(notifier) = notifier.as_ref() {
                            notifier.notify();
                        }
                        continue;
                    }
                    shutdown_initiated = true;
                    info!(
                        signal = sig,
                        "Received signal, initiating graceful shutdown"
                    );
                    shutdown.store(true, Ordering::SeqCst);
                    if let Some(notifier) = notifier.as_ref() {
                        notifier.notify();
                    }
                }
            })
            .map_err(|e| {
                DaemonError::SignalSetup(format!("failed to spawn signal handler: {}", e))
            })?;

        Ok(Self { _handle: handle })
    }
}
