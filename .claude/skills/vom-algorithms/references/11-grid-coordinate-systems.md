# Grid Coordinate Systems for Terminal UIs

## Overview

Terminal UIs operate on a discrete 2D grid. Understanding coordinate systems is essential for element positioning, hit testing, and cursor management.

## Coordinate Origin and Direction

```
(0,0)────────────────────────▶ X (columns)
  │  ┌─────────────────────────┐
  │  │ Terminal Screen         │
  │  │                         │
  │  │    (x, y)               │
  │  │      ●                  │
  │  │                         │
  │  └─────────────────────────┘
  ▼
  Y (rows)
```

| Property | Value |
|----------|-------|
| Origin | Top-left (0, 0) |
| X-axis | Left to right (columns) |
| Y-axis | Top to bottom (rows) |
| Units | Character cells |

## VOM Coordinate Types

```rust
pub struct Rect {
    pub x: u16,      // Left edge (column)
    pub y: u16,      // Top edge (row)
    pub width: u16,  // Horizontal span
    pub height: u16, // Vertical span (typically 1)
}

pub struct CursorPosition {
    pub row: u16,    // Y coordinate
    pub col: u16,    // X coordinate
    pub visible: bool,
}
```

## Row-Major vs. Column-Major Storage

VOM uses **row-major** order (standard for terminals):

```rust
// Row-major: cells[row][col]
struct ScreenBuffer {
    cells: Vec<Vec<Cell>>,  // cells[y][x]
}

// Access pattern
fn get_cell(buffer: &ScreenBuffer, x: u16, y: u16) -> &Cell {
    &buffer.cells[y as usize][x as usize]
}
```

Row-major is cache-friendly for horizontal (raster) scanning.

## Coordinate Transformations

### Screen to Buffer

```rust
fn screen_to_buffer(screen_x: u16, screen_y: u16) -> (usize, usize) {
    (screen_y as usize, screen_x as usize)  // (row, col)
}
```

### Buffer to Screen

```rust
fn buffer_to_screen(row: usize, col: usize) -> (u16, u16) {
    (col as u16, row as u16)  // (x, y)
}
```

### Click to Element

```rust
fn element_at_position(
    components: &[Component],
    click_x: u16,
    click_y: u16,
) -> Option<&Component> {
    components.iter().find(|c| {
        click_x >= c.rect.x
            && click_x < c.rect.x + c.rect.width
            && click_y >= c.rect.y
            && click_y < c.rect.y + c.rect.height
    })
}
```

## Boundary Conditions

### Inclusive vs. Exclusive Bounds

VOM uses **inclusive start, exclusive end**:

```rust
impl Rect {
    // Inclusive
    fn left(&self) -> u16 { self.x }
    fn top(&self) -> u16 { self.y }

    // Exclusive
    fn right(&self) -> u16 { self.x + self.width }
    fn bottom(&self) -> u16 { self.y + self.height }

    fn contains(&self, px: u16, py: u16) -> bool {
        px >= self.left() && px < self.right()
            && py >= self.top() && py < self.bottom()
    }
}
```

### Screen Bounds Checking

```rust
fn is_valid_position(x: u16, y: u16, screen_width: u16, screen_height: u16) -> bool {
    x < screen_width && y < screen_height
}
```

## Cursor Position Semantics

The cursor indicates the **next write position**:

```rust
// After "Hello" at row 0:
// Screen: H e l l o _
// Cursor: (5, 0) - after the 'o'

fn cursor_within_cluster(cursor: &CursorPosition, cluster: &Rect) -> bool {
    cursor.row == cluster.y
        && cursor.col >= cluster.x
        && cursor.col < cluster.x + cluster.width
}
```

## Scrolling and Viewport

For scrollable regions:

```rust
struct Viewport {
    scroll_offset: u16,  // Lines scrolled
    visible_height: u16, // Visible rows
}

impl Viewport {
    fn screen_to_content(&self, screen_y: u16) -> u16 {
        screen_y + self.scroll_offset
    }

    fn content_to_screen(&self, content_y: u16) -> Option<u16> {
        if content_y >= self.scroll_offset
            && content_y < self.scroll_offset + self.visible_height
        {
            Some(content_y - self.scroll_offset)
        } else {
            None  // Not visible
        }
    }
}
```

## Multi-Cell Character Positioning

Wide characters complicate coordinate math:

```rust
// "A漢B" in columns:
// Col: 0  1  2  3
// Char: A  漢    B
//          ↑ occupies cols 1-2

fn click_to_character(text: &str, click_col: u16, start_col: u16) -> Option<usize> {
    let relative_col = click_col.checked_sub(start_col)?;
    let mut col = 0;

    for (idx, ch) in text.chars().enumerate() {
        let width = ch.width().unwrap_or(1) as u16;
        if col + width > relative_col {
            return Some(idx);
        }
        col += width;
    }

    None
}
```

## References

- [VT100 Coordinate System](https://vt100.net/docs/vt100-ug/chapter3.html)
- [Row-major order - Wikipedia](https://en.wikipedia.org/wiki/Row-_and_column-major_order)
- [Coordinate system - Wikipedia](https://en.wikipedia.org/wiki/Coordinate_system)
