use crate::domain::core::CursorPosition;
use crate::domain::DomainCursorPosition;

pub fn core_cursor_to_domain(cursor: &CursorPosition) -> DomainCursorPosition {
    DomainCursorPosition {
        row: cursor.row,
        col: cursor.col,
        visible: cursor.visible,
    }
}
