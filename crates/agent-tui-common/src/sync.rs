use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::sync::RwLockWriteGuard;

pub fn rwlock_read_or_recover<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| {
        eprintln!("Warning: recovering from poisoned rwlock (read)");
        poisoned.into_inner()
    })
}

pub fn rwlock_write_or_recover<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|poisoned| {
        eprintln!("Warning: recovering from poisoned rwlock (write)");
        poisoned.into_inner()
    })
}

pub fn mutex_lock_or_recover<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| {
        eprintln!("Warning: recovering from poisoned mutex");
        poisoned.into_inner()
    })
}
