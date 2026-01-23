# Lexical Analysis for Terminal Output

## Algorithm Overview

Lexical analysis (tokenization) partitions an input stream into meaningful units. In VOM, this manifests as converting a grid of styled cells into discrete UI element tokens (clusters).

## Lexer vs. VOM Segmentation

| Traditional Lexer | VOM Segmentation |
|-------------------|------------------|
| Character stream → Tokens | Cell grid → Clusters |
| Pattern: regex/DFA | Pattern: style equality |
| Output: (type, lexeme, position) | Output: (style, text, rect) |

## Maximal Munch Principle

Both lexers and VOM follow **maximal munch**: consume the longest valid token.

```rust
// Traditional lexer maximal munch
fn next_token(input: &str, pos: usize) -> Token {
    let mut end = pos;
    while end < input.len() && is_valid_continuation(input, pos, end + 1) {
        end += 1;
    }
    Token::new(&input[pos..end])
}

// VOM maximal munch (style-based)
fn next_cluster(row: &[Cell], pos: usize, style: &CellStyle) -> (usize, String) {
    let mut end = pos;
    let mut text = String::new();

    while end < row.len() && row[end].style == *style {
        text.push(row[end].char);
        end += 1;
    }

    (end, text)
}
```

## Token Categories in TUI Context

| VOM Role | Analogous Lexer Token | Pattern |
|----------|----------------------|---------|
| Button | KEYWORD | `[text]`, `<text>` |
| Input | IDENTIFIER | Cursor-containing, `___` |
| Checkbox | LITERAL | `[x]`, `[ ]`, `☑` |
| MenuItem | OPERATOR | `> text`, `• text` |
| Panel | DELIMITER | Box-drawing chars |
| StaticText | STRING | Default |

## Streaming vs. Batch Processing

### Batch (VOM Current)

```rust
fn segment_screen(buffer: &ScreenBuffer) -> Vec<Cluster> {
    let mut all_clusters = Vec::new();
    for (y, row) in buffer.cells.iter().enumerate() {
        all_clusters.extend(segment_row(row, y));
    }
    all_clusters
}
```

### Streaming (Memory-Efficient Alternative)

```rust
struct ClusterIterator<'a> {
    buffer: &'a ScreenBuffer,
    row: usize,
    col: usize,
    current_cluster: Option<ClusterBuilder>,
}

impl<'a> Iterator for ClusterIterator<'a> {
    type Item = Cluster;

    fn next(&mut self) -> Option<Cluster> {
        loop {
            if self.row >= self.buffer.cells.len() {
                return self.current_cluster.take().map(|b| b.build());
            }

            let row = &self.buffer.cells[self.row];
            if self.col >= row.len() {
                self.row += 1;
                self.col = 0;
                if let Some(cluster) = self.current_cluster.take() {
                    return Some(cluster.build());
                }
                continue;
            }

            let cell = &row[self.col];
            self.col += 1;

            match &mut self.current_cluster {
                Some(builder) if builder.style == cell.style => {
                    builder.extend(cell.char);
                }
                Some(builder) => {
                    let completed = std::mem::replace(
                        builder,
                        ClusterBuilder::new(self.col - 1, self.row, cell),
                    );
                    return Some(completed.build());
                }
                None => {
                    self.current_cluster = Some(ClusterBuilder::new(
                        self.col - 1,
                        self.row,
                        cell,
                    ));
                }
            }
        }
    }
}
```

## Lookahead in Classification

Some classifications require lookahead (examining characters beyond current position):

```rust
// Single-character lookahead for bracket matching
fn classify_bracketed(text: &str) -> Option<Role> {
    let chars: Vec<char> = text.chars().collect();

    if chars.len() < 2 {
        return None;
    }

    match (chars.first(), chars.last()) {
        (Some('['), Some(']')) => {
            let inner = &text[1..text.len()-1];
            if is_checkbox_content(inner) {
                Some(Role::Checkbox)
            } else {
                Some(Role::Button)
            }
        }
        (Some('<'), Some('>')) => Some(Role::Button),
        (Some('('), Some(')')) => Some(Role::Button),
        _ => None,
    }
}
```

## Error Recovery

Unlike compiler lexers, VOM never fails—unrecognized patterns become StaticText:

```rust
fn classify_with_fallback(cluster: &Cluster) -> Role {
    // Try all specific patterns
    if let Some(role) = try_button(cluster) { return role; }
    if let Some(role) = try_input(cluster) { return role; }
    if let Some(role) = try_checkbox(cluster) { return role; }
    // ... other patterns

    // Fallback: everything is valid static text
    Role::StaticText
}
```

## Complexity Analysis

| Operation | Time | Space |
|-----------|------|-------|
| Single row segmentation | O(w) | O(k) clusters |
| Full screen segmentation | O(w × h) | O(total clusters) |
| Classification per cluster | O(text length) | O(1) |

## References

- [Lexical analysis - Wikipedia](https://en.wikipedia.org/wiki/Lexical_analysis)
- [Maximal Munch Tokenization in Linear Time](https://research.cs.wisc.edu/wpis/papers/toplas98b.pdf)
- [Dragon Book Chapter 3: Lexical Analysis](https://www.pearson.com/en-us/subject-catalog/p/compilers-principles-techniques-and-tools/P200000003269)
- [General Incremental Lexical Analysis](https://www.researchgate.net/publication/2376535_General_Incremental_Lexical_Analysis)
