# Axis-Aligned Bounding Box (AABB) Computation

## Algorithm Overview

An Axis-Aligned Bounding Box is the smallest rectangle with edges parallel to coordinate axes that completely encloses a set of points or region. In VOM, each cluster's AABB defines its spatial extent.

## Formal Definition

Given a set of points `P = {(x₁,y₁), (x₂,y₂), ..., (xₙ,yₙ)}`:

```
AABB(P) = Rect {
    x:      min(x₁, x₂, ..., xₙ)
    y:      min(y₁, y₂, ..., yₙ)
    width:  max(x₁, x₂, ..., xₙ) - min(x₁, x₂, ..., xₙ) + 1
    height: max(y₁, y₂, ..., yₙ) - min(y₁, y₂, ..., yₙ) + 1
}
```

## VOM Rect Structure

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u16,      // Left column
    pub y: u16,      // Top row
    pub width: u16,  // Columns spanned
    pub height: u16, // Rows spanned (typically 1 for VOM clusters)
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub fn right(&self) -> u16 {
        self.x + self.width
    }

    pub fn bottom(&self) -> u16 {
        self.y + self.height
    }

    pub fn contains(&self, px: u16, py: u16) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}
```

## Incremental AABB Construction

During raster scan, AABB grows as cells are added:

```rust
struct ClusterBuilder {
    start_x: u16,
    start_y: u16,
    end_x: u16,
    // end_y equals start_y for single-row clusters
}

impl ClusterBuilder {
    fn new(x: u16, y: u16) -> Self {
        Self { start_x: x, start_y: y, end_x: x }
    }

    fn extend(&mut self) {
        self.end_x += 1;
    }

    fn to_rect(&self) -> Rect {
        Rect::new(
            self.start_x,
            self.start_y,
            self.end_x - self.start_x + 1,
            1,  // Single row
        )
    }
}
```

## AABB Merging (for Multi-Row Components)

If extending VOM to support multi-row components:

```rust
impl Rect {
    fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());

        Rect::new(x, y, right - x, bottom - y)
    }
}
```

## Point-in-AABB Test (Click Targeting)

```rust
fn find_element_at(components: &[Component], click_x: u16, click_y: u16) -> Option<&Component> {
    components.iter().find(|c| c.rect.contains(click_x, click_y))
}
```

## AABB Properties

| Property | Formula | Use Case |
|----------|---------|----------|
| Area | width × height | Size comparison |
| Center | (x + width/2, y + height/2) | Distance calculations |
| Perimeter | 2 × (width + height) | Border detection |
| Aspect ratio | width / height | Shape classification |

## Spatial Indexing with AABBs

For large component counts, spatial indexing accelerates queries:

```rust
// Simple grid-based spatial index
struct SpatialGrid {
    cell_size: u16,
    cells: HashMap<(u16, u16), Vec<usize>>,  // grid cell -> component indices
}

impl SpatialGrid {
    fn insert(&mut self, idx: usize, rect: &Rect) {
        let start_cell_x = rect.x / self.cell_size;
        let start_cell_y = rect.y / self.cell_size;
        let end_cell_x = rect.right() / self.cell_size;
        let end_cell_y = rect.bottom() / self.cell_size;

        for cy in start_cell_y..=end_cell_y {
            for cx in start_cell_x..=end_cell_x {
                self.cells.entry((cx, cy)).or_default().push(idx);
            }
        }
    }

    fn query(&self, point: (u16, u16)) -> impl Iterator<Item = usize> + '_ {
        let cell = (point.0 / self.cell_size, point.1 / self.cell_size);
        self.cells.get(&cell).into_iter().flatten().copied()
    }
}
```

## References

- [Minimum bounding box - Wikipedia](https://en.wikipedia.org/wiki/Minimum_bounding_box)
- [Axis-aligned bounding box - Wikipedia](https://en.wikipedia.org/wiki/Bounding_volume#Common_bounding_volumes)
- [Collision detection with AABBs](https://developer.mozilla.org/en-US/docs/Games/Techniques/3D_collision_detection)
