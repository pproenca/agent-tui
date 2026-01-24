---
name: rust-coder
description: Write production-quality Rust code with idiomatic patterns and best practices. Use when writing Rust code for CLI tools, TUI applications, daemons, RPC services, terminal emulation, visual processing, data serialization, or any Rust project. Triggers on: "write Rust", "Rust code", "Rust patterns", "idiomatic Rust", "Rust best practices", "Rust daemon", "Rust RPC", "Rust TUI", "Rust CLI", "Rust serialization", "Rust error handling", "Rust concurrency", "Rust testing", "Rust macros", "Rust traits", "Rust types", or when implementing Rust features. Especially relevant for agent-tui development.
---

# Rust Code Patterns

Write production-quality Rust code following patterns from production systems like [facebook/below](https://github.com/facebookincubator/below).

## Pattern Reference (80+ patterns)

Load the relevant reference file based on what you're implementing:

| Category | Patterns | Reference | Load when |
|----------|----------|-----------|-----------|
| **Architecture** | workspace, dependency graph, layers, crate boundaries, API surface, features, bin vs lib | [architecture.md](references/architecture.md) | Starting a new project, organizing crates, designing module structure |
| **Naming & Style** | identifiers, prefixes, suffixes, file org, co-location, comments, docs, constants | [naming-and-style.md](references/naming-and-style.md) | Naming variables/functions, organizing files, writing documentation |
| **Performance** | Big-O, memory efficiency, threading, async, channels, sockets, resource limits, backoff | [performance.md](references/performance.md) | Optimizing hot paths, reducing allocations, profiling, tuning |
| **Error Handling** | thiserror, Result alias, context, source, graceful degradation, ENOENT, channels | [error-handling.md](references/error-handling.md) | Defining custom errors, adding context, handling missing files |
| **Type Design** | aliases, Option fields, newtype, builder, enum strategy, associated types, marker traits, bounds | [type-design.md](references/type-design.md) | Designing data types, choosing between structs/enums, builder patterns |
| **Parsing** | macros, whitespace split, key-value, ranges/sets, special values, FromStr, conditionals | [parsing.md](references/parsing.md) | Parsing text formats, implementing FromStr, tokenizing input |
| **Traits & Generics** | extension, hierarchies, blanket impl, object safety, containers, trait objects vs generics | [traits-and-generics.md](references/traits-and-generics.md) | Defining traits, adding extension methods, generic abstractions |
| **Concurrency** | named threads, Arc-Mutex, channels, scopeguard, condvar, atomics, OnceLock | [concurrency.md](references/concurrency.md) | Multi-threading, shared state, worker pools, synchronization |
| **Data Structures** | hierarchical trees, time-series, cursors, index access, deltas, composites | [data-structures.md](references/data-structures.md) | Custom collections, tree structures, cursor iteration |
| **Macros** | declarative, proc-macro derive, conditional compilation, code generation, helpers | [macros.md](references/macros.md) | Creating derive macros, reducing boilerplate, DSLs |
| **TUI** | screens, view state, events, rendering, keyboard nav, refresh | [tui-patterns.md](references/tui-patterns.md) | Building terminal UIs, handling input, screen management |
| **CLI** | clap derive, subcommands, validation, config integration, dispatch, completions | [cli-patterns.md](references/cli-patterns.md) | Command-line argument parsing, subcommand dispatch |
| **Serialization** | serde, CBOR storage, JSON, CSV, OpenMetrics, custom serializers | [serialization.md](references/serialization.md) | JSON/CBOR encoding, custom serde implementations |
| **Daemon & RPC** | daemon lifecycle, worker threads, gRPC services, PTY handling, signal handling, store/persistence (below, alacritty, tikv, vector, ripgrep) | [daemon-rpc-patterns.md](references/daemon-rpc-patterns.md) | Building services, background processes, RPC handlers |
| **Polling & Waiting** | condition polling, timeouts, stability detection, backoff, cancellation (tokio, reqwest, fd) | [polling-patterns.md](references/polling-patterns.md) | Waiting for conditions, timeouts, retry logic, backoff |
| **Terminal Raw I/O** | raw mode, termios, terminal size, ANSI sequences, stdin/stdout bridging (alacritty, crossterm, console) | [terminal-raw-io.md](references/terminal-raw-io.md) | Raw terminal mode, escape sequences, PTY handling |
| **Visual Processing** | grid buffers, raster scanning, connected components, heuristic classification, visual hashing (image-rs, euclid) | [visual-processing.md](references/visual-processing.md) | Grid/pixel analysis, element detection, visual algorithms |
| **Logging** | slog setup, structured, threading, levels, contextual | [logging.md](references/logging.md) | Setting up logging, structured log output, log levels |
| **Testing** | unit organization, fixtures, property, integration, mocks, utilities | [testing.md](references/testing.md) | Writing tests, test organization, mocking, property tests |
| **File I/O** | buffered, mmap, atomic writes, locking, directories | [file-io.md](references/file-io.md) | File operations, atomic writes, memory mapping |

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

## When to Load Reference Files

**Project setup:**
- Starting a new workspace → [architecture.md](references/architecture.md)
- Defining module structure → [architecture.md](references/architecture.md)

**Core patterns:**
- Custom error types → [error-handling.md](references/error-handling.md)
- Type design decisions → [type-design.md](references/type-design.md)
- Naming conventions → [naming-and-style.md](references/naming-and-style.md)

**Concurrency & async:**
- Threading, channels, atomics → [concurrency.md](references/concurrency.md)
- Polling, waiting, timeouts → [polling-patterns.md](references/polling-patterns.md)

**I/O & persistence:**
- File operations → [file-io.md](references/file-io.md)
- Serialization formats → [serialization.md](references/serialization.md)

**Application types:**
- CLI tools → [cli-patterns.md](references/cli-patterns.md)
- TUI applications → [tui-patterns.md](references/tui-patterns.md)
- Daemons/services → [daemon-rpc-patterns.md](references/daemon-rpc-patterns.md)
- Terminal emulation → [terminal-raw-io.md](references/terminal-raw-io.md)

**Domain-specific:**
- Text/data parsing → [parsing.md](references/parsing.md)
- Visual/grid processing → [visual-processing.md](references/visual-processing.md)
- Logging setup → [logging.md](references/logging.md)
- Testing patterns → [testing.md](references/testing.md)

**Advanced:**
- Trait design → [traits-and-generics.md](references/traits-and-generics.md)
- Custom data structures → [data-structures.md](references/data-structures.md)
- Macro development → [macros.md](references/macros.md)
- Performance tuning → [performance.md](references/performance.md)
