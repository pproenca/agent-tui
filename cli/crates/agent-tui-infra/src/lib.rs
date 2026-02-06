#![deny(clippy::all)]
#![allow(dead_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

//! Infrastructure adapters crate.

pub mod infra;
pub use infra::*;

pub mod common {
    pub use agent_tui_common::common::*;
}

pub mod domain {
    pub use agent_tui_domain::domain::*;
}

pub mod usecases {
    pub use agent_tui_usecases::usecases::*;
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;
    use std::sync::MutexGuard;
    use std::sync::OnceLock;

    pub use crate::infra::ipc::process::mock::MockProcessController;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock poisoned")
    }
}
