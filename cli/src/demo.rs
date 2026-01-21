use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Write};
use std::time::Duration;

struct DemoState {
    name_value: String,
    notifications_checked: bool,
    focused_field: usize,
    submitted: bool,
    cancelled: bool,
    cursor_visible: bool,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            name_value: String::new(),
            notifications_checked: false,
            focused_field: 0,
            submitted: false,
            cancelled: false,
            cursor_visible: true,
        }
    }
}

pub fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();
    let mut state = DemoState::default();

    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let result = run_demo_loop(&mut stdout, &mut state);

    execute!(stdout, LeaveAlternateScreen, Show)?;
    terminal::disable_raw_mode()?;

    if let Err(e) = result {
        eprintln!("Demo error: {}", e);
        return Err(e);
    }

    if state.submitted {
        println!("Form submitted!");
        println!("  Name: {}", state.name_value);
        println!(
            "  Notifications: {}",
            if state.notifications_checked {
                "enabled"
            } else {
                "disabled"
            }
        );
    } else if state.cancelled {
        println!("Form cancelled.");
    }

    Ok(())
}

fn run_demo_loop(
    stdout: &mut std::io::Stdout,
    state: &mut DemoState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        render(stdout, state)?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(state, key) {
                    break;
                }
            }
        }

        state.cursor_visible = !state.cursor_visible;
    }
    Ok(())
}

fn handle_key(state: &mut DemoState, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.cancelled = true;
            return true;
        }
        KeyCode::Esc => {
            state.cancelled = true;
            return true;
        }
        KeyCode::Tab | KeyCode::Down => {
            state.focused_field = (state.focused_field + 1) % 4;
        }
        KeyCode::BackTab | KeyCode::Up => {
            state.focused_field = (state.focused_field + 3) % 4;
        }
        KeyCode::Enter => match state.focused_field {
            0 => {
                state.focused_field = 1;
            }
            1 => {
                state.notifications_checked = !state.notifications_checked;
            }
            2 => {
                state.submitted = true;
                return true;
            }
            3 => {
                state.cancelled = true;
                return true;
            }
            _ => {}
        },
        KeyCode::Char(' ') => {
            if state.focused_field == 1 {
                state.notifications_checked = !state.notifications_checked;
            }
        }
        KeyCode::Char(c) => {
            if state.focused_field == 0 {
                state.name_value.push(c);
            }
        }
        KeyCode::Backspace => {
            if state.focused_field == 0 {
                state.name_value.pop();
            }
        }
        _ => {}
    }
    false
}

fn render(
    stdout: &mut std::io::Stdout,
    state: &DemoState,
) -> Result<(), Box<dyn std::error::Error>> {
    execute!(stdout, Clear(ClearType::All))?;

    let (width, height) = terminal::size()?;
    let box_width = 40u16;
    let box_height = 11u16;
    let start_x = width.saturating_sub(box_width) / 2;
    let start_y = height.saturating_sub(box_height) / 2;

    draw_box(stdout, start_x, start_y, box_width, box_height)?;

    execute!(
        stdout,
        MoveTo(start_x + 2, start_y + 1),
        SetForegroundColor(Color::Cyan),
        Print("agent-tui Demo"),
        ResetColor
    )?;

    let name_focused = state.focused_field == 0;
    let input_label = "Name";
    let input_value = &state.name_value;
    let cursor = if name_focused && state.cursor_visible {
        "_"
    } else {
        ""
    };
    let padding = 16usize.saturating_sub(input_value.len() + cursor.len());
    let underscores = "_".repeat(padding);

    execute!(
        stdout,
        MoveTo(start_x + 2, start_y + 3),
        if name_focused {
            SetForegroundColor(Color::Yellow)
        } else {
            SetForegroundColor(Color::White)
        },
        Print(format!(
            "{}: [{}{}{}]",
            input_label, input_value, cursor, underscores
        )),
        ResetColor
    )?;

    let checkbox_focused = state.focused_field == 1;
    let checkbox_marker = if state.notifications_checked {
        "x"
    } else {
        " "
    };

    execute!(
        stdout,
        MoveTo(start_x + 2, start_y + 5),
        if checkbox_focused {
            SetForegroundColor(Color::Yellow)
        } else {
            SetForegroundColor(Color::White)
        },
        Print(format!("[{}] Enable notifications", checkbox_marker)),
        ResetColor
    )?;

    let submit_focused = state.focused_field == 2;
    let cancel_focused = state.focused_field == 3;

    execute!(
        stdout,
        MoveTo(start_x + 4, start_y + 7),
        if submit_focused {
            SetBackgroundColor(Color::Blue)
        } else {
            SetBackgroundColor(Color::Reset)
        },
        if submit_focused {
            SetForegroundColor(Color::White)
        } else {
            SetForegroundColor(Color::Green)
        },
        Print("[Submit]"),
        ResetColor,
        Print("   "),
        if cancel_focused {
            SetBackgroundColor(Color::Blue)
        } else {
            SetBackgroundColor(Color::Reset)
        },
        if cancel_focused {
            SetForegroundColor(Color::White)
        } else {
            SetForegroundColor(Color::Red)
        },
        Print("[Cancel]"),
        ResetColor
    )?;

    execute!(
        stdout,
        MoveTo(start_x + 2, start_y + 9),
        SetForegroundColor(Color::DarkGrey),
        Print("Tab: next | Enter: select | Esc: quit"),
        ResetColor
    )?;

    stdout.flush()?;
    Ok(())
}

fn draw_box(
    stdout: &mut std::io::Stdout,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    execute!(
        stdout,
        MoveTo(x, y),
        Print("┌"),
        Print("─".repeat((width - 2) as usize)),
        Print("┐")
    )?;

    for row in 1..height - 1 {
        execute!(
            stdout,
            MoveTo(x, y + row),
            Print("│"),
            MoveTo(x + width - 1, y + row),
            Print("│")
        )?;
    }

    execute!(
        stdout,
        MoveTo(x, y + height - 1),
        Print("└"),
        Print("─".repeat((width - 2) as usize)),
        Print("┘")
    )?;

    Ok(())
}
