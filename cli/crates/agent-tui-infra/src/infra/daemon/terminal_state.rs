//! Terminal state management.

use crate::domain::session_types::TerminalSize;
use crate::infra::terminal::CursorPosition;
use crate::infra::terminal::ScreenBuffer;
use crate::infra::terminal::VirtualTerminal;

pub struct TerminalState {
    terminal: VirtualTerminal,
}

impl TerminalState {
    pub fn new(size: TerminalSize) -> Self {
        Self {
            terminal: VirtualTerminal::new(size),
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

    pub fn size(&self) -> TerminalSize {
        self.terminal.size()
    }

    pub fn resize(&mut self, size: TerminalSize) {
        self.terminal.resize(size);
    }
}
