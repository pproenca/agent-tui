use super::*;
use crate::test_support::env_lock;
use tempfile::tempdir;

struct HomeGuard(Option<String>);

impl Drop for HomeGuard {
    fn drop(&mut self) {
        if let Some(home) = self.0.take() {
            // SAFETY: Test-only environment restoration after HOME override.
            unsafe {
                std::env::set_var("HOME", home);
            }
        } else {
            // SAFETY: Test-only cleanup of HOME override.
            unsafe {
                std::env::remove_var("HOME");
            }
        }
    }
}

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: Test-only environment override.
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, prev }
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: Test-only environment override.
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(prev) = self.prev.take() {
            // SAFETY: Test-only environment restoration.
            unsafe {
                std::env::set_var(self.key, prev);
            }
        } else {
            // SAFETY: Test-only environment cleanup.
            unsafe {
                std::env::remove_var(self.key);
            }
        }
    }
}

struct SessionCleanup<'a> {
    manager: &'a SessionManager,
    session_ids: Vec<SessionId>,
}

impl<'a> SessionCleanup<'a> {
    fn new(manager: &'a SessionManager) -> Self {
        Self {
            manager,
            session_ids: Vec::new(),
        }
    }

    fn track(&mut self, session_id: SessionId) -> SessionId {
        self.session_ids.push(session_id.clone());
        session_id
    }
}

impl Drop for SessionCleanup<'_> {
    fn drop(&mut self) {
        for session_id in self.session_ids.drain(..) {
            let _ = self.manager.kill(&session_id);
        }
    }
}

#[cfg(unix)]
fn spawn_session_or_skip(manager: &SessionManager, session_id: &str) -> Option<SessionId> {
    match manager.spawn(
        "sh",
        &[],
        None,
        None,
        Some(SessionId::try_new(session_id).expect("test session id should be valid")),
        TerminalSize::default(),
    ) {
        Ok((id, _)) => Some(id),
        Err(SessionError::Terminal(_)) => None,
        Err(e) => panic!("unexpected spawn error: {e}"),
    }
}

fn wait_for_file_contents(path: &std::path::Path, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(contents) = fs::read_to_string(path) {
            return Some(contents);
        }
        std::thread::park_timeout(Duration::from_millis(25));
    }
    fs::read_to_string(path).ok()
}

#[test]
fn test_persisted_session_serialization() {
    let session = PersistedSession {
        id: "test123".to_string(),
        command: "bash".to_string(),
        pid: 12345,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        size: TerminalSize::default(),
    };

    let json = serde_json::to_string(&session).expect("session should serialize");
    let parsed: PersistedSession = serde_json::from_str(&json).expect("session JSON should parse");

    assert_eq!(parsed.id, session.id);
    assert_eq!(parsed.command, session.command);
    assert_eq!(parsed.pid, session.pid);
}

#[test]
fn test_is_process_running() {
    let current_pid = std::process::id();
    assert!(is_process_running(current_pid));

    assert!(!is_process_running(999999999));
}

#[test]
fn test_process_info_current_pid() {
    let current_pid = std::process::id();
    let info = process_info(current_pid);
    assert!(info.is_some());
}

#[test]
fn test_spawn_rejects_duplicate_session_id() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }

    let manager = SessionManager::with_max_sessions(2).expect("session manager should initialize");
    let session_id = SessionId::try_new("dup-session").expect("valid session id");
    match manager.spawn(
        "sh",
        &[],
        None,
        None,
        Some(session_id.clone()),
        TerminalSize::default(),
    ) {
        Ok(_) => {}
        Err(SessionError::Terminal(_)) => return, // PTY unavailable, skip
        Err(e) => panic!("unexpected error from first spawn: {e}"),
    }

    let result = manager.spawn(
        "sh",
        &[],
        None,
        None,
        Some(session_id.clone()),
        TerminalSize::default(),
    );

    assert!(matches!(
        result,
        Err(SessionError::AlreadyExists(id)) if id == session_id.as_str()
    ));

    let _ = manager.kill(&session_id);
}

