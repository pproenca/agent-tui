# Serialization Patterns

## Table of Contents
1. [Serde Basics](#1-serde-basics)
2. [CBOR for Storage](#2-cbor-for-storage)
3. [JSON Output](#3-json-output)
4. [CSV Export](#4-csv-export)
5. [OpenMetrics Format](#5-openmetrics-format)
6. [Custom Serializers](#6-custom-serializers)

---

## 1. Serde Basics

Derive serialization with proper defaults:

```rust
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub timestamp: SystemTime,

    #[serde(default)]
    pub cpu: CpuSample,

    #[serde(default)]
    pub memory: MemorySample,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuSample>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuSample {
    #[serde(default)]
    pub user: u64,

    #[serde(default)]
    pub system: u64,

    #[serde(default)]
    pub idle: u64,

    /// Renamed field for compatibility
    #[serde(rename = "iowait", default)]
    pub io_wait: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    Stopped,
    Sleeping,
    DiskSleep,
}

/// Config with serde defaults
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub interval: u64,
    pub store_dir: PathBuf,
    pub compress: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: 5,
            store_dir: PathBuf::from("/var/log/mytool"),
            compress: true,
        }
    }
}
```

## 2. CBOR for Storage

Efficient binary serialization:

```rust
use serde_cbor;

pub struct Store {
    data_file: File,
    compress: bool,
}

impl Store {
    pub fn append<T: Serialize>(&mut self, sample: &T) -> Result<()> {
        let bytes = serde_cbor::to_vec(sample)
            .context("CBOR serialization failed")?;

        let data = if self.compress {
            zstd::encode_all(&bytes[..], 3)?
        } else {
            bytes
        };

        // Write length prefix
        self.data_file.write_all(&(data.len() as u32).to_le_bytes())?;
        self.data_file.write_all(&data)?;

        Ok(())
    }

    pub fn read<T: DeserializeOwned>(&self, offset: u64) -> Result<T> {
        let mut len_buf = [0u8; 4];
        self.data_file.read_exact_at(&mut len_buf, offset)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut data = vec![0u8; len];
        self.data_file.read_exact_at(&mut data, offset + 4)?;

        let bytes = if self.compress {
            zstd::decode_all(&data[..])?
        } else {
            data
        };

        serde_cbor::from_slice(&bytes)
            .context("CBOR deserialization failed")
    }
}

/// Dictionary compression for better ratios
pub struct DictCompressor {
    dict: Vec<u8>,
    compressor: zstd::bulk::Compressor<'static>,
}

impl DictCompressor {
    pub fn new(samples: &[Vec<u8>]) -> Result<Self> {
        let dict = zstd::dict::from_samples(&samples, 64 * 1024)?;
        let compressor = zstd::bulk::Compressor::with_dictionary(3, &dict)?;

        Ok(Self { dict, compressor })
    }

    pub fn compress(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.compressor.compress(data)
            .map_err(Into::into)
    }
}
```

## 3. JSON Output

Structured JSON for scripting:

```rust
use serde_json;

#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub timestamp: String,
    pub data: serde_json::Value,
}

pub struct JsonWriter<W: Write> {
    writer: W,
    pretty: bool,
    first: bool,
}

impl<W: Write> JsonWriter<W> {
    pub fn new(writer: W, pretty: bool) -> Self {
        Self {
            writer,
            pretty,
            first: true,
        }
    }

    pub fn write_record<T: Serialize>(&mut self, record: &T) -> Result<()> {
        if !self.first {
            writeln!(self.writer)?;
        }
        self.first = false;

        if self.pretty {
            serde_json::to_writer_pretty(&mut self.writer, record)?;
        } else {
            serde_json::to_writer(&mut self.writer, record)?;
        }

        Ok(())
    }

    /// Write as JSON array
    pub fn write_array<T: Serialize>(mut self, records: &[T]) -> Result<()> {
        if self.pretty {
            serde_json::to_writer_pretty(&mut self.writer, records)?;
        } else {
            serde_json::to_writer(&mut self.writer, records)?;
        }
        Ok(())
    }
}

/// Convert model to JSON value dynamically
pub fn model_to_json<T: Queriable>(model: &T, fields: &[T::FieldId]) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for field_id in fields {
        if let Some(value) = model.query(field_id) {
            let key = field_id.name().to_string();
            map.insert(key, value.to_json());
        }
    }

    serde_json::Value::Object(map)
}
```

## 4. CSV Export

Tabular output:

```rust
use std::io::Write;

pub struct CsvWriter<W: Write> {
    writer: W,
    delimiter: char,
    header_written: bool,
}

impl<W: Write> CsvWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            delimiter: ',',
            header_written: false,
        }
    }

    pub fn with_delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = delimiter;
        self
    }

    pub fn write_header(&mut self, fields: &[&str]) -> Result<()> {
        let line = fields.join(&self.delimiter.to_string());
        writeln!(self.writer, "{}", line)?;
        self.header_written = true;
        Ok(())
    }

    pub fn write_row(&mut self, values: &[String]) -> Result<()> {
        let escaped: Vec<String> = values.iter()
            .map(|v| self.escape_value(v))
            .collect();
        let line = escaped.join(&self.delimiter.to_string());
        writeln!(self.writer, "{}", line)?;
        Ok(())
    }

    fn escape_value(&self, value: &str) -> String {
        if value.contains(self.delimiter) || value.contains('"') || value.contains('\n') {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }
}

/// Generate CSV from queriable model
pub fn dump_csv<T: Queriable, W: Write>(
    models: &[T],
    fields: &[T::FieldId],
    writer: W,
) -> Result<()> {
    let mut csv = CsvWriter::new(writer);

    // Header
    let headers: Vec<&str> = fields.iter()
        .map(|f| f.name())
        .collect();
    csv.write_header(&headers)?;

    // Rows
    for model in models {
        let values: Vec<String> = fields.iter()
            .map(|f| {
                model.query(f)
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            })
            .collect();
        csv.write_row(&values)?;
    }

    Ok(())
}
```

## 5. OpenMetrics Format

Prometheus-compatible output:

```rust
#[derive(Debug, Clone)]
pub struct OpenMetricsConfig {
    pub metric_name: String,
    pub metric_type: OpenMetricsType,
    pub labels: Vec<(String, String)>,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum OpenMetricsType {
    Gauge,
    Counter,
    Histogram,
    Summary,
}

impl std::fmt::Display for OpenMetricsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gauge => write!(f, "gauge"),
            Self::Counter => write!(f, "counter"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
        }
    }
}

pub struct OpenMetricsWriter<W: Write> {
    writer: W,
    written_metadata: HashSet<String>,
}

impl<W: Write> OpenMetricsWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            written_metadata: HashSet::new(),
        }
    }

    pub fn write_metric(&mut self, config: &OpenMetricsConfig, value: f64) -> Result<()> {
        // Write metadata once per metric name
        if !self.written_metadata.contains(&config.metric_name) {
            if let Some(help) = &config.help {
                writeln!(self.writer, "# HELP {} {}", config.metric_name, help)?;
            }
            writeln!(self.writer, "# TYPE {} {}", config.metric_name, config.metric_type)?;
            self.written_metadata.insert(config.metric_name.clone());
        }

        // Write metric line
        write!(self.writer, "{}", config.metric_name)?;

        if !config.labels.is_empty() {
            let labels: Vec<String> = config.labels.iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, Self::escape_label_value(v)))
                .collect();
            write!(self.writer, "{{{}}}", labels.join(","))?;
        }

        writeln!(self.writer, " {}", value)?;

        Ok(())
    }

    fn escape_label_value(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    pub fn finish(mut self) -> Result<()> {
        writeln!(self.writer, "# EOF")?;
        Ok(())
    }
}
```

## 6. Custom Serializers

Handle special types:

```rust
use serde::Serializer;
use serde::Deserializer;

/// Serialize SystemTime as Unix timestamp
pub mod timestamp_serde {
    use super::*;

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(SystemTime::UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
    }
}

/// Serialize Duration as milliseconds
pub mod duration_millis {
    use super::*;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// Usage
#[derive(Serialize, Deserialize)]
pub struct Record {
    #[serde(with = "timestamp_serde")]
    pub timestamp: SystemTime,

    #[serde(with = "duration_millis")]
    pub elapsed: Duration,
}

/// Serialize Option<T> as empty string when None (for CSV)
pub fn option_to_string<T: std::fmt::Display>(opt: &Option<T>) -> String {
    match opt {
        Some(v) => v.to_string(),
        None => String::new(),
    }
}
```
