# Testing Patterns

## Table of Contents
1. [Unit Test Organization](#1-unit-test-organization)
2. [Test Fixtures](#2-test-fixtures)
3. [Property Testing](#3-property-testing)
4. [Integration Tests](#4-integration-tests)
5. [Mock Patterns](#5-mock-patterns)
6. [Test Utilities](#6-test-utilities)

---

## 1. Unit Test Organization

Co-locate tests with code:

```rust
// In src/parser.rs

pub fn parse_cpu_stat(line: &str) -> Result<CpuStat> {
    // implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_stat_basic() {
        let line = "cpu0 1000 200 300 4000 50 60 70 0 0 0";
        let stat = parse_cpu_stat(line).unwrap();

        assert_eq!(stat.user, 1000);
        assert_eq!(stat.nice, 200);
        assert_eq!(stat.system, 300);
        assert_eq!(stat.idle, 4000);
    }

    #[test]
    fn test_parse_cpu_stat_missing_fields() {
        let line = "cpu0 1000 200";
        let result = parse_cpu_stat(line);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cpu_stat_empty() {
        let line = "";
        let result = parse_cpu_stat(line);

        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "invalid digit")]
    fn test_parse_cpu_stat_invalid_number() {
        let line = "cpu0 abc 200 300 4000 50 60 70 0 0 0";
        parse_cpu_stat(line).unwrap();
    }
}
```

## 2. Test Fixtures

Create reusable test data:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        Config {
            interval: 5,
            store_dir: PathBuf::from("/tmp/test"),
            compress: false,
        }
    }

    fn sample_cpu_stat() -> CpuStat {
        CpuStat {
            user: 1000,
            nice: 100,
            system: 500,
            idle: 8000,
            iowait: 200,
            irq: 50,
            softirq: 30,
        }
    }

    fn sample_model() -> Model {
        Model {
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(1000),
            cpu: sample_cpu_stat(),
            memory: MemoryStat::default(),
            ..Default::default()
        }
    }

    /// Builder pattern for complex fixtures
    struct SampleBuilder {
        cpu: Option<CpuStat>,
        memory: Option<MemoryStat>,
        timestamp: Option<SystemTime>,
    }

    impl SampleBuilder {
        fn new() -> Self {
            Self {
                cpu: None,
                memory: None,
                timestamp: None,
            }
        }

        fn with_cpu(mut self, cpu: CpuStat) -> Self {
            self.cpu = Some(cpu);
            self
        }

        fn with_high_cpu(self) -> Self {
            self.with_cpu(CpuStat {
                user: 9000,
                system: 500,
                idle: 500,
                ..Default::default()
            })
        }

        fn build(self) -> Sample {
            Sample {
                timestamp: self.timestamp.unwrap_or(SystemTime::now()),
                cpu: self.cpu.unwrap_or_default(),
                memory: self.memory.unwrap_or_default(),
            }
        }
    }

    #[test]
    fn test_with_builder() {
        let sample = SampleBuilder::new()
            .with_high_cpu()
            .build();

        assert!(sample.cpu.user > 8000);
    }
}
```

## 3. Property Testing

Test invariants with proptest:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_roundtrip(
            user in 0u64..1000000,
            system in 0u64..1000000,
            idle in 0u64..1000000,
        ) {
            let stat = CpuStat { user, system, idle, ..Default::default() };
            let serialized = stat.to_line();
            let parsed = parse_cpu_stat(&serialized).unwrap();

            prop_assert_eq!(parsed.user, user);
            prop_assert_eq!(parsed.system, system);
            prop_assert_eq!(parsed.idle, idle);
        }

        #[test]
        fn cpu_set_roundtrip(cpus in prop::collection::btree_set(0u32..256, 0..32)) {
            let set = CpuSet(cpus.clone());
            let formatted = set.to_string();
            let parsed: CpuSet = formatted.parse().unwrap();

            prop_assert_eq!(parsed.0, cpus);
        }

        #[test]
        fn delta_never_negative(
            curr in 0u64..1000000,
            prev in 0u64..1000000,
        ) {
            let delta = curr.saturating_sub(prev);
            prop_assert!(delta <= curr);
        }
    }

    // Custom strategy for valid timestamps
    fn timestamp_strategy() -> impl Strategy<Value = SystemTime> {
        (0u64..1_000_000_000).prop_map(|secs| {
            SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
        })
    }

    proptest! {
        #[test]
        fn model_query_never_panics(
            timestamp in timestamp_strategy(),
            cpu_user in 0u64..1000000,
        ) {
            let model = Model {
                timestamp,
                cpu: CpuStat { user: cpu_user, ..Default::default() },
                ..Default::default()
            };

            // Should never panic regardless of input
            let _ = model.query(&ModelFieldId::CpuUser);
            let _ = model.query(&ModelFieldId::Timestamp);
        }
    }
}
```

## 4. Integration Tests

Test module interactions:

```rust
// In tests/integration.rs

use mytool::*;
use tempfile::TempDir;

struct TestContext {
    temp_dir: TempDir,
    store: Store,
    logger: Logger,
}

impl TestContext {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::open(temp_dir.path()).unwrap();
        let logger = Logger::root(slog::Discard, slog::o!());

        Self { temp_dir, store, logger }
    }
}

#[test]
fn test_store_write_read_cycle() {
    let ctx = TestContext::new();

    // Write samples
    let samples: Vec<Sample> = (0..10)
        .map(|i| Sample {
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(i * 60),
            ..Default::default()
        })
        .collect();

    for sample in &samples {
        ctx.store.append(sample).unwrap();
    }

    // Read back
    for sample in &samples {
        let read = ctx.store
            .get_sample_at_timestamp(sample.timestamp, Direction::Forward)
            .unwrap()
            .unwrap();

        assert_eq!(read.0, sample.timestamp);
    }
}

#[test]
fn test_collector_to_model_pipeline() {
    let ctx = TestContext::new();

    // Mock procfs
    let procfs = MockProcfs::new()
        .with_cpu_stat(CpuStat { user: 1000, ..Default::default() });

    let collector = Collector::new(&ctx.logger, procfs);

    // Collect sample
    let sample = collector.collect().unwrap();
    assert!(sample.cpu.user > 0);

    // Store and retrieve
    ctx.store.append(&sample).unwrap();

    let retrieved = ctx.store
        .get_sample_at_timestamp(sample.timestamp, Direction::Forward)
        .unwrap()
        .unwrap();

    // Build model
    let model = Model::new(&retrieved.1, None);
    assert!(model.query(&ModelFieldId::CpuUser).is_some());
}
```

## 5. Mock Patterns

Create test doubles:

```rust
/// Trait for mockable file reading
pub trait FileReader {
    fn read_to_string(&self, path: &Path) -> Result<String>;
}

/// Real implementation
pub struct RealFileReader;

impl FileReader for RealFileReader {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path).map_err(Into::into)
    }
}

/// Test mock
#[cfg(test)]
pub struct MockFileReader {
    files: HashMap<PathBuf, String>,
}

#[cfg(test)]
impl MockFileReader {
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }

    pub fn with_file(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.insert(path.into(), content.into());
        self
    }
}

#[cfg(test)]
impl FileReader for MockFileReader {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        self.files.get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("File not found: {:?}", path))
    }
}

/// Generic over reader
pub fn parse_procfs<R: FileReader>(reader: &R) -> Result<ProcData> {
    let stat = reader.read_to_string(Path::new("/proc/stat"))?;
    parse_stat(&stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_mock_reader() {
        let reader = MockFileReader::new()
            .with_file("/proc/stat", "cpu 1000 200 300 4000\n");

        let data = parse_procfs(&reader).unwrap();
        assert_eq!(data.cpu.user, 1000);
    }
}
```

## 6. Test Utilities

Helper functions for testing:

```rust
#[cfg(test)]
mod test_utils {
    use super::*;

    /// Assert two floats are approximately equal
    pub fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
        assert!(
            (a - b).abs() < epsilon,
            "assertion failed: {} ≈ {} (epsilon: {})",
            a, b, epsilon
        );
    }

    /// Create temp file with content
    pub fn temp_file_with(content: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    /// Run with timeout
    pub fn with_timeout<F, T>(duration: Duration, f: F) -> Option<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = f();
            let _ = tx.send(result);
        });

        rx.recv_timeout(duration).ok()
    }

    /// Capture stdout for testing
    pub fn capture_stdout<F>(f: F) -> String
    where
        F: FnOnce(),
    {
        // Note: This is simplified; real impl would use gag crate
        let mut output = Vec::new();
        // redirect stdout...
        f();
        String::from_utf8(output).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::*;

    #[test]
    fn test_percentage_calculation() {
        let pct = calculate_percentage(50, 100);
        assert_approx_eq(pct, 50.0, 0.01);
    }

    #[test]
    fn test_parse_from_file() {
        let file = temp_file_with("key: value\n");
        let config = Config::load(file.path()).unwrap();
        assert_eq!(config.key, "value");
    }

    #[test]
    fn test_operation_completes() {
        let result = with_timeout(Duration::from_secs(1), || {
            heavy_computation()
        });

        assert!(result.is_some(), "Operation timed out");
    }
}
```
