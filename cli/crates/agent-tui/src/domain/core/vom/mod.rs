//! Visual Object Model (VOM) primitives and analysis.

pub mod classifier;
pub mod patterns;
pub mod segmentation;

#[cfg(test)]
mod pipeline_tests;

use std::hash::Hash;
use std::hash::Hasher;

use crate::domain::core::screen::ScreenGrid;
use crate::domain::core::style::CellStyle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x
            && x < self.x.saturating_add(self.width)
            && y >= self.y
            && y < self.y.saturating_add(self.height)
    }
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub rect: Rect,
    pub text: String,
    pub style: CellStyle,
    pub is_whitespace: bool,
}

impl Cluster {
    pub fn new(x: u16, y: u16, char: char, style: CellStyle) -> Self {
        Self {
            rect: Rect::new(x, y, 1, 1),
            text: char.to_string(),
            style,
            is_whitespace: false,
        }
    }

    pub fn extend(&mut self, char: char) {
        self.text.push(char);
        self.rect.width = self.rect.width.saturating_add(1);
    }

    pub fn seal(&mut self) {
        self.is_whitespace = self.text.trim().is_empty();
    }
}

#[derive(Debug, Clone)]
pub struct Component {
    pub role: Role,
    pub bounds: Rect,
    pub text_content: String,
    pub visual_hash: u64,
    pub selected: bool,
}

impl Component {
    pub fn new(role: Role, bounds: Rect, text_content: String, visual_hash: u64) -> Self {
        Self {
            role,
            bounds,
            text_content,
            visual_hash,
            selected: false,
        }
    }

    pub fn with_selected(
        role: Role,
        bounds: Rect,
        text_content: String,
        visual_hash: u64,
        selected: bool,
    ) -> Self {
        Self {
            role,
            bounds,
            text_content,
            visual_hash,
            selected,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Button,
    Tab,
    Input,
    StaticText,
    Panel,
    Checkbox,
    MenuItem,
    Status,
    ToolBlock,
    PromptMarker,
    ProgressBar,
    Link,
    ErrorMessage,
    DiffLine,
    CodeBlock,
}

impl Role {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            Role::Button
                | Role::Tab
                | Role::Input
                | Role::Checkbox
                | Role::MenuItem
                | Role::PromptMarker
                | Role::Link
        )
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Button => write!(f, "button"),
            Role::Tab => write!(f, "tab"),
            Role::Input => write!(f, "input"),
            Role::StaticText => write!(f, "text"),
            Role::Panel => write!(f, "panel"),
            Role::Checkbox => write!(f, "checkbox"),
            Role::MenuItem => write!(f, "menuitem"),
            Role::Status => write!(f, "status"),
            Role::ToolBlock => write!(f, "toolblock"),
            Role::PromptMarker => write!(f, "prompt"),
            Role::ProgressBar => write!(f, "progressbar"),
            Role::Link => write!(f, "link"),
            Role::ErrorMessage => write!(f, "error"),
            Role::DiffLine => write!(f, "diff"),
            Role::CodeBlock => write!(f, "codeblock"),
        }
    }
}

pub use classifier::ClassifyOptions;
pub use classifier::classify;
pub use segmentation::segment_buffer;

pub fn analyze(buffer: &impl ScreenGrid, cursor: &super::CursorPosition) -> Vec<Component> {
    let clusters = segment_buffer(buffer);
    classify(clusters, cursor, &ClassifyOptions::default())
}

pub fn hash_cluster(cluster: &Cluster) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    cluster.rect.hash(&mut hasher);
    cluster.text.hash(&mut hasher);
    cluster.style.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 5, 20, 10);
        assert!(rect.contains(10, 5));
        assert!(rect.contains(15, 8));
        assert!(!rect.contains(30, 5));
        assert!(!rect.contains(10, 15));
        assert!(!rect.contains(5, 5));
    }

    #[test]
    fn test_cluster_extend() {
        let mut cluster = Cluster::new(0, 0, 'H', CellStyle::default());
        cluster.extend('i');
        cluster.seal();
        assert_eq!(cluster.text, "Hi");
        assert_eq!(cluster.rect.width, 2);
        assert!(!cluster.is_whitespace);
    }

    #[test]
    fn test_cluster_whitespace() {
        let mut cluster = Cluster::new(0, 0, ' ', CellStyle::default());
        cluster.extend(' ');
        cluster.seal();
        assert!(cluster.is_whitespace);
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::Button.to_string(), "button");
        assert_eq!(Role::Tab.to_string(), "tab");
        assert_eq!(Role::Input.to_string(), "input");
    }
}
