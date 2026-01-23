# Unicode and Wide Character Handling in Terminals

## Overview

Terminals display characters in a fixed-width grid, but Unicode introduces variable-width characters. Understanding character width is essential for accurate element positioning.

## Character Width Categories

| Category | Display Width | Examples |
|----------|---------------|----------|
| ASCII | 1 cell | `A`, `1`, `@` |
| Latin Extended | 1 cell | `é`, `ñ`, `ü` |
| CJK Ideographs | 2 cells | `漢`, `字`, `한` |
| Emoji | 2 cells | `😀`, `🎉`, `❤️` |
| Combining Marks | 0 cells | `◌́` (acute accent) |
| Zero-Width | 0 cells | ZWSP, ZWJ |
| Box Drawing | 1 cell | `─`, `│`, `┌` |

## Unicode Width Determination

The `unicode-width` crate provides standard width calculation:

```rust
use unicode_width::UnicodeWidthChar;

fn cell_width(ch: char) -> usize {
    ch.width().unwrap_or(0)
}

// Examples:
// 'A'.width() = Some(1)
// '漢'.width() = Some(2)
// '\u{0301}'.width() = Some(0)  // Combining acute
```

## Impact on VOM Segmentation

### Problem: Column Counting

```
Text:    "Hello漢字World"
Bytes:   H e l l o 漢    字    W o r l d
Chars:   0 1 2 3 4 5     6     7 8 9 10 11
Columns: 0 1 2 3 4 5  6  7  8  9 10 11 12 13
```

Wide characters occupy 2 columns but are 1 character.

### Solution: Width-Aware Positioning

```rust
fn text_display_width(text: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    text.width()
}

fn column_to_char_index(text: &str, target_col: usize) -> Option<usize> {
    let mut col = 0;
    for (idx, ch) in text.chars().enumerate() {
        if col >= target_col {
            return Some(idx);
        }
        col += ch.width().unwrap_or(1);
    }
    None
}
```

## vt100 Crate Handling

The vt100 crate stores one cell per terminal column. Wide characters occupy two cells:

```rust
// Screen representation of "A漢B"
// Column: 0   1   2   3
// Cell:   'A' '漢' ' ' 'B'
//              ↑
//         Wide char in cell 1, cell 2 is padding
```

VOM must handle this when extracting text:

```rust
fn extract_text_from_cells(cells: &[Cell]) -> String {
    let mut text = String::new();
    let mut skip_next = false;

    for cell in cells {
        if skip_next {
            skip_next = false;
            continue;
        }

        let ch = cell.char;
        text.push(ch);

        // Wide character occupies next cell too
        if ch.width().unwrap_or(1) > 1 {
            skip_next = true;
        }
    }

    text
}
```

## Box Drawing Characters

Box drawing is crucial for panel detection:

```rust
const BOX_LIGHT: &[char] = &['─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼'];
const BOX_HEAVY: &[char] = &['━', '┃', '┏', '┓', '┗', '┛', '┣', '┫', '┳', '┻', '╋'];
const BOX_DOUBLE: &[char] = &['═', '║', '╔', '╗', '╚', '╝', '╠', '╣', '╦', '╩', '╬'];
const BOX_ROUNDED: &[char] = &['╭', '╮', '╯', '╰'];

fn is_box_drawing(ch: char) -> bool {
    ('\u{2500}'..='\u{257F}').contains(&ch)  // Box Drawing block
}
```

## Emoji and Grapheme Clusters

Some "characters" are multiple codepoints:

```rust
// "👨‍👩‍👧" is:
// U+1F468 (man) + U+200D (ZWJ) + U+1F469 (woman) + U+200D (ZWJ) + U+1F467 (girl)

use unicode_segmentation::UnicodeSegmentation;

fn count_graphemes(text: &str) -> usize {
    text.graphemes(true).count()
}
```

## Normalization

Unicode has multiple representations for the same visual character:

```rust
use unicode_normalization::UnicodeNormalization;

// "é" can be:
// - U+00E9 (precomposed)
// - U+0065 U+0301 (decomposed: 'e' + combining acute)

fn normalize_for_comparison(text: &str) -> String {
    text.nfc().collect()  // Canonical decomposition, then composition
}
```

## Terminal-Safe String Truncation

```rust
fn truncate_to_width(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthStr;

    let mut result = String::new();
    let mut width = 0;

    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(1);
        if width + ch_width > max_width {
            break;
        }
        result.push(ch);
        width += ch_width;
    }

    result
}
```

## References

- [Unicode Standard Annex #11: East Asian Width](https://www.unicode.org/reports/tr11/)
- [Unicode Standard Annex #29: Text Segmentation](https://www.unicode.org/reports/tr29/)
- [unicode-width crate](https://crates.io/crates/unicode-width)
- [unicode-segmentation crate](https://crates.io/crates/unicode-segmentation)
- [Terminal emulators and Unicode](https://github.com/microsoft/terminal/blob/main/doc/specs/%235185%20-%20Input%20Encoding.md)
