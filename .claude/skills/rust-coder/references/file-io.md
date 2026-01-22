# File I/O Patterns

## Table of Contents
1. [Buffered Reading](#1-buffered-reading)
2. [Memory Mapping](#2-memory-mapping)
3. [Atomic Writes](#3-atomic-writes)
4. [File Locking](#4-file-locking)
5. [Directory Operations](#5-directory-operations)

---

## 1. Buffered Reading

Efficient file reading with reusable buffers:

```rust
use std::cell::RefCell;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

/// Reader with reusable internal buffer
pub struct BufferedFileReader {
    path_buf: PathBuf,
    buffer: RefCell<Vec<u8>>,
}

impl BufferedFileReader {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            path_buf: base_path.into(),
            buffer: RefCell::new(Vec::with_capacity(4096)),
        }
    }

    /// Read file into reusable buffer
    pub fn read_file(&self, relative_path: &str) -> Result<&str> {
        let full_path = self.path_buf.join(relative_path);

        let mut buffer = self.buffer.borrow_mut();
        buffer.clear();

        let mut file = File::open(&full_path)
            .with_context(|| format!("Failed to open: {:?}", full_path))?;

        file.read_to_end(&mut buffer)?;

        // Safety: we control the buffer lifetime
        Ok(unsafe {
            std::str::from_utf8_unchecked(
                std::slice::from_raw_parts(buffer.as_ptr(), buffer.len())
            )
        })
    }
}

/// Line-by-line reading for large files
pub fn read_lines(path: &Path) -> Result<impl Iterator<Item = Result<String>>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().map(|r| r.map_err(Into::into)))
}

/// Process file line by line with early exit
pub fn process_lines<F>(path: &Path, mut f: F) -> Result<()>
where
    F: FnMut(&str) -> Result<bool>, // Return false to stop
{
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if !f(&line)? {
            break;
        }
    }

    Ok(())
}
```

## 2. Memory Mapping

Zero-copy file access:

```rust
use memmap2::Mmap;
use memmap2::MmapOptions;

pub struct MappedFile {
    mmap: Mmap,
    path: PathBuf,
}

impl MappedFile {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let file = File::open(&path)?;

        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .context("Failed to mmap file")?
        };

        Ok(Self { mmap, path })
    }

    /// Open with specific length (for growing files)
    pub fn open_with_len(path: impl Into<PathBuf>, len: usize) -> Result<Self> {
        let path = path.into();
        let file = File::open(&path)?;

        let mmap = unsafe {
            MmapOptions::new()
                .len(len)
                .map(&file)?
        };

        Ok(Self { mmap, path })
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.mmap
    }

    /// Read at offset
    pub fn read_at(&self, offset: usize, len: usize) -> Option<&[u8]> {
        if offset + len <= self.mmap.len() {
            Some(&self.mmap[offset..offset + len])
        } else {
            None
        }
    }

    /// Read struct at offset
    pub fn read_struct_at<T: Copy>(&self, offset: usize) -> Option<T> {
        let size = std::mem::size_of::<T>();
        let bytes = self.read_at(offset, size)?;

        // Safety: T is Copy (POD type), bytes are aligned
        Some(unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const T) })
    }
}

/// Read-only access with CRC validation
pub fn read_validated<T: DeserializeOwned>(
    mmap: &Mmap,
    offset: usize,
    len: usize,
    expected_crc: u32,
) -> Result<T> {
    let bytes = &mmap[offset..offset + len];
    let actual_crc = crc32fast::hash(bytes);

    if actual_crc != expected_crc {
        anyhow::bail!("CRC mismatch: expected {}, got {}", expected_crc, actual_crc);
    }

    serde_cbor::from_slice(bytes).map_err(Into::into)
}
```

## 3. Atomic Writes

Safe file updates:

```rust
use std::fs::rename;
use tempfile::NamedTempFile;

/// Write file atomically via temp file + rename
pub fn write_atomic(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent()
        .ok_or_else(|| anyhow::anyhow!("No parent directory"))?;

    // Create temp file in same directory (for same-filesystem rename)
    let mut temp = NamedTempFile::new_in(parent)?;
    temp.write_all(content)?;
    temp.flush()?;

    // Atomic rename
    temp.persist(path)?;

    Ok(())
}

/// Write with fsync for durability
pub fn write_durable(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent()
        .ok_or_else(|| anyhow::anyhow!("No parent directory"))?;

    let mut temp = NamedTempFile::new_in(parent)?;
    temp.write_all(content)?;
    temp.as_file().sync_all()?; // Fsync data

    let temp_path = temp.path().to_owned();
    temp.persist(path)?;

    // Fsync parent directory for rename durability
    let dir = File::open(parent)?;
    dir.sync_all()?;

    Ok(())
}

/// Append-only file with CRC
pub struct AppendOnlyFile {
    file: File,
    offset: u64,
}

impl AppendOnlyFile {
    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        let offset = file.metadata()?.len();

        Ok(Self { file, offset })
    }

    pub fn append(&mut self, data: &[u8]) -> Result<(u64, u32)> {
        let crc = crc32fast::hash(data);
        let start_offset = self.offset;

        // Write length prefix
        self.file.write_all(&(data.len() as u32).to_le_bytes())?;
        // Write CRC
        self.file.write_all(&crc.to_le_bytes())?;
        // Write data
        self.file.write_all(data)?;

        self.offset += 8 + data.len() as u64;

        Ok((start_offset, crc))
    }

    pub fn sync(&self) -> Result<()> {
        self.file.sync_all().map_err(Into::into)
    }
}
```

## 4. File Locking

Prevent concurrent access:

```rust
use fs2::FileExt;
use std::fs::OpenOptions;

/// Exclusive lock for writers
pub struct ExclusiveLock {
    file: File,
}

impl ExclusiveLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)?;

        file.try_lock_exclusive()
            .with_context(|| format!("Failed to acquire lock: {:?}", path))?;

        Ok(Self { file })
    }

    /// Block until lock acquired
    pub fn acquire_blocking(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)?;

        file.lock_exclusive()?;

        Ok(Self { file })
    }
}

impl Drop for ExclusiveLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Shared lock for readers
pub struct SharedLock {
    file: File,
}

impl SharedLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        file.lock_shared()?;
        Ok(Self { file })
    }
}

impl Drop for SharedLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Use lock for critical operations
pub fn with_lock<F, T>(lock_path: &Path, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let _lock = ExclusiveLock::acquire(lock_path)?;
    f()
}
```

## 5. Directory Operations

Working with directories:

```rust
use std::fs;
use walkdir::WalkDir;

/// List files matching pattern
pub fn list_files(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|e| e == extension) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

/// Recursively find files
pub fn find_files(root: &Path, predicate: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| predicate(p))
        .collect()
}

/// Create directory with parents
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {:?}", path))?;
    }
    Ok(())
}

/// Clean old files (retention policy)
pub fn cleanup_old_files(dir: &Path, max_age: Duration) -> Result<usize> {
    let cutoff = SystemTime::now() - max_age;
    let mut removed = 0;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff {
                    fs::remove_file(entry.path())?;
                    removed += 1;
                }
            }
        }
    }

    Ok(removed)
}

/// Disk space check
pub fn check_disk_space(path: &Path, min_bytes: u64) -> Result<bool> {
    use nix::sys::statvfs::statvfs;

    let stat = statvfs(path)?;
    let available = stat.blocks_available() * stat.block_size();

    Ok(available >= min_bytes)
}
```
