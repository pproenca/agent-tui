# Type Design Patterns

## Table of Contents
1. [Type Aliases for Semantic Clarity](#1-type-aliases-for-semantic-clarity)
2. [Option for Nullable Fields](#2-option-for-nullable-fields)
3. [Newtype Pattern](#3-newtype-pattern)
4. [Builder Pattern](#4-builder-pattern)
5. [Enum-Based Strategy](#5-enum-based-strategy)
6. [Associated Types](#6-associated-types)
7. [Marker Traits](#7-marker-traits)
8. [Generic Bounds](#8-generic-bounds)

---

## 1. Type Aliases for Semantic Clarity

Name collections to convey meaning:

```rust
use std::collections::BTreeMap;
use std::collections::HashMap;

/// Process ID to process info mapping
pub type PidMap = BTreeMap<u32, PidInfo>;

/// Device name to disk stats mapping
pub type DiskMap = HashMap<String, DiskStat>;

/// Network interface to stats mapping
pub type NetMap = BTreeMap<String, NetStat>;

/// Timestamp in microseconds since epoch
pub type Usec = u64;

/// Duration in nanoseconds
pub type Nsec = u64;
```

## 2. Option for Nullable Fields

Distinguish "not present" from "zero":

```rust
#[derive(Debug, Default)]
pub struct MemoryStat {
    /// Always present
    pub total: u64,
    pub free: u64,

    /// May not exist on older kernels
    pub available: Option<u64>,

    /// Only present when cgroup v2 enabled
    pub cgroup_usage: Option<u64>,

    /// Only collected when GPU present
    pub gpu_memory: Option<GpuMemory>,
}

#[derive(Debug, Default)]
pub struct CpuStat {
    pub user_usec: Option<u64>,
    pub system_usec: Option<u64>,
    pub usage_usec: Option<u64>,
    pub nr_periods: Option<u64>,
    pub nr_throttled: Option<u64>,
    pub throttled_usec: Option<u64>,
}
```

## 3. Newtype Pattern

Wrap primitives for type safety:

```rust
/// Process ID (guaranteed positive)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pid(u32);

impl Pid {
    pub fn new(pid: u32) -> Option<Self> {
        if pid > 0 { Some(Self(pid)) } else { None }
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// Bytes count (for readable formatting)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bytes(pub u64);

impl std::fmt::Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        match self.0 {
            b if b >= GB => write!(f, "{:.1}G", b as f64 / GB as f64),
            b if b >= MB => write!(f, "{:.1}M", b as f64 / MB as f64),
            b if b >= KB => write!(f, "{:.1}K", b as f64 / KB as f64),
            b => write!(f, "{}B", b),
        }
    }
}

/// CPU percentage (0-100 range)
#[derive(Debug, Clone, Copy)]
pub struct CpuPercent(f64);

impl CpuPercent {
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 100.0))
    }
}
```

## 4. Builder Pattern

Fluent configuration construction:

```rust
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub title: String,
    pub width: usize,
    pub format: RenderFormat,
    pub suffix: Option<String>,
}

#[derive(Debug, Default)]
pub struct RenderConfigBuilder {
    title: Option<String>,
    width: Option<usize>,
    format: Option<RenderFormat>,
    suffix: Option<String>,
}

impl RenderConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    pub fn format(mut self, format: RenderFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn build(self) -> RenderConfig {
        RenderConfig {
            title: self.title.unwrap_or_default(),
            width: self.width.unwrap_or(10),
            format: self.format.unwrap_or(RenderFormat::Default),
            suffix: self.suffix,
        }
    }
}

impl From<RenderConfigBuilder> for RenderConfig {
    fn from(builder: RenderConfigBuilder) -> Self {
        builder.build()
    }
}
```

## 5. Enum-Based Strategy

Encode behavior variants in enums:

```rust
#[derive(Debug, Clone, Copy)]
pub enum RenderFormat {
    /// Display as-is
    Default,
    /// Format as human-readable size (KB, MB, GB)
    ReadableSize,
    /// Format as duration (1h 23m 45s)
    Duration,
    /// Format with N decimal places
    Precision(u8),
    /// Format as percentage
    Percent,
}

impl RenderFormat {
    pub fn format(&self, value: f64) -> String {
        match self {
            Self::Default => format!("{}", value),
            Self::ReadableSize => Bytes(value as u64).to_string(),
            Self::Duration => format_duration(value as u64),
            Self::Precision(n) => format!("{:.1$}", value, *n as usize),
            Self::Percent => format!("{:.1}%", value),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Csv,
    OpenMetrics,
    Human,
}
```

## 6. Associated Types

Link types through traits:

```rust
pub trait FieldId: Clone {
    type Queriable: Queriable;
}

pub trait Queriable {
    type FieldId: FieldId<Queriable = Self>;

    fn query(&self, field_id: &Self::FieldId) -> Option<Field>;
}

pub trait Store: Send + Sync {
    type SampleType;
    type Offset: Clone;

    fn get_sample_at_timestamp(
        &self,
        timestamp: SystemTime,
        direction: Direction,
    ) -> Result<Option<(SystemTime, Self::SampleType)>>;
}

pub trait Cursor {
    type Item;
    type Offset: Clone;

    fn get_offset(&self) -> Self::Offset;
    fn advance(&mut self, direction: Direction) -> Result<bool>;
    fn get(&self) -> Result<Self::Item>;
}
```

## 7. Marker Traits

Signal capabilities without methods:

```rust
/// Indicates a model can be queried recursively
pub trait Recursive {}

/// Indicates a model has a displayable name
pub trait Nameable {
    fn name(&self) -> &str;
}

/// Indicates a type can be rendered in a table
pub trait Tabular: Queriable + Nameable {}

// Blanket implementation
impl<T: Queriable + Nameable> Tabular for T {}
```

## 8. Generic Bounds

Constrain generics appropriately:

```rust
use std::fmt::Debug;
use std::str::FromStr;

/// Read and parse a file into any FromStr type
pub fn read_file<T>(path: &Path) -> Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let contents = std::fs::read_to_string(path)?;
    contents.trim().parse().map_err(Into::into)
}

/// Accept any string-like type
pub fn process<S: AsRef<str>>(input: S) -> String {
    input.as_ref().to_uppercase()
}

/// Require multiple bounds
pub fn serialize_and_log<T>(value: &T) -> Result<String>
where
    T: Serialize + Debug,
{
    debug!(logger, "Serializing"; "value" => ?value);
    serde_json::to_string(value).map_err(Into::into)
}
```

---

## Related Patterns

- [Traits & Generics](traits-and-generics.md) - Trait hierarchies and generic bounds
- [Error Handling](error-handling.md) - Custom error type patterns
- [Serialization](serialization.md) - Serde derive patterns
- [Data Structures](data-structures.md) - Complex type compositions
