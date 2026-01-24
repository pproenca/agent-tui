use uuid::Uuid;

use super::screen::ScreenGrid;
use super::style::{CellStyle, Color};
use super::vom::{Cluster, Component, Rect, Role};
use crate::core::{Element, ElementType, Position};

#[derive(Debug, Clone)]
pub struct Cell {
    pub char: char,
    pub style: CellStyle,
}

#[derive(Debug)]
pub struct MockScreenBuffer {
    pub cells: Vec<Vec<Cell>>,
}

impl MockScreenBuffer {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cells = (0..rows)
            .map(|_| {
                (0..cols)
                    .map(|_| Cell {
                        char: ' ',
                        style: CellStyle::default(),
                    })
                    .collect()
            })
            .collect();
        Self { cells }
    }

    pub fn set_line(&mut self, row: usize, content: &[(char, CellStyle)]) {
        if row >= self.cells.len() {
            return;
        }
        for (col, (ch, style)) in content.iter().enumerate() {
            if col < self.cells[row].len() {
                self.cells[row][col] = Cell {
                    char: *ch,
                    style: style.clone(),
                };
            }
        }
    }
}

impl ScreenGrid for MockScreenBuffer {
    fn rows(&self) -> usize {
        self.cells.len()
    }

    fn cols(&self) -> usize {
        self.cells.first().map(|r| r.len()).unwrap_or(0)
    }

    fn cell(&self, row: usize, col: usize) -> Option<(char, CellStyle)> {
        self.cells
            .get(row)
            .and_then(|r| r.get(col))
            .map(|c| (c.char, c.style.clone()))
    }
}

pub fn make_cell(char: char, bold: bool, bg: Option<Color>) -> Cell {
    Cell {
        char,
        style: CellStyle {
            bold,
            underline: false,
            inverse: false,
            fg_color: None,
            bg_color: bg,
        },
    }
}

pub fn make_buffer(cells: Vec<Vec<Cell>>) -> MockScreenBuffer {
    MockScreenBuffer { cells }
}

pub fn make_cluster(text: &str, style: CellStyle, x: u16, y: u16) -> Cluster {
    Cluster {
        rect: Rect::new(x, y, text.len() as u16, 1),
        text: text.to_string(),
        style,
        is_whitespace: false,
    }
}

pub fn make_component(role: Role, text: &str, x: u16, y: u16, width: u16) -> Component {
    Component {
        id: Uuid::new_v4(),
        role,
        bounds: Rect::new(x, y, width, 1),
        text_content: text.to_string(),
        visual_hash: 0,
        selected: false,
    }
}

pub fn make_element(ref_str: &str, element_type: ElementType) -> Element {
    Element {
        element_ref: ref_str.to_string(),
        element_type,
        label: Some("test".to_string()),
        value: None,
        position: Position {
            row: 0,
            col: 0,
            width: Some(10),
            height: Some(1),
        },
        focused: false,
        selected: false,
        checked: None,
        disabled: None,
        hint: None,
    }
}

pub struct ElementBuilder {
    element_ref: String,
    element_type: ElementType,
    label: Option<String>,
    value: Option<String>,
    row: u16,
    col: u16,
    width: Option<u16>,
    height: Option<u16>,
    focused: bool,
    selected: bool,
    checked: Option<bool>,
    disabled: Option<bool>,
    hint: Option<String>,
}

impl ElementBuilder {
    pub fn new() -> Self {
        Self {
            element_ref: "@e1".to_string(),
            element_type: ElementType::Button,
            label: None,
            value: None,
            row: 0,
            col: 0,
            width: Some(10),
            height: Some(1),
            focused: false,
            selected: false,
            checked: None,
            disabled: None,
            hint: None,
        }
    }

    pub fn element_ref(mut self, ref_str: &str) -> Self {
        self.element_ref = ref_str.to_string();
        self
    }

    pub fn button(mut self) -> Self {
        self.element_type = ElementType::Button;
        self
    }

    pub fn input(mut self) -> Self {
        self.element_type = ElementType::Input;
        self
    }

    pub fn checkbox(mut self) -> Self {
        self.element_type = ElementType::Checkbox;
        self
    }

    pub fn menu_item(mut self) -> Self {
        self.element_type = ElementType::MenuItem;
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
        self
    }

    pub fn position(mut self, row: u16, col: u16) -> Self {
        self.row = row;
        self.col = col;
        self
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn focused(mut self) -> Self {
        self.focused = true;
        self
    }

    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }

    pub fn checked(mut self, state: bool) -> Self {
        self.checked = Some(state);
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = Some(true);
        self
    }

    pub fn hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string());
        self
    }

    pub fn build(self) -> Element {
        Element {
            element_ref: self.element_ref,
            element_type: self.element_type,
            label: self.label,
            value: self.value,
            position: Position {
                row: self.row,
                col: self.col,
                width: self.width,
                height: self.height,
            },
            focused: self.focused,
            selected: self.selected,
            checked: self.checked,
            disabled: self.disabled,
            hint: self.hint,
        }
    }
}

impl Default for ElementBuilder {
    fn default() -> Self {
        Self::new()
    }
}
