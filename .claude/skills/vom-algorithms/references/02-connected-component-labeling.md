# Connected-Component Labeling (CCL)

## Algorithm Overview

CCL identifies and labels distinct connected regions in an image or grid based on pixel/cell attributes. VOM uses a simplified 1D variant within each row (horizontal connectivity only) combined with style-based equivalence.

## Classical Two-Pass Algorithm

### Pass 1: Provisional Labeling

```
for each pixel p in raster order (left-to-right, top-to-bottom):
    if p is foreground:
        neighbors = already-labeled 4-connected neighbors of p
        if neighbors is empty:
            p.label = next_label++
        else:
            p.label = min(neighbors)
            if |unique(neighbors)| > 1:
                record_equivalence(neighbors)
```

### Pass 2: Resolve Equivalences

```
for each pixel p:
    p.label = find_root(p.label)  // Union-Find lookup
```

## VOM Simplification: Row-Local CCL

VOM uses a **degenerate 1D case**: cells are connected within a row only if they share identical style attributes. No vertical merging required.

```rust
// Each row processed independently
// 4-connectivity degenerates to 2-connectivity (left neighbor only)
for (x, cell) in row.iter().enumerate() {
    if current.style == cell.style {
        current.extend(cell);  // Same component
    } else {
        emit(current);
        current = new_component(cell);
    }
}
```

## Equivalence as Style Identity

In classical CCL, equivalence is spatial adjacency of foreground pixels. In VOM:

```
equivalent(cell_a, cell_b) ⟺
    adjacent(cell_a, cell_b) ∧
    cell_a.style == cell_b.style
```

## Complexity Analysis

| Algorithm | Time | Space | Notes |
|-----------|------|-------|-------|
| Two-Pass CCL | O(n) | O(n) | Requires Union-Find |
| VOM Row-RLE | O(n) | O(k) | k = clusters |
| One-Pass with Union-Find | O(n·α(n)) | O(n) | α = inverse Ackermann |

## When to Use Full CCL

If VOM needed to detect **multi-row components** (e.g., 2D boxes, dialog frames), full CCL would be required:

```rust
// Hypothetical 2D component detection
fn detect_boxes(buffer: &ScreenBuffer) -> Vec<Component> {
    let labels = two_pass_ccl(buffer);
    let regions = extract_regions(labels);
    regions.into_iter()
        .filter(|r| is_rectangular(r))
        .map(|r| Component::new(Role::Panel, r.bounds))
        .collect()
}
```

## Union-Find for Equivalence Resolution

```rust
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]); // Path compression
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx != ry {
            // Union by rank
            if self.rank[rx] < self.rank[ry] {
                self.parent[rx] = ry;
            } else if self.rank[rx] > self.rank[ry] {
                self.parent[ry] = rx;
            } else {
                self.parent[ry] = rx;
                self.rank[rx] += 1;
            }
        }
    }
}
```

## References

- [Connected-component labeling - Wikipedia](https://en.wikipedia.org/wiki/Connected-component_labeling)
- [Two pass Connected Component Labelling with Union-Find](https://jacklj.github.io/ccl/)
- [Optimizing two-pass connected-component labeling algorithms](https://dl.acm.org/doi/abs/10.1007/s10044-008-0109-y)
- [Disjoint-set data structure (Union-Find)](https://en.wikipedia.org/wiki/Disjoint-set_data_structure)
