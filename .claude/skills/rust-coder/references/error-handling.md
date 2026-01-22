# Error Handling Patterns

## Table of Contents
1. [Custom Error Types with thiserror](#1-custom-error-types-with-thiserror)
2. [Result Type Alias](#2-result-type-alias)
3. [Context Enrichment](#3-context-enrichment)
4. [Error Source Attribution](#4-error-source-attribution)
5. [Graceful Degradation](#5-graceful-degradation)
6. [ENOENT Handling](#6-enoent-handling)
7. [Channel-Based Error Collection](#7-channel-based-error-collection)

---

## 1. Custom Error Types with thiserror

Define domain-specific errors with descriptive messages:

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid file format: {0:?}")]
    InvalidFileFormat(PathBuf),

    #[error("Failed to parse {field} in {path:?}: expected {expected}, got {actual:?}")]
    ParseError {
        path: PathBuf,
        field: &'static str,
        expected: &'static str,
        actual: String,
    },

    #[error("Configuration missing required field: {0}")]
    MissingField(&'static str),

    #[error("Value out of range: {value} (expected {min}..{max})")]
    OutOfRange {
        value: i64,
        min: i64,
        max: i64,
    },
}
```

## 2. Result Type Alias

Create crate-local Result type for cleaner signatures:

```rust
pub type Result<T> = std::result::Result<T, Error>;

// Usage
fn read_stats(path: &Path) -> Result<Stats> {
    // ...
}
```

## 3. Context Enrichment

Add context with anyhow for debugging:

```rust
use anyhow::Context;
use anyhow::Result;

fn load_config(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;

    toml::from_str(&contents)
        .with_context(|| format!("Failed to parse TOML in {}", path.display()))
}

fn process_file(path: &Path) -> Result<Data> {
    let raw = read_raw(path)
        .context("Reading raw data")?;

    parse_data(&raw)
        .context("Parsing data format")?;

    validate(&data)
        .context("Validating parsed data")
}
```

## 4. Error Source Attribution

Chain errors with #[source] for cause tracking:

```rust
#[derive(Error, Debug)]
pub enum Error {
    #[error("{1:?}: {0:?}")]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("Serialization failed for {path:?}")]
    SerializeError {
        path: PathBuf,
        #[source]
        source: serde_cbor::Error,
    },

    #[error("Network request failed: {url}")]
    NetworkError {
        url: String,
        #[source]
        source: reqwest::Error,
    },
}
```

## 5. Graceful Degradation

Convert errors to Option for optional features:

```rust
/// Wrap errors as None for optional data sources
fn wrap<T>(result: Result<T>) -> Option<T> {
    match result {
        Ok(v) => Some(v),
        Err(Error::NotFound(_)) => None,
        Err(e) => {
            warn!(logger, "Unexpected error: {}", e);
            None
        }
    }
}

/// Collect data with graceful fallbacks
fn collect_sample() -> Sample {
    Sample {
        cpu: read_cpu_stats().ok(),
        memory: read_memory_stats().ok(),
        gpu: wrap(read_gpu_stats()),  // Optional subsystem
        pressure: pressure_wrap(read_pressure()),  // May not exist
    }
}
```

## 6. ENOENT Handling

Distinguish missing files from real errors:

```rust
use std::io::ErrorKind;

fn handle_enoent<T>(result: std::io::Result<T>) -> Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Skip processes that disappeared during collection
fn read_all_pids(pids: &[u32]) -> Result<Vec<PidInfo>> {
    let mut results = Vec::new();

    for pid in pids {
        match read_pid_info(*pid) {
            Ok(info) => results.push(info),
            Err(Error::IoError(_, ref e))
                if e.raw_os_error().is_some_and(|ec| {
                    ec == libc::ENOENT || ec == libc::ESRCH
                }) =>
            {
                continue; // Process exited, skip it
            }
            Err(e) => return Err(e),
        }
    }

    Ok(results)
}
```

## 7. Channel-Based Error Collection

Collect errors from background tasks:

```rust
use std::sync::mpsc;

fn dump_data(
    data: &[Record],
    error_rx: mpsc::Receiver<Error>,
) -> Result<()> {
    let (tx, rx) = mpsc::channel();

    // Spawn background workers
    for chunk in data.chunks(100) {
        let tx = tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = process_chunk(chunk) {
                let _ = tx.send(e);
            }
        });
    }
    drop(tx);

    // Collect any errors
    let errors: Vec<_> = rx.iter().collect();
    if !errors.is_empty() {
        anyhow::bail!("Background tasks failed: {:?}", errors);
    }

    Ok(())
}
```
