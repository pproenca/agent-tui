# Traits and Generics Patterns

## Table of Contents
1. [Extension Traits](#1-extension-traits)
2. [Trait Hierarchies](#2-trait-hierarchies)
3. [Blanket Implementations](#3-blanket-implementations)
4. [Object Safety](#4-object-safety)
5. [Generic Containers](#5-generic-containers)
6. [Trait Objects vs Generics](#6-trait-objects-vs-generics)

---

## 1. Extension Traits

Add methods to existing types:

```rust
/// Extension trait for converting enum to/from char
pub trait PidStateExt {
    fn from_char(c: char) -> Option<PidState>;
    fn as_char(&self) -> char;
}

impl PidStateExt for PidState {
    fn from_char(c: char) -> Option<PidState> {
        match c {
            'R' => Some(PidState::Running),
            'S' => Some(PidState::Sleeping),
            'D' => Some(PidState::DiskSleep),
            _ => None,
        }
    }

    fn as_char(&self) -> char {
        match self {
            PidState::Running => 'R',
            PidState::Sleeping => 'S',
            PidState::DiskSleep => 'D',
        }
    }
}

/// Extension trait for Option arithmetic
pub trait OptionExt<T> {
    fn opt_add(self, other: Option<T>) -> Option<T>;
    fn opt_sub(self, other: Option<T>) -> Option<T>;
}

impl<T: std::ops::Add<Output = T>> OptionExt<T> for Option<T> {
    fn opt_add(self, other: Option<T>) -> Option<T> {
        match (self, other) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    fn opt_sub(self, other: Option<T>) -> Option<T>
    where
        T: std::ops::Sub<Output = T>,
    {
        match (self, other) {
            (Some(a), Some(b)) => Some(a - b),
            _ => None,
        }
    }
}
```

## 2. Trait Hierarchies

Build capabilities through inheritance:

```rust
/// Base trait for any renderable item
pub trait HasRenderConfig {
    fn get_render_config_builder(&self) -> RenderConfigBuilder;

    fn get_render_config(&self) -> RenderConfig {
        self.get_render_config_builder().build()
    }
}

/// Extended trait for dump-capable items
pub trait HasRenderConfigForDump: HasRenderConfig {
    fn get_openmetrics_config_builder(&self) -> RenderOpenMetricsConfigBuilder;

    fn render_for_dump(&self, value: &Field) -> String {
        let config = self.get_render_config();
        config.format.format(value.as_f64())
    }
}

/// Query capability trait
pub trait Queriable {
    type FieldId: FieldId<Queriable = Self>;

    fn query(&self, field_id: &Self::FieldId) -> Option<Field>;
}

/// Combined trait for fully-featured models
pub trait FullModel: Queriable + HasRenderConfigForDump + Recursive + Nameable {}

// Blanket implementation
impl<T> FullModel for T where T: Queriable + HasRenderConfigForDump + Recursive + Nameable {}
```

## 3. Blanket Implementations

Implement traits generically:

```rust
/// Blanket impl for all types that can be queried
impl<T: Queriable> HasRenderConfig for T {
    fn get_render_config_builder(&self) -> RenderConfigBuilder {
        RenderConfigBuilder::new()
    }
}

/// Blanket impl for containers
impl<T: Queriable, K: Clone + Ord> QueriableContainer for BTreeMap<K, T> {
    type Key = K;
    type Item = T;

    fn get_item(&self, key: &Self::Key) -> Option<&Self::Item> {
        self.get(key)
    }

    fn keys(&self) -> impl Iterator<Item = &Self::Key> {
        BTreeMap::keys(self)
    }
}

/// Blanket impl for Vec
impl<T: Queriable> QueriableContainer for Vec<T> {
    type Key = usize;
    type Item = T;

    fn get_item(&self, key: &Self::Key) -> Option<&Self::Item> {
        self.get(*key)
    }

    fn keys(&self) -> impl Iterator<Item = &Self::Key> {
        (0..self.len()).collect::<Vec<_>>().into_iter()
    }
}
```

## 4. Object Safety

Design traits for dynamic dispatch:

```rust
/// Object-safe store trait
pub trait Store: Send + Sync {
    fn get_sample_at_timestamp(
        &self,
        timestamp: SystemTime,
        direction: Direction,
    ) -> Result<Option<(SystemTime, Box<dyn Sample>)>>;

    fn get_time_range(&self) -> Result<Option<(SystemTime, SystemTime)>>;
}

/// NOT object-safe (has associated type with Self bound)
pub trait Cursor {
    type Item;  // Prevents object safety
    fn next(&mut self) -> Option<Self::Item>;
}

/// Object-safe alternative
pub trait DynCursor: Send {
    fn next_boxed(&mut self) -> Option<Box<dyn std::any::Any>>;
}

/// Object-safe with explicit Box
pub trait Writer: Send + Sync {
    fn write_record(&mut self, record: &Record) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

// Can use as trait object
fn create_writer(format: OutputFormat) -> Box<dyn Writer> {
    match format {
        OutputFormat::Json => Box::new(JsonWriter::new()),
        OutputFormat::Csv => Box::new(CsvWriter::new()),
    }
}
```

## 5. Generic Containers

Abstract over collection types:

```rust
/// Container that can be queried by key
pub trait QueriableContainer {
    type Key: Clone;
    type Item: Queriable;

    fn get_item(&self, key: &Self::Key) -> Option<&Self::Item>;
    fn keys(&self) -> impl Iterator<Item = &Self::Key>;

    fn query_item(&self, key: &Self::Key, field: &<Self::Item as Queriable>::FieldId) -> Option<Field> {
        self.get_item(key).and_then(|item| item.query(field))
    }
}

/// Field ID for container queries
#[derive(Debug, Clone)]
pub struct ContainerFieldId<K, F> {
    pub key: K,
    pub field: F,
}

impl<K, F> ContainerFieldId<K, F> {
    pub fn new(key: K, field: F) -> Self {
        Self { key, field }
    }
}

/// Generic query function
fn query_container<C, K, F>(container: &C, key: &K, field: &F) -> Option<Field>
where
    C: QueriableContainer<Key = K>,
    C::Item: Queriable<FieldId = F>,
    K: Clone,
    F: Clone,
{
    container.get_item(key)?.query(field)
}
```

## 6. Trait Objects vs Generics

Choose the right abstraction:

```rust
// GENERICS: Use when types known at compile time, need performance
fn process_all<T: Queriable>(items: &[T], field: &T::FieldId) -> Vec<Option<Field>> {
    items.iter().map(|item| item.query(field)).collect()
}

// TRAIT OBJECTS: Use when types vary at runtime
fn process_dynamic(items: &[Box<dyn Sample>]) -> Vec<String> {
    items.iter().map(|item| item.summary()).collect()
}

// ENUM DISPATCH: Use for fixed set of variants (faster than trait objects)
enum Output {
    Json(JsonWriter),
    Csv(CsvWriter),
    Text(TextWriter),
}

impl Output {
    fn write(&mut self, record: &Record) -> Result<()> {
        match self {
            Output::Json(w) => w.write(record),
            Output::Csv(w) => w.write(record),
            Output::Text(w) => w.write(record),
        }
    }
}

// IMPL TRAIT: Use for returning iterators or closures
fn get_fields(model: &Model) -> impl Iterator<Item = Field> + '_ {
    model.fields.iter().filter_map(|f| f.value.clone())
}
```

---

## Related Patterns

- [Type Design](type-design.md) - Associated types and bounds
- [Data Structures](data-structures.md) - Generic container traits
- [Architecture](architecture.md) - Public trait design
- [Macros](macros.md) - Derive macro patterns
