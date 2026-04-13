use super::join_reader_thread;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

#[test]
fn join_reader_thread_waits_for_completion() {
    let completed = Arc::new(AtomicBool::new(false));
    let completed_for_thread = Arc::clone(&completed);
    let mut reader_join = Some(thread::spawn(move || {
        thread::park_timeout(Duration::from_millis(25));
        completed_for_thread.store(true, Ordering::SeqCst);
    }));

    join_reader_thread(&mut reader_join);

    assert!(completed.load(Ordering::SeqCst));
    assert!(reader_join.is_none());
}