#[cfg(unix)]
#[test]
fn test_spawn_persistence_failure_does_not_register_session() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let store_path = temp_home.path().join("session-store.jsonl");
    let _store_guard = EnvGuard::set(
        "AGENT_TUI_SESSION_STORE",
        store_path
            .to_str()
            .expect("store path should be valid UTF-8"),
    );

    let manager = SessionManager::with_max_sessions(1).expect("session manager should initialize");
    if store_path.exists() {
        fs::remove_file(&store_path).expect("session store file should be removable");
    }
    fs::create_dir(&store_path).expect("session store path should become a directory");
    let session_id = SessionId::try_new("persist-fail").expect("valid session id");
    let result = manager.spawn(
        "sh",
        &["-c".to_string(), ":".to_string()],
        None,
        None,
        Some(session_id.clone()),
        TerminalSize::default(),
    );

    match result {
        Ok(_) => panic!("spawn should fail when persistence cannot append to a directory"),
        Err(SessionError::Terminal(_)) => return,
        Err(SessionError::Persistence { .. }) => {}
        Err(err) => panic!("unexpected spawn error: {err}"),
    }

    assert_eq!(manager.session_count(), 0);
    assert!(manager.active_session_id().is_none());
    assert!(matches!(
        manager.get(&session_id),
        Err(SessionError::NotFound(_))
    ));
}

#[test]
fn test_with_max_sessions_surfaces_cleanup_failure() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::set("AGENT_TUI_SESSION_STORE", "/dev/null/session-store.jsonl");

    let err = match SessionManager::with_max_sessions(1) {
        Ok(_) => {
            panic!("constructor should fail when stale-session cleanup cannot acquire storage")
        }
        Err(err) => err,
    };
    assert!(matches!(err, SessionError::Persistence { .. }));
}

#[cfg(unix)]
#[test]
fn test_list_locked_session_preserves_persisted_metadata() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let manager = SessionManager::with_max_sessions(2).expect("session manager should initialize");
    let mut cleanup = SessionCleanup::new(&manager);
    let session_id = match spawn_session_or_skip(&manager, "locked-list") {
        Some(id) => cleanup.track(id),
        None => return,
    };
    let persisted = manager
        .persistence
        .load()
        .into_iter()
        .find(|session| session.id == session_id.as_str())
        .expect("spawned session should be persisted");

    let session = manager.get(&session_id).expect("session should exist");
    let _guard = mutex_lock_or_recover(&session);

    let listed = manager
        .list()
        .into_iter()
        .find(|info| info.id == session_id)
        .expect("locked session should still be listed");

    assert_eq!(listed.command, persisted.command);
    assert_eq!(listed.pid, persisted.pid);
    assert_eq!(listed.created_at, persisted.created_at);
    assert_eq!(listed.size, persisted.size);
    assert!(
        listed.running,
        "locked sessions should stay conservatively running"
    );
}

#[cfg(unix)]
#[test]
fn test_resolve_without_active_falls_back_to_most_recent_running_session() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let manager = SessionManager::with_max_sessions(4).expect("session manager should initialize");
    let mut cleanup = SessionCleanup::new(&manager);

    let older = match spawn_session_or_skip(&manager, "older-running") {
        Some(id) => cleanup.track(id),
        None => return,
    };
    std::thread::park_timeout(Duration::from_millis(5));
    let newer = match spawn_session_or_skip(&manager, "newer-running") {
        Some(id) => cleanup.track(id),
        None => return,
    };

    {
        let mut active = rwlock_write_or_recover(&manager.active_session);
        *active = None;
    }

    let resolved = manager
        .resolve(None)
        .expect("resolve should return fallback session");
    let resolved_id = mutex_lock_or_recover(&resolved).id.clone();

    assert_eq!(resolved_id, newer);
    assert_ne!(resolved_id, older);
    assert_eq!(manager.active_session_id(), Some(newer));
}

#[cfg(unix)]
#[test]
fn test_resolve_repairs_stale_active_session_with_running_fallback() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let manager = SessionManager::with_max_sessions(4).expect("session manager should initialize");
    let mut cleanup = SessionCleanup::new(&manager);

    let fallback = match spawn_session_or_skip(&manager, "fallback-running") {
        Some(id) => cleanup.track(id),
        None => return,
    };

    {
        let mut active = rwlock_write_or_recover(&manager.active_session);
        *active = Some(SessionId::try_new("stale-active-id").expect("valid session id"));
    }

    let resolved = manager
        .resolve(None)
        .expect("resolve should repair stale active");
    let resolved_id = mutex_lock_or_recover(&resolved).id.clone();

    assert_eq!(resolved_id, fallback);
    assert_eq!(manager.active_session_id(), Some(fallback));
}

