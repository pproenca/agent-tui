# Parsing Patterns

## Table of Contents
1. [Macro-Based Field Parsing](#1-macro-based-field-parsing)
2. [Line-Based Whitespace Splitting](#2-line-based-whitespace-splitting)
3. [Key-Value File Parsing](#3-key-value-file-parsing)
4. [Range and Set Parsing](#4-range-and-set-parsing)
5. [Special Value Handling](#5-special-value-handling)
6. [FromStr Implementations](#6-fromstr-implementations)
7. [Conditional Field Extraction](#7-conditional-field-extraction)

---

## 1. Macro-Based Field Parsing

Create macros for repetitive parsing with error context:

```rust
/// Parse next item from iterator or return error
macro_rules! parse_item {
    ($iter:expr, $type:ty, $line:expr, $field:expr) => {
        $iter
            .next()
            .ok_or_else(|| Error::ParseError {
                line: $line.to_string(),
                field: $field,
                expected: stringify!($type),
            })?
            .parse::<$type>()
            .map_err(|_| Error::ParseError {
                line: $line.to_string(),
                field: $field,
                expected: stringify!($type),
            })?
    };
}

/// Parse optional item, returning None if missing
macro_rules! parse_opt {
    ($iter:expr, $type:ty) => {
        $iter.next().and_then(|s| s.parse::<$type>().ok())
    };
}

/// Parse kilobytes value (expects "1234 kB" format)
macro_rules! parse_kb {
    ($iter:expr, $line:expr) => {{
        let value = parse_item!($iter, u64, $line, "kb_value");
        let _unit = $iter.next(); // Skip "kB"
        value * 1024
    }};
}

// Usage
fn parse_meminfo_line(line: &str) -> Result<(String, u64)> {
    let mut parts = line.split_whitespace();
    let key = parse_item!(parts, String, line, "key");
    let value = parse_kb!(parts, line);
    Ok((key.trim_end_matches(':').to_string(), value))
}
```

## 2. Line-Based Whitespace Splitting

Parse positional data from space-separated fields:

```rust
fn parse_cpu_stat(line: &str) -> Result<CpuStat> {
    let mut parts = line.split_whitespace();

    let cpu_id = parts.next()
        .ok_or_else(|| Error::InvalidFormat("missing cpu id"))?;

    let user = parts.next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::InvalidFormat("missing user time"))?;

    let nice = parts.next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::InvalidFormat("missing nice time"))?;

    let system = parts.next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::InvalidFormat("missing system time"))?;

    // Continue for remaining fields...

    Ok(CpuStat { cpu_id, user, nice, system, /* ... */ })
}

/// Parse with enumerated indices for complex formats
fn parse_mount_info(line: &str) -> Result<MountInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Mount info has variable fields, find separator
    let sep_idx = parts.iter()
        .position(|&s| s == "-")
        .ok_or_else(|| Error::InvalidFormat("missing separator"))?;

    Ok(MountInfo {
        mount_id: parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0),
        parent_id: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        mount_point: parts.get(4).map(|s| s.to_string()).unwrap_or_default(),
        fs_type: parts.get(sep_idx + 1).map(|s| s.to_string()).unwrap_or_default(),
        source: parts.get(sep_idx + 2).map(|s| s.to_string()).unwrap_or_default(),
    })
}
```

## 3. Key-Value File Parsing

Parse files with "key value" or "key: value" format:

```rust
/// Trait for types that can be parsed from key-value files
pub trait KVRead: Sized {
    fn read_from_kv<I: Iterator<Item = (String, String)>>(iter: I) -> Result<Self>;
}

/// Parse "key value" format (space-separated)
fn parse_kv_space(content: &str) -> impl Iterator<Item = (String, String)> + '_ {
    content.lines().filter_map(|line| {
        let mut parts = line.splitn(2, ' ');
        let key = parts.next()?.to_string();
        let value = parts.next()?.to_string();
        Some((key, value))
    })
}

/// Parse "key: value" format (colon-separated)
fn parse_kv_colon(content: &str) -> impl Iterator<Item = (String, String)> + '_ {
    content.lines().filter_map(|line| {
        let (key, value) = line.split_once(':')?;
        Some((key.trim().to_string(), value.trim().to_string()))
    })
}

/// Generate KVRead impl with macro
macro_rules! impl_kv_read {
    ($type:ty, $($field:ident: $key:literal),+ $(,)?) => {
        impl KVRead for $type {
            fn read_from_kv<I: Iterator<Item = (String, String)>>(iter: I) -> Result<Self> {
                let mut result = Self::default();
                for (key, value) in iter {
                    match key.as_str() {
                        $($key => result.$field = value.parse().ok(),)+
                        _ => {}
                    }
                }
                Ok(result)
            }
        }
    };
}

impl_kv_read!(MemoryStat,
    anon: "anon",
    file: "file",
    slab: "slab",
    sock: "sock",
);
```

## 4. Range and Set Parsing

Parse CPU/memory ranges like "0-3,5,7-9":

```rust
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct CpuSet(BTreeSet<u32>);

impl std::str::FromStr for CpuSet {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut set = BTreeSet::new();

        for part in s.trim().split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((start, end)) = part.split_once('-') {
                let start: u32 = start.parse()
                    .map_err(|_| Error::InvalidFormat("range start"))?;
                let end: u32 = end.parse()
                    .map_err(|_| Error::InvalidFormat("range end"))?;
                set.extend(start..=end);
            } else {
                let num: u32 = part.parse()
                    .map_err(|_| Error::InvalidFormat("cpu number"))?;
                set.insert(num);
            }
        }

        Ok(CpuSet(set))
    }
}

impl std::fmt::Display for CpuSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Compress back to ranges
        let mut ranges: Vec<(u32, u32)> = Vec::new();
        for &n in &self.0 {
            match ranges.last_mut() {
                Some((_, end)) if *end + 1 == n => *end = n,
                _ => ranges.push((n, n)),
            }
        }

        let formatted: Vec<String> = ranges.iter()
            .map(|(s, e)| if s == e { format!("{}", s) } else { format!("{}-{}", s, e) })
            .collect();

        write!(f, "{}", formatted.join(","))
    }
}
```

## 5. Special Value Handling

Handle kernel's special string values:

```rust
/// Parse value that can be "max" or a number
#[derive(Debug, Clone, Copy)]
pub enum MaybeMax<T> {
    Value(T),
    Max,
}

impl<T: std::str::FromStr> std::str::FromStr for MaybeMax<T> {
    type Err = T::Err;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.trim().eq_ignore_ascii_case("max") {
            Ok(MaybeMax::Max)
        } else {
            s.parse().map(MaybeMax::Value)
        }
    }
}

/// CPU max format: "quota period" where quota can be "max"
#[derive(Debug, Clone)]
pub struct CpuMax {
    pub quota: MaybeMax<u64>,
    pub period: u64,
}

impl std::str::FromStr for CpuMax {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split_whitespace();
        let quota = parts.next()
            .ok_or(Error::InvalidFormat("missing quota"))?
            .parse()
            .map_err(|_| Error::InvalidFormat("invalid quota"))?;
        let period = parts.next()
            .ok_or(Error::InvalidFormat("missing period"))?
            .parse()
            .map_err(|_| Error::InvalidFormat("invalid period"))?;

        Ok(CpuMax { quota, period })
    }
}
```

## 6. FromStr Implementations

Implement standard parsing trait:

```rust
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidState {
    Running,
    Sleeping,
    DiskSleep,
    Stopped,
    Zombie,
    Dead,
    Idle,
}

impl FromStr for PidState {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.chars().next() {
            Some('R') => Ok(Self::Running),
            Some('S') => Ok(Self::Sleeping),
            Some('D') => Ok(Self::DiskSleep),
            Some('T') => Ok(Self::Stopped),
            Some('Z') => Ok(Self::Zombie),
            Some('X') => Ok(Self::Dead),
            Some('I') => Ok(Self::Idle),
            _ => Err(Error::InvalidFormat("unknown process state")),
        }
    }
}

impl PidState {
    pub fn as_char(&self) -> char {
        match self {
            Self::Running => 'R',
            Self::Sleeping => 'S',
            Self::DiskSleep => 'D',
            Self::Stopped => 'T',
            Self::Zombie => 'Z',
            Self::Dead => 'X',
            Self::Idle => 'I',
        }
    }
}
```

## 7. Conditional Field Extraction

Extract values between delimiters:

```rust
/// Extract substring between parentheses
fn extract_comm(line: &str) -> Option<&str> {
    let start = line.find('(')?;
    let end = line.rfind(')')?;
    if start < end {
        Some(&line[start + 1..end])
    } else {
        None
    }
}

/// Extract value after prefix
fn extract_after_prefix<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    line.strip_prefix(prefix).map(|s| s.trim())
}

/// Extract nth field (0-indexed)
fn extract_field(line: &str, n: usize) -> Option<&str> {
    line.split_whitespace().nth(n)
}

/// Parse line with format: "key: value unit"
fn parse_with_unit(line: &str) -> Option<(String, u64, String)> {
    let (key, rest) = line.split_once(':')?;
    let mut parts = rest.trim().split_whitespace();
    let value: u64 = parts.next()?.parse().ok()?;
    let unit = parts.next().unwrap_or("").to_string();
    Some((key.to_string(), value, unit))
}
```

---

## Related Patterns

- [Type Design](type-design.md) - FromStr implementations and newtype wrappers
- [Error Handling](error-handling.md) - Parse error types and context
- [File I/O](file-io.md) - Reading files for parsing
- [Macros](macros.md) - Parsing helper macros
