use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::error;

static POISON_RECOVERY_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn poison_recovery_count() -> u64 {
    POISON_RECOVERY_COUNT.load(Ordering::Relaxed)
}

fn record_poison_recovery() {
    POISON_RECOVERY_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn rwlock_read_or_recover<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| {
        record_poison_recovery();
        error!(
            "ERROR: RwLock poisoned (read) - a thread panicked while holding this lock. \
             Data may be inconsistent. Attempting recovery."
        );
        poisoned.into_inner()
    })
}

pub fn rwlock_write_or_recover<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|poisoned| {
        record_poison_recovery();
        error!(
            "ERROR: RwLock poisoned (write) - a thread panicked while holding this lock. \
             Data may be inconsistent. Attempting recovery."
        );
        poisoned.into_inner()
    })
}

pub fn mutex_lock_or_recover<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| {
        record_poison_recovery();
        error!(
            "ERROR: Mutex poisoned - a thread panicked while holding this lock. \
             Data may be inconsistent. Attempting recovery."
        );
        poisoned.into_inner()
    })
}
