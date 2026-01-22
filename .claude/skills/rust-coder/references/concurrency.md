# Concurrency Patterns

## Table of Contents
1. [Thread Spawning with Names](#1-thread-spawning-with-names)
2. [Arc-Mutex Shared State](#2-arc-mutex-shared-state)
3. [Channel Communication](#3-channel-communication)
4. [Scopeguard Cleanup](#4-scopeguard-cleanup)
5. [Condvar Signaling](#5-condvar-signaling)
6. [Atomic Exit Data](#6-atomic-exit-data)
7. [Thread-Safe Singletons](#7-thread-safe-singletons)

---

## 1. Thread Spawning with Names

Always name threads for debugging:

```rust
use std::thread;
use std::thread::JoinHandle;

fn spawn_collector(name: &str, work: impl FnOnce() + Send + 'static) -> JoinHandle<()> {
    thread::Builder::new()
        .name(name.to_string())
        .spawn(work)
        .expect("Failed to spawn thread")
}

fn spawn_named<F, T>(name: &str, f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    thread::Builder::new()
        .name(name.to_string())
        .stack_size(2 * 1024 * 1024)  // 2MB stack
        .spawn(f)
        .expect("thread spawn failed")
}

// Usage
let collector_handle = spawn_named("data-collector", move || {
    loop {
        collect_sample();
        thread::sleep(Duration::from_secs(1));
    }
});

let writer_handle = spawn_named("store-writer", move || {
    for sample in rx {
        store.append(&sample).unwrap();
    }
});
```

## 2. Arc-Mutex Shared State

Share mutable state safely:

```rust
use std::sync::Arc;
use std::sync::Mutex;

pub struct ViewState {
    pub model: Arc<Mutex<Model>>,
    pub collapsed: Arc<Mutex<HashSet<String>>>,
    pub selection: Arc<Mutex<String>>,
}

impl ViewState {
    pub fn new(model: Model) -> Self {
        Self {
            model: Arc::new(Mutex::new(model)),
            collapsed: Arc::new(Mutex::new(HashSet::new())),
            selection: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn update_model(&self, new_model: Model) {
        let mut model = self.model.lock().unwrap();
        *model = new_model;
    }

    pub fn toggle_collapsed(&self, path: &str) {
        let mut collapsed = self.collapsed.lock().unwrap();
        if collapsed.contains(path) {
            collapsed.remove(path);
        } else {
            collapsed.insert(path.to_string());
        }
    }
}

// Clone Arc for thread sharing
let state = Arc::new(ViewState::new(initial_model));
let state_clone = Arc::clone(&state);

thread::spawn(move || {
    loop {
        let new_model = collect_model();
        state_clone.update_model(new_model);
    }
});
```

## 3. Channel Communication

Use channels for inter-thread messaging:

```rust
use std::sync::mpsc;
use tokio::sync::mpsc as tokio_mpsc;

// Bounded channel for backpressure
fn start_pipeline() -> Result<()> {
    let (tx, rx) = mpsc::sync_channel::<Sample>(100);

    // Producer
    let producer = spawn_named("producer", move || {
        loop {
            let sample = collect_sample();
            if tx.send(sample).is_err() {
                break; // Receiver dropped
            }
        }
    });

    // Consumer
    let consumer = spawn_named("consumer", move || {
        for sample in rx {
            process_sample(sample);
        }
    });

    producer.join().unwrap();
    consumer.join().unwrap();
    Ok(())
}

// Shutdown signal
fn with_shutdown<F>(work: F) -> JoinHandle<()>
where
    F: FnOnce(mpsc::Receiver<()>) + Send + 'static,
{
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    // Store tx somewhere to send shutdown signal later
    SHUTDOWN_TX.set(shutdown_tx).unwrap();

    spawn_named("worker", move || work(shutdown_rx))
}

// Async channel with tokio
async fn async_pipeline(mut shutdown_rx: tokio_mpsc::Receiver<()>) {
    let (tx, mut rx) = tokio_mpsc::channel::<Sample>(100);

    tokio::spawn(async move {
        while let Some(sample) = rx.recv().await {
            process_sample(sample).await;
        }
    });

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => break,
            sample = collect_sample_async() => {
                let _ = tx.send(sample).await;
            }
        }
    }
}
```

## 4. Scopeguard Cleanup

Ensure cleanup on all exit paths:

```rust
use scopeguard::guard;
use scopeguard::defer;

fn with_temp_file<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&Path) -> Result<T>,
{
    let path = tempfile::NamedTempFile::new()?.into_temp_path();

    // Cleanup on drop (including panic)
    let _guard = guard(path.to_path_buf(), |p| {
        let _ = std::fs::remove_file(p);
    });

    f(&path)
}

fn with_lock_file<F, T>(lock_path: &Path, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let lock = std::fs::File::create(lock_path)?;
    lock.lock_exclusive()?;

    // Release lock on scope exit
    defer! {
        let _ = lock.unlock();
        let _ = std::fs::remove_file(lock_path);
    }

    f()
}

fn setup_signal_handler() {
    let original_handler = signal::signal(Signal::SIGINT, SigHandler::SigIgn);

    let _guard = guard(original_handler, |h| {
        if let Ok(handler) = h {
            let _ = signal::signal(Signal::SIGINT, handler);
        }
    });

    // Do work that shouldn't be interrupted
    do_critical_work();
}
```

## 5. Condvar Signaling

Coordinate threads with condition variables:

```rust
use std::sync::Condvar;

pub struct Notifier {
    data: Mutex<Option<Notification>>,
    condvar: Condvar,
}

impl Notifier {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(None),
            condvar: Condvar::new(),
        }
    }

    pub fn notify(&self, notification: Notification) {
        let mut guard = self.data.lock().unwrap();
        *guard = Some(notification);
        self.condvar.notify_all();
    }

    pub fn wait(&self) -> Notification {
        let mut guard = self.data.lock().unwrap();
        while guard.is_none() {
            guard = self.condvar.wait(guard).unwrap();
        }
        guard.take().unwrap()
    }

    pub fn wait_timeout(&self, timeout: Duration) -> Option<Notification> {
        let mut guard = self.data.lock().unwrap();
        let (mut guard, result) = self.condvar
            .wait_timeout_while(guard, timeout, |data| data.is_none())
            .unwrap();

        if result.timed_out() {
            None
        } else {
            guard.take()
        }
    }
}

// Usage
let notifier = Arc::new(Notifier::new());
let notifier_clone = Arc::clone(&notifier);

// Waiter thread
thread::spawn(move || {
    loop {
        let notification = notifier_clone.wait();
        handle_notification(notification);
    }
});

// Notifier
notifier.notify(Notification::DataReady);
```

## 6. Atomic Exit Data

Share process exit info safely:

```rust
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub struct Collector {
    /// Shared exit data from process reaper
    exit_data: Arc<Mutex<PidMap>>,
    /// Flag to stop collection
    running: Arc<AtomicBool>,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            exit_data: Arc::new(Mutex::new(PidMap::new())),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn get_exit_data(&self) -> PidMap {
        // Atomically drain exit data
        std::mem::take(&mut *self.exit_data.lock().unwrap())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn run(&self) {
        while self.running.load(Ordering::SeqCst) {
            let sample = self.collect();
            thread::sleep(Duration::from_secs(1));
        }
    }
}

// Process reaper adds exit data
fn on_process_exit(collector: &Collector, pid: u32, info: PidInfo) {
    collector.exit_data.lock().unwrap().insert(pid, info);
}
```

## 7. Thread-Safe Singletons

Global state with OnceLock:

```rust
use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();
static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_config(config: Config) -> Result<()> {
    CONFIG.set(config)
        .map_err(|_| anyhow::anyhow!("Config already initialized"))
}

pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}

pub fn init_logger(logger: Logger) {
    let _ = LOGGER.set(logger);
}

pub fn logger() -> &'static Logger {
    LOGGER.get_or_init(|| {
        // Fallback to default logger
        Logger::root(slog::Discard, slog::o!())
    })
}

// Lazy initialization with closure
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    })
}
```
