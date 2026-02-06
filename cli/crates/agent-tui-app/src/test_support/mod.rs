//! Shared test support utilities for the crate.
#![allow(unused_imports)]

use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

pub use crate::infra::ipc::MockClient;
pub use crate::infra::ipc::process::mock::MockProcessController;
pub use crate::usecases::ports::test_support::MockError;
pub use crate::usecases::ports::test_support::MockSession;
pub use crate::usecases::ports::test_support::MockSessionRepository;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock poisoned")
}
