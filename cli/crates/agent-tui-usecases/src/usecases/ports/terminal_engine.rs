//! Terminal engine port.

use crate::domain::core::ScreenSnapshot;
use crate::domain::session_types::TerminalSize;

pub trait TerminalEngine: Send {
    fn process_bytes(&mut self, bytes: &[u8]);
    fn resize(&mut self, size: TerminalSize);
    fn snapshot(&self) -> ScreenSnapshot;
    fn plain_text(&self) -> String;
}
