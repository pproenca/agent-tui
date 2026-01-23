# VT100 Terminal Emulation State Machine

## Algorithm Overview

Terminal emulation parses a byte stream containing text and ANSI escape sequences. The VT100 state machine processes input character-by-character, transitioning between states and executing actions based on the current state and input.

## State Machine Architecture

```
Input Byte Stream → State Machine → Screen Buffer + Cursor + Attributes
                         ↓
                    [State, Action]
```

## Core States

| State | Description | Entry Condition |
|-------|-------------|-----------------|
| Ground | Normal text processing | Default, after sequence complete |
| Escape | ESC received | 0x1B byte |
| CsiEntry | CSI sequence started | ESC + '[' or 0x9B |
| CsiParam | Reading parameters | Digit or ';' in CsiEntry |
| CsiIntermediate | Intermediate chars | 0x20-0x2F in CSI |
| OscString | Operating System Command | ESC + ']' |

## State Transition Diagram

```
                    ┌─────────────────────────────────────┐
                    │                                     │
        ┌───────────┼─────────────────┐                   │
        │           │                 │                   │
        ▼           │                 ▼                   │
    ┌───────┐   ESC │           ┌──────────┐             │
    │ Ground│───────┼──────────▶│  Escape  │             │
    └───────┘       │           └──────────┘             │
        ▲           │                 │                   │
        │           │                 │ '['               │
        │           │                 ▼                   │
        │           │           ┌──────────┐             │
        │           │           │ CsiEntry │             │
        │           │           └──────────┘             │
        │           │                 │                   │
        │           │      0-9, ';'   │  0x40-0x7E       │
        │           │                 ▼         │         │
        │           │           ┌──────────┐   │         │
        │           │           │ CsiParam │───┘         │
        │           │           └──────────┘             │
        │           │                                     │
        └───────────┴─────────────────────────────────────┘
                        (dispatch or cancel)
```

## Escape Sequence Structure

```
ESC [ Pn ; Pn ; ... m
│   │ └──────────┘  │
│   │      │        └── Final byte (action identifier)
│   │      └─────────── Parameters (semicolon-separated)
│   └────────────────── CSI introducer
└────────────────────── Escape (0x1B)
```

## Common CSI Sequences

| Sequence | Name | Effect |
|----------|------|--------|
| `ESC[Pn;PnH` | CUP | Cursor Position (row;col) |
| `ESC[PnA` | CUU | Cursor Up |
| `ESC[PnB` | CUD | Cursor Down |
| `ESC[PnC` | CUF | Cursor Forward |
| `ESC[PnD` | CUB | Cursor Back |
| `ESC[2J` | ED | Erase Display |
| `ESC[K` | EL | Erase Line |
| `ESC[Pnm` | SGR | Select Graphic Rendition |

## SGR (Style) Parameters

| Code | Effect | VOM Style Field |
|------|--------|-----------------|
| 0 | Reset all | All fields default |
| 1 | Bold | `bold: true` |
| 4 | Underline | `underline: true` |
| 7 | Inverse | `inverse: true` |
| 30-37 | Foreground color | `fg_color: Indexed(n-30)` |
| 40-47 | Background color | `bg_color: Indexed(n-40)` |
| 38;5;n | 256-color FG | `fg_color: Indexed(n)` |
| 48;5;n | 256-color BG | `bg_color: Indexed(n)` |
| 38;2;r;g;b | RGB FG | `fg_color: Rgb(r,g,b)` |
| 48;2;r;g;b | RGB BG | `bg_color: Rgb(r,g,b)` |

## State Machine Implementation Pattern

```rust
pub struct Parser {
    state: State,
    params: Vec<u16>,
    intermediate: Vec<u8>,
}

impl Parser {
    pub fn process(&mut self, byte: u8, screen: &mut Screen) {
        match (self.state, byte) {
            // Ground state: print or control
            (State::Ground, 0x20..=0x7E) => screen.print(byte as char),
            (State::Ground, 0x1B) => self.state = State::Escape,
            (State::Ground, 0x0D) => screen.carriage_return(),
            (State::Ground, 0x0A) => screen.line_feed(),

            // Escape state
            (State::Escape, b'[') => {
                self.state = State::CsiEntry;
                self.params.clear();
                self.intermediate.clear();
            }

            // CSI parameter collection
            (State::CsiEntry | State::CsiParam, b'0'..=b'9') => {
                self.state = State::CsiParam;
                self.accumulate_digit(byte);
            }
            (State::CsiParam, b';') => {
                self.push_param();
            }

            // CSI dispatch
            (State::CsiEntry | State::CsiParam, 0x40..=0x7E) => {
                self.push_param();
                self.dispatch_csi(byte, screen);
                self.state = State::Ground;
            }

            // Cancel/ignore
            _ => self.state = State::Ground,
        }
    }

    fn dispatch_csi(&self, final_byte: u8, screen: &mut Screen) {
        match final_byte {
            b'H' | b'f' => screen.cursor_position(
                self.params.get(0).copied().unwrap_or(1),
                self.params.get(1).copied().unwrap_or(1),
            ),
            b'm' => self.apply_sgr(screen),
            b'J' => screen.erase_display(self.params.get(0).copied().unwrap_or(0)),
            b'K' => screen.erase_line(self.params.get(0).copied().unwrap_or(0)),
            // ... other sequences
            _ => {}
        }
    }
}
```

## Integration with VOM

The vt100 crate handles parsing internally. VOM extracts the resulting screen state:

```rust
use vt100::Parser;

let mut parser = Parser::new(rows, cols, 0);
parser.process(raw_bytes);

let screen = parser.screen();
for row in 0..screen.size().0 {
    for col in 0..screen.size().1 {
        let cell = screen.cell(row, col).unwrap();
        // Extract: cell.contents(), cell.bold(), cell.fgcolor(), etc.
    }
}
```

## References

- [VT100.net: DEC ANSI Parser](https://vt100.net/emu/dec_ansi_parser) - Complete state machine specification
- [ANSI escape code - Wikipedia](https://en.wikipedia.org/wiki/ANSI_escape_code)
- [VT100 User Guide Chapter 3](https://vt100.net/docs/vt100-ug/chapter3.html)
- [ECMA-48 Standard](https://www.ecma-international.org/publications-and-standards/standards/ecma-48/)
- [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
