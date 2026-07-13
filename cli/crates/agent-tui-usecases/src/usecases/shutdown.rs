//! Shutdown use case.

use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::domain::ShutdownInput;
use crate::usecases::ports::ShutdownNotifier;

pub fn shutdown<N: ShutdownNotifier + ?Sized>(
    shutdown_flag: &AtomicBool,
    notifier: &N,
    _input: ShutdownInput,
) -> Result<(), io::Error> {
    shutdown_flag.store(true, Ordering::SeqCst);
    notifier.notify()?;

    Ok(())
}

#[cfg(test)]
#[path = "shutdown_tests.rs"]
mod tests;
