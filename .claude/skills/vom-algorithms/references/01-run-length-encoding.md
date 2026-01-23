# Run-Length Encoding (RLE) for Terminal Segmentation

## Algorithm Overview

Run-Length Encoding compresses consecutive identical values into (value, count) pairs. In VOM, a variant groups consecutive cells sharing the **same style attributes** rather than identical content.

## Formal Definition

Given input sequence `S = [s₁, s₂, ..., sₙ]` with predicate `same(a, b)`:

```
RLE(S) → [(v₁, start₁, len₁), (v₂, start₂, len₂), ...]

where each run (vᵢ, startᵢ, lenᵢ) satisfies:
  - same(S[startᵢ + j], vᵢ) for all j ∈ [0, lenᵢ)
  - ¬same(S[startᵢ + lenᵢ], vᵢ) (maximality)
```

## VOM Segmentation Implementation

```rust
// Style-homogeneous RLE for terminal cells
fn segment_row(cells: &[Cell]) -> Vec<Cluster> {
    let mut clusters = Vec::new();
    let mut current: Option<Cluster> = None;

    for (x, cell) in cells.iter().enumerate() {
        let style_matches = current
            .as_ref()
            .map(|c| c.style == cell.style)
            .unwrap_or(false);

        if style_matches {
            current.as_mut().unwrap().extend(cell.char);
        } else {
            if let Some(c) = current.take() {
                clusters.push(c);
            }
            current = Some(Cluster::new(x, cell.char, cell.style.clone()));
        }
    }

    if let Some(c) = current {
        clusters.push(c);
    }
    clusters
}
```

## Key Properties

| Property | Value | Implication |
|----------|-------|-------------|
| Time Complexity | O(n) | Single pass over cells |
| Space Complexity | O(k) | k = number of distinct runs |
| Predicate | Style equality | Bold, color, inverse, underline |
| Output | Clusters with bounds | Ready for classification |

## Style Attributes for Grouping

```rust
struct CellStyle {
    bold: bool,
    underline: bool,
    inverse: bool,
    fg_color: Option<Color>,
    bg_color: Option<Color>,
}

// Equality determines run boundaries
impl PartialEq for CellStyle {
    fn eq(&self, other: &Self) -> bool {
        self.bold == other.bold
            && self.underline == other.underline
            && self.inverse == other.inverse
            && self.fg_color == other.fg_color
            && self.bg_color == other.bg_color
    }
}
```

## Row-by-Row Processing Pattern

```rust
for (y, row) in buffer.cells.iter().enumerate() {
    // RLE within each row independently
    let row_clusters = segment_row(row);

    for cluster in row_clusters {
        cluster.y = y;
        all_clusters.push(cluster);
    }
}
```

## Whitespace Filtering

After RLE, filter whitespace-only clusters:

```rust
clusters.into_iter()
    .filter(|c| !c.text.trim().is_empty())
    .collect()
```

## References

- [Run-length encoding - Wikipedia](https://en.wikipedia.org/wiki/Run-length_encoding)
- [RLE in TIFF, BMP, PCX file formats](https://www.fileformat.info/mirror/egff/ch09_03.htm)
- [AQA Teaching Guide: Run-length encoding](https://filestore.aqa.org.uk/resources/computing/AQA-8525-TG-RLE.PDF)
