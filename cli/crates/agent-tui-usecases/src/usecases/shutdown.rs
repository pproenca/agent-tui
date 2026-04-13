//! Shutdown use case.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::domain::ShutdownInput;
use crate::domain::ShutdownOutput;
use crate::usecases::ports::ShutdownNotifier;

pub trait ShutdownUseCase: Send + Sync {
    fn execute(&self, input: ShutdownInput) -> ShutdownOutput;
}

pub struct ShutdownUseCaseImpl {
    shutdown_flag: Arc<AtomicBool>,
    notifier: Arc<dyn ShutdownNotifier>,
}

impl ShutdownUseCaseImpl {
    pub fn new(shutdown_flag: Arc<AtomicBool>, notifier: Arc<dyn ShutdownNotifier>) -> Self {
        Self {
            shutdown_flag,
            notifier,
        }
    }
}

impl ShutdownUseCase for ShutdownUseCaseImpl {
    fn execute(&self, _input: ShutdownInput) -> ShutdownOutput {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        let acknowledged = self.notifier.notify().is_ok();

        ShutdownOutput { acknowledged }
    }
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
