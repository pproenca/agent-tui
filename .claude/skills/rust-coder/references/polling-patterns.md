# Polling, Waiting, and Retry Patterns

Patterns drawn from production Rust projects: [tokio](https://github.com/tokio-rs/tokio) (27k stars), [reqwest](https://github.com/seanmonstar/reqwest) (10k stars), [fd](https://github.com/sharkdp/fd) (35k stars), [ripgrep](https://github.com/BurntSushi/ripgrep) (49k stars).

## Table of Contents
1. [Condition-Based Polling](#1-condition-based-polling)
2. [Timeout Wrapping (tokio)](#2-timeout-wrapping-tokio)
3. [Multi-Tier Timeouts (reqwest)](#3-multi-tier-timeouts-reqwest)
4. [Stability Detection](#4-stability-detection)
5. [Exponential Backoff](#5-exponential-backoff)
6. [Cooperative Cancellation (fd)](#6-cooperative-cancellation-fd)
7. [Strategy-Based Condition Checking (ripgrep)](#7-strategy-based-condition-checking-ripgrep)
8. [Deadline Management](#8-deadline-management)

---

## 1. Condition-Based Polling

Generic wait-until pattern with configurable conditions:

```rust
use std::time::{Duration, Instant};
use std::thread;

#[derive(Debug, Clone)]
pub enum WaitCondition {
    Text(String),
    Element(String),
    Predicate(String),  // Named predicate
    Stable,
    Gone(String),
}

impl WaitCondition {
    pub fn parse(condition: Option<&str>, target: Option<&str>) -> Option<Self> {
        match condition {
            Some("text") => target.map(|t| Self::Text(t.to_string())),
            Some("element") => target.map(|t| Self::Element(t.to_string())),
            Some("stable") => Some(Self::Stable),
            Some("gone") => target.map(|t| Self::Gone(t.to_string())),
            None => target.map(|t| Self::Text(t.to_string())),
            _ => None,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Text(t) => format!("text \"{}\"", t),
            Self::Element(e) => format!("element {}", e),
            Self::Predicate(p) => format!("predicate {}", p),
            Self::Stable => "state to stabilize".to_string(),
            Self::Gone(t) => format!("\"{}\" to disappear", t),
        }
    }
}

pub struct WaitResult {
    pub success: bool,
    pub elapsed: Duration,
    pub polls: u32,
}

pub fn wait_until<F>(
    condition: &WaitCondition,
    mut check: F,
    timeout: Duration,
    poll_interval: Duration,
) -> WaitResult
where
    F: FnMut(&WaitCondition) -> bool,
{
    let start = Instant::now();
    let deadline = start + timeout;
    let mut polls = 0;

    loop {
        polls += 1;

        if check(condition) {
            return WaitResult {
                success: true,
                elapsed: start.elapsed(),
                polls,
            };
        }

        if Instant::now() >= deadline {
            return WaitResult {
                success: false,
                elapsed: start.elapsed(),
                polls,
            };
        }

        thread::sleep(poll_interval);
    }
}
```

## 2. Timeout Wrapping (tokio)

Async timeout wrapper pattern:

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub struct Timeout<F> {
    value: F,
    deadline: Instant,
}

impl<F> Timeout<F> {
    pub fn new(value: F, duration: Duration) -> Self {
        // Handle overflow gracefully
        let deadline = Instant::now()
            .checked_add(duration)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400 * 365));

        Self { value, deadline }
    }

    /// Consume timeout and return inner future
    pub fn into_inner(self) -> F {
        self.value
    }
}

#[derive(Debug)]
pub struct Elapsed;

impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, Elapsed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        // Try the inner future first
        let value = unsafe { Pin::new_unchecked(&mut this.value) };
        if let Poll::Ready(v) = value.poll(cx) {
            return Poll::Ready(Ok(v));
        }

        // Check deadline only if inner is pending
        if Instant::now() >= this.deadline {
            Poll::Ready(Err(Elapsed))
        } else {
            // Schedule wakeup near deadline
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// Convenience function
pub fn timeout<F: Future>(duration: Duration, future: F) -> Timeout<F> {
    Timeout::new(future, duration)
}
```

## 3. Multi-Tier Timeouts (reqwest)

Separate timeouts for different phases:

```rust
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TimeoutConfig {
    /// Timeout for establishing connection
    pub connect: Option<Duration>,
    /// Timeout for each read operation (resets on success)
    pub read: Option<Duration>,
    /// Total timeout from start to finish
    pub total: Option<Duration>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect: Some(Duration::from_secs(30)),
            read: Some(Duration::from_secs(30)),
            total: None,
        }
    }
}

impl TimeoutConfig {
    pub fn builder() -> TimeoutConfigBuilder {
        TimeoutConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct TimeoutConfigBuilder {
    connect: Option<Duration>,
    read: Option<Duration>,
    total: Option<Duration>,
}

impl TimeoutConfigBuilder {
    pub fn connect(mut self, timeout: Duration) -> Self {
        self.connect = Some(timeout);
        self
    }

    pub fn read(mut self, timeout: Duration) -> Self {
        self.read = Some(timeout);
        self
    }

    pub fn total(mut self, timeout: Duration) -> Self {
        self.total = Some(timeout);
        self
    }

    pub fn build(self) -> TimeoutConfig {
        TimeoutConfig {
            connect: self.connect,
            read: self.read,
            total: self.total,
        }
    }
}

// Usage in operation context
pub struct OperationContext {
    started: Instant,
    last_read: Instant,
    config: TimeoutConfig,
}

impl OperationContext {
    pub fn new(config: TimeoutConfig) -> Self {
        let now = Instant::now();
        Self {
            started: now,
            last_read: now,
            config,
        }
    }

    pub fn check_total_timeout(&self) -> Result<(), Elapsed> {
        if let Some(total) = self.config.total {
            if self.started.elapsed() > total {
                return Err(Elapsed);
            }
        }
        Ok(())
    }

    pub fn check_read_timeout(&self) -> Result<(), Elapsed> {
        if let Some(read) = self.config.read {
            if self.last_read.elapsed() > read {
                return Err(Elapsed);
            }
        }
        Ok(())
    }

    pub fn record_read(&mut self) {
        self.last_read = Instant::now();
    }
}
```

## 4. Stability Detection

Hash-based change detection for "screen stable" conditions:

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Default)]
pub struct StabilityTracker {
    history: Vec<u64>,
    required_consecutive: usize,
}

impl StabilityTracker {
    pub fn new(required_consecutive: usize) -> Self {
        Self {
            history: Vec::with_capacity(required_consecutive),
            required_consecutive,
        }
    }

    pub fn add_sample<T: Hash>(&mut self, value: &T) -> bool {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();

        self.history.push(hash);

        // Keep only needed history
        if self.history.len() > self.required_consecutive {
            self.history.remove(0);
        }

        self.is_stable()
    }

    pub fn is_stable(&self) -> bool {
        if self.history.len() < self.required_consecutive {
            return false;
        }

        let first = self.history[0];
        self.history.iter().all(|&h| h == first)
    }

    pub fn reset(&mut self) {
        self.history.clear();
    }
}

// Higher-level wrapper
pub fn wait_for_stable<F, T>(
    mut get_state: F,
    timeout: Duration,
    poll_interval: Duration,
    consecutive_matches: usize,
) -> Result<T, WaitError>
where
    F: FnMut() -> T,
    T: Hash + Clone,
{
    let mut tracker = StabilityTracker::new(consecutive_matches);
    let deadline = Instant::now() + timeout;
    let mut last_state = None;

    loop {
        let state = get_state();

        if tracker.add_sample(&state) {
            return Ok(state);
        }

        last_state = Some(state);

        if Instant::now() >= deadline {
            return Err(WaitError::Timeout);
        }

        thread::sleep(poll_interval);
    }
}

#[derive(Debug)]
pub enum WaitError {
    Timeout,
    Cancelled,
}
```

## 5. Exponential Backoff

Retry with exponential backoff and jitter:

```rust
use std::time::Duration;
use rand::Rng;

#[derive(Clone, Debug)]
pub struct BackoffConfig {
    pub initial: Duration,
    pub max: Duration,
    pub multiplier: f64,
    pub jitter: f64,  // 0.0 to 1.0
    pub max_retries: Option<u32>,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial: Duration::from_millis(100),
            max: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: 0.1,
            max_retries: Some(10),
        }
    }
}

pub struct Backoff {
    config: BackoffConfig,
    current: Duration,
    attempt: u32,
}

impl Backoff {
    pub fn new(config: BackoffConfig) -> Self {
        let current = config.initial;
        Self {
            config,
            current,
            attempt: 0,
        }
    }

    pub fn next_delay(&mut self) -> Option<Duration> {
        if let Some(max) = self.config.max_retries {
            if self.attempt >= max {
                return None;
            }
        }

        self.attempt += 1;

        let delay = self.current;

        // Calculate next delay with jitter
        let base_next = Duration::from_secs_f64(
            self.current.as_secs_f64() * self.config.multiplier
        );

        let jitter_range = base_next.as_secs_f64() * self.config.jitter;
        let jitter = rand::thread_rng().gen_range(-jitter_range..jitter_range);
        let jittered = Duration::from_secs_f64(
            (base_next.as_secs_f64() + jitter).max(0.0)
        );

        self.current = jittered.min(self.config.max);

        Some(delay)
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn reset(&mut self) {
        self.current = self.config.initial;
        self.attempt = 0;
    }
}

// Retry helper
pub fn retry_with_backoff<F, T, E>(
    mut operation: F,
    config: BackoffConfig,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut backoff = Backoff::new(config);

    loop {
        match operation() {
            Ok(v) => return Ok(v),
            Err(e) => {
                match backoff.next_delay() {
                    Some(delay) => thread::sleep(delay),
                    None => return Err(e),
                }
            }
        }
    }
}
```

## 6. Cooperative Cancellation (fd)

Atomic flags for graceful shutdown:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    interrupted: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            interrupted: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn interrupt(&self) {
        // Second signal = force quit
        if self.interrupted.fetch_or(true, Ordering::SeqCst) {
            std::process::exit(130);
        }
        self.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn clone_for_handler(&self) -> Self {
        Self {
            cancelled: Arc::clone(&self.cancelled),
            interrupted: Arc::clone(&self.interrupted),
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

// Worker loop pattern
pub fn worker_loop<F>(token: &CancellationToken, mut work: F)
where
    F: FnMut() -> WorkResult,
{
    loop {
        if token.is_cancelled() {
            break;
        }

        match work() {
            WorkResult::Continue => {}
            WorkResult::Done => break,
            WorkResult::Error(e) => {
                eprintln!("Worker error: {}", e);
                break;
            }
        }
    }
}

pub enum WorkResult {
    Continue,
    Done,
    Error(String),
}

// Signal handler setup
pub fn setup_ctrlc_handler(token: CancellationToken) -> Result<(), ctrlc::Error> {
    ctrlc::set_handler(move || {
        token.interrupt();
    })
}
```

## 7. Strategy-Based Condition Checking (ripgrep)

Abstract condition checking behind traits:

```rust
pub trait Condition: Send + Sync {
    fn check(&self, state: &State) -> bool;
    fn description(&self) -> String;
}

pub struct TextCondition {
    needle: String,
}

impl Condition for TextCondition {
    fn check(&self, state: &State) -> bool {
        state.content().contains(&self.needle)
    }

    fn description(&self) -> String {
        format!("text \"{}\"", self.needle)
    }
}

pub struct ElementCondition {
    selector: String,
}

impl Condition for ElementCondition {
    fn check(&self, state: &State) -> bool {
        state.find_element(&self.selector).is_some()
    }

    fn description(&self) -> String {
        format!("element {}", self.selector)
    }
}

pub struct CompositeCondition {
    conditions: Vec<Box<dyn Condition>>,
    mode: CompositeMode,
}

pub enum CompositeMode {
    All,  // AND
    Any,  // OR
}

impl Condition for CompositeCondition {
    fn check(&self, state: &State) -> bool {
        match self.mode {
            CompositeMode::All => self.conditions.iter().all(|c| c.check(state)),
            CompositeMode::Any => self.conditions.iter().any(|c| c.check(state)),
        }
    }

    fn description(&self) -> String {
        let sep = match self.mode {
            CompositeMode::All => " AND ",
            CompositeMode::Any => " OR ",
        };
        self.conditions
            .iter()
            .map(|c| c.description())
            .collect::<Vec<_>>()
            .join(sep)
    }
}

// Usage
pub fn wait_for<C: Condition>(
    condition: &C,
    state_provider: &mut dyn StateProvider,
    timeout: Duration,
    interval: Duration,
) -> Result<(), WaitError> {
    let deadline = Instant::now() + timeout;

    loop {
        let state = state_provider.current_state();

        if condition.check(&state) {
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(WaitError::Timeout);
        }

        thread::sleep(interval);
    }
}
```

## 8. Deadline Management

Deadline-aware operations with remaining time calculation:

```rust
use std::time::{Duration, Instant};

pub struct Deadline {
    instant: Instant,
}

impl Deadline {
    pub fn after(duration: Duration) -> Self {
        Self {
            instant: Instant::now() + duration,
        }
    }

    pub fn at(instant: Instant) -> Self {
        Self { instant }
    }

    pub fn remaining(&self) -> Option<Duration> {
        let now = Instant::now();
        if now >= self.instant {
            None
        } else {
            Some(self.instant - now)
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.instant
    }

    pub fn instant(&self) -> Instant {
        self.instant
    }
}

// Deadline-aware polling
pub fn poll_until_deadline<F, T>(
    deadline: Deadline,
    interval: Duration,
    mut poll: F,
) -> Option<T>
where
    F: FnMut() -> Option<T>,
{
    loop {
        if let Some(result) = poll() {
            return Some(result);
        }

        match deadline.remaining() {
            None => return None,
            Some(remaining) => {
                // Don't sleep longer than remaining time
                let sleep_time = interval.min(remaining);
                thread::sleep(sleep_time);
            }
        }
    }
}

// Pass deadline to sub-operations
pub fn operation_with_deadline<F, T>(
    deadline: &Deadline,
    operation: F,
) -> Result<T, TimeoutError>
where
    F: FnOnce(Duration) -> Result<T, OperationError>,
{
    let remaining = deadline.remaining().ok_or(TimeoutError::Expired)?;
    operation(remaining).map_err(|e| match e {
        OperationError::Timeout => TimeoutError::Expired,
        OperationError::Other(msg) => TimeoutError::OperationFailed(msg),
    })
}

#[derive(Debug)]
pub enum TimeoutError {
    Expired,
    OperationFailed(String),
}

#[derive(Debug)]
pub enum OperationError {
    Timeout,
    Other(String),
}
```
