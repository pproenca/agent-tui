//! Shutdown notifier port.

use std::io;
use std::sync::Arc;

pub trait ShutdownNotifier: Send + Sync {
    fn notify(&self) -> Result<(), io::Error>;
}

#[derive(Default)]
pub struct NoopShutdownNotifier;

impl ShutdownNotifier for NoopShutdownNotifier {
    fn notify(&self) -> Result<(), io::Error> {
        Ok(())
    }
}

pub type ShutdownNotifierHandle = Arc<dyn ShutdownNotifier>;
