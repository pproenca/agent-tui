# Macro Patterns

## Table of Contents
1. [Declarative Macros for Repetition](#1-declarative-macros-for-repetition)
2. [Proc-Macro Derive](#2-proc-macro-derive)
3. [Conditional Compilation Shims](#3-conditional-compilation-shims)
4. [Code Generation Macros](#4-code-generation-macros)
5. [Helper Macros](#5-helper-macros)

---

## 1. Declarative Macros for Repetition

Reduce boilerplate for similar patterns:

```rust
/// Generate key-value parsing for structs
macro_rules! impl_kv_read {
    ($type:ty { $($field:ident: $key:literal),+ $(,)? }) => {
        impl KVRead for $type {
            fn read_from_kv<I>(iter: I) -> Result<Self>
            where
                I: Iterator<Item = (String, String)>,
            {
                let mut result = Self::default();
                for (key, value) in iter {
                    match key.as_str() {
                        $($key => {
                            result.$field = value.parse().ok();
                        })+
                        _ => {}
                    }
                }
                Ok(result)
            }
        }
    };
}

impl_kv_read!(MemoryStat {
    anon: "anon",
    file: "file",
    kernel: "kernel",
    slab: "slab",
    sock: "sock",
    shmem: "shmem",
});

/// Generate enum conversions
macro_rules! impl_enum_str {
    ($enum:ty { $($variant:ident => $str:literal),+ $(,)? }) => {
        impl std::fmt::Display for $enum {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$variant => write!(f, $str),)+
                }
            }
        }

        impl std::str::FromStr for $enum {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self> {
                match s {
                    $($str => Ok(Self::$variant),)+
                    _ => Err(Error::InvalidVariant(s.to_string())),
                }
            }
        }
    };
}

impl_enum_str!(OutputFormat {
    Json => "json",
    Csv => "csv",
    Human => "human",
    OpenMetrics => "openmetrics",
});
```

## 2. Proc-Macro Derive

Generate trait implementations from struct definitions:

```rust
// In below_derive/src/lib.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

#[proc_macro_derive(Queriable, attributes(queriable))]
pub fn derive_queriable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    impl_queriable(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn impl_queriable(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let field_id_name = format_ident!("{}FieldId", name);

    let fields = match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => return Err(syn::Error::new_spanned(input, "Only structs supported")),
    };

    let variants: Vec<_> = fields.iter()
        .filter(|f| !has_attribute(f, "ignore"))
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            let variant = to_pascal_case(&name.to_string());
            quote! { #variant }
        })
        .collect();

    let match_arms: Vec<_> = fields.iter()
        .filter(|f| !has_attribute(f, "ignore"))
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let variant = to_pascal_case(&field_name.to_string());
            if has_attribute(f, "subquery") {
                quote! {
                    #field_id_name::#variant(sub) => self.#field_name.query(sub)
                }
            } else {
                quote! {
                    #field_id_name::#variant => Some(Field::from(&self.#field_name))
                }
            }
        })
        .collect();

    Ok(quote! {
        #[derive(Clone, Debug, PartialEq)]
        pub enum #field_id_name {
            #(#variants),*
        }

        impl Queriable for #name {
            type FieldId = #field_id_name;

            fn query(&self, field_id: &Self::FieldId) -> Option<Field> {
                match field_id {
                    #(#match_arms),*
                }
            }
        }
    })
}

/// Attribute macro for deriving multiple traits at once
#[proc_macro_attribute]
pub fn queriable_derives(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    quote! {
        #[derive(Clone, Debug, Default, PartialEq, Queriable)]
        #input
    }.into()
}
```

## 3. Conditional Compilation Shims

Abstract platform differences:

```rust
/// Macro to switch between open-source and internal builds
#[macro_export]
macro_rules! open_source_shim {
    () => {
        #[cfg(fbcode_build)]
        pub use crate::facebook::*;

        #[cfg(not(fbcode_build))]
        pub use crate::oss::*;
    };

    ($visibility:vis) => {
        #[cfg(fbcode_build)]
        $visibility use crate::facebook::*;

        #[cfg(not(fbcode_build))]
        $visibility use crate::oss::*;
    };
}

// Usage in modules
open_source_shim!();
open_source_shim!(pub(crate));

/// Feature-gated functionality
macro_rules! with_feature {
    ($feature:literal, $enabled:expr, $disabled:expr) => {
        #[cfg(feature = $feature)]
        { $enabled }

        #[cfg(not(feature = $feature))]
        { $disabled }
    };
}

// Usage
fn get_backtrace() -> Option<String> {
    with_feature!("enable_backtrace",
        Some(std::backtrace::Backtrace::capture().to_string()),
        None
    )
}

/// Platform-specific implementations
macro_rules! platform_impl {
    (
        linux => $linux:expr,
        macos => $macos:expr,
        default => $default:expr $(,)?
    ) => {
        #[cfg(target_os = "linux")]
        { $linux }

        #[cfg(target_os = "macos")]
        { $macos }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        { $default }
    };
}
```

## 4. Code Generation Macros

Generate repetitive code structures:

```rust
/// Generate pressure stat reading with optional "full" support
macro_rules! impl_read_pressure {
    ($struct:ty, $path:literal) => {
        impl $struct {
            pub fn read(reader: &CgroupReader) -> Result<Self> {
                let content = reader.read_file($path)?;
                let mut result = Self::default();

                for line in content.lines() {
                    let mut parts = line.split_whitespace();
                    match parts.next() {
                        Some("some") => {
                            result.some = Self::parse_pressure_line(parts)?;
                        }
                        Some("full") => {
                            result.full = Some(Self::parse_pressure_line(parts)?);
                        }
                        _ => {}
                    }
                }

                Ok(result)
            }
        }
    };
}

impl_read_pressure!(CpuPressure, "cpu.pressure");
impl_read_pressure!(MemoryPressure, "memory.pressure");
impl_read_pressure!(IoPressure, "io.pressure");

/// Generate field accessor methods
macro_rules! field_accessors {
    ($($field:ident: $type:ty),+ $(,)?) => {
        $(
            pub fn $field(&self) -> Option<$type> {
                self.$field
            }

            paste::paste! {
                pub fn [<set_ $field>](&mut self, value: $type) {
                    self.$field = Some(value);
                }
            }
        )+
    };
}

impl Stats {
    field_accessors! {
        cpu_usage: f64,
        memory_bytes: u64,
        io_read_bytes: u64,
        io_write_bytes: u64,
    }
}

/// Generate render config implementations
macro_rules! impl_render_config {
    ($field_id:ty { $($variant:ident => $config:expr),+ $(,)? }) => {
        impl HasRenderConfig for $field_id {
            fn get_render_config_builder(&self) -> RenderConfigBuilder {
                match self {
                    $(Self::$variant => $config,)+
                }
            }
        }
    };
}

impl_render_config!(CpuFieldId {
    UsagePercent => RenderConfigBuilder::new()
        .title("CPU%")
        .format(RenderFormat::Percent)
        .width(6),
    UserTime => RenderConfigBuilder::new()
        .title("User")
        .format(RenderFormat::Duration),
    SystemTime => RenderConfigBuilder::new()
        .title("Sys")
        .format(RenderFormat::Duration),
});
```

## 5. Helper Macros

Utility macros for common operations:

```rust
/// Parse with error context
macro_rules! parse_or_err {
    ($expr:expr, $type:ty, $context:expr) => {
        $expr
            .parse::<$type>()
            .with_context(|| format!("Failed to parse {} as {}", $context, stringify!($type)))
    };
}

/// Unwrap Option or continue loop
macro_rules! some_or_continue {
    ($expr:expr) => {
        match $expr {
            Some(v) => v,
            None => continue,
        }
    };
}

/// Unwrap Result or continue loop
macro_rules! ok_or_continue {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(_) => continue,
        }
    };
}

/// Log and return error
macro_rules! bail_with_log {
    ($logger:expr, $($arg:tt)+) => {{
        let msg = format!($($arg)+);
        error!($logger, "{}", msg);
        anyhow::bail!(msg)
    }};
}

/// Timing helper
macro_rules! timed {
    ($logger:expr, $label:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        let result = $block;
        debug!($logger, "{} completed"; "duration_ms" => start.elapsed().as_millis());
        result
    }};
}

// Usage
fn process_entries(logger: &Logger, entries: &[Entry]) -> Result<Vec<Output>> {
    let mut results = Vec::new();

    for entry in entries {
        let id = some_or_continue!(entry.id.as_ref());
        let value = ok_or_continue!(entry.value.parse::<i64>());

        let output = timed!(logger, "process_entry", {
            transform(id, value)?
        });

        results.push(output);
    }

    Ok(results)
}
```
