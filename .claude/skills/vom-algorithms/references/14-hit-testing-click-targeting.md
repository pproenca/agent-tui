# Hit Testing and Click Targeting Algorithms

## Overview

Hit testing determines which UI element (if any) is at a given coordinate. This is essential for implementing click, hover, and focus operations on VOM components.

## Basic Point-in-Rectangle Test

```rust
impl Rect {
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x
            && x < self.x + self.width
            && y >= self.y
            && y < self.y + self.height
    }
}

pub fn hit_test(components: &[Component], x: u16, y: u16) -> Option<&Component> {
    components.iter().find(|c| c.rect.contains(x, y))
}
```

**Complexity**: O(n) where n = number of components

## Priority-Based Hit Testing

When components overlap, priority determines which one receives the click:

```rust
fn hit_test_with_priority(components: &[Component], x: u16, y: u16) -> Option<&Component> {
    let mut candidates: Vec<_> = components
        .iter()
        .filter(|c| c.rect.contains(x, y))
        .collect();

    // Sort by priority: interactive elements first
    candidates.sort_by_key(|c| match c.role {
        Role::Button => 0,
        Role::Input => 1,
        Role::Checkbox => 2,
        Role::MenuItem => 3,
        Role::Tab => 4,
        Role::Panel => 5,
        Role::StaticText => 6,
    });

    candidates.first().copied()
}
```

## Z-Order (Layering)

For overlapping panels:

```rust
struct LayeredComponent {
    component: Component,
    z_index: u32,  // Higher = on top
}

fn hit_test_layered(layers: &[LayeredComponent], x: u16, y: u16) -> Option<&Component> {
    layers
        .iter()
        .filter(|l| l.component.rect.contains(x, y))
        .max_by_key(|l| l.z_index)
        .map(|l| &l.component)
}
```

## Spatial Indexing for Large Component Counts

### Grid-Based Acceleration

```rust
struct SpatialGrid {
    cell_size: u16,
    grid: HashMap<(u16, u16), Vec<usize>>,  // Grid cell -> component indices
}

impl SpatialGrid {
    fn build(components: &[Component], cell_size: u16) -> Self {
        let mut grid = HashMap::new();

        for (idx, comp) in components.iter().enumerate() {
            let start_gx = comp.rect.x / cell_size;
            let start_gy = comp.rect.y / cell_size;
            let end_gx = (comp.rect.x + comp.rect.width) / cell_size;
            let end_gy = (comp.rect.y + comp.rect.height) / cell_size;

            for gy in start_gy..=end_gy {
                for gx in start_gx..=end_gx {
                    grid.entry((gx, gy)).or_default().push(idx);
                }
            }
        }

        Self { cell_size, grid }
    }

    fn query(&self, x: u16, y: u16) -> impl Iterator<Item = usize> + '_ {
        let gx = x / self.cell_size;
        let gy = y / self.cell_size;
        self.grid.get(&(gx, gy)).into_iter().flatten().copied()
    }
}

fn hit_test_accelerated(
    components: &[Component],
    grid: &SpatialGrid,
    x: u16,
    y: u16,
) -> Option<&Component> {
    grid.query(x, y)
        .filter_map(|idx| components.get(idx))
        .find(|c| c.rect.contains(x, y))
}
```

**Complexity**: O(1) average case with good cell size

### R-Tree (for Complex Scenes)

```rust
use rstar::{RTree, AABB, RTreeObject, PointDistance};

struct RTreeComponent {
    component_idx: usize,
    rect: Rect,
}

impl RTreeObject for RTreeComponent {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.rect.x as f64, self.rect.y as f64],
            [(self.rect.x + self.rect.width) as f64, (self.rect.y + self.rect.height) as f64],
        )
    }
}

fn build_rtree(components: &[Component]) -> RTree<RTreeComponent> {
    let items: Vec<_> = components
        .iter()
        .enumerate()
        .map(|(idx, c)| RTreeComponent { component_idx: idx, rect: c.rect })
        .collect();
    RTree::bulk_load(items)
}
```

## Click Target Resolution

### By Role

```rust
pub fn find_clickable_at(components: &[Component], x: u16, y: u16) -> Option<&Component> {
    components
        .iter()
        .filter(|c| c.rect.contains(x, y))
        .find(|c| matches!(c.role, Role::Button | Role::Tab | Role::Checkbox | Role::MenuItem))
}
```

### By Text Content

```rust
pub fn find_by_text(components: &[Component], text: &str) -> Option<&Component> {
    components.iter().find(|c| c.text.contains(text))
}

pub fn find_button_by_label(components: &[Component], label: &str) -> Option<&Component> {
    components
        .iter()
        .filter(|c| c.role == Role::Button)
        .find(|c| c.text.trim().trim_matches(&['[', ']', '<', '>', '(', ')'][..]) == label)
}
```

### By Hash (Stable Targeting)

```rust
pub fn find_by_hash(components: &[Component], hash: u64) -> Option<&Component> {
    components.iter().find(|c| c.visual_hash == hash)
}
```

## Nearest Element Search

For click correction or accessibility:

```rust
fn find_nearest(components: &[Component], x: u16, y: u16, role: Role) -> Option<&Component> {
    components
        .iter()
        .filter(|c| c.role == role)
        .min_by_key(|c| {
            let cx = c.rect.x + c.rect.width / 2;
            let cy = c.rect.y + c.rect.height / 2;
            let dx = (x as i32 - cx as i32).abs();
            let dy = (y as i32 - cy as i32).abs();
            dx * dx + dy * dy  // Squared Euclidean distance
        })
}
```

## Fuzzy Hit Testing

Allow slight misses for touch-like input:

```rust
fn hit_test_fuzzy(
    components: &[Component],
    x: u16,
    y: u16,
    tolerance: u16,
) -> Option<&Component> {
    // Try exact hit first
    if let Some(c) = hit_test(components, x, y) {
        return Some(c);
    }

    // Expand search area
    let expanded_rect = Rect {
        x: x.saturating_sub(tolerance),
        y: y.saturating_sub(tolerance),
        width: tolerance * 2 + 1,
        height: tolerance * 2 + 1,
    };

    components
        .iter()
        .filter(|c| c.rect.intersects(&expanded_rect))
        .min_by_key(|c| distance_to_rect(x, y, &c.rect))
}

fn distance_to_rect(x: u16, y: u16, rect: &Rect) -> u32 {
    let dx = if x < rect.x {
        rect.x - x
    } else if x >= rect.x + rect.width {
        x - (rect.x + rect.width - 1)
    } else {
        0
    };

    let dy = if y < rect.y {
        rect.y - y
    } else if y >= rect.y + rect.height {
        y - (rect.y + rect.height - 1)
    } else {
        0
    };

    (dx as u32) * (dx as u32) + (dy as u32) * (dy as u32)
}
```

## References

- [Point in polygon - Wikipedia](https://en.wikipedia.org/wiki/Point_in_polygon)
- [R-tree - Wikipedia](https://en.wikipedia.org/wiki/R-tree)
- [Spatial index - Wikipedia](https://en.wikipedia.org/wiki/Spatial_database#Spatial_index)
- [Hit testing in GUI frameworks](https://developer.mozilla.org/en-US/docs/Web/API/Element/getBoundingClientRect)
