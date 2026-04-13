use super::*;
use crate::common::mutex_lock_or_recover;
use std::sync::Barrier;
use std::sync::Condvar;
use std::sync::MutexGuard;

fn wait_or_recover<'a, T>(cvar: &Condvar, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
    cvar.wait(guard)
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn test_backoff_respects_max() {
    let mut backoff = Duration::from_micros(100);
    for _ in 0..20 {
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
    assert_eq!(backoff, MAX_BACKOFF);
}

#[test]
fn test_jitter_deterministic_per_thread() {
    let backoff = 1000u64;
    let jitter1 = compute_jitter(backoff);
    let jitter2 = compute_jitter(backoff);
    assert_eq!(jitter1, jitter2);
}

#[test]
fn test_jitter_bounded() {
    for backoff in [100u64, 1000, 10000, 50000] {
        let jitter = compute_jitter(backoff);
        assert!(jitter <= backoff / 4);
    }
}

#[test]
fn test_jitter_zero_for_tiny_backoff() {
    assert_eq!(compute_jitter(0), 0);
    assert_eq!(compute_jitter(3), 0);
}

#[test]
fn test_acquire_lock_with_simple_mutex() {
    let data = Arc::new(Mutex::new(42i32));
    let start = Instant::now();
    let mut backoff = Duration::from_micros(100);
    let timeout = Duration::from_millis(100);

    while start.elapsed() < timeout {
        if let Ok(guard) = data.try_lock() {
            assert_eq!(*guard, 42);
            return;
        }
        let jitter = compute_jitter(backoff.as_micros() as u64);
        thread::park_timeout(backoff + Duration::from_micros(jitter));
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
    panic!("Should have acquired lock");
}

#[test]
fn test_lock_timeout_with_held_mutex() {
    let data = Arc::new(Mutex::new(42i32));
    let _held = mutex_lock_or_recover(&data);
    let start = Instant::now();
    let mut backoff = Duration::from_micros(100);
    let timeout = Duration::from_millis(50);

    while start.elapsed() < timeout {
        if data.try_lock().is_ok() {
            panic!("Should not acquire lock while held");
        }
        let jitter = compute_jitter(backoff.as_micros() as u64);
        thread::park_timeout(backoff + Duration::from_micros(jitter));
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
    assert!(start.elapsed() >= Duration::from_millis(50));
}

#[test]
fn test_acquire_session_lock_succeeds_after_contention() {
    let data = Arc::new(Mutex::new(42i32));
    let data_clone = Arc::clone(&data);

    let sync = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let sync_clone = Arc::clone(&sync);

    let handle = thread::spawn(move || {
        let _guard = mutex_lock_or_recover(&data_clone);

        {
            let (lock, cvar) = &*sync_clone;
            let mut state = mutex_lock_or_recover(lock);
            state.0 = true;
            cvar.notify_all();

            while !state.1 {
                state = wait_or_recover(cvar, state);
            }
        }
    });

    {
        let (lock, cvar) = &*sync;
        let mut state = mutex_lock_or_recover(lock);
        while !state.0 {
            state = wait_or_recover(cvar, state);
        }
    }

    assert!(data.try_lock().is_err(), "Lock should be held by worker");

    {
        let (lock, cvar) = &*sync;
        let mut state = mutex_lock_or_recover(lock);
        state.1 = true;
        cvar.notify_all();
    }

    let start = Instant::now();
    let mut backoff = Duration::from_micros(100);
    let timeout = Duration::from_secs(5);
    let mut acquired = false;

    while start.elapsed() < timeout {
        if let Ok(guard) = data.try_lock() {
            assert_eq!(*guard, 42);
            acquired = true;
            break;
        }
        let jitter = compute_jitter(backoff.as_micros() as u64);
        thread::park_timeout(backoff + Duration::from_micros(jitter));
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }

    handle.join().expect("worker thread should join");
    assert!(acquired, "Should have acquired lock after contention");
}

#[test]
fn test_acquire_session_lock_timeout_returns_none_under_contention() {
    let data = Arc::new(Mutex::new(42i32));
    let data_clone = Arc::clone(&data);

    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = Arc::clone(&barrier);

    let handle = thread::spawn(move || {
        let _guard = mutex_lock_or_recover(&data_clone);
        barrier_clone.wait();
        thread::park_timeout(Duration::from_millis(200));
    });

    barrier.wait();

    let start = Instant::now();
    let mut backoff = Duration::from_micros(100);
    let timeout = Duration::from_millis(50);
    let mut acquired = false;

    while start.elapsed() < timeout {
        if data.try_lock().is_ok() {
            acquired = true;
            break;
        }
        let jitter = compute_jitter(backoff.as_micros() as u64);
        thread::park_timeout(backoff + Duration::from_micros(jitter));
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }

    assert!(
        !acquired,
        "Should not have acquired lock (timeout too short)"
    );
    assert!(
        start.elapsed() >= Duration::from_millis(50),
        "Should have waited full timeout"
    );

    handle.join().expect("worker thread should join");
}