#[cfg(unix)]
#[test]
fn test_kill_promotes_most_recent_remaining_running_session_to_active() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let manager = SessionManager::with_max_sessions(4).expect("session manager should initialize");
    let mut cleanup = SessionCleanup::new(&manager);

    let older = match spawn_session_or_skip(&manager, "older-remaining") {
        Some(id) => cleanup.track(id),
        None => return,
    };
    std::thread::park_timeout(Duration::from_millis(5));
    let active = match spawn_session_or_skip(&manager, "active-to-kill") {
        Some(id) => cleanup.track(id),
        None => return,
    };

    assert_eq!(manager.active_session_id(), Some(active.clone()));

    manager
        .kill(&active)
        .expect("kill should succeed for active session");

    assert_eq!(manager.active_session_id(), Some(older.clone()));
    let resolved = manager
        .resolve(None)
        .expect("resolve should use promoted active");
    assert_eq!(mutex_lock_or_recover(&resolved).id, older);
}

#[cfg(unix)]
#[test]
fn test_kill_persistence_failure_keeps_session_registered() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let store_path = temp_home.path().join("session-store.jsonl");
    let _store_guard = EnvGuard::set(
        "AGENT_TUI_SESSION_STORE",
        store_path
            .to_str()
            .expect("store path should be valid UTF-8"),
    );

    let manager = SessionManager::with_max_sessions(2).expect("session manager should initialize");
    let session_id = match manager.spawn(
        "sleep",
        &["30".to_string()],
        None,
        None,
        Some(SessionId::try_new("kill-persist-fail").expect("valid session id")),
        TerminalSize::default(),
    ) {
        Ok((session_id, _pid)) => session_id,
        Err(SessionError::Terminal(_)) => return,
        Err(err) => panic!("unexpected spawn error: {err}"),
    };

    fs::remove_file(&store_path).expect("session store file should be removable");
    fs::create_dir(&store_path).expect("session store path should become a directory");

    let err = manager
        .kill(&session_id)
        .expect_err("kill should surface persistence failure");
    assert!(matches!(err, SessionError::Persistence { .. }));
    assert_eq!(manager.session_count(), 1);
    let session = manager
        .get(&session_id)
        .expect("killed session should stay registered after persistence failure");
    assert!(
        !mutex_lock_or_recover(&session).is_running(),
        "killed session should stay visible as stopped"
    );

    fs::remove_dir(&store_path).expect("directory store path should be removable");
    File::create(&store_path).expect("session store file should be recreated");
    manager
        .kill(&session_id)
        .expect("cleanup kill should succeed after store restoration");
    assert_eq!(manager.session_count(), 0);
}

#[test]
fn test_resolve_without_running_sessions_returns_no_active_session() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let manager = SessionManager::with_max_sessions(1).expect("session manager should initialize");
    {
        let mut active = rwlock_write_or_recover(&manager.active_session);
        *active = Some(SessionId::try_new("stale-active-id").expect("valid session id"));
    }

    let result = manager.resolve(None);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
    assert!(manager.active_session_id().is_none());
}

#[cfg(unix)]
#[test]
fn test_resolve_with_explicit_session_id_does_not_reroute() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let manager = SessionManager::with_max_sessions(4).expect("session manager should initialize");
    let mut cleanup = SessionCleanup::new(&manager);

    let requested = match spawn_session_or_skip(&manager, "explicit-target") {
        Some(id) => cleanup.track(id),
        None => return,
    };
    let _other = match spawn_session_or_skip(&manager, "other-running") {
        Some(id) => cleanup.track(id),
        None => return,
    };

    {
        let mut active = rwlock_write_or_recover(&manager.active_session);
        *active = None;
    }

    let resolved = manager
        .resolve(Some(&requested))
        .expect("explicit session resolution should succeed");
    assert_eq!(mutex_lock_or_recover(&resolved).id, requested);
    assert!(manager.active_session_id().is_none());
}

