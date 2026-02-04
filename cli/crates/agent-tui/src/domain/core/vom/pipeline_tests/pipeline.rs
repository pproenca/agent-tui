//! VOM pipeline test cases.

use crate::domain::core::CursorPosition;
use crate::domain::core::style::CellStyle;
use crate::domain::core::test_fixtures::MockScreenBuffer;
use crate::domain::core::vom;
use crate::domain::core::vom::Role;

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

#[test]
fn test_pipeline_button_detection() {
    let mut buffer = MockScreenBuffer::new(10, 5);
    buffer.set_line(0, &make_screen_line("[OK]", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());
    let button = components.iter().find(|c| c.role == Role::Button);

    assert!(button.is_some(), "Button should be detected");
    assert!(button.unwrap().text_content.contains("OK"));
}

#[test]
fn test_pipeline_link_url_detection() {
    let mut buffer = MockScreenBuffer::new(30, 5);
    buffer.set_line(
        0,
        &make_screen_line("https://example.com", CellStyle::default()),
    );

    let components = vom::analyze(&buffer, &no_cursor());
    let link = components.iter().find(|c| c.role == Role::Link);

    assert!(link.is_some(), "Link should be detected");
}

#[test]
fn test_pipeline_file_path_link_detection() {
    let mut buffer = MockScreenBuffer::new(30, 5);
    buffer.set_line(0, &make_screen_line("src/main.rs:42", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());
    let link = components.iter().find(|c| c.role == Role::Link);

    assert!(link.is_some(), "File path link should be detected");
}

#[test]
fn test_pipeline_progress_bar_detection() {
    let mut buffer = MockScreenBuffer::new(20, 5);
    buffer.set_line(0, &make_screen_line("████░░░░", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());
    let progress = components.iter().find(|c| c.role == Role::ProgressBar);

    assert!(progress.is_some(), "Progress bar should be detected");
}

#[test]
fn test_pipeline_error_message_detection() {
    let mut buffer = MockScreenBuffer::new(40, 5);
    buffer.set_line(
        0,
        &make_screen_line("Error: compilation failed", CellStyle::default()),
    );

    let components = vom::analyze(&buffer, &no_cursor());
    let error = components.iter().find(|c| c.role == Role::ErrorMessage);

    assert!(error.is_some(), "Error message should be detected");
}

#[test]
fn test_pipeline_diff_addition_detection() {
    let mut buffer = MockScreenBuffer::new(20, 5);
    buffer.set_line(0, &make_screen_line("+ added line", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());
    let diff = components.iter().find(|c| c.role == Role::DiffLine);

    assert!(diff.is_some(), "Diff addition should be detected");
}

#[test]
fn test_pipeline_diff_deletion_detection() {
    let mut buffer = MockScreenBuffer::new(20, 5);
    buffer.set_line(0, &make_screen_line("-removed", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());
    let diff = components.iter().find(|c| c.role == Role::DiffLine);

    assert!(diff.is_some(), "Diff deletion should be detected");
}

#[test]
fn test_pipeline_checkbox_detection() {
    let mut buffer = MockScreenBuffer::new(10, 5);
    buffer.set_line(0, &make_screen_line("[x]", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());
    let checkbox = components.iter().find(|c| c.role == Role::Checkbox);

    assert!(checkbox.is_some(), "Checkbox should be detected");
}

#[test]
fn test_pipeline_status_spinner_detection() {
    let mut buffer = MockScreenBuffer::new(20, 5);
    buffer.set_line(0, &make_screen_line("⠋ Loading...", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());
    let status = components.iter().find(|c| c.role == Role::Status);

    assert!(status.is_some(), "Status spinner should be detected");
}

#[test]
fn test_pipeline_tool_block_border_detection() {
    let mut buffer = MockScreenBuffer::new(30, 5);
    buffer.set_line(
        0,
        &make_screen_line("╭─ Write ─────────────────╮", CellStyle::default()),
    );

    let components = vom::analyze(&buffer, &no_cursor());
    let toolblock = components.iter().find(|c| c.role == Role::ToolBlock);

    assert!(toolblock.is_some(), "Tool block should be detected");
}

#[test]
fn test_pipeline_prompt_marker_detection() {
    let mut buffer = MockScreenBuffer::new(5, 5);
    buffer.set_line(0, &make_screen_line(">", CellStyle::default()));

    let components = vom::analyze(&buffer, &no_cursor());
    let prompt = components.iter().find(|c| c.role == Role::PromptMarker);

    assert!(prompt.is_some(), "Prompt marker should be detected");
}
