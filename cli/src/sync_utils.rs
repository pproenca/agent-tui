//! Synchronization utilities with poison recovery
//!
//! This module provides helper functions for acquiring locks on Mutex and RwLock
//! that automatically recover from poisoned state. Poison recovery allows the
//! application to continue operating even if a thread panicked while holding a lock.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Recover from a poisoned RwLock read guard
pub fn rwlock_read_or_recover<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| {
        eprintln!("Warning: recovering from poisoned rwlock (read)");
        poisoned.into_inner()
    })
}

/// Recover from a poisoned RwLock write guard
pub fn rwlock_write_or_recover<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|poisoned| {
        eprintln!("Warning: recovering from poisoned rwlock (write)");
        poisoned.into_inner()
    })
}

/// Recover from a poisoned Mutex
pub fn mutex_lock_or_recover<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| {
        eprintln!("Warning: recovering from poisoned mutex");
        poisoned.into_inner()
    })
}
