use crate::domain::core::CursorPosition;
use crate::domain::core::style::CellStyle;
use crate::domain::core::test_fixtures::MockScreenBuffer;
use crate::domain::core::vom::{self as vom, Role};

fn make_screen_line(content: &str, style: CellStyle) -> Vec<(char, CellStyle)> {
    content.chars().map(|c| (c, style.clone())).collect()
}

fn no_cursor() -> CursorPosition {
    CursorPosition {
        row: 99,
        col: 99,
        visible: false,
    }
}

fn styled_line(content: &str, bold: bool, inverse: bool) -> Vec<(char, CellStyle)> {
    let style = CellStyle {
        bold,
        inverse,
        ..CellStyle::default()
    };
    content.chars().map(|c| (c, style.clone())).collect()
}

#[test]
fn golden_permission_dialog() {
    let mut buffer = MockScreenBuffer::new(50, 10);

    buffer.set_line(
        0,
        &make_screen_line(
            "╭─ Write ─────────────────────────────────────╮",
            CellStyle::default(),
        ),
    );
    buffer.set_line(
        1,
        &make_screen_line(
            "│ Writing to file: src/main.rs              │",
            CellStyle::default(),
        ),
    );
    buffer.set_line(
        2,
        &make_screen_line(
            "╰─────────────────────────────────────────────╯",
            CellStyle::default(),
        ),
    );
    buffer.set_line(4, &make_screen_line("[Allow]", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());

    let toolblocks: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::ToolBlock)
        .collect();
    let buttons: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::Button)
        .collect();

    assert!(!toolblocks.is_empty(), "Should detect tool block borders");
    assert!(
        !buttons.is_empty(),
        "Should detect button: {:?}",
        buttons.iter().map(|b| &b.text_content).collect::<Vec<_>>()
    );
}

#[test]
fn golden_tool_block_with_code() {
    let mut buffer = MockScreenBuffer::new(50, 8);

    buffer.set_line(
        0,
        &make_screen_line(
            "╭─ Bash ──────────────────────────────────────╮",
            CellStyle::default(),
        ),
    );
    buffer.set_line(
        1,
        &make_screen_line(
            "│ cargo test --workspace                      │",
            CellStyle::default(),
        ),
    );
    buffer.set_line(
        2,
        &make_screen_line(
            "╰─────────────────────────────────────────────╯",
            CellStyle::default(),
        ),
    );

    let components = vom::analyze(&buffer, &no_cursor());

    let toolblocks: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::ToolBlock)
        .collect();

    assert!(
        toolblocks.len() >= 2,
        "Should detect tool block header and footer: {:?}",
        components
            .iter()
            .map(|c| (&c.role, &c.text_content))
            .collect::<Vec<_>>()
    );
}

#[test]
fn golden_status_spinner() {
    let mut buffer = MockScreenBuffer::new(40, 5);

    buffer.set_line(
        0,
        &make_screen_line("⠋ Analyzing codebase...", CellStyle::default()),
    );
    buffer.set_line(
        1,
        &make_screen_line("  Found 42 files", CellStyle::default()),
    );

    let components = vom::analyze(&buffer, &no_cursor());

    let status: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::Status)
        .collect();
    assert!(!status.is_empty(), "Should detect status spinner");
    assert!(
        status[0].text_content.contains("⠋"),
        "Status should contain spinner character"
    );
}

#[test]
fn golden_error_output() {
    let mut buffer = MockScreenBuffer::new(60, 6);

    buffer.set_line(
        0,
        &make_screen_line("Error: compilation failed", CellStyle::default()),
    );
    buffer.set_line(
        1,
        &make_screen_line("  --> src/main.rs:42:10", CellStyle::default()),
    );
    buffer.set_line(2, &make_screen_line("   |", CellStyle::default()));
    buffer.set_line(
        3,
        &make_screen_line("42 |     let x = foo();", CellStyle::default()),
    );
    buffer.set_line(
        4,
        &make_screen_line("   |         ^^^ undefined", CellStyle::default()),
    );

    let components = vom::analyze(&buffer, &no_cursor());

    let errors: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::ErrorMessage)
        .collect();
    let links: Vec<_> = components.iter().filter(|c| c.role == Role::Link).collect();

    assert!(!errors.is_empty(), "Should detect error message");
    assert!(
        !links.is_empty(),
        "Should detect file path link: {:?}",
        components
            .iter()
            .map(|c| (&c.role, &c.text_content))
            .collect::<Vec<_>>()
    );
}

#[test]
fn golden_diff_output() {
    let mut buffer = MockScreenBuffer::new(50, 6);

    buffer.set_line(
        0,
        &make_screen_line("@@ -1,5 +1,6 @@", CellStyle::default()),
    );
    buffer.set_line(
        1,
        &make_screen_line(" unchanged line", CellStyle::default()),
    );
    buffer.set_line(2, &make_screen_line("-removed line", CellStyle::default()));
    buffer.set_line(3, &make_screen_line("+added line", CellStyle::default()));
    buffer.set_line(
        4,
        &make_screen_line(" another unchanged", CellStyle::default()),
    );

    let components = vom::analyze(&buffer, &no_cursor());

    let diffs: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::DiffLine)
        .collect();
    assert!(
        diffs.len() >= 3,
        "Should detect at least 3 diff lines (header, +, -): {:?}",
        diffs.iter().map(|d| &d.text_content).collect::<Vec<_>>()
    );
}

#[test]
fn golden_progress_indicator() {
    let mut buffer = MockScreenBuffer::new(50, 4);

    buffer.set_line(
        0,
        &make_screen_line("████████████░░░░░░░░░░░░", CellStyle::default()),
    );
    buffer.set_line(1, &make_screen_line("50% complete", CellStyle::default()));
    buffer.set_line(2, &make_screen_line("✓ Step 1 done", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());

    let progress: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::ProgressBar)
        .collect();
    let status: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::Status)
        .collect();

    assert!(!progress.is_empty(), "Should detect progress bar");
    assert!(
        !status.is_empty(),
        "Should detect status indicator (checkmark)"
    );
}

#[test]
fn golden_prompt_input() {
    let mut buffer = MockScreenBuffer::new(40, 3);

    buffer.set_line(0, &make_screen_line(">", CellStyle::default()));
    buffer.set_line(
        1,
        &make_screen_line("Enter your query:", CellStyle::default()),
    );

    let components = vom::analyze(&buffer, &no_cursor());

    let prompts: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::PromptMarker)
        .collect();
    assert!(!prompts.is_empty(), "Should detect prompt marker");
}

#[test]
fn golden_selected_menu_item() {
    let mut buffer = MockScreenBuffer::new(30, 8);

    buffer.set_line(0, &make_screen_line("Options:", CellStyle::default()));
    buffer.set_line(3, &make_screen_line("> First option", CellStyle::default()));
    buffer.set_line(4, &styled_line("❯ Second option", false, true));
    buffer.set_line(5, &make_screen_line("> Third option", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());

    let menu_items: Vec<_> = components
        .iter()
        .filter(|c| c.role == Role::MenuItem)
        .collect();
    let selected: Vec<_> = menu_items.iter().filter(|m| m.selected).collect();

    assert!(
        menu_items.len() >= 2,
        "Should detect menu items: {:?}",
        components
            .iter()
            .map(|c| (&c.role, &c.text_content))
            .collect::<Vec<_>>()
    );
    assert!(
        !selected.is_empty(),
        "Should detect selected menu item with ❯ prefix and inverse: {:?}",
        menu_items
            .iter()
            .map(|m| (&m.text_content, m.selected))
            .collect::<Vec<_>>()
    );
}
