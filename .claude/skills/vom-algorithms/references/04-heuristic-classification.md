# Heuristic Classification for UI Elements

## Algorithm Overview

Heuristic classification uses a **priority-ordered rule cascade** to assign roles to clusters based on geometric properties, text patterns, and style attributes.

## Rule Cascade Pattern

```rust
fn classify(cluster: &Cluster, context: &Context) -> Role {
    // Rules evaluated in priority order
    // First matching rule wins

    if matches_input_cursor(cluster, context) {
        return Role::Input;
    }
    if matches_button_pattern(&cluster.text) {
        return Role::Button;
    }
    if cluster.style.inverse && cluster.y <= 2 {
        return Role::Tab;
    }
    if cluster.style.inverse {
        return Role::MenuItem;
    }
    if matches_tab_color(&cluster.style) {
        return Role::Tab;
    }
    if matches_input_field(&cluster.text) {
        return Role::Input;
    }
    if matches_checkbox(&cluster.text) {
        return Role::Checkbox;
    }
    if matches_menu_prefix(&cluster.text) {
        return Role::MenuItem;
    }
    if matches_panel_border(&cluster.text) {
        return Role::Panel;
    }

    Role::StaticText  // Default fallback
}
```

## Rule Categories

### 1. Context-Based Rules (Highest Priority)

```rust
// Cursor position implies input field
fn matches_input_cursor(cluster: &Cluster, cursor_row: u16, cursor_col: u16) -> bool {
    cluster.y == cursor_row
        && cursor_col >= cluster.x
        && cursor_col < cluster.x + cluster.width
}
```

### 2. Text Pattern Rules

```rust
// Button: bracketed text (excluding checkboxes)
fn is_button_text(text: &str) -> bool {
    let text = text.trim();
    if text.len() <= 2 { return false; }

    // [OK], [Submit], [Cancel] - but not [x], [ ], [✓]
    if text.starts_with('[') && text.ends_with(']') {
        let inner = text[1..text.len()-1].trim();
        return !matches!(inner, "x" | "X" | " " | "" | "✓" | "✔");
    }

    // (Button) - but not radio buttons
    if text.starts_with('(') && text.ends_with(')') {
        let inner = text[1..text.len()-1].trim();
        return !matches!(inner, "" | " " | "o" | "O" | "●" | "◉");
    }

    // <Button>
    text.starts_with('<') && text.ends_with('>')
}

// Checkbox patterns
fn is_checkbox(text: &str) -> bool {
    matches!(text.trim(),
        "[x]" | "[X]" | "[ ]" | "[✓]" | "[✔]" |
        "◉" | "◯" | "●" | "○" | "◼" | "◻" | "☐" | "☑" | "☒"
    )
}

// Input field markers
fn is_input_field(text: &str) -> bool {
    text.contains("___")
        || (!text.is_empty() && text.chars().all(|c| c == '_'))
        || text.ends_with(": _") || text.ends_with(":_")
}

// Menu item prefixes
fn is_menu_item(text: &str) -> bool {
    text.starts_with('>') || text.starts_with('❯')
        || text.starts_with('›') || text.starts_with('→')
        || text.starts_with('▶') || text.starts_with("• ")
        || text.starts_with("* ") || text.starts_with("- ")
}
```

### 3. Style-Based Rules

```rust
// Inverse video at top = tab bar
if cluster.style.inverse && cluster.rect.y <= 2 {
    return Role::Tab;
}

// Inverse video elsewhere = menu item
if cluster.style.inverse {
    return Role::MenuItem;
}

// Common tab background colors (blue, cyan)
if let Some(Color::Indexed(idx)) = &cluster.style.bg_color {
    if *idx == 4 || *idx == 6 {
        return Role::Tab;
    }
}
```

### 4. Content Density Rules

```rust
// Panel detection via box-drawing character density
fn is_panel_border(text: &str) -> bool {
    const BOX_CHARS: &[char] = &[
        '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼',
        '═', '║', '╔', '╗', '╚', '╝', '╠', '╣', '╦', '╩', '╬',
    ];

    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 { return false; }

    let box_count = text.chars().filter(|c| BOX_CHARS.contains(c)).count();
    box_count > total / 2  // >50% box characters
}
```

## Decision Tree Representation

```
                         classify(cluster)
                               │
                    ┌──────────┴──────────┐
                    │ cursor intersects?  │
                    └──────────┬──────────┘
                         │ yes
                         ▼
                      Input ←──────────────────────────────┐
                         │ no                              │
                    ┌────┴─────┐                           │
                    │ button?  │                           │
                    └────┬─────┘                           │
                         │ yes                             │
                         ▼                                 │
                      Button                               │
                         │ no                              │
                    ┌────┴─────┐                           │
                    │ inverse? │                           │
                    └────┬─────┘                           │
                    │ yes                                  │
               ┌────┴────┐                                 │
               │ y <= 2? │                                 │
               └────┬────┘                                 │
          yes │         │ no                               │
              ▼         ▼                                  │
            Tab    MenuItem                                │
                         │ no inverse                      │
                    ┌────┴─────┐                           │
                    │ tab bg?  │                           │
                    └────┬─────┘                           │
                         │ yes                             │
                         ▼                                 │
                       Tab                                 │
                         │ no                              │
                    ┌────┴─────┐                           │
                    │ input?   │───────────────────────────┘
                    └────┬─────┘
                         │ no
                    ┌────┴─────┐
                    │ checkbox │
                    └────┬─────┘
                         │ yes
                         ▼
                     Checkbox
                         │ no
                    ┌────┴─────┐
                    │ menu?    │
                    └────┬─────┘
                         │ yes
                         ▼
                     MenuItem
                         │ no
                    ┌────┴─────┐
                    │ panel?   │
                    └────┬─────┘
                         │ yes
                         ▼
                      Panel
                         │ no
                         ▼
                    StaticText
```

## Rule Ordering Rationale

1. **Context beats content**: Cursor position is the strongest signal
2. **Explicit beats implicit**: Bracketed buttons are explicit UI affordances
3. **Style beats text**: Inverse video is a strong TUI convention
4. **Specific beats general**: Checkboxes before generic input fields
5. **Default is text**: Unknown elements are static text

## References

- [Decision tree learning - Wikipedia](https://en.wikipedia.org/wiki/Decision_tree_learning)
- [Pattern recognition - Wikipedia](https://en.wikipedia.org/wiki/Pattern_recognition)
- [Rule-based systems and pattern recognition](https://www.researchgate.net/publication/223116102_Rule-based_systems_and_pattern_recognition)
