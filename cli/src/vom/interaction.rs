//! Interaction Module ("Effector")
//!
//! Provides O(1) mouse injection using ANSI CSI sequences.
//! Instead of Tab-navigating through elements, we teleport directly
//! to the target coordinates.
//!
//! Supports SGR 1006 format mouse events (most modern terminals).

use crate::vom::Component;

/// Mouse button identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left = 0,
    Middle = 1,
    Right = 2,
    /// Scroll up (button 4 in X11, encoded as 64 in SGR)
    ScrollUp = 64,
    /// Scroll down (button 5 in X11, encoded as 65 in SGR)
    ScrollDown = 65,
}

impl MouseButton {
    /// Get the button code for SGR 1006 format
    pub fn code(&self) -> u8 {
        match self {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            MouseButton::ScrollUp => 64,
            MouseButton::ScrollDown => 65,
        }
    }
}

/// Generate a mouse click sequence (press + release) in SGR 1006 format.
///
/// SGR 1006 is the modern mouse encoding format that uses:
/// - CSI < button ; x ; y M for press
/// - CSI < button ; x ; y m for release
///
/// # Arguments
/// - `button`: The mouse button to click
/// - `x`: Column position (0-indexed, will be converted to 1-indexed)
/// - `y`: Row position (0-indexed, will be converted to 1-indexed)
///
/// # Returns
/// A byte vector containing both press and release sequences
pub fn mouse_click_sequence(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    let mut seq = Vec::new();

    // Terminal coordinates are 1-based
    let x = x + 1;
    let y = y + 1;
    let btn = button.code();

    // Press event: CSI < button ; x ; y M
    let press = format!("\x1b[<{};{};{}M", btn, x, y);
    seq.extend_from_slice(press.as_bytes());

    // Release event: CSI < button ; x ; y m
    // CRUCIAL: Many apps wait for release to trigger action
    let release = format!("\x1b[<{};{};{}m", btn, x, y);
    seq.extend_from_slice(release.as_bytes());

    seq
}

/// Generate a mouse press sequence (no release) in SGR 1006 format.
pub fn mouse_press_sequence(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    let x = x + 1;
    let y = y + 1;
    let btn = button.code();

    format!("\x1b[<{};{};{}M", btn, x, y).into_bytes()
}

/// Generate a mouse release sequence in SGR 1006 format.
pub fn mouse_release_sequence(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    let x = x + 1;
    let y = y + 1;
    let btn = button.code();

    format!("\x1b[<{};{};{}m", btn, x, y).into_bytes()
}

/// Generate a mouse move sequence (drag or hover tracking).
/// Uses button code 35 (32 + 3) for motion with no button pressed.
pub fn mouse_move_sequence(x: u16, y: u16) -> Vec<u8> {
    let x = x + 1;
    let y = y + 1;

    // Motion event: button code 35 (32 + 3 = motion with no button)
    format!("\x1b[<35;{};{}M", x, y).into_bytes()
}

/// Generate a scroll sequence.
pub fn scroll_sequence(direction: MouseButton, x: u16, y: u16) -> Vec<u8> {
    // Scroll events are just button press, no release
    mouse_press_sequence(direction, x, y)
}

/// Click a component by generating mouse sequences for its center.
///
/// # Arguments
/// - `component`: The component to click
/// - `button`: Which mouse button to use (default: Left)
///
/// # Returns
/// Byte sequence to inject into the PTY
pub fn click_component(component: &Component, button: MouseButton) -> Vec<u8> {
    let (cx, cy) = component.bounds.center();
    mouse_click_sequence(button, cx, cy)
}

/// Click at specific coordinates.
pub fn click_at(x: u16, y: u16, button: MouseButton) -> Vec<u8> {
    mouse_click_sequence(button, x, y)
}

/// Generate a double-click sequence.
/// This is simply two click sequences in rapid succession.
pub fn double_click_sequence(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    let mut seq = mouse_click_sequence(button, x, y);
    seq.extend(mouse_click_sequence(button, x, y));
    seq
}

/// Legacy mouse encoding (X10 format) for terminals that don't support SGR.
/// Encodes position in bytes, limiting to 223 columns/rows.
pub mod legacy {
    use super::MouseButton;

    /// Generate X10 format mouse click (limited to 223x223 grid)
    pub fn mouse_click_x10(button: MouseButton, x: u16, y: u16) -> Option<Vec<u8>> {
        // X10 format can only encode positions up to 223
        if x > 222 || y > 222 {
            return None;
        }

        let btn = button.code() + 32; // X10 adds 32 to button
        let x_byte = (x as u8) + 33; // X10 adds 33 to position
        let y_byte = (y as u8) + 33;

        // CSI M button x y (raw bytes)
        Some(vec![0x1b, b'[', b'M', btn, x_byte, y_byte])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vom::Rect;

    #[test]
    fn test_mouse_click_sequence() {
        let seq = mouse_click_sequence(MouseButton::Left, 10, 5);
        let expected_press = "\x1b[<0;11;6M"; // 10+1=11, 5+1=6
        let expected_release = "\x1b[<0;11;6m";

        let seq_str = String::from_utf8_lossy(&seq);
        assert!(seq_str.contains(expected_press));
        assert!(seq_str.contains(expected_release));
    }

    #[test]
    fn test_click_component() {
        let component = Component::new(
            crate::vom::Role::Button,
            Rect::new(10, 5, 20, 1), // center = (20, 5)
            "[Submit]".to_string(),
            12345,
        );

        let seq = click_component(&component, MouseButton::Left);
        let seq_str = String::from_utf8_lossy(&seq);

        // Center is (10 + 20/2, 5 + 1/2) = (20, 5)
        // 1-indexed: (21, 6)
        assert!(seq_str.contains("\x1b[<0;21;6M"));
        assert!(seq_str.contains("\x1b[<0;21;6m"));
    }

    #[test]
    fn test_right_click() {
        let seq = mouse_click_sequence(MouseButton::Right, 0, 0);
        let seq_str = String::from_utf8_lossy(&seq);
        assert!(seq_str.contains("\x1b[<2;1;1M")); // Button 2 = right
    }

    #[test]
    fn test_scroll() {
        let seq = scroll_sequence(MouseButton::ScrollUp, 10, 10);
        let seq_str = String::from_utf8_lossy(&seq);
        assert!(seq_str.contains("\x1b[<64;11;11M")); // 64 = scroll up
    }

    #[test]
    fn test_legacy_x10() {
        let seq = legacy::mouse_click_x10(MouseButton::Left, 10, 5).unwrap();
        assert_eq!(seq[0], 0x1b);
        assert_eq!(seq[1], b'[');
        assert_eq!(seq[2], b'M');
        assert_eq!(seq[3], 32); // 0 + 32
        assert_eq!(seq[4], 43); // 10 + 33
        assert_eq!(seq[5], 38); // 5 + 33
    }

    #[test]
    fn test_legacy_x10_out_of_range() {
        let seq = legacy::mouse_click_x10(MouseButton::Left, 300, 5);
        assert!(seq.is_none());
    }
}
