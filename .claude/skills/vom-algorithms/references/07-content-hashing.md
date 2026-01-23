# Content-Based Hashing for Element Identity

## Algorithm Overview

Content-based hashing generates a stable identifier for UI elements based on their visual properties. This enables tracking elements across screen updates without relying on unstable positional indices.

## Hash Input Components

VOM hashes combine multiple element properties:

```rust
fn hash_cluster(cluster: &Cluster) -> u64 {
    let mut hasher = DefaultHasher::new();  // SipHash-1-3

    cluster.text.hash(&mut hasher);
    cluster.style.hash(&mut hasher);
    // Note: Position intentionally excluded for stability

    hasher.finish()
}
```

## Hash Stability Properties

| Property | Included? | Rationale |
|----------|-----------|-----------|
| Text content | Yes | Primary identity |
| Style (bold, color, etc.) | Yes | Visual distinction |
| Position (x, y) | No | Elements may move |
| Dimensions | Implicit | From text length |

## SipHash (Rust Default)

Rust's `DefaultHasher` uses SipHash-1-3, a keyed hash function:

```
SipHash-c-d:
  c = compression rounds per block
  d = finalization rounds

SipHash-1-3: Fast, suitable for hash tables
SipHash-2-4: More secure, DoS-resistant
```

### SipHash Properties

- **Output**: 64 bits
- **Key**: 128 bits (random per process in Rust)
- **Speed**: ~1 cycle/byte for short inputs
- **Security**: Resistant to hash-flooding attacks

## Implementation for Derivable Hash

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellStyle {
    pub bold: bool,
    pub underline: bool,
    pub inverse: bool,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

// Cluster hashing via derived Hash
pub fn compute_visual_hash(cluster: &Cluster) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    cluster.text.hash(&mut hasher);
    cluster.style.hash(&mut hasher);
    hasher.finish()
}
```

## Element Matching Across Frames

```rust
struct ElementTracker {
    previous: HashMap<u64, Component>,
}

impl ElementTracker {
    fn match_elements(&mut self, current: Vec<Component>) -> Vec<TrackedComponent> {
        let mut result = Vec::new();

        for component in current {
            let hash = component.visual_hash;
            let tracking = if let Some(prev) = self.previous.remove(&hash) {
                Tracking::Matched { prev_position: prev.rect }
            } else {
                Tracking::New
            };

            result.push(TrackedComponent { component, tracking });
        }

        // Remaining in self.previous are disappeared elements
        self.previous = result.iter()
            .map(|tc| (tc.component.visual_hash, tc.component.clone()))
            .collect();

        result
    }
}
```

## Collision Handling

Hash collisions (different elements with same hash) are rare but possible:

```rust
fn find_by_hash(components: &[Component], target_hash: u64) -> Vec<&Component> {
    // Return all matches; caller disambiguates if multiple
    components.iter().filter(|c| c.visual_hash == target_hash).collect()
}

fn disambiguate(matches: &[&Component], hint: Option<Rect>) -> Option<&Component> {
    match matches.len() {
        0 => None,
        1 => Some(matches[0]),
        _ => {
            // Use position hint if available
            hint.and_then(|h| matches.iter().find(|c| c.rect.intersects(&h)).copied())
                .or(Some(matches[0]))
        }
    }
}
```

## Alternative Hash Functions

| Function | Speed | Output | Use Case |
|----------|-------|--------|----------|
| SipHash-1-3 | Fast | 64-bit | Hash tables (Rust default) |
| SipHash-2-4 | Medium | 64-bit | DoS-resistant tables |
| FNV-1a | Fastest | 32/64-bit | Non-adversarial data |
| xxHash | Very fast | 64/128-bit | Large data, checksums |
| AHash | Fastest | 64-bit | Hash tables (aHash crate) |

## FNV-1a Implementation (Alternative)

```rust
fn fnv1a_64(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;

    let mut hash = FNV_OFFSET;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
```

## References

- [SipHash - Wikipedia](https://en.wikipedia.org/wiki/SipHash)
- [SipHash Paper (Aumasson & Bernstein)](https://www.aumasson.jp/siphash/siphash.pdf)
- [Fowler-Noll-Vo hash function - Wikipedia](https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function)
- [IETF FNV Draft](https://datatracker.ietf.org/doc/draft-eastlake-fnv/)
- [Rust Hash trait documentation](https://doc.rust-lang.org/std/hash/trait.Hash.html)
