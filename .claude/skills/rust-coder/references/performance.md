# Performance & Systems Programming

## Table of Contents
1. [Complexity Analysis](#1-complexity-analysis)
2. [Memory Efficiency](#2-memory-efficiency)
3. [Threading Patterns](#3-threading-patterns)
4. [Async Runtime](#4-async-runtime)
5. [Channel Patterns](#5-channel-patterns)
6. [Networking](#6-networking)
7. [System Resources](#7-system-resources)

---

## 1. Complexity Analysis

### Time Complexity Guidelines

| Operation | Preferred | Avoid |
|-----------|-----------|-------|
| Lookup | O(1) HashMap, O(log n) BTreeMap | O(n) linear search |
| Insert | O(1) amortized Vec push | O(n) Vec insert at front |
| Iteration | O(n) single pass | O(n²) nested loops on same data |
| Sorting | O(n log n) stable sort | O(n²) bubble/insertion for large n |

### Data Structure Selection

```rust
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

// O(1) lookup, O(n) iteration, unordered
// Use for: pid maps, caches, seen sets
let pid_info: HashMap<u32, PidInfo> = HashMap::new();

// O(log n) lookup, O(n) ordered iteration
// Use for: sorted output, range queries, deterministic iteration
let cgroups: BTreeMap<String, CgroupModel> = BTreeMap::new();

// O(1) membership test
// Use for: dedup, filter sets
let seen_pids: HashSet<u32> = HashSet::new();

// O(log n) membership, ordered iteration
// Use for: sorted unique values, range checks
let cpu_set: BTreeSet<u32> = BTreeSet::new();

// O(1) push/pop both ends
// Use for: sliding windows, recent history
let recent_samples: VecDeque<Sample> = VecDeque::with_capacity(100);

// O(1) index, O(1) amortized push
// Use for: sequential data, known-size collections
let samples: Vec<Sample> = Vec::with_capacity(expected_count);
```

### Algorithmic Patterns

```rust
// GOOD: Single-pass O(n) processing
fn compute_totals(samples: &[Sample]) -> Totals {
    samples.iter().fold(Totals::default(), |mut acc, s| {
        acc.cpu += s.cpu;
        acc.memory += s.memory;
        acc.count += 1;
        acc
    })
}

// BAD: O(n²) nested iteration
fn find_duplicates_bad(items: &[Item]) -> Vec<&Item> {
    let mut dupes = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            if items[i] == items[j] {
                dupes.push(&items[i]);
            }
        }
    }
    dupes
}

// GOOD: O(n) with hash set
fn find_duplicates_good(items: &[Item]) -> Vec<&Item> {
    let mut seen = HashSet::new();
    let mut dupes = Vec::new();
    for item in items {
        if !seen.insert(item) {
            dupes.push(item);
        }
    }
    dupes
}

// GOOD: Binary search for sorted data O(log n)
fn find_sample_at_time(samples: &[Sample], target: SystemTime) -> Option<&Sample> {
    samples
        .binary_search_by_key(&target, |s| s.timestamp)
        .ok()
        .map(|i| &samples[i])
}
```

### Space Complexity

```rust
// O(1) space: Process in streaming fashion
fn sum_large_file(path: &Path) -> Result<u64> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut sum = 0u64;
    for line in reader.lines() {
        sum += line?.parse::<u64>().unwrap_or(0);
    }
    Ok(sum)
}

// O(n) space: Avoid when possible for large data
fn sum_large_file_bad(path: &Path) -> Result<u64> {
    let content = std::fs::read_to_string(path)?; // Loads entire file
    Ok(content.lines().filter_map(|l| l.parse::<u64>().ok()).sum())
}
```

## 2. Memory Efficiency

### Buffer Reuse

```rust
use std::cell::RefCell;

/// Reader with reusable internal buffer
pub struct BufferedReader {
    buffer: RefCell<Vec<u8>>,
}

impl BufferedReader {
    pub fn new() -> Self {
        Self {
            buffer: RefCell::new(Vec::with_capacity(64 * 1024)),
        }
    }

    /// Read file reusing internal buffer
    pub fn read_file(&self, path: &Path) -> Result<&str> {
        let mut buf = self.buffer.borrow_mut();
        buf.clear(); // Reuse capacity, don't reallocate

        let mut file = File::open(path)?;
        file.read_to_end(&mut buf)?;

        // Safety: Caller ensures buffer outlives return
        Ok(unsafe { std::str::from_utf8_unchecked(&buf) })
    }
}

/// Incremental buffer growth
const CHUNK_SIZE: usize = 64 * 1024;

fn read_with_growth(file: &mut File, buf: &mut Vec<u8>) -> Result<()> {
    loop {
        if buf.capacity() - buf.len() < CHUNK_SIZE {
            buf.reserve(CHUNK_SIZE); // Grow by fixed chunks
        }

        let start = buf.len();
        buf.resize(buf.capacity(), 0);

        match file.read(&mut buf[start..]) {
            Ok(0) => {
                buf.truncate(start);
                return Ok(());
            }
            Ok(n) => buf.truncate(start + n),
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
}
```

### Zero-Copy Patterns

```rust
use bytes::Bytes;

/// Zero-copy buffer sharing
pub struct SharedBuffer {
    data: Bytes,
}

impl SharedBuffer {
    pub fn from_vec(vec: Vec<u8>) -> Self {
        Self { data: Bytes::from(vec) }
    }

    /// Slice without copying
    pub fn slice(&self, range: std::ops::Range<usize>) -> Bytes {
        self.data.slice(range)
    }
}

/// Avoid cloning with references
fn process_samples(samples: &[Sample]) -> Summary {
    // Borrow, don't clone
    let max_cpu = samples.iter().map(|s| s.cpu_pct).max();
    let total_mem: u64 = samples.iter().map(|s| s.memory).sum();
    Summary { max_cpu, total_mem }
}

/// Use Cow for conditional ownership
use std::borrow::Cow;

fn normalize_path(path: &str) -> Cow<'_, str> {
    if path.starts_with('/') {
        Cow::Borrowed(path) // No allocation
    } else {
        Cow::Owned(format!("/{}", path)) // Allocate only when needed
    }
}
```

### In-Place Operations

```rust
use std::mem;

/// Drain without cloning
fn take_exit_data(data: &Mutex<PidMap>) -> PidMap {
    let mut guard = data.lock().unwrap();
    mem::take(&mut *guard) // Swap with default, return old value
}

/// Reuse allocations
fn update_model(model: &mut Model, sample: &Sample) {
    model.processes.clear(); // Keep capacity
    for (pid, info) in &sample.processes {
        model.processes.insert(*pid, ProcessModel::from(info));
    }
}

/// Avoid intermediate allocations
fn merge_maps(base: &mut HashMap<u32, Data>, overlay: HashMap<u32, Data>) {
    for (k, v) in overlay {
        base.entry(k).or_insert(v); // Insert only if missing
    }
}
```

### Pre-allocation

```rust
/// Pre-allocate with known size
fn collect_pids(proc_dir: &Path) -> Result<Vec<u32>> {
    let entries: Vec<_> = std::fs::read_dir(proc_dir)?.collect();
    let mut pids = Vec::with_capacity(entries.len());

    for entry in entries {
        if let Ok(pid) = entry?.file_name().to_string_lossy().parse() {
            pids.push(pid);
        }
    }
    Ok(pids)
}

/// Compression buffer sizing
fn compress(data: &[u8]) -> Result<Vec<u8>> {
    let max_size = zstd_safe::compress_bound(data.len());
    let mut output = Vec::with_capacity(max_size);
    // ... compress into output
    Ok(output)
}
```

## 3. Threading Patterns

### Named Thread Spawning

```rust
use std::thread;
use std::thread::JoinHandle;

fn spawn_collector(name: &str, interval: Duration) -> JoinHandle<Result<()>> {
    thread::Builder::new()
        .name(name.to_string())
        .stack_size(2 * 1024 * 1024) // 2MB stack
        .spawn(move || {
            loop {
                collect_sample()?;
                thread::sleep(interval);
            }
        })
        .expect("Failed to spawn thread")
}

/// Spawn multiple workers
fn spawn_workers(count: usize) -> Vec<JoinHandle<()>> {
    (0..count)
        .map(|i| {
            thread::Builder::new()
                .name(format!("worker-{}", i))
                .spawn(move || worker_loop(i))
                .unwrap()
        })
        .collect()
}
```

### Thread-Safe Shared State

```rust
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

/// Read-heavy shared state
pub struct SharedModel {
    model: RwLock<Model>,
    stats: Mutex<Stats>,
}

impl SharedModel {
    /// Many readers, no blocking
    pub fn get_model(&self) -> Model {
        self.model.read().unwrap().clone()
    }

    /// Single writer
    pub fn update_model(&self, new_model: Model) {
        *self.model.write().unwrap() = new_model;
    }

    /// Mutex for stats (frequent writes)
    pub fn increment_count(&self) {
        self.stats.lock().unwrap().count += 1;
    }
}

/// Atomic for simple counters
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct Collector {
    sample_count: AtomicU64,
    running: AtomicBool,
}

impl Collector {
    pub fn increment(&self) {
        self.sample_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}
```

### Graceful Shutdown

```rust
use std::sync::mpsc;

pub struct Service {
    workers: Vec<JoinHandle<()>>,
    shutdown_tx: mpsc::Sender<()>,
}

impl Service {
    pub fn start(worker_count: usize) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        let workers = (0..worker_count)
            .map(|i| {
                let rx = shutdown_rx.clone();
                thread::Builder::new()
                    .name(format!("worker-{}", i))
                    .spawn(move || {
                        loop {
                            match rx.recv_timeout(Duration::from_millis(100)) {
                                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                                Err(mpsc::RecvTimeoutError::Timeout) => {
                                    do_work();
                                }
                            }
                        }
                    })
                    .unwrap()
            })
            .collect();

        Self { workers, shutdown_tx }
    }

    pub fn shutdown(self) {
        drop(self.shutdown_tx); // Signal all workers
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}
```

## 4. Async Runtime

### Tokio Patterns

```rust
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::interval;

/// Async collector with interval
async fn collect_loop(
    mut shutdown_rx: oneshot::Receiver<()>,
    sample_tx: mpsc::Sender<Sample>,
) {
    let mut ticker = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                break;
            }
            _ = ticker.tick() => {
                if let Ok(sample) = collect_sample().await {
                    let _ = sample_tx.send(sample).await;
                }
            }
        }
    }
}

/// Spawn background task
fn start_background_collector(runtime: &tokio::runtime::Runtime) -> oneshot::Sender<()> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (sample_tx, mut sample_rx) = mpsc::channel(100);

    runtime.spawn(collect_loop(shutdown_rx, sample_tx));

    runtime.spawn(async move {
        while let Some(sample) = sample_rx.recv().await {
            process_sample(sample).await;
        }
    });

    shutdown_tx
}
```

### Async I/O

```rust
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;

async fn read_file_async(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    Ok(contents)
}

async fn write_file_async(path: &Path, data: &[u8]) -> Result<()> {
    let mut file = File::create(path).await?;
    file.write_all(data).await?;
    file.sync_all().await?;
    Ok(())
}
```

## 5. Channel Patterns

### Bounded vs Unbounded

```rust
use std::sync::mpsc;

// Bounded: Backpressure, prevents OOM
let (tx, rx) = mpsc::sync_channel::<Sample>(10);

// Unbounded: Use only when producer is slower than consumer
let (tx, rx) = mpsc::channel::<Event>();

/// Writer with backpressure
fn store_writer_loop(rx: mpsc::Receiver<Sample>, store: &mut Store) {
    for sample in rx {
        if let Err(e) = store.append(&sample) {
            error!("Store write failed: {}", e);
        }
    }
}

/// Non-blocking error check
fn check_errors(error_rx: &mpsc::Receiver<Error>) -> Option<Error> {
    match error_rx.recv_timeout(Duration::from_millis(10)) {
        Ok(e) => Some(e),
        Err(mpsc::RecvTimeoutError::Timeout) => None,
        Err(mpsc::RecvTimeoutError::Disconnected) => None,
    }
}
```

### Multi-Producer Patterns

```rust
use std::sync::mpsc;

/// Multiple collectors feeding single writer
fn start_collectors(
    count: usize,
    sample_tx: mpsc::SyncSender<Sample>,
) -> Vec<JoinHandle<()>> {
    (0..count)
        .map(|i| {
            let tx = sample_tx.clone();
            thread::spawn(move || {
                loop {
                    let sample = collect_from_source(i);
                    if tx.send(sample).is_err() {
                        break; // Receiver dropped
                    }
                }
            })
        })
        .collect()
}
```

## 6. Networking

### TCP Client

```rust
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpStream;

pub struct Client {
    stream: BufReader<TcpStream>,
}

impl Client {
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;

        Ok(Self {
            stream: BufReader::new(stream),
        })
    }

    pub fn request(&mut self, req: &[u8]) -> Result<Vec<u8>> {
        self.stream.get_mut().write_all(req)?;
        self.stream.get_mut().flush()?;

        let mut response = Vec::new();
        self.stream.read_until(b'\n', &mut response)?;
        Ok(response)
    }
}
```

### TCP Server

```rust
use std::net::TcpListener;

pub fn start_server(port: u16) -> Result<JoinHandle<()>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    listener.set_nonblocking(false)?;

    Ok(thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(move || handle_client(stream));
                }
                Err(e) => {
                    error!("Accept failed: {}", e);
                }
            }
        }
    }))
}

fn handle_client(mut stream: TcpStream) {
    let peer = stream.peer_addr().ok();
    info!("Client connected: {:?}", peer);

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();

    while reader.read_line(&mut line).is_ok() && !line.is_empty() {
        let response = process_request(&line);
        if stream.write_all(response.as_bytes()).is_err() {
            break;
        }
        line.clear();
    }
}
```

### Unix Domain Sockets

```rust
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;

pub fn start_unix_server(path: &Path) -> Result<JoinHandle<()>> {
    // Remove existing socket
    let _ = std::fs::remove_file(path);

    let listener = UnixListener::bind(path)?;

    Ok(thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                thread::spawn(move || handle_unix_client(stream));
            }
        }
    }))
}

pub fn connect_unix(path: &Path) -> Result<UnixStream> {
    let stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    Ok(stream)
}
```

## 7. System Resources

### Resource Limits

```rust
use nix::sys::resource::Resource;
use nix::sys::resource::getrlimit;
use nix::sys::resource::setrlimit;

/// Raise memory lock limit for BPF
fn raise_memlock_limit() -> Result<()> {
    let limit = 128 * 1024 * 1024; // 128MB
    setrlimit(Resource::RLIMIT_MEMLOCK, limit, limit)?;
    Ok(())
}

/// Check open file limit
fn check_file_limit() -> Result<u64> {
    let (soft, _hard) = getrlimit(Resource::RLIMIT_NOFILE)?;
    Ok(soft)
}
```

### Memory Locking

```rust
use nix::sys::mman::mlockall;
use nix::sys::mman::MlockallFlags;

/// Lock all memory to prevent swapping (requires CAP_IPC_LOCK)
fn lock_memory() -> Result<()> {
    mlockall(MlockallFlags::MCL_CURRENT | MlockallFlags::MCL_FUTURE)?;
    Ok(())
}
```

### Exponential Backoff

```rust
pub struct Backoff {
    current: Duration,
    max: Duration,
    multiplier: u32,
}

impl Backoff {
    pub fn new(initial: Duration, max: Duration) -> Self {
        Self {
            current: initial,
            max,
            multiplier: 2,
        }
    }

    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        self.current = (self.current * self.multiplier).min(self.max);
        delay
    }

    pub fn reset(&mut self, initial: Duration) {
        self.current = initial;
    }
}

/// Usage in collector with transient failures
fn collect_with_backoff(source: &mut Source) -> Option<Data> {
    let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(900));

    loop {
        match source.collect() {
            Ok(data) => {
                backoff.reset(Duration::from_secs(1));
                return Some(data);
            }
            Err(e) if e.is_transient() => {
                let delay = backoff.next_delay();
                warn!("Collection failed, retrying in {:?}: {}", delay, e);
                thread::sleep(delay);
            }
            Err(e) => {
                error!("Collection failed permanently: {}", e);
                return None;
            }
        }
    }
}
```

### Sampling for Hot Paths

```rust
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Execute action every N calls
macro_rules! every_n {
    ($n:expr, $counter:expr, $action:expr) => {{
        let count = $counter.fetch_add(1, Ordering::Relaxed);
        if count % $n == 0 {
            $action
        }
    }};
}

static LOG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hot_path() {
    // Only log every 1000 iterations
    every_n!(1000, LOG_COUNTER, {
        debug!("Hot path executed {} times", LOG_COUNTER.load(Ordering::Relaxed));
    });

    // ... actual work
}
```

---

## Related Patterns

- [Concurrency](concurrency.md) - Threading and async performance
- [Data Structures](data-structures.md) - Memory-efficient containers
- [Polling Patterns](polling-patterns.md) - Backoff and throttling strategies
- [File I/O](file-io.md) - Buffered I/O and memory mapping