#[cfg(unix)]
#[test]
fn test_restart_preserves_launch_context_and_replaces_session() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let working_dir = temp_home.path().join("restart-cwd");
    fs::create_dir_all(&working_dir).expect("working dir should be created");
    let marker_path = temp_home.path().join("restart-marker.txt");

    let manager = SessionManager::with_max_sessions(4).expect("session manager should initialize");
    let mut cleanup = SessionCleanup::new(&manager);
    let expected_working_dir =
        fs::canonicalize(&working_dir).unwrap_or_else(|_| working_dir.clone());

    let mut env = HashMap::new();
    env.insert("MYVAR".to_string(), "preserved-env".to_string());
    let args = vec![
        "-c".to_string(),
        format!(
            "printf \"%s|%s\" \"$PWD\" \"$MYVAR\" > \"{}\"; sleep 30",
            marker_path.display()
        ),
    ];

    let initial_session_id = match manager.spawn(
        "sh",
        &args,
        Some(
            working_dir
                .to_str()
                .expect("working directory should be valid UTF-8"),
        ),
        Some(&env),
        Some(SessionId::try_new("restart-src").expect("valid session id")),
        TerminalSize::default(),
    ) {
        Ok((session_id, _pid)) => cleanup.track(session_id),
        Err(SessionError::Terminal(_)) => return,
        Err(e) => panic!("unexpected spawn error: {e}"),
    };

    let expected_marker = format!("{}|preserved-env", expected_working_dir.display());
    let initial_contents = wait_for_file_contents(&marker_path, Duration::from_secs(2))
        .expect("spawned session should write marker");
    assert_eq!(initial_contents, expected_marker);
    fs::remove_file(&marker_path).expect("marker file should be removed before restart");

    let restarted = manager
        .restart(Some(&initial_session_id))
        .expect("restart should succeed");
    cleanup.track(restarted.new_session_id.clone());

    assert_eq!(restarted.old_session_id, initial_session_id);
    assert_ne!(restarted.new_session_id, initial_session_id);
    assert_eq!(
        manager.active_session_id(),
        Some(restarted.new_session_id.clone())
    );
    assert!(matches!(
        manager.get(&initial_session_id),
        Err(SessionError::NotFound(_))
    ));

    let restarted_contents = wait_for_file_contents(&marker_path, Duration::from_secs(2))
        .expect("restarted session should write marker");
    assert_eq!(restarted_contents, expected_marker);

    let session = manager
        .get(&restarted.new_session_id)
        .expect("restarted session should exist");
    let launch = mutex_lock_or_recover(&session).launch_spec();
    let env_value = launch
        .env
        .as_ref()
        .and_then(|vars| vars.get("MYVAR"))
        .map(String::as_str);

    assert_eq!(launch.command, "sh");
    assert_eq!(launch.args, args);
    assert_eq!(launch.cwd.as_deref(), working_dir.to_str());
    assert_eq!(env_value, Some("preserved-env"));
}

#[cfg(unix)]
#[test]
fn test_restart_persistence_failure_keeps_existing_session_running() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let store_path = temp_home.path().join("session-store.jsonl");
    let _store_guard = EnvGuard::set(
        "AGENT_TUI_SESSION_STORE",
        store_path
            .to_str()
            .expect("store path should be valid UTF-8"),
    );

    let manager = SessionManager::with_max_sessions(2).expect("session manager should initialize");
    let session_id = match manager.spawn(
        "sleep",
        &["30".to_string()],
        None,
        None,
        Some(SessionId::try_new("restart-persist-fail").expect("valid session id")),
        TerminalSize::default(),
    ) {
        Ok((session_id, _pid)) => session_id,
        Err(SessionError::Terminal(_)) => return,
        Err(err) => panic!("unexpected spawn error: {err}"),
    };

    fs::remove_file(&store_path).expect("session store file should be removable");
    fs::create_dir(&store_path).expect("session store path should become a directory");

    let err = manager
        .restart(Some(&session_id))
        .expect_err("restart should surface persistence failure before replacing the session");
    assert!(matches!(err, SessionError::Persistence { .. }));
    assert_eq!(manager.session_count(), 1);
    assert_eq!(manager.active_session_id(), Some(session_id.clone()));
    let session = manager
        .get(&session_id)
        .expect("original session should remain registered");
    assert!(
        mutex_lock_or_recover(&session).is_running(),
        "original session should keep running when replacement persistence fails"
    );

    fs::remove_dir(&store_path).expect("directory store path should be removable");
    File::create(&store_path).expect("session store file should be recreated");
    manager
        .kill(&session_id)
        .expect("cleanup kill should succeed after store restoration");
}

