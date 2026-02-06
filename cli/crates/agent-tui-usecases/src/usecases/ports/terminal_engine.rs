//! Terminal engine port.

use crate::domain::core::ScreenSnapshot;

pub trait TerminalEngine: Send {
    fn process_bytes(&mut self, bytes: &[u8]);
    fn resize(&mut self, cols: u16, rows: u16);
    fn snapshot(&self) -> ScreenSnapshot;
    fn plain_text(&self) -> String;
}
