# Naming, Co-location & Comments

## Table of Contents
1. [Naming Conventions](#1-naming-conventions)
2. [File Organization](#2-file-organization)
3. [Code Co-location](#3-code-co-location)
4. [Comment Philosophy](#4-comment-philosophy)
5. [Documentation Style](#5-documentation-style)
6. [Constants and Statics](#6-constants-and-statics)

---

## 1. Naming Conventions

### Identifiers

| Element | Convention | Examples |
|---------|------------|----------|
| Crates | `kebab-case` | `below-model`, `below-store` |
| Modules | `snake_case` | `cgroup_view`, `process_model` |
| Types | `PascalCase` | `CgroupModel`, `StoreWriter` |
| Traits | `PascalCase` | `Queriable`, `HasRenderConfig` |
| Functions | `snake_case` | `get_sample`, `parse_stat` |
| Variables | `snake_case` | `cpu_usage`, `prev_sample` |
| Constants | `SCREAMING_SNAKE_CASE` | `SHARD_TIME`, `MAX_CHUNK_SIZE` |
| Type params | Single uppercase or descriptive | `T`, `S: Store`, `F: Fn()` |
| Lifetimes | Short lowercase | `'a`, `'de`, `'src` |

### Semantic Prefixes

**Function prefixes:**
```rust
// Retrieval
fn get_sample() -> Sample;
fn get_by_path() -> Option<&Node>;

// Conversion
fn convert_bytes(b: u64) -> String;
fn timestamp_to_datetime(ts: u64) -> DateTime;

// Parsing
fn parse_stat(line: &str) -> Result<Stat>;
fn parse_kv_file(content: &str) -> HashMap<String, String>;

// Predicates
fn is_valid() -> bool;
fn is_cpu_significant(pct: f64) -> bool;
fn has_children() -> bool;

// Computation
fn calc_delta(curr: u64, prev: u64) -> u64;
fn compute_rate(delta: u64, elapsed: Duration) -> f64;

// Aggregation
fn aggr_top_level_val() -> u64;
fn sum_children() -> u64;

// State changes
fn set_sort_order(order: SortOrder);
fn toggle_collapsed();
fn update_model(model: Model);

// Creation
fn new() -> Self;
fn with_config(config: Config) -> Self;
fn from_sample(sample: &Sample) -> Self;
```

**Variable prefixes for temporal data:**
```rust
let curr_sample = collect();
let prev_sample = last_sample.take();
let begin_time = range.start;
let end_time = range.end;
let last_update = state.timestamp;
let delta_secs = elapsed.as_secs_f64();
```

### Unit Suffixes

Encode units in names for clarity:

```rust
// Time
cpu_usec: u64,           // microseconds
elapsed_secs: f64,       // seconds
timeout_ms: u64,         // milliseconds
duration_ns: u64,        // nanoseconds

// Rates
bytes_per_sec: f64,
ops_per_sec: f64,
requests_per_min: u64,

// Percentages
cpu_pct: f64,            // 0.0 - 100.0
usage_ratio: f64,        // 0.0 - 1.0

// Sizes
memory_bytes: u64,
buffer_kb: u64,
cache_mb: u64,
```

### Type Naming Patterns

```rust
// Models (data structures)
struct SystemModel { /* ... */ }
struct CgroupModel { /* ... */ }
struct ProcessModel { /* ... */ }

// Field IDs (for query systems)
enum SystemModelFieldId { /* ... */ }
enum CgroupModelFieldId { /* ... */ }

// Samples (raw collected data)
struct SystemSample { /* ... */ }
struct CpuSample { /* ... */ }

// Stats (parsed system data)
struct CpuStat { /* ... */ }
struct MemoryStat { /* ... */ }
struct IoStat { /* ... */ }

// Readers (data sources)
struct ProcReader { /* ... */ }
struct CgroupReader { /* ... */ }

// Writers (output)
struct StoreWriter { /* ... */ }
struct JsonWriter { /* ... */ }

// Views (UI components)
struct ProcessView { /* ... */ }
struct CgroupView { /* ... */ }

// State (mutable UI state)
struct ViewState { /* ... */ }
struct ProcessState { /* ... */ }
```

## 2. File Organization

### Module File Structure

```rust
// 1. License header (if required)
// Copyright (c) Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// 2. Crate-level attributes
#![deny(clippy::all)]

// 3. Module declarations (alphabetical)
mod compression;
mod cursor;
mod writer;

// 4. Conditional modules
#[cfg(test)]
mod tests;

// 5. Re-exports (public API)
pub use compression::CompressionMode;
pub use cursor::Cursor;
pub use writer::StoreWriter;

// 6. Imports (grouped)
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::common::util;
use crate::model::Model;

// 7. Constants
const SHARD_TIME: Duration = Duration::from_secs(86400);
const MAX_BUFFER_SIZE: usize = 64 * 1024;

// 8. Type aliases
pub type SampleMap = BTreeMap<SystemTime, Sample>;

// 9. Trait definitions
pub trait Store: Send + Sync {
    fn get(&self, ts: SystemTime) -> Result<Option<Sample>>;
}

// 10. Struct/Enum definitions
pub struct LocalStore {
    path: PathBuf,
    writer: StoreWriter,
}

// 11. Implementations (in order: inherent, then traits alphabetically)
impl LocalStore {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> { /* ... */ }
}

impl Store for LocalStore {
    fn get(&self, ts: SystemTime) -> Result<Option<Sample>> { /* ... */ }
}

impl Drop for LocalStore {
    fn drop(&mut self) { /* ... */ }
}

// 12. Private helper functions
fn validate_path(path: &Path) -> Result<()> { /* ... */ }

// 13. Tests at end
#[cfg(test)]
mod tests {
    use super::*;
    // ...
}
```

### Import Organization

```rust
// Group 1: Standard library (std, core, alloc)
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

// Group 2: External crates (alphabetical)
use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use slog::debug;
use slog::info;
use slog::Logger;
use thiserror::Error;

// Group 3: Internal crates (workspace members)
use common::util;
use model::Model;
use model::Sample;

// Group 4: Current crate
use crate::compression::Compressor;
use crate::cursor::Cursor;
```

## 3. Code Co-location

### Tests with Code

```rust
// src/parser.rs

pub fn parse_cpu_stat(line: &str) -> Result<CpuStat> {
    // implementation
}

pub fn parse_memory_stat(content: &str) -> Result<MemoryStat> {
    // implementation
}

// Tests at bottom of same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_stat_valid() {
        let line = "cpu0 1000 200 300 4000";
        let stat = parse_cpu_stat(line).unwrap();
        assert_eq!(stat.user, 1000);
    }

    #[test]
    fn test_parse_cpu_stat_empty() {
        assert!(parse_cpu_stat("").is_err());
    }

    #[test]
    fn test_parse_memory_stat() {
        let content = "MemTotal: 16000 kB\nMemFree: 8000 kB\n";
        let stat = parse_memory_stat(content).unwrap();
        assert_eq!(stat.total, 16000 * 1024);
    }
}
```

### Related Types Together

```rust
// Keep related types in same file

/// Raw CPU statistics from /proc/stat
#[derive(Debug, Clone, Default)]
pub struct CpuStat {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
}

/// Computed CPU model with rates
#[derive(Debug, Clone, Default)]
pub struct CpuModel {
    pub usage_pct: Option<f64>,
    pub user_pct: Option<f64>,
    pub system_pct: Option<f64>,
}

/// Query identifier for CpuModel fields
#[derive(Debug, Clone, PartialEq)]
pub enum CpuModelFieldId {
    UsagePct,
    UserPct,
    SystemPct,
}

impl CpuModel {
    pub fn from_samples(curr: &CpuStat, prev: Option<&CpuStat>, elapsed: Duration) -> Self {
        // computation
    }
}

impl Queriable for CpuModel {
    type FieldId = CpuModelFieldId;

    fn query(&self, field: &Self::FieldId) -> Option<Field> {
        // implementation
    }
}
```

### Impl Blocks Organization

```rust
pub struct Store {
    path: PathBuf,
    writer: Writer,
}

// 1. Constructors and factory methods
impl Store {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> { /* ... */ }
    pub fn open(path: &Path) -> Result<Self> { /* ... */ }
    pub fn with_config(path: PathBuf, config: Config) -> Result<Self> { /* ... */ }
}

// 2. Public methods (the API)
impl Store {
    pub fn append(&mut self, sample: &Sample) -> Result<()> { /* ... */ }
    pub fn get(&self, ts: SystemTime) -> Result<Option<Sample>> { /* ... */ }
    pub fn range(&self, start: SystemTime, end: SystemTime) -> Result<Vec<Sample>> { /* ... */ }
}

// 3. Private helpers (same impl block or separate)
impl Store {
    fn validate_sample(&self, sample: &Sample) -> Result<()> { /* ... */ }
    fn write_index_entry(&mut self, entry: &IndexEntry) -> Result<()> { /* ... */ }
}

// 4. Trait implementations (alphabetical by trait name)
impl Drop for Store {
    fn drop(&mut self) { /* ... */ }
}

impl Store for LocalStore {
    // ...
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { /* ... */ }
}
```

## 4. Comment Philosophy

### When to Comment

**Comment the WHY, not the WHAT:**

```rust
// BAD: Describes what code does (obvious from code)
// Increment counter by one
counter += 1;

// BAD: Restates the code
// Check if value is greater than threshold
if value > threshold { /* ... */ }

// GOOD: Explains why this approach was chosen
// Use saturating_sub to handle counter wraparound on 32-bit overflow
let delta = curr.saturating_sub(prev);

// GOOD: Documents non-obvious behavior
// Kernel may partially fill buffer; n < BUFFER_SIZE doesn't indicate EOF
while let Ok(n) = file.read(&mut buffer) {
    if n == 0 { break; }
    // ...
}

// GOOD: Explains business logic
// Skip processes that exited between listing and reading stats
// (ENOENT/ESRCH are expected races, not errors)
if e.kind() == ErrorKind::NotFound { continue; }
```

### Minimal Comments for Self-Documenting Code

```rust
// Instead of comments, use descriptive names:

// BAD
let x = calc(a, b); // Calculate the percentage

// GOOD
let usage_pct = calculate_cpu_percentage(user_time, total_time);

// BAD
if flag { // Check if we should compress

// GOOD
if compression_enabled {

// BAD
for i in items { // Loop through each item

// GOOD
for sample in samples {
```

### TODO and FIXME

```rust
// TODO with tracking ID for external systems
// TODO(T118356932): Handle rounding edge case for 0 bytes

// TODO for future improvement (no external tracking)
// TODO: Consider using arena allocation for better cache locality

// FIXME for known bugs
// FIXME: Race condition when reader and writer access same shard

// HACK for temporary solutions
// HACK: Workaround for kernel bug in cgroup v2 pressure stats

// NOTE for important context
// NOTE: This must be called before dropping the file handle

// SAFETY for unsafe blocks (required by convention)
// SAFETY: Buffer is guaranteed to be valid UTF-8 by prior validation
unsafe { std::str::from_utf8_unchecked(bytes) }
```

## 5. Documentation Style

### Module Documentation

```rust
//! Store module - Time-series data persistence
//!
//! Implements append-only storage with:
//! - Time-based sharding (24-hour files)
//! - CRC validation for corruption detection
//! - Optional zstd compression
//!
//! # Architecture
//!
//! Each shard contains:
//! - Data file: Serialized samples (CBOR format)
//! - Index file: Timestamp → offset mappings
//!
//! # Example
//!
//! ```
//! use store::Store;
//!
//! let mut store = Store::open("/var/log/myapp")?;
//! store.append(&sample)?;
//! ```
```

### Function Documentation

```rust
/// Parse CPU statistics from /proc/stat format.
///
/// # Arguments
///
/// * `line` - A line from /proc/stat (e.g., "cpu0 1000 200 300 4000")
///
/// # Returns
///
/// Parsed `CpuStat` or error if format is invalid.
///
/// # Example
///
/// ```
/// let stat = parse_cpu_stat("cpu0 1000 200 300 4000")?;
/// assert_eq!(stat.user, 1000);
/// ```
pub fn parse_cpu_stat(line: &str) -> Result<CpuStat> {
    // ...
}

/// Compute CPU usage percentage from consecutive samples.
///
/// Returns `None` if delta would be negative (counter wraparound)
/// or elapsed time is zero.
pub fn compute_cpu_pct(curr: &CpuStat, prev: &CpuStat, elapsed: Duration) -> Option<f64> {
    // ...
}
```

### Struct Documentation

```rust
/// Configuration for the data store.
///
/// Controls storage behavior including compression, retention,
/// and file organization.
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Directory for data files
    pub store_dir: PathBuf,

    /// Compression mode for stored samples
    pub compression: CompressionMode,

    /// Days to retain data before cleanup
    pub retention_days: u32,

    /// Maximum uncompressed chunk size in bytes
    pub max_chunk_size: usize,
}
```

## 6. Constants and Statics

### Constant Organization

```rust
// Group related constants with comment headers

// === Time Constants ===
const SHARD_DURATION: Duration = Duration::from_secs(24 * 60 * 60);
const REFRESH_INTERVAL: Duration = Duration::from_millis(250);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// === Size Constants ===
const INDEX_ENTRY_SIZE: usize = 24;
const MAX_BUFFER_SIZE: usize = 64 * 1024;
const DEFAULT_CHUNK_SIZE: usize = 4 * 1024 * 1024;

// === Magic Numbers ===
const STORE_MAGIC: u32 = 0x42454C4F; // "BELO"
const INDEX_VERSION: u8 = 1;

// === Limits ===
const MAX_OPEN_FILES: usize = 100;
const MAX_CACHED_SAMPLES: usize = 1000;
```

### Static vs Const

```rust
// Use const for compile-time values
const MAX_SIZE: usize = 1024;
const DEFAULT_NAME: &str = "unnamed";

// Use static for runtime-initialized singletons
static CONFIG: OnceLock<Config> = OnceLock::new();
static LOGGER: OnceLock<Logger> = OnceLock::new();

// Use lazy_static or OnceLock for complex initialization
static REGEX: OnceLock<Regex> = OnceLock::new();

fn get_regex() -> &'static Regex {
    REGEX.get_or_init(|| {
        Regex::new(r"^\d+$").unwrap()
    })
}
```

### Public Constants

```rust
// In lib.rs or dedicated constants module

/// Default storage directory
pub const DEFAULT_STORE_DIR: &str = "/var/log/myapp";

/// Default collection interval
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(5);

/// Supported output formats
pub mod formats {
    pub const JSON: &str = "json";
    pub const CSV: &str = "csv";
    pub const HUMAN: &str = "human";
}
```

---

## Related Patterns

- [Architecture](architecture.md) - Module and crate organization
- [Type Design](type-design.md) - Type naming conventions
- [CLI Patterns](cli-patterns.md) - Command and flag naming
- [Testing](testing.md) - Test naming conventions
