use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

/// Acquires a read lock, recovering from poison if a thread panicked while holding it.
///
/// # Warning
/// If this function recovers from a poisoned lock, it means another thread panicked
/// while holding the lock. The data may be in an inconsistent state. This recovery
/// is intentional to allow the daemon to continue operating, but errors should be
/// investigated.
pub fn rwlock_read_or_recover<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| {
        eprintln!(
            "ERROR: RwLock poisoned (read) - a thread panicked while holding this lock. \
             Data may be inconsistent. Attempting recovery."
        );
        poisoned.into_inner()
    })
}

/// Acquires a write lock, recovering from poison if a thread panicked while holding it.
///
/// # Warning
/// If this function recovers from a poisoned lock, it means another thread panicked
/// while holding the lock. The data may be in an inconsistent state. This recovery
/// is intentional to allow the daemon to continue operating, but errors should be
/// investigated.
pub fn rwlock_write_or_recover<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|poisoned| {
        eprintln!(
            "ERROR: RwLock poisoned (write) - a thread panicked while holding this lock. \
             Data may be inconsistent. Attempting recovery."
        );
        poisoned.into_inner()
    })
}

/// Acquires a mutex lock, recovering from poison if a thread panicked while holding it.
///
/// # Warning
/// If this function recovers from a poisoned lock, it means another thread panicked
/// while holding the lock. The data may be in an inconsistent state. This recovery
/// is intentional to allow the daemon to continue operating, but errors should be
/// investigated.
pub fn mutex_lock_or_recover<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| {
        eprintln!(
            "ERROR: Mutex poisoned - a thread panicked while holding this lock. \
             Data may be inconsistent. Attempting recovery."
        );
        poisoned.into_inner()
    })
}
