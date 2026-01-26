use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use tracing::info;

use crate::infra::daemon::DaemonError;
use crate::usecases::ports::ShutdownNotifierHandle;

pub struct SignalHandler {
    #[allow(dead_code)]
    handle: JoinHandle<()>,
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
                if let Some(sig) = signals.forever().next() {
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

        Ok(Self { handle })
    }
}
