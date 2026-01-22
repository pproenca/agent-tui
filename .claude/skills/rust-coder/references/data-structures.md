# Data Structures Patterns

## Table of Contents
1. [Hierarchical Trees](#1-hierarchical-trees)
2. [Time-Series Storage](#2-time-series-storage)
3. [Bidirectional Cursors](#3-bidirectional-cursors)
4. [Index-Based Access](#4-index-based-access)
5. [Delta Computation](#5-delta-computation)
6. [Composite Models](#6-composite-models)

---

## 1. Hierarchical Trees

Model nested data like cgroups:

```rust
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct TreeNode<T> {
    pub data: T,
    pub children: BTreeSet<TreeNode<T>>,
    pub depth: u32,
}

impl<T: Clone> TreeNode<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            children: BTreeSet::new(),
            depth: 0,
        }
    }

    pub fn with_depth(data: T, depth: u32) -> Self {
        Self {
            data,
            children: BTreeSet::new(),
            depth,
        }
    }

    /// Recursive descent by path segments
    pub fn get_by_path<I, S>(&self, mut path: I) -> Option<&Self>
    where
        I: Iterator<Item = S>,
        S: AsRef<str>,
        T: Named,
    {
        match path.next() {
            None => Some(self),
            Some(segment) => {
                self.children
                    .iter()
                    .find(|c| c.data.name() == segment.as_ref())
                    .and_then(|child| child.get_by_path(path))
            }
        }
    }

    /// Functional path traversal
    pub fn get_by_path_iter<I, S>(&self, path: I) -> Option<&Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
        T: Named,
    {
        path.into_iter().try_fold(self, |cur, segment| {
            cur.children
                .iter()
                .find(|c| c.data.name() == segment.as_ref())
        })
    }

    /// Count all nodes in subtree
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }

    /// Iterate depth-first
    pub fn iter_dfs(&self) -> impl Iterator<Item = &Self> {
        std::iter::once(self).chain(
            self.children.iter().flat_map(|c| c.iter_dfs())
        )
    }
}

pub trait Named {
    fn name(&self) -> &str;
}
```

## 2. Time-Series Storage

Append-only storage with CRC validation:

```rust
use std::time::SystemTime;

const SHARD_DURATION: Duration = Duration::from_secs(24 * 60 * 60);
const INDEX_ENTRY_SIZE: usize = 24; // timestamp(8) + offset(8) + len(4) + crc(4)

#[derive(Debug)]
pub struct TimeSeriesStore {
    store_dir: PathBuf,
    current_shard: Option<Shard>,
}

#[derive(Debug)]
struct Shard {
    timestamp: SystemTime,
    data_file: File,
    index_file: File,
    data_offset: u64,
}

#[derive(Debug)]
struct IndexEntry {
    timestamp: SystemTime,
    offset: u64,
    length: u32,
    crc: u32,
}

impl TimeSeriesStore {
    pub fn append<T: Serialize>(&mut self, timestamp: SystemTime, data: &T) -> Result<()> {
        let shard = self.get_or_create_shard(timestamp)?;

        // Serialize with CBOR
        let bytes = serde_cbor::to_vec(data)?;
        let crc = crc32fast::hash(&bytes);

        // Write data
        let offset = shard.data_offset;
        shard.data_file.write_all(&bytes)?;
        shard.data_offset += bytes.len() as u64;

        // Write index entry
        let entry = IndexEntry {
            timestamp,
            offset,
            length: bytes.len() as u32,
            crc,
        };
        shard.index_file.write_all(&entry.to_bytes())?;

        Ok(())
    }

    pub fn read(&self, timestamp: SystemTime) -> Result<Option<Vec<u8>>> {
        let shard = self.find_shard(timestamp)?;
        let entry = self.find_index_entry(&shard, timestamp)?;

        match entry {
            None => Ok(None),
            Some(entry) => {
                let mut data = vec![0u8; entry.length as usize];
                shard.data_file.read_exact_at(&mut data, entry.offset)?;

                // Validate CRC
                if crc32fast::hash(&data) != entry.crc {
                    return Err(Error::CorruptData(timestamp));
                }

                Ok(Some(data))
            }
        }
    }

    fn get_or_create_shard(&mut self, timestamp: SystemTime) -> Result<&mut Shard> {
        let shard_ts = Self::shard_timestamp(timestamp);

        if self.current_shard.as_ref().map(|s| s.timestamp) != Some(shard_ts) {
            self.current_shard = Some(self.open_shard(shard_ts)?);
        }

        Ok(self.current_shard.as_mut().unwrap())
    }

    fn shard_timestamp(ts: SystemTime) -> SystemTime {
        let duration = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let shard_secs = (duration.as_secs() / SHARD_DURATION.as_secs()) * SHARD_DURATION.as_secs();
        SystemTime::UNIX_EPOCH + Duration::from_secs(shard_secs)
    }
}
```

## 3. Bidirectional Cursors

Navigate data forward and backward:

```rust
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Forward,
    Backward,
}

pub trait Cursor {
    type Item;
    type Offset: Clone;

    fn get_offset(&self) -> Self::Offset;
    fn set_offset(&mut self, offset: Self::Offset);
    fn advance(&mut self, direction: Direction) -> Result<bool>;
    fn get(&self) -> Result<Self::Item>;

    /// Move until valid item found
    fn next(&mut self, direction: Direction) -> Result<Option<Self::Item>> {
        while self.advance(direction)? {
            match self.get() {
                Ok(item) => return Ok(Some(item)),
                Err(Error::InvalidEntry) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(None)
    }
}

pub trait KeyedCursor<K: Ord>: Cursor {
    fn get_key(&self) -> Result<K>;

    /// Jump to first entry >= key
    fn jump_to_key(&mut self, key: &K, direction: Direction) -> Result<bool> {
        // First, move backward to find lower bound
        while let Ok(current_key) = self.get_key() {
            if &current_key < key {
                break;
            }
            if !self.advance(Direction::Backward)? {
                break;
            }
        }

        // Then move forward to find exact or next
        while let Ok(current_key) = self.get_key() {
            match current_key.cmp(key) {
                std::cmp::Ordering::Less => {
                    if !self.advance(Direction::Forward)? {
                        return Ok(false);
                    }
                }
                _ => return Ok(true),
            }
        }

        Ok(false)
    }
}
```

## 4. Index-Based Access

Fast lookup with memory-mapped indices:

```rust
use memmap2::Mmap;

pub struct IndexedStore {
    data_mmap: Mmap,
    index: Vec<IndexEntry>,
}

impl IndexedStore {
    pub fn open(path: &Path) -> Result<Self> {
        let data_file = File::open(path.join("data"))?;
        let data_mmap = unsafe { Mmap::map(&data_file)? };

        let index_bytes = std::fs::read(path.join("index"))?;
        let index = Self::parse_index(&index_bytes)?;

        Ok(Self { data_mmap, index })
    }

    /// Binary search by timestamp
    pub fn find_by_timestamp(&self, ts: SystemTime) -> Option<&[u8]> {
        let idx = self.index
            .binary_search_by_key(&ts, |e| e.timestamp)
            .ok()?;

        let entry = &self.index[idx];
        let start = entry.offset as usize;
        let end = start + entry.length as usize;

        Some(&self.data_mmap[start..end])
    }

    /// Range query
    pub fn range(&self, start: SystemTime, end: SystemTime) -> impl Iterator<Item = &[u8]> {
        let start_idx = self.index
            .partition_point(|e| e.timestamp < start);
        let end_idx = self.index
            .partition_point(|e| e.timestamp <= end);

        self.index[start_idx..end_idx].iter().map(|entry| {
            let start = entry.offset as usize;
            let end = start + entry.length as usize;
            &self.data_mmap[start..end]
        })
    }
}
```

## 5. Delta Computation

Calculate rates from cumulative counters:

```rust
#[derive(Debug, Clone)]
pub struct Sample {
    pub timestamp: SystemTime,
    pub cpu_time: u64,      // Cumulative
    pub io_bytes: u64,      // Cumulative
    pub memory: u64,        // Point-in-time
}

#[derive(Debug, Clone)]
pub struct Model {
    pub timestamp: SystemTime,
    pub elapsed: Duration,
    pub cpu_pct: Option<f64>,       // Derived
    pub io_bytes_per_sec: Option<f64>, // Derived
    pub memory: u64,                // Direct
}

impl Model {
    pub fn new(
        current: &Sample,
        previous: Option<(&Sample, Duration)>,
    ) -> Self {
        let (cpu_pct, io_bytes_per_sec) = match previous {
            Some((prev, elapsed)) => {
                let elapsed_secs = elapsed.as_secs_f64();

                let cpu_delta = current.cpu_time.saturating_sub(prev.cpu_time);
                let cpu_pct = Some((cpu_delta as f64 / elapsed_secs) * 100.0);

                let io_delta = current.io_bytes.saturating_sub(prev.io_bytes);
                let io_per_sec = Some(io_delta as f64 / elapsed_secs);

                (cpu_pct, io_per_sec)
            }
            None => (None, None),
        };

        Self {
            timestamp: current.timestamp,
            elapsed: previous.map(|(_, d)| d).unwrap_or_default(),
            cpu_pct,
            io_bytes_per_sec,
            memory: current.memory,
        }
    }
}

/// Macro for per-second rate calculation
macro_rules! count_per_sec {
    ($curr:expr, $prev:expr, $field:ident, $elapsed:expr) => {
        match ($curr.$field, $prev.map(|p| p.$field)) {
            (Some(c), Some(Some(p))) if $elapsed.as_secs_f64() > 0.0 => {
                Some((c.saturating_sub(p) as f64) / $elapsed.as_secs_f64())
            }
            _ => None,
        }
    };
}

/// Macro for percentage calculation
macro_rules! usec_pct {
    ($curr:expr, $prev:expr, $field:ident, $elapsed:expr) => {
        match ($curr.$field, $prev.map(|p| p.$field)) {
            (Some(c), Some(Some(p))) => {
                let delta = c.saturating_sub(p) as f64;
                let elapsed_usec = $elapsed.as_micros() as f64;
                Some((delta / elapsed_usec) * 100.0)
            }
            _ => None,
        }
    };
}
```

## 6. Composite Models

Aggregate multiple data sources:

```rust
#[derive(Debug, Default)]
pub struct SystemModel {
    pub timestamp: SystemTime,
    pub elapsed: Duration,

    // Core subsystems
    pub cpu: CpuModel,
    pub memory: MemoryModel,
    pub io: IoModel,
    pub network: NetworkModel,

    // Optional subsystems
    pub gpu: Option<GpuModel>,
    pub pressure: Option<PressureModel>,
}

#[derive(Debug, Default)]
pub struct FullModel {
    pub system: SystemModel,
    pub cgroups: CgroupModel,
    pub processes: BTreeMap<u32, ProcessModel>,
}

impl FullModel {
    pub fn new(sample: &Sample, prev: Option<(&Sample, Duration)>) -> Self {
        Self {
            system: SystemModel::new(&sample.system, prev.map(|(s, d)| (&s.system, d))),
            cgroups: CgroupModel::new(&sample.cgroups, prev.map(|(s, d)| (&s.cgroups, d))),
            processes: sample.processes.iter()
                .map(|(&pid, proc)| {
                    let prev_proc = prev.and_then(|(s, d)| {
                        s.processes.get(&pid).map(|p| (p, d))
                    });
                    (pid, ProcessModel::new(proc, prev_proc))
                })
                .collect(),
        }
    }

    /// Query any field by path
    pub fn query(&self, path: &str) -> Option<Field> {
        let mut parts = path.split('.');
        match parts.next()? {
            "system" => self.system.query_path(parts),
            "cgroup" => self.cgroups.query_path(parts),
            "process" => {
                let pid: u32 = parts.next()?.parse().ok()?;
                self.processes.get(&pid)?.query_path(parts)
            }
            _ => None,
        }
    }
}
```
