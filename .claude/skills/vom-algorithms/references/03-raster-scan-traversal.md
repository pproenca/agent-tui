# Raster Scan Traversal

## Algorithm Overview

Raster scanning processes a 2D grid in a fixed order: left-to-right within each row, top-to-bottom across rows. This is the fundamental traversal pattern for VOM segmentation.

## Traversal Order

```
(0,0) → (1,0) → (2,0) → ... → (W-1,0)
(0,1) → (1,1) → (2,1) → ... → (W-1,1)
...
(0,H-1) → (1,H-1) → ... → (W-1,H-1)
```

## Implementation

```rust
fn raster_scan<T, F>(grid: &Vec<Vec<T>>, mut visit: F)
where
    F: FnMut(usize, usize, &T),
{
    for (y, row) in grid.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            visit(x, y, cell);
        }
    }
}
```

## VOM Segmentation with Raster Scan

```rust
pub fn segment_buffer(buffer: &ScreenBuffer) -> Vec<Cluster> {
    let mut clusters = Vec::new();

    // Outer loop: top to bottom
    for (y, row) in buffer.cells.iter().enumerate() {
        let mut current: Option<Cluster> = None;

        // Inner loop: left to right
        for (x, cell) in row.iter().enumerate() {
            // Style comparison with left neighbor
            let style_match = current
                .as_ref()
                .map(|c| c.style == cell.style)
                .unwrap_or(false);

            if style_match {
                current.as_mut().unwrap().extend(cell.char);
            } else {
                if let Some(mut c) = current.take() {
                    c.seal();
                    clusters.push(c);
                }
                current = Some(Cluster::new(x as u16, y as u16, cell.char, cell.style.clone()));
            }
        }

        // End of row: emit pending cluster
        if let Some(mut c) = current {
            c.seal();
            clusters.push(c);
        }
    }

    clusters.into_iter().filter(|c| !c.is_whitespace).collect()
}
```

## Neighbor Access During Raster Scan

Because of left-to-right processing, only **left** and **above** neighbors are already processed:

```
Already processed:
  ↑ (x, y-1)  - above
  ← (x-1, y)  - left

Not yet processed:
  → (x+1, y)  - right
  ↓ (x, y+1)  - below
```

This property is essential for single-pass algorithms like CCL first pass.

## Complexity

| Metric | Value |
|--------|-------|
| Time | O(W × H) |
| Space | O(1) for traversal itself |
| Memory Access | Sequential, cache-friendly |
| Parallelization | Row-parallel possible |

## Row-Parallel Variant

```rust
fn parallel_raster_scan(buffer: &ScreenBuffer) -> Vec<Cluster> {
    buffer.cells
        .par_iter()  // Rayon parallel iterator
        .enumerate()
        .flat_map(|(y, row)| segment_row(row, y as u16))
        .collect()
}
```

## Scanline Rendering Connection

Raster scan is also the basis for **scanline rendering** in computer graphics:

```
For each scanline y:
    Find active edges intersecting y
    Sort intersection points by x
    Fill between pairs of intersections
```

## References

- [Raster scan - Wikipedia](https://en.wikipedia.org/wiki/Raster_scan)
- [Scanline rendering - Wikipedia](https://en.wikipedia.org/wiki/Scanline_rendering)
- [MIT 6.837 Line Rasterization](https://groups.csail.mit.edu/graphics/classes/6.837/F02/lectures/6.837-7_Line.pdf)
