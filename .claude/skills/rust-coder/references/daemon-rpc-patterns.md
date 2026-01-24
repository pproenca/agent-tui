# Daemon, RPC, and Terminal Emulation Patterns

Patterns drawn from production Rust projects: [facebook/below](https://github.com/facebookincubator/below) (2.5k stars), [alacritty](https://github.com/alacritty/alacritty) (56k stars), [vector](https://github.com/vectordotdev/vector) (17k stars), [tikv](https://github.com/tikv/tikv) (15k stars), [ripgrep](https://github.com/BurntSushi/ripgrep) (49k stars).

## Table of Contents
1. [Daemon Lifecycle (below)](#1-daemon-lifecycle-below)
2. [Worker Thread Coordination (below)](#2-worker-thread-coordination-below)
3. [Signal Handling (below, vector)](#3-signal-handling-below-vector)
4. [Error Channel Aggregation (below)](#4-error-channel-aggregation-below)
5. [gRPC Service Patterns (tikv)](#5-grpc-service-patterns-tikv)
6. [PTY Handling (alacritty)](#6-pty-handling-alacritty)
7. [Store/Persistence Layer (below)](#7-storepersistence-layer-below)
8. [TUI View State (below)](#8-tui-view-state-below)
9. [Graceful Shutdown (vector)](#9-graceful-shutdown-vector)
10. [CLI Entry Patterns (ripgrep)](#10-cli-entry-patterns-ripgrep)

---

## 1. Daemon Lifecycle (below)

Service mode enum and command dispatch:

```rust
/// Service activation state
pub enum Service {
    On(Option<u16>),  // Network service enabled with optional port
    Off,              // Standalone operation
}

/// Command dispatch with run wrapper
enum Command {
    Live { interval_s: u64, host: Option<String>, port: Option<u16> },
    Record { interval_s: u64, retain_for_s: Option<u64>, /* ... */ },
    Replay { time: String, host: Option<String>, port: Option<u16> },
    Dump { /* ... */ },
}

fn main() {
    let opts = Opt::parse();

    match &opts.cmd {
        Command::Record { interval_s, retain_for_s, port, /* ... */ } => {
            run(
                debug,
                &below_config,
                Service::On(*port),
                |logger, errs| record(logger, errs, *interval_s, *retain_for_s),
            )
        }
        Command::Live { interval_s, host, port } => {
            run(
                debug,
                &below_config,
                Service::Off,
                |logger, errs| live(logger, errs, *interval_s, host, *port),
            )
        }
        // ...
    }
}

/// Unified run wrapper with error channel setup
fn run<F>(debug: bool, config: &Config, service: Service, command: F) -> i32
where
    F: FnOnce(slog::Logger, Receiver<Error>) -> Result<()>,
{
    let logger = setup_logger(debug);
    let (err_sender, err_receiver) = channel();

    // Setup signal handler sending to err_sender
    setup_signal_handler(logger.clone(), err_sender.clone());

    let res = command(logger.clone(), err_receiver);

    match res {
        Ok(()) => 0,
        Err(e) => {
            error!(logger, "{:#}", e);
            1
        }
    }
}
```

## 2. Worker Thread Coordination (below)

Named threads with bounded channels and scopeguard cleanup:

```rust
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread::{self, JoinHandle};
use scopeguard::guard;

pub enum WorkerTask {
    WriteSample(DataFrame),
    NewShard,
    Shutdown,
}

fn start_store_writer_thread(
    logger: slog::Logger,
    mut store: StoreWriter,
    buffer_size: usize,
) -> Result<(JoinHandle<()>, SyncSender<WorkerTask>)> {
    // Bounded channel provides backpressure
    let (send_task, recv_task) = sync_channel::<WorkerTask>(buffer_size);

    let handle = thread::Builder::new()
        .name("store_writer".to_owned())
        .spawn(move || {
            for task in recv_task {
                match task {
                    WorkerTask::WriteSample(sample) => {
                        if let Err(e) = store.put(&sample) {
                            error!(logger, "Write failed: {:#}", e);
                        }
                    }
                    WorkerTask::NewShard => {
                        if let Err(e) = store.discard_earlier(retention_cutoff) {
                            warn!(logger, "Discard failed: {:#}", e);
                        }
                    }
                    WorkerTask::Shutdown => break,
                }
            }
        })?;

    Ok((handle, send_task))
}

/// Scopeguard ensures thread cleanup when sender drops
fn start_collection_loop(
    logger: slog::Logger,
    store_sender: SyncSender<WorkerTask>,
    writer_thread: JoinHandle<()>,
    interval: Duration,
    errs: Receiver<Error>,
) -> Result<()> {
    // Guard joins writer thread when sender drops
    let store_sender = guard(store_sender, |s| {
        drop(s);
        let _ = writer_thread.join();
    });

    loop {
        // Check for shutdown signals
        match errs.recv_timeout(interval) {
            Ok(e) => bail!(e),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => bail!("Error channel disconnected"),
        }

        let sample = collect_sample()?;
        store_sender.send(WorkerTask::WriteSample(sample))?;
    }
}
```

Collector thread with exponential backoff:

```rust
fn start_collector_thread(
    logger: slog::Logger,
    target_interval: Duration,
) -> Result<Receiver<Sample>> {
    let (tx, rx) = channel();

    thread::Builder::new()
        .name("collector".to_owned())
        .spawn(move || {
            let mut interval = target_interval;
            let mut backoff_count = 0;

            loop {
                thread::sleep(interval);

                match collect() {
                    Ok(sample) => {
                        interval = target_interval;
                        backoff_count = 0;
                        if tx.send(sample).is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(e) => {
                        // Exponential backoff on failure
                        backoff_count += 1;
                        interval = target_interval * 2u32.pow(backoff_count.min(5));
                        warn!(logger, "Collection failed, backing off: {:#}", e);
                    }
                }
            }
        })?;

    Ok(rx)
}
```

## 3. Signal Handling (below, vector)

below-style signal handler with error channel:

```rust
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;

#[derive(Clone, Debug)]
struct StopSignal {
    signal: i32,
}

impl std::error::Error for StopSignal {}

impl std::fmt::Display for StopSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Stopped by signal: {}", self.signal)
    }
}

fn setup_signal_handler(
    logger: slog::Logger,
    err_sender: Sender<anyhow::Error>,
) -> Result<()> {
    match Signals::new([SIGINT, SIGTERM]) {
        Ok(mut signals) => {
            thread::Builder::new()
                .name("sighandler".to_owned())
                .spawn(move || {
                    let mut term_now = false;
                    for signal in signals.forever() {
                        if term_now {
                            error!(logger, "Force exit after second signal");
                            std::process::exit(1);
                        }
                        info!(logger, "Received signal {}, shutting down...", signal);
                        term_now = true;
                        let _ = err_sender.send(anyhow::anyhow!(StopSignal { signal }));
                    }
                })?;
        }
        Err(e) => {
            warn!(logger, "Failed to setup signal handler: {:#}", e);
        }
    }
    Ok(())
}
```

vector-style broadcast channel for signals:

```rust
use tokio::sync::broadcast;

pub enum SignalTo {
    ReloadFromDisk,
    ReloadComponents,
    Shutdown(Option<String>),
    Quit,
}

async fn run_with_signals(
    mut signal_rx: broadcast::Receiver<SignalTo>,
    mut graceful_crash: mpsc::Receiver<String>,
) -> ExitStatus {
    let signal = loop {
        tokio::select! {
            signal = signal_rx.recv() => {
                if let Ok(signal) = signal {
                    match handle_signal(signal).await {
                        Some(s) => break s,
                        None => continue,
                    }
                }
            }
            error = graceful_crash.recv() => {
                break SignalTo::Shutdown(error);
            }
        }
    };

    match signal {
        SignalTo::Shutdown(reason) => {
            info!("Shutting down: {:?}", reason);
            graceful_stop().await
        }
        SignalTo::Quit => {
            warn!("Force quit requested");
            ExitStatus::from(1)
        }
        _ => ExitStatus::from(0),
    }
}
```

## 4. Error Channel Aggregation (below)

Single channel aggregates errors from multiple sources:

```rust
use std::sync::mpsc::{channel, Receiver, Sender};

fn run_daemon(config: &Config, logger: slog::Logger) -> Result<()> {
    let (err_sender, err_receiver) = channel::<anyhow::Error>();

    // Signal handler sends to err_sender
    setup_signal_handler(logger.clone(), err_sender.clone())?;

    // BPF errors send to err_sender
    let (exit_buffer, bpf_err_rx) = start_exitstat(logger.clone());
    if let Some(rx) = bpf_err_rx {
        let sender = err_sender.clone();
        thread::spawn(move || {
            if let Ok(e) = rx.recv() {
                let _ = sender.send(e.into());
            }
        });
    }

    // Main loop polls err_receiver
    main_loop(logger, err_receiver, exit_buffer)
}

fn main_loop(
    logger: slog::Logger,
    errs: Receiver<anyhow::Error>,
    exit_buffer: Arc<Mutex<PidMap>>,
) -> Result<()> {
    let interval = Duration::from_secs(5);

    loop {
        // Non-blocking check for errors
        match errs.recv_timeout(interval) {
            Ok(e) => {
                // Graceful shutdown on any error
                info!(logger, "Stopping: {:#}", e);
                return Ok(());
            }
            Err(RecvTimeoutError::Timeout) => {
                // Normal operation - collect and write
            }
            Err(RecvTimeoutError::Disconnected) => {
                bail!("Error channel unexpectedly disconnected");
            }
        }

        // Collect data
        let sample = collect_sample(&exit_buffer)?;
        process_sample(sample)?;
    }
}
```

## 5. gRPC Service Patterns (tikv)

Service struct with dependencies:

```rust
use tonic::{Request, Response, Status};

pub struct Service<E: Engine, L: LockManager> {
    cluster_id: u64,
    store_id: u64,
    storage: Storage<E, L>,
    copr: Endpoint<E>,
}

impl<E: Engine, L: LockManager> Service<E, L> {
    pub fn new(
        cluster_id: u64,
        store_id: u64,
        storage: Storage<E, L>,
        copr: Endpoint<E>,
    ) -> Self {
        Self { cluster_id, store_id, storage, copr }
    }
}
```

Request handler macro pattern:

```rust
macro_rules! handle_request {
    ($fn_name:ident, $future_fn:ident, $req_ty:ty, $resp_ty:ty) => {
        fn $fn_name(
            &mut self,
            ctx: RpcContext<'_>,
            req: $req_ty,
            sink: UnarySink<$resp_ty>,
        ) {
            // Validate cluster ID
            if !self.check_cluster_id(&req, &ctx) {
                return;
            }

            let begin = Instant::now();
            let task = self
                .$future_fn(req)
                .map(move |v| {
                    let elapsed = begin.elapsed();
                    GRPC_MSG_HISTOGRAM
                        .$fn_name
                        .observe(elapsed.as_secs_f64());

                    let mut resp = match v {
                        Ok(resp) => resp,
                        Err(e) => {
                            let mut resp = <$resp_ty>::default();
                            resp.set_error(extract_error(&e));
                            resp
                        }
                    };
                    resp
                });

            ctx.spawn(task.then(move |resp| sink.success(resp)));
        }
    };
}

// Usage
handle_request!(kv_get, future_get, GetRequest, GetResponse);
handle_request!(kv_put, future_put, PutRequest, PutResponse);
```

Region error extraction:

```rust
fn extract_region_error<T>(res: &Result<T, StorageError>) -> Option<RegionError> {
    match res {
        Err(StorageError::Txn(TxnError::Engine(EngineError::Request(e)))) => {
            Some(e.clone())
        }
        Err(StorageError::Txn(TxnError::Mvcc(MvccError::Engine(
            EngineError::Request(e),
        )))) => Some(e.clone()),
        _ => None,
    }
}

fn build_response<T>(result: Result<T, StorageError>) -> Response<T::Response>
where
    T: IntoResponse,
{
    let mut resp = T::Response::default();

    if let Some(region_err) = extract_region_error(&result) {
        resp.set_region_error(region_err);
    } else {
        match result {
            Ok(data) => resp.set_data(data),
            Err(e) => resp.set_error(extract_key_error(&e)),
        }
    }

    Response::new(resp)
}
```

## 6. PTY Handling (alacritty)

PTY creation with rustix:

```rust
use rustix::pty::{openpty, Winsize};
use std::os::unix::io::{AsRawFd, OwnedFd, RawFd};

pub struct Pty {
    master: OwnedFd,
    child: Child,
}

impl Pty {
    pub fn new(
        config: &PtyConfig,
        window_size: WindowSize,
        working_dir: Option<&Path>,
    ) -> Result<Self, Error> {
        let winsize = window_size.to_winsize();

        // Open PTY pair
        let pty = openpty(None, Some(&winsize))?;
        let (master, slave) = (pty.controller, pty.user);

        let master_fd = master.as_raw_fd();
        let slave_fd = slave.as_raw_fd();

        // Fork child process
        let mut builder = Command::new(&config.shell.program);
        builder
            .args(&config.shell.args)
            .stdin(unsafe { Stdio::from_raw_fd(slave_fd) })
            .stdout(unsafe { Stdio::from_raw_fd(slave_fd) })
            .stderr(unsafe { Stdio::from_raw_fd(slave_fd) });

        if let Some(dir) = working_dir {
            builder.current_dir(dir);
        }

        // pre_exec runs in child after fork, before exec
        unsafe {
            builder.pre_exec(move || {
                // Create new session
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                // Set controlling terminal
                if libc::ioctl(slave_fd, libc::TIOCSCTTY, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                // Close FDs
                libc::close(slave_fd);
                libc::close(master_fd);

                Ok(())
            });
        }

        let child = builder.spawn()?;

        Ok(Self { master, child })
    }

    pub fn resize(&self, size: WindowSize) -> Result<(), Error> {
        let winsize = size.to_winsize();

        let res = unsafe {
            libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &winsize)
        };

        if res < 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        Ok(())
    }
}

impl WindowSize {
    fn to_winsize(self) -> Winsize {
        Winsize {
            ws_row: self.num_lines as libc::c_ushort,
            ws_col: self.num_cols as libc::c_ushort,
            ws_xpixel: (self.num_cols * self.cell_width) as libc::c_ushort,
            ws_ypixel: (self.num_lines * self.cell_height) as libc::c_ushort,
        }
    }
}
```

Signal pipe for SIGCHLD:

```rust
use signal_hook::low_level::pipe as signal_pipe;

fn setup_signals() -> Result<(UnixStream, signal_hook::SigId), Error> {
    let (sender, receiver) = UnixStream::pair()?;
    receiver.set_nonblocking(true)?;

    let sig_id = signal_pipe::register(libc::SIGCHLD, sender)?;

    Ok((receiver, sig_id))
}

// Non-blocking FD setup
unsafe fn set_nonblocking(fd: RawFd) {
    let flags = libc::fcntl(fd, libc::F_GETFL, 0);
    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
}
```

## 7. Store/Persistence Layer (below)

Store trait with direction-based traversal:

```rust
use std::time::SystemTime;

pub enum Direction {
    Forward,
    Reverse,
}

pub trait Store: Send + Sync {
    type SampleType;

    fn get_sample_at_timestamp(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
    ) -> Result<Option<(SystemTime, Self::SampleType)>>;
}
```

StoreWriter with append-only semantics:

```rust
pub struct StoreWriter {
    logger: slog::Logger,
    dir: PathBuf,
    index: File,
    data: File,
    data_len: u64,
    shard: u64,
    compressor: Option<zstd::bulk::Compressor<'static>>,
    compression_mode: CompressionMode,
}

pub enum CompressionMode {
    None,
    Zstd,
    ZstdDictionary(ChunkSizePo2),
}

impl StoreWriter {
    pub fn new(dir: PathBuf, compression: CompressionMode) -> Result<Self> {
        let shard = calculate_shard(SystemTime::now());
        let (index, data) = Self::open_shard_files(&dir, shard)?;

        // Acquire exclusive lock
        nix::fcntl::flock(
            index.as_raw_fd(),
            nix::fcntl::FlockArg::LockExclusiveNonblock,
        )?;

        Ok(Self {
            logger: slog::Logger::root(slog::Discard, slog::o!()),
            dir,
            index,
            data,
            data_len: 0,
            shard,
            compressor: None,
            compression_mode: compression,
        })
    }

    pub fn put(&mut self, sample: &DataFrame) -> Result<bool> {
        let timestamp = sample.timestamp;
        let new_shard = calculate_shard(timestamp);

        // Rotate to new shard if needed
        let shard_changed = if new_shard != self.shard {
            self.rotate_shard(new_shard)?;
            true
        } else {
            false
        };

        // Serialize
        let frame_bytes = serde_cbor::to_vec(sample)?;

        // Compress if enabled
        let data = match &mut self.compressor {
            Some(c) => c.compress(&frame_bytes)?,
            None => frame_bytes,
        };

        // Write data
        let offset = self.data_len;
        self.data.write_all(&data)?;
        self.data_len += data.len() as u64;

        // Write index entry with CRC
        let entry = IndexEntry {
            timestamp: get_unix_timestamp(timestamp),
            offset,
            len: data.len() as u32,
            flags: self.compression_flags(),
            data_crc: data.crc32(),
            index_crc: 0, // Computed below
        };

        let entry_bytes = entry.to_bytes_with_crc();
        self.index.write_all(&entry_bytes)?;

        Ok(shard_changed)
    }
}
```

CRC validation:

```rust
const CRC32_TABLE: [u32; 256] = /* precomputed */;

trait Crc32 {
    fn crc32(&self) -> u32;
}

impl Crc32 for [u8] {
    fn crc32(&self) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for byte in self {
            crc = (crc >> 8) ^ CRC32_TABLE[((crc & 0xFF) as u8 ^ *byte) as usize];
        }
        crc
    }
}

#[repr(C)]
struct IndexEntry {
    timestamp: u64,
    offset: u64,
    len: u32,
    flags: IndexEntryFlags,
    data_crc: u32,
    index_crc: u32,
}

const INDEX_ENTRY_SIZE: usize = 32;
```

Time-based sharding:

```rust
const SHARD_TIME: u64 = 24 * 60 * 60; // 24 hours

fn calculate_shard(timestamp: SystemTime) -> u64 {
    let secs = timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    secs - (secs % SHARD_TIME)
}

impl StoreWriter {
    pub fn discard_earlier(&mut self, cutoff: SystemTime) -> Result<()> {
        let cutoff_shard = calculate_shard(cutoff);

        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name();
            if let Some(shard) = parse_shard_filename(&name) {
                if shard < cutoff_shard {
                    std::fs::remove_file(entry.path())?;
                }
            }
        }

        Ok(())
    }
}
```

## 8. TUI View State (below)

View state with Arc<Mutex> shared data:

```rust
use cursive::Cursive;
use cursive::views::ScreensView;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct ViewState {
    pub time_elapsed: Duration,
    pub timestamp: SystemTime,

    // Thread-safe model references
    pub model: Arc<Mutex<Model>>,
    pub system: Arc<Mutex<SystemModel>>,
    pub cgroup: Arc<Mutex<CgroupModel>>,
    pub process: Arc<Mutex<ProcessModel>>,

    // View management
    pub main_view_state: MainViewState,
    pub main_view_screens: HashMap<String, ScreenId>,
    pub mode: ViewMode,

    // Event handling
    pub event_controllers: Arc<Mutex<HashMap<Event, Controllers>>>,
    pub cmd_controllers: Arc<Mutex<HashMap<&'static str, Controllers>>>,
}

pub enum ViewMode {
    Live(Arc<Mutex<Advance>>),
    Pause(Arc<Mutex<Advance>>),
    Replay(Arc<Mutex<Advance>>),
}

pub enum MainViewState {
    Cgroup,
    Process(ProcessZoomState),
    System,
}

impl ViewState {
    pub fn update(&mut self, model: Model) {
        self.time_elapsed = model.time_elapsed;
        self.timestamp = model.timestamp;
        *self.model.lock().unwrap() = model.clone();
        *self.system.lock().unwrap() = model.system;
        *self.cgroup.lock().unwrap() = model.cgroup;
        *self.process.lock().unwrap() = model.process;
    }
}
```

Screen registration and switching:

```rust
fn setup_screens(siv: &mut Cursive, state: &mut ViewState) {
    let mut screens_view = ScreensView::new();

    // Register each view panel
    state.main_view_screens.insert(
        "cgroup".to_owned(),
        screens_view.add_screen(BoxedView::boxed(
            ResizedView::with_full_screen(CgroupView::new()),
        )),
    );

    state.main_view_screens.insert(
        "process".to_owned(),
        screens_view.add_screen(BoxedView::boxed(
            ResizedView::with_full_screen(ProcessView::new()),
        )),
    );

    siv.add_fullscreen_layer(screens_view.with_name("main_view_screens"));
}

pub fn set_active_screen(c: &mut Cursive, name: &str) {
    let state = c.user_data::<ViewState>().unwrap();
    if let Some(&screen_id) = state.main_view_screens.get(name) {
        c.call_on_name("main_view_screens", |screens: &mut ScreensView| {
            screens.set_active_screen(screen_id);
        });
    }
}
```

Refresh propagation:

```rust
fn refresh(c: &mut Cursive) {
    // Always refresh common views
    status_bar::refresh(c);
    summary_view::refresh(c);

    // Refresh active view based on state
    let state = c.user_data::<ViewState>().unwrap();
    match &state.main_view_state {
        MainViewState::Cgroup => CgroupView::refresh(c),
        MainViewState::Process(_) => ProcessView::refresh(c),
        MainViewState::System => SystemView::refresh(c),
    }
}
```

## 9. Graceful Shutdown (vector)

Dual-timeout shutdown pattern:

```rust
use tokio::time::timeout;

pub async fn graceful_stop(
    topology_controller: TopologyController,
    mut signal_rx: broadcast::Receiver<SignalTo>,
) -> ExitStatus {
    emit!(ApplicationStopping);

    tokio::select! {
        // Graceful shutdown path
        result = topology_controller.stop() => {
            match result {
                Ok(()) => {
                    emit!(ApplicationStopped);
                    ExitStatus::from(0)
                }
                Err(e) => {
                    error!("Shutdown error: {:#}", e);
                    ExitStatus::from(1)
                }
            }
        }
        // Force quit on second signal
        _ = signal_rx.recv() => {
            warn!("Force quit on second signal");
            ExitStatus::from(1)
        }
    }
}
```

Worker thread count with atomic single-init:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

static WORKER_THREADS: AtomicUsize = AtomicUsize::new(0);

pub fn set_worker_threads(threads: usize) -> Result<(), &'static str> {
    WORKER_THREADS
        .compare_exchange(0, threads, Ordering::SeqCst, Ordering::Relaxed)
        .map(|_| ())
        .map_err(|_| "Worker threads already initialized")
}

pub fn worker_threads() -> usize {
    let threads = WORKER_THREADS.load(Ordering::Relaxed);
    if threads == 0 {
        num_cpus::get()
    } else {
        threads
    }
}
```

## 10. CLI Entry Patterns (ripgrep)

Three-state argument parsing:

```rust
use std::process::ExitCode;

pub enum ParseResult<T> {
    Ok(T),
    Err(anyhow::Error),
    Special(SpecialMode),
}

pub enum SpecialMode {
    Help,
    Version,
    ShortHelp,
}

fn main() -> ExitCode {
    match run(flags::parse()) {
        Ok(code) => code,
        Err(err) => handle_error(err),
    }
}

fn run(result: ParseResult<Args>) -> anyhow::Result<ExitCode> {
    let args = match result {
        ParseResult::Err(err) => return Err(err),
        ParseResult::Special(mode) => return special(mode),
        ParseResult::Ok(args) => args,
    };

    // Main logic
    let matched = search(&args)?;

    Ok(if matched && !messages::errored() {
        ExitCode::from(0)
    } else if messages::errored() {
        ExitCode::from(2)
    } else {
        ExitCode::from(1)
    })
}
```

Graceful broken pipe handling:

```rust
fn handle_error(err: anyhow::Error) -> ExitCode {
    // Check for broken pipe (common when piping to head/less)
    for cause in err.chain() {
        if let Some(ioerr) = cause.downcast_ref::<std::io::Error>() {
            if ioerr.kind() == std::io::ErrorKind::BrokenPipe {
                return ExitCode::from(0);
            }
        }
    }

    // Print error and exit with error code
    eprintln!("{:#}", err);
    ExitCode::from(2)
}
```

Exit code conventions:
- `0`: Success (found matches or completed successfully)
- `1`: No results (search completed but no matches)
- `2`: Error occurred

---

## Related Patterns

- [Error Handling](error-handling.md) - Error types, context, and graceful degradation
- [Concurrency](concurrency.md) - Thread coordination, channels, and synchronization
- [Signal Handling](concurrency.md) - Interrupt handling patterns
- [CLI Patterns](cli-patterns.md) - Command dispatch and argument parsing
- [TUI Patterns](tui-patterns.md) - View state management and screen handling
- [Serialization](serialization.md) - CBOR storage and data persistence
- [Polling Patterns](polling-patterns.md) - Wait conditions and backoff strategies
