use crate::common::RealTestHarness;
use once_cell::sync::Lazy;
use std::sync::{Mutex, MutexGuard};

static E2E_HARNESS: Lazy<Mutex<RealTestHarness>> = Lazy::new(|| Mutex::new(RealTestHarness::new()));

pub fn shared_harness() -> MutexGuard<'static, RealTestHarness> {
    E2E_HARNESS.lock().expect("e2e harness lock poisoned")
}
