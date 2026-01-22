---
name: rust-coder
description: Write production-quality Rust code with idiomatic patterns and best practices. Use when writing Rust code for CLI tools, TUI applications, system utilities, data processing, or any Rust project. Triggers on requests to "write Rust", "Rust code", "Rust patterns", "idiomatic Rust", "Rust best practices", or when implementing Rust features.
---

# Rust Code Patterns

Write production-quality Rust code following patterns from production systems like [facebook/below](https://github.com/facebookincubator/below).

## Pattern Reference (60+ patterns)

Load the relevant reference file based on what you're implementing:

| Category | Patterns | Reference |
|----------|----------|-----------|
| **Architecture** | workspace, dependency graph, layers, crate boundaries, API surface, features, bin vs lib | [architecture.md](references/architecture.md) |
| **Naming & Style** | identifiers, prefixes, suffixes, file org, co-location, comments, docs, constants | [naming-and-style.md](references/naming-and-style.md) |
| **Performance** | Big-O, memory efficiency, threading, async, channels, sockets, resource limits, backoff | [performance.md](references/performance.md) |
| **Error Handling** | thiserror, Result alias, context, source, graceful degradation, ENOENT, channels | [error-handling.md](references/error-handling.md) |
| **Type Design** | aliases, Option fields, newtype, builder, enum strategy, associated types, marker traits, bounds | [type-design.md](references/type-design.md) |
| **Parsing** | macros, whitespace split, key-value, ranges/sets, special values, FromStr, conditionals | [parsing.md](references/parsing.md) |
| **Traits & Generics** | extension, hierarchies, blanket impl, object safety, containers, trait objects vs generics | [traits-and-generics.md](references/traits-and-generics.md) |
| **Concurrency** | named threads, Arc-Mutex, channels, scopeguard, condvar, atomics, OnceLock | [concurrency.md](references/concurrency.md) |
| **Data Structures** | hierarchical trees, time-series, cursors, index access, deltas, composites | [data-structures.md](references/data-structures.md) |
| **Macros** | declarative, proc-macro derive, conditional compilation, code generation, helpers | [macros.md](references/macros.md) |
| **TUI** | screens, view state, events, rendering, keyboard nav, refresh | [tui-patterns.md](references/tui-patterns.md) |
| **CLI** | clap derive, subcommands, validation, config integration, dispatch, completions | [cli-patterns.md](references/cli-patterns.md) |
| **Serialization** | serde, CBOR storage, JSON, CSV, OpenMetrics, custom serializers | [serialization.md](references/serialization.md) |
| **Logging** | slog setup, structured, threading, levels, contextual | [logging.md](references/logging.md) |
| **Testing** | unit organization, fixtures, property, integration, mocks, utilities | [testing.md](references/testing.md) |
| **File I/O** | buffered, mmap, atomic writes, locking, directories | [file-io.md](references/file-io.md) |

## Quick Start

### Project Setup

```toml
# Cargo.toml
[package]
edition = "2021"

[dependencies]
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_cbor = "0.11"
slog = "2"
slog-term = "2"
slog-async = "2"
tokio = { version = "1", features = ["full"] }
scopeguard = "1"
```

```toml
# rustfmt.toml
edition = "2024"
format_code_in_doc_comments = true
group_imports = "StdExternalCrate"
imports_granularity = "Item"
merge_derives = false
use_field_init_shorthand = true
```

```toml
# clippy.toml
too-many-lines-threshold = 200
```

```rust
// lib.rs
#![deny(clippy::all)]
```

### Core Conventions

**Imports**: std → external → internal, one per line:
```rust
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;

use crate::model::Model;
```

**Error Handling**: anyhow + thiserror:
```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid format: {0:?}")]
    InvalidFormat(PathBuf),
}
pub type Result<T> = std::result::Result<T, Error>;

fn load(path: &Path) -> anyhow::Result<Data> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Reading {}", path.display()))?;
    Ok(parse(&content)?)
}
```

**Function Signatures**: logger first, config by ref, return Result:
```rust
pub fn process(logger: &Logger, config: &Config, data: &Data) -> Result<Output> {
    debug!(logger, "Processing"; "count" => data.len());
    // ...
}
```

**Option Arithmetic**:
```rust
fn opt_add(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x + y),
        (Some(x), None) | (None, Some(x)) => Some(x),
        _ => None,
    }
}
```

**Named Threads**:
```rust
std::thread::Builder::new()
    .name("collector".to_string())
    .spawn(work)
    .expect("spawn failed")
```

**Cleanup with Scopeguard**:
```rust
let _guard = scopeguard::guard((), |_| cleanup());
```

### Module Structure

```
src/
├── lib.rs           # #![deny(clippy::all)], re-exports
├── main.rs          # Entry, CLI dispatch
├── commands.rs      # clap definitions
├── handlers.rs      # Command logic
├── model/           # Data models with Queriable derive
├── store/           # Persistence (CBOR, CRC, shards)
├── view/            # TUI (cursive screens)
└── common/          # Utilities by domain
```

### Key Patterns Summary

1. **Custom derive** for query patterns (`#[derive(Queriable)]`)
2. **Append-only storage** with CRC validation, time-based shards
3. **Delta computation** for rate metrics from cumulative counters
4. **Screen-based TUI** with HashMap view registry
5. **Structured logging** with slog, logger passed explicitly
6. **Graceful degradation** wrapping errors as Option for optional features
