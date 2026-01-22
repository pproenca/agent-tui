# Logging Patterns

## Table of Contents
1. [Slog Setup](#1-slog-setup)
2. [Structured Logging](#2-structured-logging)
3. [Logger Threading](#3-logger-threading)
4. [Log Levels](#4-log-levels)
5. [Contextual Logging](#5-contextual-logging)

---

## 1. Slog Setup

Initialize structured logger:

```rust
use slog::Drain;
use slog::Logger;
use slog::o;
use slog_term;

pub fn setup_logger(verbose: u8) -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator)
        .build()
        .fuse();

    let level = match verbose {
        0 => slog::Level::Info,
        1 => slog::Level::Debug,
        _ => slog::Level::Trace,
    };

    let drain = slog::LevelFilter::new(drain, level).fuse();
    let drain = slog_async::Async::new(drain)
        .build()
        .fuse();

    Logger::root(drain, o!("version" => env!("CARGO_PKG_VERSION")))
}

/// Compact format for production
pub fn setup_production_logger() -> Logger {
    let decorator = slog_term::PlainDecorator::new(std::io::stderr());
    let drain = slog_term::CompactFormat::new(decorator)
        .build()
        .fuse();

    let drain = slog_async::Async::new(drain)
        .chan_size(4096)
        .overflow_strategy(slog_async::OverflowStrategy::Drop)
        .build()
        .fuse();

    Logger::root(drain, o!())
}

/// JSON format for log aggregation
pub fn setup_json_logger<W: std::io::Write + Send + 'static>(writer: W) -> Logger {
    let drain = slog_json::Json::new(writer)
        .add_default_keys()
        .build()
        .fuse();

    let drain = slog_async::Async::new(drain)
        .build()
        .fuse();

    Logger::root(drain, o!())
}
```

## 2. Structured Logging

Log with key-value pairs:

```rust
use slog::debug;
use slog::error;
use slog::info;
use slog::trace;
use slog::warn;

fn process_request(logger: &Logger, request: &Request) -> Result<Response> {
    debug!(logger, "Processing request";
        "id" => &request.id,
        "method" => &request.method,
        "path" => &request.path,
    );

    let start = Instant::now();

    let result = handle_request(request);

    match &result {
        Ok(response) => {
            info!(logger, "Request completed";
                "id" => &request.id,
                "status" => response.status,
                "duration_ms" => start.elapsed().as_millis(),
            );
        }
        Err(e) => {
            error!(logger, "Request failed";
                "id" => &request.id,
                "error" => %e,
                "duration_ms" => start.elapsed().as_millis(),
            );
        }
    }

    result
}

/// Format specifiers
fn log_with_formats(logger: &Logger, data: &Data) {
    info!(logger, "Data point";
        // Display format (uses Display trait)
        "name" => %data.name,
        // Debug format (uses Debug trait)
        "config" => ?data.config,
        // Owned value
        "id" => data.id.to_string(),
        // Optional with default
        "extra" => data.extra.as_deref().unwrap_or("none"),
    );
}
```

## 3. Logger Threading

Pass logger to threads and functions:

```rust
fn start_workers(logger: &Logger, config: &Config) -> Result<Vec<JoinHandle<()>>> {
    let mut handles = Vec::new();

    for i in 0..config.worker_count {
        // Create child logger with worker context
        let worker_logger = logger.new(o!("worker" => i));

        let handle = std::thread::Builder::new()
            .name(format!("worker-{}", i))
            .spawn(move || {
                info!(worker_logger, "Worker started");
                worker_loop(&worker_logger);
                info!(worker_logger, "Worker stopped");
            })?;

        handles.push(handle);
    }

    Ok(handles)
}

/// Function signature convention: logger first
pub fn collect_sample(logger: &Logger, source: &DataSource) -> Result<Sample> {
    trace!(logger, "Starting collection"; "source" => source.name());

    let data = source.read()
        .map_err(|e| {
            warn!(logger, "Collection failed"; "error" => %e);
            e
        })?;

    debug!(logger, "Collection complete"; "records" => data.len());

    Ok(Sample::from(data))
}

/// Async with logger
async fn async_process(logger: Logger, input: Input) -> Result<Output> {
    info!(logger, "Async processing started"; "input_id" => input.id);

    let result = tokio::spawn(async move {
        // Logger moved into async block
        process_internal(&logger, input).await
    }).await?;

    Ok(result)
}
```

## 4. Log Levels

Use appropriate levels:

```rust
/// TRACE: Very detailed debugging, high volume
fn parse_line(logger: &Logger, line: &str) -> Result<Record> {
    trace!(logger, "Parsing line"; "content" => line);
    // ...
}

/// DEBUG: Debugging information, moderate volume
fn process_batch(logger: &Logger, batch: &[Record]) -> Result<()> {
    debug!(logger, "Processing batch"; "size" => batch.len());
    // ...
}

/// INFO: Normal operational messages
fn start_service(logger: &Logger, config: &Config) -> Result<()> {
    info!(logger, "Service starting";
        "port" => config.port,
        "workers" => config.worker_count,
    );
    // ...
}

/// WARN: Unexpected but recoverable situations
fn handle_timeout(logger: &Logger, request_id: &str) {
    warn!(logger, "Request timed out, retrying";
        "request_id" => request_id,
    );
}

/// ERROR: Errors that affect operation
fn handle_failure(logger: &Logger, error: &Error) {
    error!(logger, "Operation failed";
        "error" => %error,
        "backtrace" => ?error.backtrace(),
    );
}

/// CRITICAL: Use error! with context for critical issues
fn handle_critical(logger: &Logger, error: &Error) {
    error!(logger, "CRITICAL: System invariant violated";
        "error" => %error,
        "action" => "shutting down",
    );
}
```

## 5. Contextual Logging

Add context progressively:

```rust
fn handle_connection(root_logger: &Logger, conn: Connection) -> Result<()> {
    // Add connection context
    let logger = root_logger.new(o!(
        "conn_id" => conn.id.to_string(),
        "remote_addr" => conn.remote_addr.to_string(),
    ));

    info!(logger, "Connection established");

    loop {
        let request = match conn.read_request() {
            Ok(req) => req,
            Err(e) => {
                warn!(logger, "Failed to read request"; "error" => %e);
                break;
            }
        };

        // Add request context
        let req_logger = logger.new(o!(
            "request_id" => request.id.to_string(),
            "method" => request.method.clone(),
        ));

        handle_request(&req_logger, request)?;
    }

    info!(logger, "Connection closed");
    Ok(())
}

/// Macro for operation timing
macro_rules! timed_operation {
    ($logger:expr, $name:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        debug!($logger, concat!($name, " started"));

        let result = $block;

        let elapsed = start.elapsed();
        if elapsed > std::time::Duration::from_secs(1) {
            warn!($logger, concat!($name, " slow");
                "duration_ms" => elapsed.as_millis(),
            );
        } else {
            debug!($logger, concat!($name, " completed");
                "duration_ms" => elapsed.as_millis(),
            );
        }

        result
    }};
}

// Usage
fn process(logger: &Logger, data: &Data) -> Result<Output> {
    let parsed = timed_operation!(logger, "parse", {
        parse_data(data)?
    });

    let transformed = timed_operation!(logger, "transform", {
        transform_data(&parsed)?
    });

    Ok(transformed)
}
```
