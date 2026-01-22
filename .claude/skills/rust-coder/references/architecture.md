# Project Architecture Patterns

## Table of Contents
1. [Workspace Organization](#1-workspace-organization)
2. [Crate Dependency Graph](#2-crate-dependency-graph)
3. [Layer Separation](#3-layer-separation)
4. [Crate Boundaries](#4-crate-boundaries)
5. [API Surface Design](#5-api-surface-design)
6. [Feature Flags](#6-feature-flags)
7. [Binary vs Library](#7-binary-vs-library)

---

## 1. Workspace Organization

Structure the workspace with single-responsibility crates:

```
project/
├── Cargo.toml              # Workspace manifest
├── Cargo.lock              # Shared lockfile
├── rustfmt.toml            # Shared formatting
├── clippy.toml             # Shared lints
├── app/                    # Main binary crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Entry point, CLI dispatch
│       ├── commands.rs     # clap definitions
│       └── handlers.rs     # Command implementations
├── model/                  # Core domain types
├── store/                  # Persistence layer
├── view/                   # UI/presentation
├── render/                 # Output formatting
├── common/                 # Shared utilities
├── config/                 # Configuration
└── *_derive/               # Proc macros (separate crate required)
```

**Workspace Cargo.toml:**

```toml
[workspace]
members = [
    "app",
    "model",
    "store",
    "view",
    "render",
    "common",
    "config",
    "model_derive",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
authors = ["Your Team"]

[workspace.dependencies]
# Internal crates
model = { path = "model" }
store = { path = "store" }
view = { path = "view" }
render = { path = "render" }
common = { path = "common" }
config = { path = "config" }
model_derive = { path = "model_derive" }

# External - pin versions once for all crates
anyhow = "1"
thiserror = "1"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
slog = "2"
```

**Individual crate Cargo.toml:**

```toml
[package]
name = "project-model"
version.workspace = true
edition.workspace = true

[dependencies]
common.workspace = true
model_derive.workspace = true
anyhow.workspace = true
serde.workspace = true
```

## 2. Crate Dependency Graph

Design dependencies as a directed acyclic graph flowing toward the binary:

```
                    ┌─────────┐
                    │   app   │  ← Binary (aggregates everything)
                    └────┬────┘
          ┌──────────────┼──────────────┐
          │              │              │
     ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
     │  view   │    │  store  │    │  dump   │  ← Feature crates
     └────┬────┘    └────┬────┘    └────┬────┘
          │              │              │
     ┌────▼────┐         │              │
     │ render  │         │              │
     └────┬────┘         │              │
          └──────────────┼──────────────┘
                    ┌────▼────┐
                    │  model  │  ← Core domain
                    └────┬────┘
          ┌──────────────┼──────────────┐
          │              │              │
     ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
     │ procfs  │    │cgroupfs │    │  common │  ← Data sources / utilities
     └─────────┘    └─────────┘    └─────────┘
```

**Rules:**
- Dependencies flow downward (toward leaves)
- No cycles allowed
- Lower layers never depend on higher layers
- `common` only contains truly shared utilities
- `model` defines core types, depends on data sources

**Anti-patterns to avoid:**
```rust
// BAD: View depending on store implementation details
// view/src/lib.rs
use store::internal::CursorImpl;  // ❌ Leaking internals

// GOOD: View depending on store's public trait
use store::Store;  // ✓ Public interface
```

## 3. Layer Separation

Organize into distinct architectural layers:

### Data Sources Layer (Leaves)
Interfaces to external systems, no internal dependencies:

```rust
// procfs/src/lib.rs - Linux /proc interface
pub struct ProcReader { /* ... */ }

impl ProcReader {
    pub fn read_stat(&self) -> Result<ProcStat>;
    pub fn read_meminfo(&self) -> Result<MemInfo>;
}

// cgroupfs/src/lib.rs - Cgroup interface
pub struct CgroupReader { /* ... */ }

impl CgroupReader {
    pub fn read_cpu_stat(&self) -> Result<CpuStat>;
    pub fn read_memory_stat(&self) -> Result<MemoryStat>;
}
```

### Model Layer (Core Domain)
Pure domain logic, depends only on data sources:

```rust
// model/src/lib.rs
use procfs::ProcStat;
use cgroupfs::CpuStat;

/// Raw collected data (no computed fields)
pub struct Sample {
    pub timestamp: SystemTime,
    pub proc: ProcStat,
    pub cgroup: CpuStat,
}

/// Computed model (rates, percentages)
pub struct Model {
    pub timestamp: SystemTime,
    pub cpu_pct: Option<f64>,
    pub memory_bytes: u64,
}

impl Model {
    /// Compute model from current and previous samples
    pub fn new(current: &Sample, previous: Option<&Sample>) -> Self {
        // Delta computation logic
    }
}
```

### Storage Layer
Persistence, depends on model:

```rust
// store/src/lib.rs
use model::Sample;

pub trait Store: Send + Sync {
    fn append(&mut self, sample: &Sample) -> Result<()>;
    fn get(&self, timestamp: SystemTime) -> Result<Option<Sample>>;
    fn range(&self, start: SystemTime, end: SystemTime) -> Result<Vec<Sample>>;
}

pub struct LocalStore { /* ... */ }
impl Store for LocalStore { /* ... */ }
```

### Render Layer
Formatting/presentation logic, depends on model:

```rust
// render/src/lib.rs
use model::Model;

pub trait Render {
    fn render(&self, model: &Model) -> String;
}

pub struct TableRenderer { /* ... */ }
pub struct JsonRenderer { /* ... */ }
```

### View Layer (UI)
User interface, depends on model, render, store:

```rust
// view/src/lib.rs
use model::Model;
use render::Render;
use store::Store;

pub struct App<S: Store, R: Render> {
    store: S,
    renderer: R,
    state: ViewState,
}
```

### Application Layer (Binary)
Wires everything together:

```rust
// app/src/main.rs
use model::Collector;
use store::LocalStore;
use view::App;
use config::Config;

fn main() -> Result<()> {
    let config = Config::load()?;
    let store = LocalStore::open(&config.store_path)?;
    let collector = Collector::new()?;
    let app = App::new(store, collector);
    app.run()
}
```

## 4. Crate Boundaries

Define clear boundaries between crates:

### What Goes Where

| Crate | Contains | Does NOT Contain |
|-------|----------|------------------|
| `common` | Error types, time utils, string helpers | Domain logic, I/O |
| `model` | Domain types, computation | Storage, UI, I/O |
| `store` | Persistence, serialization | Domain computation |
| `render` | Formatting, display | State, I/O |
| `view` | UI components, interaction | Direct file I/O |
| `config` | Config loading, validation | Runtime state |
| `app` | CLI, main(), wiring | Reusable logic |

### Boundary Enforcement

```rust
// model/src/lib.rs

// PUBLIC: Types other crates need
pub struct Model { /* ... */ }
pub struct Sample { /* ... */ }
pub trait Queriable { /* ... */ }

// PRIVATE: Implementation details
mod computation;  // Not pub
mod internal;     // Not pub

// CRATE-VISIBLE: For tests and internal modules
pub(crate) fn compute_delta(a: u64, b: u64) -> u64 { /* ... */ }
```

### Re-export Pattern

```rust
// model/src/lib.rs
mod system;
mod cgroup;
mod process;

// Re-export public types at crate root
pub use system::SystemModel;
pub use cgroup::CgroupModel;
pub use process::ProcessModel;

// Users import from crate root
// use model::SystemModel;  ✓
// use model::system::SystemModel;  Still works but discouraged
```

## 5. API Surface Design

Design public APIs for stability and usability:

### Minimize Public Surface

```rust
// BAD: Everything public
pub struct Store {
    pub data_file: File,        // ❌ Implementation detail
    pub index: Vec<IndexEntry>, // ❌ Implementation detail
    pub cache: LruCache,        // ❌ Implementation detail
}

// GOOD: Only necessary items public
pub struct Store {
    data_file: File,
    index: Vec<IndexEntry>,
    cache: LruCache,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn append(&mut self, sample: &Sample) -> Result<()>;
    pub fn get(&self, ts: SystemTime) -> Result<Option<Sample>>;
}
```

### Trait-Based Abstraction

```rust
// Public trait for consumers
pub trait Store: Send + Sync {
    fn get(&self, ts: SystemTime) -> Result<Option<Sample>>;
}

// Concrete types can be pub or pub(crate)
pub struct LocalStore { /* ... */ }
pub struct RemoteStore { /* ... */ }

impl Store for LocalStore { /* ... */ }
impl Store for RemoteStore { /* ... */ }

// Consumers depend on trait, not concrete type
pub fn process<S: Store>(store: &S) -> Result<()> {
    let sample = store.get(SystemTime::now())?;
    // ...
}
```

### Sealed Traits for Extensibility Control

```rust
// Only this crate can implement this trait
mod private {
    pub trait Sealed {}
}

pub trait Store: private::Sealed {
    fn get(&self, ts: SystemTime) -> Result<Option<Sample>>;
}

impl private::Sealed for LocalStore {}
impl Store for LocalStore { /* ... */ }

// External crates cannot implement Store
```

## 6. Feature Flags

Use features for optional functionality:

```toml
# model/Cargo.toml
[features]
default = []
gpu = ["dep:nvml-wrapper"]
btrfs = ["dep:btrfs-crate"]

[dependencies]
nvml-wrapper = { version = "0.9", optional = true }
btrfs-crate = { version = "0.1", optional = true }
```

```rust
// model/src/lib.rs
pub struct Sample {
    pub system: SystemSample,

    #[cfg(feature = "gpu")]
    pub gpu: Option<GpuSample>,

    #[cfg(feature = "btrfs")]
    pub btrfs: Option<BtrfsSample>,
}

#[cfg(feature = "gpu")]
mod gpu;

#[cfg(feature = "gpu")]
pub use gpu::GpuSample;
```

**Feature naming conventions:**
- `gpu` - Enable GPU monitoring
- `btrfs` - Enable Btrfs support
- `no-vendor` - Use system libraries instead of bundled

## 7. Binary vs Library

Structure for both library consumers and CLI users:

```
project/
├── Cargo.toml          # Workspace
├── lib/                # Core library (reusable)
│   ├── Cargo.toml
│   └── src/lib.rs
└── cli/                # Binary (uses library)
    ├── Cargo.toml
    └── src/main.rs
```

**Library crate (lib/Cargo.toml):**
```toml
[package]
name = "myproject"  # Library name

[lib]
name = "myproject"

[dependencies]
# Only library dependencies
```

**Binary crate (cli/Cargo.toml):**
```toml
[package]
name = "myproject-cli"

[[bin]]
name = "myproject"  # Binary name matches library

[dependencies]
myproject = { path = "../lib" }
clap = "4"  # CLI-only deps
```

**Library API (lib/src/lib.rs):**
```rust
//! MyProject - System monitoring library
//!
//! # Example
//! ```
//! use myproject::{Collector, Model};
//!
//! let collector = Collector::new()?;
//! let sample = collector.collect()?;
//! let model = Model::from_sample(&sample);
//! ```

pub mod collector;
pub mod model;
pub mod store;

pub use collector::Collector;
pub use model::Model;
pub use store::Store;
```

**Binary entry (cli/src/main.rs):**
```rust
use myproject::{Collector, Model, Store};
use clap::Parser;

#[derive(Parser)]
struct Cli { /* ... */ }

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // Use library types
}
```

This separation allows:
- Library users to depend on `myproject` without CLI bloat
- CLI to be thin wrapper around library
- Independent versioning if needed
- Clear documentation boundary
