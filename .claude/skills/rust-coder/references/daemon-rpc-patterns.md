# Daemon, RPC, and TUI Emulation Patterns

## Table of Contents
1. [Daemon Lifecycle](#1-daemon-lifecycle)
2. [Unix Socket IPC](#2-unix-socket-ipc)
3. [JSON-RPC 2.0 Protocol](#3-json-rpc-20-protocol)
4. [Request/Response Types](#4-requestresponse-types)
5. [Session Management](#5-session-management)
6. [PTY Emulation](#6-pty-emulation)
7. [Terminal Screen Buffer](#7-terminal-screen-buffer)
8. [Client Implementation](#8-client-implementation)
9. [Background Service Patterns](#9-background-service-patterns)
10. [Graceful Shutdown](#10-graceful-shutdown)

---

## 1. Daemon Lifecycle

Singleton daemon with file locking:

```rust
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

pub fn socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("myapp.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/myapp.sock"))
}

pub fn start_daemon() -> std::io::Result<()> {
    let socket_path = socket_path();
    let lock_path = socket_path.with_extension("lock");

    // Acquire exclusive lock (prevents multiple instances)
    let lock_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;

    let fd = lock_file.as_raw_fd();
    let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if result != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            "Another daemon instance is running",
        ));
    }

    // Write PID for debugging
    use std::io::Write;
    let mut lock_file = lock_file;
    lock_file.set_len(0)?;
    writeln!(lock_file, "{}", std::process::id())?;

    // Clean stale socket
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    eprintln!("Daemon started on {}", socket_path.display());

    let server = Arc::new(Server::new());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = Arc::clone(&server);
                thread::Builder::new()
                    .name("client-handler".to_string())
                    .spawn(move || server.handle_client(stream))
                    .expect("spawn failed");
            }
            Err(e) => eprintln!("Connection error: {}", e),
        }
    }

    Ok(())
}
```

## 2. Unix Socket IPC

Line-delimited JSON over Unix sockets:

```rust
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

impl Server {
    pub fn handle_client(&self, stream: UnixStream) {
        let reader_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to clone stream: {}", e);
                return;
            }
        };

        let reader = BufReader::new(reader_stream);
        let mut writer = stream;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break, // Connection closed
            };

            if line.trim().is_empty() {
                continue;
            }

            let response = match serde_json::from_str::<Request>(&line) {
                Ok(request) => self.handle_request(request),
                Err(e) => Response::parse_error(e),
            };

            let json = serde_json::to_string(&response).unwrap_or_else(|_| {
                r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Serialization failed"}}"#
                    .to_string()
            });

            if writeln!(writer, "{}", json).is_err() {
                break;
            }
        }
    }
}

// TCP fallback for cross-machine communication
pub fn start_tcp_listener(port: u16) -> std::io::Result<()> {
    use std::net::TcpListener;

    let listener = TcpListener::bind(("127.0.0.1", port))?;

    for stream in listener.incoming() {
        // Handle similar to Unix socket
    }
    Ok(())
}
```

## 3. JSON-RPC 2.0 Protocol

Implement standard JSON-RPC:

```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct Request {
    #[allow(dead_code)]
    jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

// Standard JSON-RPC error codes
const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;
const APP_ERROR: i32 = -32000; // Application-specific

impl Response {
    pub fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: u64, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    pub fn parse_error(e: serde_json::Error) -> Self {
        Self::error(0, PARSE_ERROR, &format!("Parse error: {}", e))
    }

    pub fn method_not_found(id: u64, method: &str) -> Self {
        Self::error(id, METHOD_NOT_FOUND, &format!("Method not found: {}", method))
    }
}
```

## 4. Request/Response Types

Parameter extraction helpers:

```rust
impl Request {
    pub fn param_str(&self, key: &str) -> Option<&str> {
        self.params.as_ref()?.get(key)?.as_str()
    }

    pub fn param_bool(&self, key: &str) -> Option<bool> {
        self.params.as_ref()?.get(key)?.as_bool()
    }

    pub fn param_u64(&self, key: &str, default: u64) -> u64 {
        self.params
            .as_ref()
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_u64())
            .unwrap_or(default)
    }

    pub fn require_str(&self, key: &str) -> Result<&str, Response> {
        self.param_str(key)
            .ok_or_else(|| Response::error(self.id, INVALID_PARAMS, &format!("Missing '{}'", key)))
    }

    pub fn require_array(&self, key: &str) -> Result<&Vec<Value>, Response> {
        self.params
            .as_ref()
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_array())
            .ok_or_else(|| Response::error(self.id, INVALID_PARAMS, &format!("Missing '{}'", key)))
    }
}

// Method dispatch
impl Server {
    fn handle_request(&self, request: Request) -> Response {
        match request.method.as_str() {
            "health" => self.handle_health(request),
            "spawn" => self.handle_spawn(request),
            "snapshot" => self.handle_snapshot(request),
            "click" => self.handle_click(request),
            "fill" => self.handle_fill(request),
            "keystroke" => self.handle_keystroke(request),
            "wait" => self.handle_wait(request),
            "kill" => self.handle_kill(request),
            "sessions" => self.handle_sessions(request),
            _ => Response::method_not_found(request.id, &request.method),
        }
    }

    fn handle_health(&self, request: Request) -> Response {
        Response::success(
            request.id,
            json!({
                "status": "healthy",
                "pid": std::process::id(),
                "uptime_ms": self.start_time.elapsed().as_millis() as u64,
                "version": env!("CARGO_PKG_VERSION")
            }),
        )
    }
}
```

## 5. Session Management

Thread-safe session registry:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

pub struct Session {
    pub id: String,
    pub command: String,
    pub pid: u32,
    // ... session state
}

pub struct SessionManager {
    sessions: Mutex<HashMap<String, Arc<Mutex<Session>>>>,
    active: Mutex<Option<String>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            active: Mutex::new(None),
        }
    }

    pub fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        session_id: Option<String>,
        cols: u16,
        rows: u16,
    ) -> Result<(String, u32), Box<dyn std::error::Error>> {
        let id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let session = Session::new(&id, command, args, cwd, cols, rows)?;
        let pid = session.pid;

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(id.clone(), Arc::new(Mutex::new(session)));

        let mut active = self.active.lock().unwrap();
        if active.is_none() {
            *active = Some(id.clone());
        }

        Ok((id, pid))
    }

    pub fn get(&self, id: &str) -> Result<Arc<Mutex<Session>>, SessionError> {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or(SessionError::NotFound(id.to_string()))
    }

    pub fn resolve(&self, id: Option<&str>) -> Result<Arc<Mutex<Session>>, SessionError> {
        match id {
            Some(id) => self.get(id),
            None => self.get_active(),
        }
    }

    fn get_active(&self) -> Result<Arc<Mutex<Session>>, SessionError> {
        let active = self.active.lock().unwrap();
        match &*active {
            Some(id) => self.get(id),
            None => Err(SessionError::NoActive),
        }
    }

    pub fn kill(&self, id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.remove(id).ok_or(SessionError::NotFound(id.to_string()))?;

        let sess = session.lock().unwrap();
        sess.terminate()?;

        // Update active if killed
        let mut active = self.active.lock().unwrap();
        if active.as_ref() == Some(&id.to_string()) {
            *active = sessions.keys().next().cloned();
        }

        Ok(())
    }
}

// Lock acquisition with timeout
pub const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

pub fn acquire_session_lock(
    session: &Arc<Mutex<Session>>,
    timeout: Duration,
) -> Option<MutexGuard<'_, Session>> {
    let start = std::time::Instant::now();
    loop {
        match session.try_lock() {
            Ok(guard) => return Some(guard),
            Err(_) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
}
```

## 6. PTY Emulation

Spawn and control pseudo-terminals:

```rust
use nix::pty::{openpty, OpenptyResult, Winsize};
use nix::unistd::{close, dup2, execvp, fork, setsid, ForkResult};
use std::ffi::CString;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

pub struct Pty {
    master_fd: RawFd,
    slave_fd: RawFd,
    pid: u32,
}

impl Pty {
    pub fn spawn(command: &str, args: &[String], cols: u16, rows: u16) -> Result<Self, PtyError> {
        let winsize = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let OpenptyResult { master, slave } = openpty(&winsize, None)?;

        match unsafe { fork()? } {
            ForkResult::Child => {
                // Child: become session leader, set controlling terminal
                close(master).ok();
                setsid().ok();

                // Set as controlling terminal
                unsafe { libc::ioctl(slave, libc::TIOCSCTTY, 0) };

                // Redirect stdio
                dup2(slave, 0).ok(); // stdin
                dup2(slave, 1).ok(); // stdout
                dup2(slave, 2).ok(); // stderr
                close(slave).ok();

                // Execute
                let cmd = CString::new(command).unwrap();
                let args: Vec<CString> = std::iter::once(cmd.clone())
                    .chain(args.iter().map(|a| CString::new(a.as_str()).unwrap()))
                    .collect();

                execvp(&cmd, &args).ok();
                std::process::exit(1);
            }
            ForkResult::Parent { child } => {
                close(slave).ok();
                Ok(Self {
                    master_fd: master,
                    slave_fd: slave,
                    pid: child.as_raw() as u32,
                })
            }
        }
    }

    pub fn write(&self, data: &[u8]) -> std::io::Result<usize> {
        nix::unistd::write(self.master_fd, data).map_err(|e| std::io::Error::from_raw_os_error(e as i32))
    }

    pub fn read(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        nix::unistd::read(self.master_fd, buf).map_err(|e| std::io::Error::from_raw_os_error(e as i32))
    }

    pub fn resize(&self, cols: u16, rows: u16) -> std::io::Result<()> {
        let winsize = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        unsafe {
            if libc::ioctl(self.master_fd, libc::TIOCSWINSZ, &winsize) == -1 {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub fn terminate(&self) -> std::io::Result<()> {
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(self.pid as i32),
            nix::sys::signal::Signal::SIGTERM,
        )
        .map_err(|e| std::io::Error::from_raw_os_error(e as i32))
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        close(self.master_fd).ok();
    }
}
```

## 7. Terminal Screen Buffer

Emulate terminal with vt100 parsing:

```rust
use vt100::Parser;

pub struct TerminalEmulator {
    parser: Parser,
    cols: u16,
    rows: u16,
}

impl TerminalEmulator {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            parser: Parser::new(rows, cols, 0),
            cols,
            rows,
        }
    }

    pub fn process(&mut self, data: &[u8]) {
        self.parser.process(data);
    }

    pub fn screen_text(&self) -> String {
        let screen = self.parser.screen();
        let mut output = String::new();

        for row in 0..self.rows {
            let line = screen.row_wrapped(row)
                .map(|cells| {
                    cells.iter()
                        .map(|c| c.contents())
                        .collect::<String>()
                })
                .unwrap_or_default();

            output.push_str(line.trim_end());
            output.push('\n');
        }

        // Trim trailing empty lines
        output.trim_end_matches('\n').to_string() + "\n"
    }

    pub fn cursor(&self) -> Cursor {
        let screen = self.parser.screen();
        Cursor {
            row: screen.cursor_position().0,
            col: screen.cursor_position().1,
            visible: !screen.hide_cursor(),
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.parser.set_size(rows, cols);
        self.cols = cols;
        self.rows = rows;
    }

    /// Get cell at position with attributes
    pub fn cell_at(&self, row: u16, col: u16) -> Option<Cell> {
        let screen = self.parser.screen();
        screen.cell(row, col).map(|c| Cell {
            char: c.contents().chars().next().unwrap_or(' '),
            fg: c.fgcolor(),
            bg: c.bgcolor(),
            bold: c.bold(),
            inverse: c.inverse(),
        })
    }

    /// Get styled text for a region (for element detection)
    pub fn region_text(&self, row: u16, col_start: u16, col_end: u16) -> String {
        let screen = self.parser.screen();
        (col_start..col_end)
            .filter_map(|col| screen.cell(row, col))
            .map(|c| c.contents())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Cursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub char: char,
    pub fg: vt100::Color,
    pub bg: vt100::Color,
    pub bold: bool,
    pub inverse: bool,
}
```

## 8. Client Implementation

Reconnecting client with request ID tracking:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use thiserror::Error;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to connect: {0}")]
    ConnectionFailed(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("RPC error ({code}): {message}")]
    RpcError { code: i32, message: String },

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Invalid response")]
    InvalidResponse,
}

pub struct Client;

impl Client {
    pub fn connect() -> Result<Self, ClientError> {
        let path = socket_path();
        if !path.exists() {
            return Err(ClientError::DaemonNotRunning);
        }

        // Test connection
        let stream = UnixStream::connect(&path)?;
        drop(stream);
        Ok(Self)
    }

    pub fn is_daemon_running() -> bool {
        let path = socket_path();
        path.exists() && UnixStream::connect(&path).is_ok()
    }

    pub fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError> {
        let path = socket_path();
        let mut stream = UnixStream::connect(&path)?;

        stream.set_read_timeout(Some(Duration::from_secs(60)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        let request = json!({
            "jsonrpc": "2.0",
            "id": REQUEST_ID.fetch_add(1, Ordering::SeqCst),
            "method": method,
            "params": params
        });

        writeln!(stream, "{}", serde_json::to_string(&request)?)?;
        stream.flush()?;

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let response: Value = serde_json::from_str(&line)?;

        if let Some(error) = response.get("error") {
            return Err(ClientError::RpcError {
                code: error["code"].as_i64().unwrap_or(-1) as i32,
                message: error["message"].as_str().unwrap_or("Unknown").to_string(),
            });
        }

        response.get("result").cloned().ok_or(ClientError::InvalidResponse)
    }
}

/// Auto-start daemon if not running
pub fn ensure_daemon() -> Result<Client, ClientError> {
    if !Client::is_daemon_running() {
        start_daemon_background()?;
    }
    Client::connect()
}

fn start_daemon_background() -> Result<(), ClientError> {
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe()?;
    Command::new(exe)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    // Wait for startup
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        if Client::is_daemon_running() {
            return Ok(());
        }
    }

    Err(ClientError::DaemonNotRunning)
}
```

## 9. Background Service Patterns

Worker threads with channels (inspired by `below`):

```rust
use std::sync::mpsc;

pub enum WorkerTask {
    CollectSample,
    WriteSample(Sample),
    Shutdown,
}

pub struct Daemon {
    collector_handle: Option<thread::JoinHandle<()>>,
    writer_handle: Option<thread::JoinHandle<()>>,
    task_tx: mpsc::SyncSender<WorkerTask>,
}

impl Daemon {
    pub fn start(config: Config) -> Result<Self, DaemonError> {
        let (task_tx, task_rx) = mpsc::sync_channel::<WorkerTask>(config.buffer_size);

        // Store writer thread
        let writer_handle = thread::Builder::new()
            .name("store-writer".to_string())
            .spawn({
                let store = Store::open(&config.store_dir)?;
                move || {
                    for task in task_rx {
                        match task {
                            WorkerTask::WriteSample(sample) => {
                                if let Err(e) = store.append(&sample) {
                                    eprintln!("Write failed: {}", e);
                                }
                            }
                            WorkerTask::Shutdown => break,
                            _ => {}
                        }
                    }
                }
            })?;

        // Collector thread
        let task_tx_clone = task_tx.clone();
        let collector_handle = thread::Builder::new()
            .name("collector".to_string())
            .spawn(move || {
                loop {
                    let sample = collect_sample();
                    if task_tx_clone.send(WorkerTask::WriteSample(sample)).is_err() {
                        break;
                    }
                    thread::sleep(Duration::from_secs(config.interval));
                }
            })?;

        Ok(Self {
            collector_handle: Some(collector_handle),
            writer_handle: Some(writer_handle),
            task_tx,
        })
    }

    pub fn shutdown(&mut self) {
        let _ = self.task_tx.send(WorkerTask::Shutdown);

        if let Some(h) = self.collector_handle.take() {
            let _ = h.join();
        }
        if let Some(h) = self.writer_handle.take() {
            let _ = h.join();
        }
    }
}
```

## 10. Graceful Shutdown

Signal handling with cleanup:

```rust
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn setup_signal_handler() -> Result<(), Box<dyn std::error::Error>> {
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGQUIT])?;

    thread::Builder::new()
        .name("signal-handler".to_string())
        .spawn(move || {
            for sig in signals.forever() {
                eprintln!("Received signal {}, shutting down...", sig);
                SHUTDOWN.store(true, Ordering::SeqCst);
                break;
            }
        })?;

    Ok(())
}

pub fn should_shutdown() -> bool {
    SHUTDOWN.load(Ordering::SeqCst)
}

// Main loop checking shutdown
pub fn run_daemon() -> Result<()> {
    setup_signal_handler()?;

    let listener = UnixListener::bind(socket_path())?;
    listener.set_nonblocking(true)?;

    while !should_shutdown() {
        match listener.accept() {
            Ok((stream, _)) => {
                // Handle client
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => eprintln!("Accept error: {}", e),
        }
    }

    // Cleanup
    let _ = std::fs::remove_file(socket_path());
    eprintln!("Daemon shutdown complete");
    Ok(())
}

// Scopeguard for cleanup
use scopeguard::defer;

pub fn with_socket_cleanup<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    defer! {
        let _ = std::fs::remove_file(socket_path());
        let _ = std::fs::remove_file(socket_path().with_extension("lock"));
    }

    f()
}
```
