# Incremental and Differential Screen Updates

## Overview

Efficient terminal interaction requires detecting **what changed** between frames rather than re-processing the entire screen. This enables responsive UIs and efficient element tracking.

## Change Detection Strategies

### 1. Full Recomputation (VOM Current)

```rust
// Simple but O(W×H) every frame
fn snapshot(buffer: &ScreenBuffer) -> Vec<Component> {
    let clusters = segment(buffer);
    classify(clusters)
}
```

### 2. Row-Level Dirty Tracking

```rust
struct DirtyTracker {
    dirty_rows: HashSet<u16>,
    previous_hashes: Vec<u64>,  // Hash per row
}

impl DirtyTracker {
    fn mark_dirty(&mut self, row: u16) {
        self.dirty_rows.insert(row);
    }

    fn update(&mut self, buffer: &ScreenBuffer) -> Vec<u16> {
        let mut changed = Vec::new();

        for (y, row) in buffer.cells.iter().enumerate() {
            let hash = hash_row(row);
            if self.previous_hashes.get(y) != Some(&hash) {
                changed.push(y as u16);
                if y < self.previous_hashes.len() {
                    self.previous_hashes[y] = hash;
                } else {
                    self.previous_hashes.push(hash);
                }
            }
        }

        changed
    }
}
```

### 3. Cell-Level Differencing

```rust
#[derive(Clone, PartialEq)]
struct CellState {
    char: char,
    style: CellStyle,
}

fn diff_buffers(old: &ScreenBuffer, new: &ScreenBuffer) -> Vec<CellChange> {
    let mut changes = Vec::new();

    for y in 0..new.cells.len().max(old.cells.len()) {
        let old_row = old.cells.get(y);
        let new_row = new.cells.get(y);

        match (old_row, new_row) {
            (Some(old_r), Some(new_r)) => {
                for x in 0..old_r.len().max(new_r.len()) {
                    let old_cell = old_r.get(x);
                    let new_cell = new_r.get(x);
                    if old_cell != new_cell {
                        changes.push(CellChange { x, y, old: old_cell.cloned(), new: new_cell.cloned() });
                    }
                }
            }
            (None, Some(_)) => changes.push(CellChange::row_added(y)),
            (Some(_), None) => changes.push(CellChange::row_removed(y)),
            (None, None) => {}
        }
    }

    changes
}
```

## Component-Level Tracking

### Hash-Based Matching

```rust
struct ComponentTracker {
    previous: HashMap<u64, Component>,  // visual_hash -> Component
}

impl ComponentTracker {
    fn track(&mut self, current: Vec<Component>) -> TrackedSnapshot {
        let mut matched = Vec::new();
        let mut added = Vec::new();
        let mut current_hashes = HashMap::new();

        for component in current {
            let hash = component.visual_hash;
            current_hashes.insert(hash, component.clone());

            if let Some(prev) = self.previous.get(&hash) {
                matched.push(MatchedComponent {
                    component,
                    moved: prev.rect != component.rect,
                    prev_rect: prev.rect,
                });
            } else {
                added.push(component);
            }
        }

        let removed: Vec<_> = self.previous
            .iter()
            .filter(|(h, _)| !current_hashes.contains_key(h))
            .map(|(_, c)| c.clone())
            .collect();

        self.previous = current_hashes;

        TrackedSnapshot { matched, added, removed }
    }
}
```

### Position-Based Matching (Fallback)

When hash collisions occur:

```rust
fn match_by_position(
    prev: &[Component],
    curr: &[Component],
    role_filter: Option<Role>,
) -> Vec<(Option<&Component>, Option<&Component>)> {
    let prev_filtered: Vec<_> = prev.iter()
        .filter(|c| role_filter.map(|r| c.role == r).unwrap_or(true))
        .collect();
    let curr_filtered: Vec<_> = curr.iter()
        .filter(|c| role_filter.map(|r| c.role == r).unwrap_or(true))
        .collect();

    // Spatial nearest-neighbor matching
    let mut matches = Vec::new();
    let mut used_curr = HashSet::new();

    for prev_comp in &prev_filtered {
        let nearest = curr_filtered.iter()
            .enumerate()
            .filter(|(i, _)| !used_curr.contains(i))
            .min_by_key(|(_, c)| manhattan_distance(&prev_comp.rect, &c.rect));

        if let Some((idx, curr_comp)) = nearest {
            if manhattan_distance(&prev_comp.rect, &curr_comp.rect) < THRESHOLD {
                used_curr.insert(idx);
                matches.push((Some(*prev_comp), Some(*curr_comp)));
            } else {
                matches.push((Some(*prev_comp), None));  // Removed
            }
        }
    }

    // Remaining curr are additions
    for (idx, curr_comp) in curr_filtered.iter().enumerate() {
        if !used_curr.contains(&idx) {
            matches.push((None, Some(*curr_comp)));
        }
    }

    matches
}

fn manhattan_distance(a: &Rect, b: &Rect) -> u32 {
    ((a.x as i32 - b.x as i32).abs() + (a.y as i32 - b.y as i32).abs()) as u32
}
```

## Event-Driven Updates

Instead of polling, react to terminal output:

```rust
struct IncrementalVom {
    buffer: ScreenBuffer,
    components: Vec<Component>,
    dirty: bool,
}

impl IncrementalVom {
    fn process_output(&mut self, data: &[u8]) {
        self.terminal.process(data);
        self.dirty = true;
    }

    fn snapshot(&mut self) -> &[Component] {
        if self.dirty {
            self.buffer = self.terminal.screen_buffer();
            self.components = vom_pipeline(&self.buffer);
            self.dirty = false;
        }
        &self.components
    }
}
```

## Complexity Comparison

| Strategy | Time (per frame) | Space | Best For |
|----------|------------------|-------|----------|
| Full recompute | O(W×H) | O(clusters) | Simple, correct baseline |
| Row-dirty | O(dirty rows × W) | O(H) hashes | Localized changes |
| Cell diff | O(changed cells) | O(W×H) prev state | Fine-grained tracking |
| Hash matching | O(clusters) | O(clusters) prev | Element persistence |

## References

- [Incremental computation - Wikipedia](https://en.wikipedia.org/wiki/Incremental_computing)
- [Efficient and Flexible Incremental Parsing](https://harmonia.cs.berkeley.edu/papers/twagner-parsing.pdf)
- [React's reconciliation algorithm](https://legacy.reactjs.org/docs/reconciliation.html)
- [Virtual DOM and diffing](https://svelte.dev/blog/virtual-dom-is-pure-overhead)
