use super::*;
use crate::domain::session_types::TerminalSize;
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::process::Command;
use std::thread;
use std::time::Duration;

const PTY_READ_BACKPRESSURE_REGRESSION_EVENT_COUNT: usize = 320;

struct ChunkedReader {
    remaining_chunks: usize,
}

impl Read for ChunkedReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.remaining_chunks == 0 {
            return Ok(0);
        }

        self.remaining_chunks -= 1;
        buf[0] = b'x';
        Ok(1)
    }
}

fn spawn_test_reader(
    reader: ChunkedReader,
) -> (channel::Receiver<ReadEvent>, thread::JoinHandle<()>) {
    #[allow(clippy::disallowed_methods)]
    let (tx, rx) = channel::unbounded();
    let join = thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 8192];
        loop {
            if !forward_read_event(&mut reader, &tx, &mut buf) {
                break;
            }
        }
    });
    (rx, join)
}

#[test]
fn test_key_to_escape_sequence() {
    assert_eq!(key_to_escape_sequence("Enter"), Some(vec![b'\r']));
    assert_eq!(key_to_escape_sequence("Tab"), Some(vec![b'\t']));
    assert_eq!(key_to_escape_sequence("Escape"), Some(vec![0x1b]));
    assert_eq!(
        key_to_escape_sequence("ArrowUp"),
        Some(vec![0x1b, b'[', b'A'])
    );
    assert_eq!(key_to_escape_sequence("Ctrl+C"), Some(vec![3]));
    assert_eq!(key_to_escape_sequence("a"), Some(vec![b'a']));
}

#[cfg(unix)]
#[test]
fn can_signal_process_group_is_false_for_non_group_leader() {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("sleep 5")
        .spawn()
        .expect("spawn child");
    let pid = child.id();
    let can_signal = can_signal_process_group(pid).expect("getpgid should succeed");
    assert!(
        !can_signal,
        "regular child process should not be treated as process-group leader"
    );
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn spawn_missing_command_is_classified_as_not_found() {
    let result = PtyHandle::spawn(
        "agent-tui-command-that-should-not-exist-for-tests",
        &[],
        None,
        None,
        TerminalSize::default(),
    );

    match result {
        Err(PtyError::Spawn { kind, reason }) => {
            assert_eq!(kind, SpawnErrorKind::NotFound);
            assert!(
                reason.contains("PATH")
                    || reason.to_ascii_lowercase().contains("not found")
                    || reason
                        .to_ascii_lowercase()
                        .contains("no such file or directory"),
                "unexpected spawn reason: {reason}"
            );
        }
        Err(other) => panic!("expected spawn error, got {other:?}"),
        Ok(_) => panic!("expected missing command to fail"),
    }
}

#[cfg(unix)]
#[test]
fn spawn_passes_explicit_environment_overrides_to_child() {
    let mut env = HashMap::new();
    env.insert(
        "AGENT_TUI_TEST_ENV".to_string(),
        "from-agent-tui".to_string(),
    );
    let args = vec![
        "-c".to_string(),
        "printf %s \"$AGENT_TUI_TEST_ENV\"".to_string(),
    ];
    let mut pty = match PtyHandle::spawn("sh", &args, None, Some(&env), TerminalSize::default()) {
        Ok(pty) => pty,
        Err(PtyError::Spawn { .. }) => return,
        Err(err) => panic!("unexpected PTY error: {err:?}"),
    };

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut collected = Vec::new();
    let mut buf = [0u8; 64];
    while Instant::now() < deadline {
        let read = pty
            .try_read(&mut buf, 100)
            .expect("PTY read should succeed");
        if read > 0 {
            collected.extend_from_slice(&buf[..read]);
            if String::from_utf8_lossy(&collected).contains("from-agent-tui") {
                break;
            }
        } else if !pty.is_running() {
            break;
        }
    }

    let output = String::from_utf8_lossy(&collected);
    assert!(
        output.contains("from-agent-tui"),
        "expected PTY child to observe explicit env override, got {output:?}"
    );
}

#[test]
fn reader_channel_is_unbounded_for_output_events() {
    let (rx, join) = spawn_test_reader(ChunkedReader {
        remaining_chunks: PTY_READ_BACKPRESSURE_REGRESSION_EVENT_COUNT,
    });

    let deadline = Instant::now() + Duration::from_secs(1);
    while !join.is_finished() && Instant::now() < deadline {
        thread::park_timeout(Duration::from_millis(10));
    }
    assert!(
        join.is_finished(),
        "reader thread should not block behind an internal bounded channel"
    );
    let _ = join.join();

    let mut data_events = 0usize;
    let mut saw_eof = false;
    while let Ok(event) = rx.try_recv() {
        match event {
            ReadEvent::Data(_) => data_events += 1,
            ReadEvent::Eof => saw_eof = true,
            ReadEvent::Error(error) => panic!("unexpected reader error: {error}"),
        }
    }

    assert_eq!(data_events, PTY_READ_BACKPRESSURE_REGRESSION_EVENT_COUNT);
    assert!(
        saw_eof,
        "reader should terminate cleanly after draining input"
    );
}

#[cfg(unix)]
#[test]
fn reader_worker_shutdown_unblocks_blocked_reader() {
    let (_writer, reader) = UnixStream::pair().expect("create socket pair");
    let (_rx, mut worker) = spawn_reader_from_fd(reader.as_raw_fd()).expect("spawn reader worker");

    let outcome = worker.shutdown();
    assert_eq!(outcome, ReaderJoinOutcome::Joined);
}
