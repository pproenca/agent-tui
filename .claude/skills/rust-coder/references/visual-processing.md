# Visual Processing and Grid-Based Detection Patterns

Patterns drawn from production Rust projects: [image-rs/image](https://github.com/image-rs/image) (5k stars), [servo/euclid](https://github.com/servo/euclid) (600 stars), and terminal UI element detection techniques.

## Table of Contents
1. [Grid Buffer Representation](#1-grid-buffer-representation)
2. [Rect and Geometry Types (euclid)](#2-rect-and-geometry-types-euclid)
3. [Raster Scanning/Segmentation](#3-raster-scanningsegmentation)
4. [Connected Component Labeling](#4-connected-component-labeling)
5. [Heuristic Classification](#5-heuristic-classification)
6. [Visual Hashing](#6-visual-hashing)
7. [Pixel/Cell Iteration (image-rs)](#7-pixelcell-iteration-image-rs)
8. [Bounds and Intersection](#8-bounds-and-intersection)

---

## 1. Grid Buffer Representation

Terminal as 2D grid of styled cells:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellStyle {
    pub bold: bool,
    pub underline: bool,
    pub inverse: bool,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            bold: false,
            underline: false,
            inverse: false,
            fg_color: None,
            bg_color: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub char: char,
    pub style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            style: CellStyle::default(),
        }
    }
}

#[derive(Debug)]
pub struct ScreenBuffer {
    pub cells: Vec<Vec<Cell>>,
    pub width: u16,
    pub height: u16,
}

impl ScreenBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        let cells = vec![vec![Cell::default(); width as usize]; height as usize];
        Self { cells, width, height }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.cells.get(y as usize)?.get(x as usize)
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if let Some(row) = self.cells.get_mut(y as usize) {
            if let Some(c) = row.get_mut(x as usize) {
                *c = cell;
            }
        }
    }

    pub fn row(&self, y: u16) -> Option<&[Cell]> {
        self.cells.get(y as usize).map(|r| r.as_slice())
    }

    pub fn iter_rows(&self) -> impl Iterator<Item = (u16, &[Cell])> {
        self.cells.iter().enumerate().map(|(y, row)| (y as u16, row.as_slice()))
    }
}
```

## 2. Rect and Geometry Types (euclid)

Rectangle with point-and-size representation:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

impl Point {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub fn from_points(p1: Point, p2: Point) -> Self {
        let min_x = p1.x.min(p2.x);
        let min_y = p1.y.min(p2.y);
        let max_x = p1.x.max(p2.x);
        let max_y = p1.y.max(p2.y);

        Self {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn max(&self) -> Point {
        Point::new(self.x + self.width, self.y + self.height)
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2, self.y + self.height / 2)
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x &&
        p.x < self.x + self.width &&
        p.y >= self.y &&
        p.y < self.y + self.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width &&
        self.x + self.width > other.x &&
        self.y < other.y + other.height &&
        self.y + self.height > other.y
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        if x1 < x2 && y1 < y2 {
            Some(Rect::new(x1, y1, x2 - x1, y2 - y1))
        } else {
            None
        }
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);

        Rect::new(x1, y1, x2 - x1, y2 - y1)
    }

    pub fn area(&self) -> u32 {
        self.width as u32 * self.height as u32
    }
}
```

## 3. Raster Scanning/Segmentation

Horizontal run-length segmentation by style:

```rust
#[derive(Debug, Clone)]
pub struct Cluster {
    pub rect: Rect,
    pub text: String,
    pub style: CellStyle,
    pub is_whitespace: bool,
}

impl Cluster {
    pub fn new(x: u16, y: u16, char: char, style: CellStyle) -> Self {
        Self {
            rect: Rect::new(x, y, 1, 1),
            text: char.to_string(),
            style,
            is_whitespace: char.is_whitespace(),
        }
    }

    pub fn extend(&mut self, char: char) {
        self.text.push(char);
        self.rect.width += 1;
        self.is_whitespace = self.is_whitespace && char.is_whitespace();
    }

    pub fn seal(&mut self) {
        // Trim trailing whitespace but preserve width
        // (visual extent matters for element detection)
    }
}

/// Segment screen buffer into style-homogeneous horizontal runs
pub fn segment_buffer(buffer: &ScreenBuffer) -> Vec<Cluster> {
    let mut clusters = Vec::new();

    for (y, row) in buffer.iter_rows() {
        let mut current: Option<Cluster> = None;

        for (x, cell) in row.iter().enumerate() {
            let x = x as u16;

            let style_match = current
                .as_ref()
                .map(|c| c.style == cell.style)
                .unwrap_or(false);

            if style_match {
                // Extend current cluster
                if let Some(c) = &mut current {
                    c.extend(cell.char);
                }
            } else {
                // Start new cluster
                if let Some(mut c) = current.take() {
                    c.seal();
                    clusters.push(c);
                }

                current = Some(Cluster::new(x, y, cell.char, cell.style.clone()));
            }
        }

        // Finish row
        if let Some(mut c) = current {
            c.seal();
            clusters.push(c);
        }
    }

    // Filter whitespace-only clusters
    clusters.into_iter().filter(|c| !c.is_whitespace).collect()
}
```

## 4. Connected Component Labeling

Merge adjacent clusters with same style:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ClusterId(usize);

pub fn merge_adjacent_clusters(clusters: Vec<Cluster>) -> Vec<Cluster> {
    if clusters.is_empty() {
        return vec![];
    }

    // Group by row
    let mut by_row: HashMap<u16, Vec<Cluster>> = HashMap::new();
    for cluster in clusters {
        by_row.entry(cluster.rect.y).or_default().push(cluster);
    }

    // Sort each row by x position
    for row in by_row.values_mut() {
        row.sort_by_key(|c| c.rect.x);
    }

    // Union-find for merging
    let all_clusters: Vec<Cluster> = by_row.into_values().flatten().collect();
    let mut parent: Vec<usize> = (0..all_clusters.len()).collect();

    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]]; // Path compression
            i = parent[i];
        }
        i
    }

    fn union(parent: &mut [usize], i: usize, j: usize) {
        let pi = find(parent, i);
        let pj = find(parent, j);
        if pi != pj {
            parent[pi] = pj;
        }
    }

    // Merge horizontally adjacent clusters with same style
    for i in 0..all_clusters.len() {
        for j in (i + 1)..all_clusters.len() {
            let ci = &all_clusters[i];
            let cj = &all_clusters[j];

            if ci.style == cj.style && are_adjacent(&ci.rect, &cj.rect) {
                union(&mut parent, i, j);
            }
        }
    }

    // Collect merged clusters
    let mut merged: HashMap<usize, Cluster> = HashMap::new();
    for (i, cluster) in all_clusters.into_iter().enumerate() {
        let root = find(&mut parent, i);

        merged.entry(root)
            .and_modify(|c| {
                c.rect = c.rect.union(&cluster.rect);
                c.text.push_str(&cluster.text);
            })
            .or_insert(cluster);
    }

    merged.into_values().collect()
}

fn are_adjacent(a: &Rect, b: &Rect) -> bool {
    // Same row and touching horizontally
    a.y == b.y && (
        a.x + a.width == b.x ||  // a immediately left of b
        b.x + b.width == a.x     // b immediately left of a
    )
}
```

## 5. Heuristic Classification

Rule-based element type inference:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Button,
    Tab,
    Input,
    Checkbox,
    MenuItem,
    Panel,
    StaticText,
}

pub fn classify(cluster: &Cluster, cursor_pos: Option<Point>) -> Role {
    let text = cluster.text.trim();

    // Cursor position indicates input focus
    if let Some(cursor) = cursor_pos {
        if cluster.rect.contains(cursor) {
            return Role::Input;
        }
    }

    // Pattern matching for common UI elements
    if is_button_pattern(text) {
        return Role::Button;
    }

    if is_checkbox_pattern(text) {
        return Role::Checkbox;
    }

    // Style-based classification
    if cluster.style.inverse {
        return if cluster.rect.y <= 2 {
            Role::Tab
        } else {
            Role::MenuItem
        };
    }

    // Background color heuristics
    if let Some(Color::Indexed(idx)) = &cluster.style.bg_color {
        if *idx == 4 || *idx == 6 {  // Blue/Cyan often used for tabs
            return Role::Tab;
        }
    }

    if is_input_pattern(text) {
        return Role::Input;
    }

    if is_menu_item_pattern(text) {
        return Role::MenuItem;
    }

    if is_panel_border(text) {
        return Role::Panel;
    }

    Role::StaticText
}

fn is_button_pattern(text: &str) -> bool {
    if text.len() <= 2 {
        return false;
    }

    // [Label], <Label>, (Label) patterns
    let is_bracketed = (text.starts_with('[') && text.ends_with(']')) ||
                       (text.starts_with('<') && text.ends_with('>')) ||
                       (text.starts_with('(') && text.ends_with(')'));

    if !is_bracketed {
        return false;
    }

    // Exclude checkbox patterns
    let inner = &text[1..text.len() - 1];
    !matches!(inner.trim(), "x" | "X" | " " | "" | "✓" | "✔" | "o" | "O" | "●")
}

fn is_checkbox_pattern(text: &str) -> bool {
    matches!(
        text,
        "[x]" | "[X]" | "[ ]" | "[✓]" | "[✔]" |
        "◉" | "◯" | "●" | "○" | "◼" | "◻" | "☐" | "☑" | "☒"
    )
}

fn is_input_pattern(text: &str) -> bool {
    text.contains("___") ||
    (!text.is_empty() && text.chars().all(|c| c == '_')) ||
    text.ends_with(": _") ||
    text.ends_with(":_")
}

fn is_menu_item_pattern(text: &str) -> bool {
    text.starts_with('>') ||
    text.starts_with('❯') ||
    text.starts_with('›') ||
    text.starts_with('→') ||
    text.starts_with('▶') ||
    text.starts_with("• ") ||
    text.starts_with("* ") ||
    text.starts_with("- ")
}

fn is_panel_border(text: &str) -> bool {
    const BOX_CHARS: &[char] = &[
        '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼',
        '═', '║', '╔', '╗', '╚', '╝', '╠', '╣', '╦', '╩', '╬',
    ];

    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 {
        return false;
    }

    let box_count = text.chars().filter(|c| BOX_CHARS.contains(c)).count();
    box_count > total / 2
}
```

## 6. Visual Hashing

Hash cluster appearance for change detection:

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisualHash(u64);

impl VisualHash {
    pub fn from_cluster(cluster: &Cluster) -> Self {
        let mut hasher = DefaultHasher::new();

        // Hash position (relative to screen)
        cluster.rect.x.hash(&mut hasher);
        cluster.rect.y.hash(&mut hasher);

        // Hash content
        cluster.text.hash(&mut hasher);

        // Hash style
        cluster.style.bold.hash(&mut hasher);
        cluster.style.inverse.hash(&mut hasher);
        cluster.style.fg_color.hash(&mut hasher);
        cluster.style.bg_color.hash(&mut hasher);

        Self(hasher.finish())
    }
}

#[derive(Debug, Clone)]
pub struct Component {
    pub role: Role,
    pub rect: Rect,
    pub text: String,
    pub visual_hash: VisualHash,
    pub focused: bool,
}

impl Component {
    pub fn new(role: Role, rect: Rect, text: String, hash: VisualHash) -> Self {
        Self {
            role,
            rect,
            text,
            visual_hash: hash,
            focused: false,
        }
    }

    /// Generate stable reference ID
    pub fn reference_id(&self) -> String {
        let prefix = match self.role {
            Role::Button => "btn",
            Role::Tab => "tab",
            Role::Input => "inp",
            Role::Checkbox => "chk",
            Role::MenuItem => "mnu",
            Role::Panel => "pnl",
            Role::StaticText => "txt",
        };

        // Use truncated hash for uniqueness
        let hash_suffix = self.visual_hash.0 & 0xFFFF;
        format!("@{}{:04x}", prefix, hash_suffix)
    }
}
```

## 7. Pixel/Cell Iteration (image-rs)

Nested loop patterns with bounds checking:

```rust
impl ScreenBuffer {
    /// Iterate over region with bounds checking
    pub fn iter_region(&self, rect: &Rect) -> impl Iterator<Item = (u16, u16, &Cell)> {
        let x_start = rect.x as usize;
        let y_start = rect.y as usize;
        let x_end = (rect.x + rect.width).min(self.width) as usize;
        let y_end = (rect.y + rect.height).min(self.height) as usize;

        (y_start..y_end).flat_map(move |y| {
            (x_start..x_end).filter_map(move |x| {
                self.cells.get(y)?.get(x).map(|c| (x as u16, y as u16, c))
            })
        })
    }

    /// Extract text from region
    pub fn region_text(&self, rect: &Rect) -> String {
        let mut lines = Vec::new();

        for y in rect.y..(rect.y + rect.height).min(self.height) {
            let mut line = String::new();
            for x in rect.x..(rect.x + rect.width).min(self.width) {
                if let Some(cell) = self.get(x, y) {
                    line.push(cell.char);
                }
            }
            lines.push(line.trim_end().to_string());
        }

        // Join and trim trailing empty lines
        lines.into_iter()
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_string()
    }

    /// Copy region to new buffer (image-rs tile pattern)
    pub fn copy_region(&self, rect: &Rect) -> ScreenBuffer {
        let width = rect.width.min(self.width.saturating_sub(rect.x));
        let height = rect.height.min(self.height.saturating_sub(rect.y));

        let mut result = ScreenBuffer::new(width, height);

        for dy in 0..height {
            for dx in 0..width {
                if let Some(cell) = self.get(rect.x + dx, rect.y + dy) {
                    result.set(dx, dy, cell.clone());
                }
            }
        }

        result
    }
}
```

## 8. Bounds and Intersection

Overflow-safe coordinate arithmetic:

```rust
/// Calculate valid overlay bounds (from image-rs)
pub fn overlay_bounds(
    base_width: u16,
    base_height: u16,
    overlay_width: u16,
    overlay_height: u16,
    x: i32,
    y: i32,
) -> Option<OverlayBounds> {
    // Check if completely outside
    if x >= base_width as i32 || y >= base_height as i32 {
        return None;
    }
    if x + overlay_width as i32 <= 0 || y + overlay_height as i32 <= 0 {
        return None;
    }

    // Calculate clamped bounds
    let base_x = x.max(0) as u16;
    let base_y = y.max(0) as u16;

    let overlay_x = (-x).max(0) as u16;
    let overlay_y = (-y).max(0) as u16;

    let width = ((x + overlay_width as i32).min(base_width as i32) - x.max(0)) as u16;
    let height = ((y + overlay_height as i32).min(base_height as i32) - y.max(0)) as u16;

    Some(OverlayBounds {
        base_x,
        base_y,
        overlay_x,
        overlay_y,
        width,
        height,
    })
}

#[derive(Debug, Clone, Copy)]
pub struct OverlayBounds {
    pub base_x: u16,
    pub base_y: u16,
    pub overlay_x: u16,
    pub overlay_y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    /// Expand rect by margin (clamped to valid coordinates)
    pub fn expand(&self, margin: u16) -> Rect {
        Rect {
            x: self.x.saturating_sub(margin),
            y: self.y.saturating_sub(margin),
            width: self.width.saturating_add(margin * 2),
            height: self.height.saturating_add(margin * 2),
        }
    }

    /// Contract rect by margin
    pub fn contract(&self, margin: u16) -> Option<Rect> {
        if self.width <= margin * 2 || self.height <= margin * 2 {
            return None;
        }

        Some(Rect {
            x: self.x + margin,
            y: self.y + margin,
            width: self.width - margin * 2,
            height: self.height - margin * 2,
        })
    }

    /// Distance to point (0 if contains)
    pub fn distance_to(&self, p: Point) -> u32 {
        let dx = if p.x < self.x {
            self.x - p.x
        } else if p.x >= self.x + self.width {
            p.x - (self.x + self.width - 1)
        } else {
            0
        };

        let dy = if p.y < self.y {
            self.y - p.y
        } else if p.y >= self.y + self.height {
            p.y - (self.y + self.height - 1)
        } else {
            0
        };

        (dx as u32).pow(2) + (dy as u32).pow(2)
    }
}
```

---

## Related Patterns

- [Data Structures](data-structures.md) - Grid and buffer data structures
- [TUI Patterns](tui-patterns.md) - Screen rendering and layout
- [Terminal Raw I/O](terminal-raw-io.md) - Cell-based terminal output
- [Performance](performance.md) - Efficient grid traversal algorithms