#[test]
fn test_persistence_migration_from_json() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let legacy_dir = temp_home.path().join(".agent-tui");
    fs::create_dir_all(&legacy_dir).expect("legacy dir should be created");
    let legacy_path = legacy_dir.join("sessions.json");
    let sessions = vec![PersistedSession {
        id: "legacy".to_string(),
        command: "sh".to_string(),
        pid: 1234,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        size: TerminalSize::default(),
    }];
    fs::write(
        &legacy_path,
        serde_json::to_string(&sessions).expect("legacy sessions should serialize"),
    )
    .expect("legacy session file should be written");

    let persistence = SessionPersistence::new();
    let loaded = persistence.load();
    assert_eq!(loaded.len(), 1);

    let jsonl_path = legacy_dir.join("sessions.jsonl");
    let backup_path = legacy_dir.join("sessions.json.bak");
    assert!(jsonl_path.exists());
    assert!(backup_path.exists());
}

#[test]
fn test_jsonl_add_remove_roundtrip() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let persistence = SessionPersistence::new();
    let session = PersistedSession {
        id: "roundtrip".to_string(),
        command: "bash".to_string(),
        pid: 777,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        size: TerminalSize::try_new(100, 40).expect("valid terminal size"),
    };
    persistence
        .add_session(session.clone())
        .expect("session should be added");
    let loaded = persistence.load();
    assert!(loaded.iter().any(|s| s.id == session.id));

    persistence
        .remove_session(&session.id)
        .expect("session should be removed");
    let loaded = persistence.load();
    assert!(loaded.is_empty());
}

#[test]
fn test_compaction_skips_unknown_records_to_preserve_forward_compatibility() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

    let log_path = temp_home.path().join(".agent-tui").join("sessions.jsonl");
    fs::create_dir_all(log_path.parent().expect("log path should have parent"))
        .expect("log directory should be created");

    let known = SessionEvent::Upsert {
        session: PersistedSession {
            id: "known".to_string(),
            command: "sh".to_string(),
            pid: 111,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: TerminalSize::default(),
        },
    };
    let unknown = serde_json::json!({
        "type": "future_event",
        "payload": "x".repeat(SESSION_STORE_COMPACT_THRESHOLD_BYTES as usize),
    });

    let before = format!(
        "{}\n{}\n",
        serde_json::to_string(&known).expect("known event should serialize"),
        serde_json::to_string(&unknown).expect("unknown event should serialize"),
    );
    fs::write(&log_path, &before).expect("session log should be seeded");

    let persistence = SessionPersistence::new();
    persistence
        .maybe_compact_unlocked()
        .expect("compaction should skip without failing");

    let after = fs::read_to_string(&log_path).expect("session log should still exist");
    let loaded = persistence.load();

    assert_eq!(after, before);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "known");
}

#[test]
fn test_session_store_env_override() {
    let _env_lock = env_lock();
    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }
    let store_path = temp_home.path().join("custom-sessions.jsonl");
    let _store_guard = EnvGuard::set(
        "AGENT_TUI_SESSION_STORE",
        store_path.to_string_lossy().as_ref(),
    );

    let persistence = SessionPersistence::new();
    persistence
        .add_session(PersistedSession {
            id: "custom".to_string(),
            command: "sh".to_string(),
            pid: 456,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: TerminalSize::default(),
        })
        .expect("custom session should be persisted");

    assert!(store_path.exists());
    let default_path = temp_home.path().join(".agent-tui").join("sessions.jsonl");
    assert!(!default_path.exists());
}

#[cfg(unix)]
#[test]
fn test_startup_cleanup_kills_persisted_session_process_group() {
    let _env_lock = env_lock();
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("sleep 10");
    // SAFETY: pre-exec runs in the child before exec.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = cmd.spawn().expect("failed to spawn test child");
    let pid = child.id();
    assert!(pid > 0);

    let persistence = SessionPersistence::new();
    persistence
        .add_session(PersistedSession {
            id: "orphan".to_string(),
            command: "sleep".to_string(),
            pid,
            created_at: Utc::now().to_rfc3339(),
            size: TerminalSize::default(),
        })
        .expect("failed to persist session");

    let _manager = SessionManager::with_max_sessions(1).expect("session manager should initialize");

    let deadline = Instant::now() + Duration::from_secs(2);
    while is_process_running(pid) && Instant::now() < deadline {
        std::thread::park_timeout(Duration::from_millis(25));
    }

    if is_process_running(pid) {
        let pid_t: libc::pid_t = pid.try_into().unwrap_or(0);
        if pid_t > 0 {
            // SAFETY: negative pid targets the process group for cleanup.
            unsafe {
                libc::kill(-pid_t, libc::SIGKILL);
            }
        }
    }

    let _ = child.wait();

    assert!(!is_process_running(pid));

    let sessions = persistence.load();
    assert!(sessions.is_empty());
}

