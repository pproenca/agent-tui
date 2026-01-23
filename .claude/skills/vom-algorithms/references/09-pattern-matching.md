# Pattern Matching for UI Element Detection

## Algorithm Overview

Pattern matching identifies UI elements by comparing text and style attributes against predefined patterns. VOM uses a combination of exact matching, prefix/suffix matching, and character class membership.

## Pattern Types

### 1. Exact Match

```rust
fn is_checkbox(text: &str) -> bool {
    matches!(text.trim(),
        "[x]" | "[X]" | "[ ]" | "[✓]" | "[✔]" |
        "◉" | "◯" | "●" | "○" | "◼" | "◻" | "☐" | "☑" | "☒"
    )
}
```

**Complexity**: O(1) with hash-based lookup or O(n) patterns × O(m) string length

### 2. Prefix Match

```rust
fn is_menu_item(text: &str) -> bool {
    text.starts_with('>')
        || text.starts_with('❯')
        || text.starts_with('›')
        || text.starts_with('→')
        || text.starts_with('▶')
        || text.starts_with("• ")
        || text.starts_with("* ")
        || text.starts_with("- ")
}
```

**Complexity**: O(prefix length) per pattern

### 3. Bracketing Match (Prefix + Suffix)

```rust
fn is_button_bracketed(text: &str) -> bool {
    let text = text.trim();
    if text.len() <= 2 { return false; }

    (text.starts_with('[') && text.ends_with(']'))
        || (text.starts_with('(') && text.ends_with(')'))
        || (text.starts_with('<') && text.ends_with('>'))
}
```

### 4. Substring Match

```rust
fn has_input_marker(text: &str) -> bool {
    text.contains("___")
}
```

### 5. Character Class Density

```rust
fn is_panel_border(text: &str) -> bool {
    const BOX_CHARS: &[char] = &[
        '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼',
        '═', '║', '╔', '╗', '╚', '╝', '╠', '╣', '╦', '╩', '╬',
    ];

    let non_ws: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if non_ws.is_empty() { return false; }

    let box_count = non_ws.iter().filter(|c| BOX_CHARS.contains(c)).count();
    box_count * 2 > non_ws.len()  // >50% threshold
}
```

## Optimized Multi-Pattern Matching

### Aho-Corasick Algorithm

For matching many patterns simultaneously:

```rust
use aho_corasick::AhoCorasick;

lazy_static! {
    static ref MENU_PREFIXES: AhoCorasick = AhoCorasick::new(&[
        ">", "❯", "›", "→", "▶", "• ", "* ", "- "
    ]).unwrap();
}

fn is_menu_item_optimized(text: &str) -> bool {
    MENU_PREFIXES.is_match(text) && text.starts_with(|c| MENU_PREFIXES.find(c).is_some())
}
```

**Complexity**: O(n + m) where n = text length, m = total pattern length

### Trie-Based Prefix Matching

```rust
struct PrefixTrie {
    children: HashMap<char, PrefixTrie>,
    is_terminal: bool,
}

impl PrefixTrie {
    fn matches_prefix(&self, text: &str) -> bool {
        let mut node = self;
        for ch in text.chars() {
            if node.is_terminal {
                return true;
            }
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return false,
            }
        }
        node.is_terminal
    }
}
```

## Negative Patterns (Exclusions)

Button detection excludes checkbox patterns:

```rust
fn is_button_text(text: &str) -> bool {
    if text.len() <= 2 { return false; }

    if text.starts_with('[') && text.ends_with(']') {
        let inner = text[1..text.len()-1].trim();
        // Exclude checkbox patterns
        return !matches!(inner, "x" | "X" | " " | "" | "✓" | "✔");
    }

    if text.starts_with('(') && text.ends_with(')') {
        let inner = text[1..text.len()-1].trim();
        // Exclude radio button patterns
        return !matches!(inner, "" | " " | "o" | "O" | "●" | "◉");
    }

    text.starts_with('<') && text.ends_with('>')
}
```

## Pattern Priority Resolution

When multiple patterns could match, priority determines the winner:

```rust
fn classify(text: &str, style: &CellStyle) -> Role {
    // Priority 1: Style-based (strongest signal)
    if style.inverse {
        return if is_top_row() { Role::Tab } else { Role::MenuItem };
    }

    // Priority 2: Explicit markers
    if is_button_bracketed(text) { return Role::Button; }
    if is_checkbox(text) { return Role::Checkbox; }

    // Priority 3: Content patterns
    if is_input_field(text) { return Role::Input; }
    if is_menu_item(text) { return Role::MenuItem; }
    if is_panel_border(text) { return Role::Panel; }

    // Default
    Role::StaticText
}
```

## Fuzzy Matching (Extension)

For approximate pattern matching (e.g., OCR errors):

```rust
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }

    for (i, ca) in a.chars().enumerate() {
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            dp[i + 1][j + 1] = (dp[i][j + 1] + 1)
                .min(dp[i + 1][j] + 1)
                .min(dp[i][j] + cost);
        }
    }

    dp[m][n]
}

fn fuzzy_checkbox_match(text: &str, threshold: usize) -> bool {
    const PATTERNS: &[&str] = &["[x]", "[X]", "[ ]", "[✓]"];
    PATTERNS.iter().any(|p| levenshtein_distance(text.trim(), p) <= threshold)
}
```

## References

- [Aho-Corasick algorithm - Wikipedia](https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm)
- [Trie - Wikipedia](https://en.wikipedia.org/wiki/Trie)
- [Levenshtein distance - Wikipedia](https://en.wikipedia.org/wiki/Levenshtein_distance)
- [Pattern matching - Wikipedia](https://en.wikipedia.org/wiki/Pattern_matching)
