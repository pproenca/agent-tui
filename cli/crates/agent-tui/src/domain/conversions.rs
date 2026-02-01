use crate::domain::DomainCursorPosition;
use crate::domain::core::CursorPosition;

pub fn core_cursor_to_domain(cursor: &CursorPosition) -> DomainCursorPosition {
    DomainCursorPosition {
        row: cursor.row,
        col: cursor.col,
        visible: cursor.visible,
    }
}