#[test]
fn test_modifier_state_applies_shift_to_keystroke_characters_and_tab() {
    let mut state = ModifierState::default();
    state.set(ModifierKey::Shift, true);

    assert_eq!(
        state
            .keystroke_bytes("a")
            .expect("shifted character keystroke should encode"),
        vec![b'A']
    );
    assert_eq!(
        state
            .keystroke_bytes("1")
            .expect("shifted punctuation keystroke should encode"),
        vec![b'!']
    );
    assert_eq!(
        state
            .keystroke_bytes("Tab")
            .expect("shifted tab keystroke should encode"),
        vec![0x1b, b'[', b'Z']
    );
}

#[test]
fn test_modifier_state_applies_ctrl_alt_to_keystroke_characters() {
    let mut state = ModifierState::default();
    state.set(ModifierKey::Ctrl, true);
    state.set(ModifierKey::Alt, true);

    assert_eq!(
        state
            .keystroke_bytes("c")
            .expect("alt+ctrl keystroke should encode"),
        vec![0x1b, 3]
    );
}

#[test]
fn test_modifier_state_applies_shift_to_typed_letters_without_remapping_punctuation() {
    let mut state = ModifierState::default();
    state.set(ModifierKey::Shift, true);

    assert_eq!(state.typed_bytes("ab-1"), b"AB-1".to_vec());
}

#[test]
fn test_modifier_state_applies_ctrl_meta_to_typed_text() {
    let mut state = ModifierState::default();
    state.set(ModifierKey::Ctrl, true);
    state.set(ModifierKey::Meta, true);

    assert_eq!(state.typed_bytes("c["), vec![0x1b, 3, 0x1b, 27]);
}

#[cfg(unix)]
#[test]
fn test_cleanup_stale_sessions_appends_remove_events_when_unknown_records_exist() {
    let _env_lock = env_lock();
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    let temp_home = tempdir().expect("temp dir should be created");
    let _home_guard = HomeGuard(std::env::var("HOME").ok());
    // SAFETY: Test-only environment override for HOME directory.
    unsafe {
        std::env::set_var("HOME", temp_home.path());
    }

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("sleep 10");
    // SAFETY: pre-exec runs in the child before exec.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = cmd.spawn().expect("failed to spawn test child");
    let pid = child.id();
    assert!(pid > 0);

    let persistence = SessionPersistence::new();
    persistence
        .add_session(PersistedSession {
            id: "orphan".to_string(),
            command: "sleep".to_string(),
            pid,
            created_at: Utc::now().to_rfc3339(),
            size: TerminalSize::default(),
        })
        .expect("failed to persist session");

    let log_path = temp_home.path().join(".agent-tui").join("sessions.jsonl");
    let mut log = OpenOptions::new()
        .append(true)
        .open(&log_path)
        .expect("open session log");
    writeln!(
        log,
        "{}",
        serde_json::to_string(&serde_json::json!({
            "type": "future_event",
            "payload": "preserve-me",
        }))
        .expect("serialize future event")
    )
    .expect("append future event");
    drop(log);

    let cleaned = persistence
        .cleanup_stale_sessions()
        .expect("cleanup should succeed");
    assert_eq!(cleaned, 1);

    let deadline = Instant::now() + Duration::from_secs(2);
    while is_process_running(pid) && Instant::now() < deadline {
        std::thread::park_timeout(Duration::from_millis(25));
    }

    if is_process_running(pid) {
        let pid_t: libc::pid_t = pid.try_into().unwrap_or(0);
        if pid_t > 0 {
            // SAFETY: negative pid targets the process group for cleanup.
            unsafe {
                libc::kill(-pid_t, libc::SIGKILL);
            }
        }
    }

    let _ = child.wait();

    let sessions = persistence.load();
    assert!(
        sessions.is_empty(),
        "cleaned session should not remain persisted"
    );

    let log_contents = fs::read_to_string(&log_path).expect("read session log");
    assert!(
        log_contents.contains("\"type\":\"future_event\""),
        "cleanup should preserve unknown records"
    );
    assert!(
        log_contents.contains("\"type\":\"remove\""),
        "cleanup should append a remove event for cleaned sessions"
    );
    assert!(
        log_contents.contains("\"session_id\":\"orphan\""),
        "cleanup should record which session was removed"
    );
}
