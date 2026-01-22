//! Visual Object Model (VOM) - Deterministic TUI element detection.
//!
//! The VOM treats the terminal as a grid of styled cells (like video memory),
//! using Connected-Component Labeling to identify UI elements based on
//! style transitions rather than regex patterns.
//!
//! Pipeline:
//! 1. Segmentation: Raster scan → Vec<Cluster> (style-homogeneous runs)
//! 2. Classification: Geometric & attribute heuristics → Vec<Component> with Roles
//! 3. Interaction: Mouse injection via ANSI CSI (O(1) teleport)
//! 4. Feedback: Layout signatures for change detection

pub mod classifier;
pub mod feedback;
pub mod interaction;
pub mod segmentation;

use crate::terminal::{CellStyle, ScreenBuffer};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

/// Geometric primitive for the terminal grid
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

    /// Get the center point of the rectangle.
    /// Uses saturating arithmetic to prevent overflow on large coordinates.
    pub fn center(&self) -> (u16, u16) {
        (
            self.x.saturating_add(self.width / 2),
            self.y.saturating_add(self.height / 2),
        )
    }

    /// Check if a point is within this rectangle.
    /// Uses saturating arithmetic to prevent overflow on large coordinates.
    #[allow(dead_code)] // Public API for external consumers
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x
            && x < self.x.saturating_add(self.width)
            && y >= self.y
            && y < self.y.saturating_add(self.height)
    }
}

/// The atomic unit of the VOM.
/// A contiguous sequence of cells on a single row sharing IDENTICAL styling.
#[derive(Debug, Clone)]
pub struct Cluster {
    pub rect: Rect,
    pub text: String,
    pub style: CellStyle,
    /// Heuristic property derived during segmentation
    pub is_whitespace: bool,
}

impl Cluster {
    /// Create a new cluster starting at a position
    pub fn new(x: u16, y: u16, char: char, style: CellStyle) -> Self {
        Self {
            rect: Rect::new(x, y, 1, 1),
            text: char.to_string(),
            style,
            is_whitespace: false,
        }
    }

    /// Extend the cluster with another character (same row, same style)
    pub fn extend(&mut self, char: char) {
        self.text.push(char);
        self.rect.width = self.rect.width.saturating_add(1);
    }

    /// Seal the cluster - finalize is_whitespace
    pub fn seal(&mut self) {
        self.is_whitespace = self.text.trim().is_empty();
    }
}

/// A Semantic UI Object.
/// Composed of one or more Clusters (e.g., a Button is a Cluster with specific background).
#[derive(Debug, Clone)]
pub struct Component {
    pub id: Uuid,
    pub role: Role,
    pub bounds: Rect,
    pub text_content: String,
    /// Structural hash for O(1) change detection
    pub visual_hash: u64,
}

impl Component {
    pub fn new(role: Role, bounds: Rect, text_content: String, visual_hash: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            bounds,
            text_content,
            visual_hash,
        }
    }
}

/// Strict enum for component roles - no stringly-typed roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Clickable element with distinct background
    Button,
    /// Clickable, usually top-row, grouped
    Tab,
    /// Contains cursor or underscores
    Input,
    /// Default - non-interactive text
    StaticText,
    /// Bordered container
    Panel,
    /// Toggle with checked state
    Checkbox,
    /// Menu/list choice
    MenuItem,
}

impl Role {
    /// Returns true if this role represents an interactive element.
    /// Used by both snapshot (for element listing) and click (for ref indexing)
    /// to ensure refs (@e1, @e2, etc.) are consistent across commands.
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            Role::Button | Role::Tab | Role::Input | Role::Checkbox | Role::MenuItem
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
        }
    }
}

// Re-exports
pub use classifier::classify;
pub use segmentation::segment_buffer;

/// Main entry point: buffer → components
/// This is the primary API for VOM analysis.
pub fn analyze(buffer: &ScreenBuffer, cursor_row: u16, cursor_col: u16) -> Vec<Component> {
    let clusters = segment_buffer(buffer);
    classify(clusters, cursor_row, cursor_col)
}

/// Find component by text content (partial match)
pub fn find_by_text<'a>(components: &'a [Component], text: &str) -> Option<&'a Component> {
    components.iter().find(|c| c.text_content.contains(text))
}

/// Find component by exact text content
#[allow(dead_code)] // Public API for external consumers
pub fn find_by_exact_text<'a>(components: &'a [Component], text: &str) -> Option<&'a Component> {
    components
        .iter()
        .find(|c| c.text_content.trim() == text.trim())
}

/// Find all components with a specific role
pub fn find_by_role(components: &[Component], role: Role) -> Vec<&Component> {
    components.iter().filter(|c| c.role == role).collect()
}

/// Find component at a specific position
#[allow(dead_code)] // Public API for external consumers
pub fn find_at_position(components: &[Component], x: u16, y: u16) -> Option<&Component> {
    components.iter().find(|c| c.bounds.contains(x, y))
}

/// Compute a hash for a cluster (used for visual_hash calculation)
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
    fn test_rect_center() {
        let rect = Rect::new(10, 5, 20, 10);
        assert_eq!(rect.center(), (20, 10));
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 5, 20, 10);
        assert!(rect.contains(10, 5)); // Top-left corner
        assert!(rect.contains(15, 8)); // Inside
        assert!(!rect.contains(30, 5)); // Right edge (exclusive)
        assert!(!rect.contains(10, 15)); // Bottom edge (exclusive)
        assert!(!rect.contains(5, 5)); // Outside left
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

    #[test]
    fn test_rect_center_odd_dimensions() {
        // Width 5: center should be at x + 2 (integer division)
        let rect = Rect::new(0, 0, 5, 3);
        assert_eq!(rect.center(), (2, 1)); // 5/2=2, 3/2=1

        // Width 1: center should be at x
        let rect = Rect::new(10, 10, 1, 1);
        assert_eq!(rect.center(), (10, 10));
    }
}
