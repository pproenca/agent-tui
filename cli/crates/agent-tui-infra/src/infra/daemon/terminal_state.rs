//! Terminal state management.

use crate::infra::terminal::CursorPosition;
use crate::infra::terminal::ScreenBuffer;
use crate::infra::terminal::VirtualTerminal;

pub struct TerminalState {
    terminal: VirtualTerminal,
}

impl TerminalState {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            terminal: VirtualTerminal::new(cols, rows),
        }
    }

    pub fn process(&mut self, data: &[u8]) {
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
}
