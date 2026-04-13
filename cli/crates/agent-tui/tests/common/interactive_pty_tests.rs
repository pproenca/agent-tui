use super::join_reader_thread;
use crossbeam_channel as channel;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;

#[test]
fn join_reader_thread_waits_for_completion() {
    let completed = Arc::new(AtomicBool::new(false));
    let completed_for_thread = Arc::clone(&completed);
    let (entered_tx, entered_rx) = channel::bounded(1);
    let (release_tx, release_rx) = channel::bounded(1);
    let (joined_tx, joined_rx) = channel::bounded(1);
    let reader_join = Some(thread::spawn(move || {
        let _ = entered_tx.send(());
        let _ = release_rx.recv();
        completed_for_thread.store(true, Ordering::SeqCst);
    }));

    entered_rx
        .recv()
        .expect("reader thread should report that it is running");
    let join_worker = thread::spawn(move || {
        let mut reader_join = reader_join;
        join_reader_thread(&mut reader_join);
        let _ = joined_tx.send(reader_join.is_none());
    });

    assert!(
        joined_rx.try_recv().is_err(),
        "join helper should still be waiting while the reader thread is blocked"
    );
    release_tx
        .send(())
        .expect("reader thread release signal should send");
    assert!(
        joined_rx
            .recv()
            .expect("join worker should report completion"),
        "reader join handle should be cleared after joining"
    );
    join_worker.join().expect("join worker should exit cleanly");
    assert!(completed.load(Ordering::SeqCst));
}
