# VOM Pipeline Architecture

## Overview

The Visual Object Model (VOM) transforms raw terminal state into a structured representation of UI elements. This document describes the complete pipeline architecture.

## Pipeline Stages

```
┌─────────────────────────────────────────────────────────────────────┐
│                        VOM Pipeline                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────┐    ┌──────────────┐    ┌───────────────┐    ┌───────┐│
│  │ Raw Byte │───▶│ VT100 Parser │───▶│ Screen Buffer │───▶│ VOM   ││
│  │ Stream   │    │ (vt100 crate)│    │ Extraction    │    │ Core  ││
│  └──────────┘    └──────────────┘    └───────────────┘    └───────┘│
│                                                               │      │
│       ┌───────────────────────────────────────────────────────┘      │
│       │                                                              │
│       ▼                                                              │
│  ┌──────────────┐    ┌────────────────┐    ┌────────────────┐       │
│  │ Segmentation │───▶│ Classification │───▶│ Component List │       │
│  │ (RLE+Raster) │    │ (Heuristics)   │    │ (Output)       │       │
│  └──────────────┘    └────────────────┘    └────────────────┘       │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Stage 1: Terminal Emulation

**Input**: Raw byte stream from PTY
**Output**: Parsed screen state

```rust
pub struct VirtualTerminal {
    parser: Arc<Mutex<vt100::Parser>>,
    cols: u16,
    rows: u16,
}

impl VirtualTerminal {
    pub fn process(&self, data: &[u8]) {
        let mut parser = self.parser.lock().unwrap();
        parser.process(data);
    }
}
```

The vt100 crate handles:
- ANSI escape sequence parsing
- Cursor positioning
- Style attribute management
- Scrolling and screen clearing

## Stage 2: Screen Buffer Extraction

**Input**: vt100::Screen
**Output**: ScreenBuffer with cells and styles

```rust
pub fn screen_buffer(&self) -> ScreenBuffer {
    let parser = self.parser.lock().unwrap();
    let screen = parser.screen();

    let mut cells = Vec::new();
    for row in 0..screen.size().0 {
        let mut row_cells = Vec::new();
        for col in 0..screen.size().1 {
            let cell = screen.cell(row, col);
            let (char, style) = extract_cell(cell);
            row_cells.push(Cell { char, style });
        }
        cells.push(row_cells);
    }

    ScreenBuffer { cells }
}
```

## Stage 3: Segmentation

**Input**: ScreenBuffer (2D grid of styled cells)
**Output**: Vec<Cluster> (style-homogeneous regions)

```rust
pub fn segment(buffer: &ScreenBuffer, cursor: &CursorPosition) -> Vec<Cluster> {
    let mut clusters = Vec::new();

    for (y, row) in buffer.cells.iter().enumerate() {
        clusters.extend(segment_row(row, y as u16));
    }

    // Filter whitespace-only clusters
    clusters.into_iter()
        .filter(|c| !c.is_whitespace)
        .collect()
}
```

Algorithm: Style-based Run-Length Encoding with raster scan traversal.

## Stage 4: Classification

**Input**: Vec<Cluster> + cursor position
**Output**: Vec<Component> with roles

```rust
pub fn classify(clusters: Vec<Cluster>, cursor_row: u16, cursor_col: u16) -> Vec<Component> {
    clusters.into_iter()
        .map(|cluster| {
            let role = infer_role(&cluster, cursor_row, cursor_col);
            let visual_hash = hash_cluster(&cluster);
            Component::new(role, cluster.rect, cluster.text.clone(), visual_hash)
        })
        .collect()
}
```

Algorithm: Priority-ordered heuristic rule cascade.

## Data Flow Types

```rust
// Stage 2 output
pub struct ScreenBuffer {
    pub cells: Vec<Vec<Cell>>,
}

pub struct Cell {
    pub char: char,
    pub style: CellStyle,
}

// Stage 3 output
pub struct Cluster {
    pub rect: Rect,
    pub text: String,
    pub style: CellStyle,
    pub is_whitespace: bool,
}

// Stage 4 output
pub struct Component {
    pub role: Role,
    pub rect: Rect,
    pub text: String,
    pub visual_hash: u64,
}

pub enum Role {
    Button,
    Tab,
    Input,
    Checkbox,
    MenuItem,
    Panel,
    StaticText,
}
```

## Pipeline Execution

```rust
pub fn vom_snapshot(terminal: &VirtualTerminal) -> Vec<Component> {
    // Stage 2: Extract buffer
    let buffer = terminal.screen_buffer();
    let cursor = terminal.cursor();

    // Stage 3: Segment
    let clusters = segment(&buffer, &cursor);

    // Stage 4: Classify
    classify(clusters, cursor.row, cursor.col)
}
```

## Performance Characteristics

| Stage | Complexity | Dominant Cost |
|-------|------------|---------------|
| Emulation | O(bytes) | Parser state machine |
| Extraction | O(W×H) | Cell copying |
| Segmentation | O(W×H) | Raster scan + RLE |
| Classification | O(clusters × pattern) | String matching |

Total: O(W×H) where W=columns, H=rows

## Extension Points

### Custom Classifiers

```rust
trait Classifier {
    fn classify(&self, cluster: &Cluster, context: &Context) -> Option<Role>;
}

fn classify_with_chain(
    cluster: &Cluster,
    context: &Context,
    classifiers: &[Box<dyn Classifier>],
) -> Role {
    for classifier in classifiers {
        if let Some(role) = classifier.classify(cluster, context) {
            return role;
        }
    }
    Role::StaticText
}
```

### Preprocessors

```rust
trait Preprocessor {
    fn preprocess(&self, buffer: &mut ScreenBuffer);
}

// Example: Noise reduction
struct NoiseFilter;
impl Preprocessor for NoiseFilter {
    fn preprocess(&self, buffer: &mut ScreenBuffer) {
        // Remove isolated styled cells
    }
}
```

### Post-processors

```rust
trait Postprocessor {
    fn postprocess(&self, components: &mut Vec<Component>);
}

// Example: Merge adjacent buttons
struct ButtonMerger;
impl Postprocessor for ButtonMerger {
    fn postprocess(&self, components: &mut Vec<Component>) {
        // Merge [OK][Cancel] into single component if needed
    }
}
```

## References

- [Pipeline pattern - Wikipedia](https://en.wikipedia.org/wiki/Pipeline_(software))
- [ETL (Extract, Transform, Load)](https://en.wikipedia.org/wiki/Extract,_transform,_load)
- [Document Object Model concepts](https://developer.mozilla.org/en-US/docs/Web/API/Document_Object_Model/Introduction)
