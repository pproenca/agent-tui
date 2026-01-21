//! Framework-specific element detectors
//!
//! Each framework has its own detector implementation that can provide
//! specialized pattern matching for that framework's UI conventions.

mod bubbletea;
mod generic;
mod ink;
mod inquirer;
mod ratatui;
mod textual;

pub use bubbletea::BubbleTeaDetector;
pub use generic::GenericDetector;
pub use ink::InkDetector;
pub use inquirer::InquirerDetector;
pub use ratatui::RatatuiDetector;
pub use textual::TextualDetector;
