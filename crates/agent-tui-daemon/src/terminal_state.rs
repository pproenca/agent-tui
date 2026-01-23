use agent_tui_core::{Component, Element, component_to_element, find_element_by_ref};
use agent_tui_terminal::{CursorPosition, ScreenBuffer, VirtualTerminal};

/// Manages terminal emulation state and element detection.
///
/// Wraps VirtualTerminal and maintains a cache of detected UI elements,
/// separate from PTY lifecycle concerns.
pub struct TerminalState {
    terminal: VirtualTerminal,
    cached_elements: Vec<Element>,
}

impl TerminalState {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            terminal: VirtualTerminal::new(cols, rows),
            cached_elements: Vec::new(),
        }
    }

    pub fn process(&self, data: &[u8]) {
        self.terminal.process(data);
    }

    pub fn screen_text(&self) -> String {
        self.terminal.screen_text()
    }

    pub fn screen_buffer(&self) -> ScreenBuffer {
        self.terminal.screen_buffer()
    }

    pub fn cursor(&self) -> CursorPosition {
        self.terminal.cursor()
    }

    pub fn size(&self) -> (u16, u16) {
        self.terminal.size()
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.terminal.resize(cols, rows);
    }

    pub fn clear(&mut self) {
        self.terminal.clear();
    }

    pub fn detect_elements(&mut self, cursor: &CursorPosition) -> &[Element] {
        let buffer = self.terminal.screen_buffer();
        let components = agent_tui_core::analyze(&buffer, cursor);

        self.cached_elements = components
            .iter()
            .filter(|c| c.role.is_interactive())
            .enumerate()
            .map(|(i, c)| component_to_element(c, i, cursor.row, cursor.col))
            .collect();

        &self.cached_elements
    }

    pub fn cached_elements(&self) -> &[Element] {
        &self.cached_elements
    }

    pub fn find_element(&self, element_ref: &str) -> Option<&Element> {
        find_element_by_ref(&self.cached_elements, element_ref)
    }

    pub fn analyze_screen(&self, cursor: &CursorPosition) -> Vec<Component> {
        let buffer = self.terminal.screen_buffer();
        agent_tui_core::analyze(&buffer, cursor)
    }
}
