use crate::vom::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MouseButton {
    Left = 0,
    Middle = 1,
    Right = 2,
    ScrollUp = 64,
    ScrollDown = 65,
}

impl MouseButton {
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

pub fn mouse_click_sequence(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    let mut seq = Vec::new();

    let x = x + 1;
    let y = y + 1;
    let btn = button.code();

    let press = format!("\x1b[<{};{};{}M", btn, x, y);
    seq.extend_from_slice(press.as_bytes());

    let release = format!("\x1b[<{};{};{}m", btn, x, y);
    seq.extend_from_slice(release.as_bytes());

    seq
}

#[allow(dead_code)]
pub fn mouse_press_sequence(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    let x = x + 1;
    let y = y + 1;
    let btn = button.code();

    format!("\x1b[<{};{};{}M", btn, x, y).into_bytes()
}

#[allow(dead_code)]
pub fn mouse_release_sequence(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    let x = x + 1;
    let y = y + 1;
    let btn = button.code();

    format!("\x1b[<{};{};{}m", btn, x, y).into_bytes()
}

#[allow(dead_code)]
pub fn mouse_move_sequence(x: u16, y: u16) -> Vec<u8> {
    let x = x + 1;
    let y = y + 1;

    format!("\x1b[<35;{};{}M", x, y).into_bytes()
}

#[allow(dead_code)]
pub fn scroll_sequence(direction: MouseButton, x: u16, y: u16) -> Vec<u8> {
    mouse_press_sequence(direction, x, y)
}

pub fn click_component(component: &Component, button: MouseButton) -> Vec<u8> {
    let (cx, cy) = component.bounds.center();
    mouse_click_sequence(button, cx, cy)
}

pub fn click_at(x: u16, y: u16, button: MouseButton) -> Vec<u8> {
    mouse_click_sequence(button, x, y)
}

#[allow(dead_code)]
pub fn double_click_sequence(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    let mut seq = mouse_click_sequence(button, x, y);
    seq.extend(mouse_click_sequence(button, x, y));
    seq
}

#[allow(dead_code)]
pub mod legacy {
    use super::MouseButton;

    pub fn mouse_click_x10(button: MouseButton, x: u16, y: u16) -> Option<Vec<u8>> {
        if x > 222 || y > 222 {
            return None;
        }

        let btn = button.code() + 32;
        let x_byte = (x as u8) + 33;
        let y_byte = (y as u8) + 33;

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
        let expected_press = "\x1b[<0;11;6M";
        let expected_release = "\x1b[<0;11;6m";

        let seq_str = String::from_utf8_lossy(&seq);
        assert!(seq_str.contains(expected_press));
        assert!(seq_str.contains(expected_release));
    }

    #[test]
    fn test_click_component() {
        let component = Component::new(
            crate::vom::Role::Button,
            Rect::new(10, 5, 20, 1),
            "[Submit]".to_string(),
            12345,
        );

        let seq = click_component(&component, MouseButton::Left);
        let seq_str = String::from_utf8_lossy(&seq);

        assert!(seq_str.contains("\x1b[<0;21;6M"));
        assert!(seq_str.contains("\x1b[<0;21;6m"));
    }

    #[test]
    fn test_right_click() {
        let seq = mouse_click_sequence(MouseButton::Right, 0, 0);
        let seq_str = String::from_utf8_lossy(&seq);
        assert!(seq_str.contains("\x1b[<2;1;1M"));
    }

    #[test]
    fn test_scroll() {
        let seq = scroll_sequence(MouseButton::ScrollUp, 10, 10);
        let seq_str = String::from_utf8_lossy(&seq);
        assert!(seq_str.contains("\x1b[<64;11;11M"));
    }

    #[test]
    fn test_legacy_x10() {
        let seq = legacy::mouse_click_x10(MouseButton::Left, 10, 5).unwrap();
        assert_eq!(seq[0], 0x1b);
        assert_eq!(seq[1], b'[');
        assert_eq!(seq[2], b'M');
        assert_eq!(seq[3], 32);
        assert_eq!(seq[4], 43);
        assert_eq!(seq[5], 38);
    }

    #[test]
    fn test_legacy_x10_out_of_range() {
        let seq = legacy::mouse_click_x10(MouseButton::Left, 300, 5);
        assert!(seq.is_none());
    }

    #[test]
    fn test_mouse_click_at_origin() {
        let seq = mouse_click_sequence(MouseButton::Left, 0, 0);
        let seq_str = String::from_utf8_lossy(&seq);

        assert!(seq_str.contains("\x1b[<0;1;1M"));
        assert!(seq_str.contains("\x1b[<0;1;1m"));
    }

    #[test]
    fn test_legacy_x10_at_boundary() {
        let seq = legacy::mouse_click_x10(MouseButton::Left, 222, 222);
        assert!(seq.is_some());
        let bytes = seq.unwrap();
        assert_eq!(bytes[4], 255);
        assert_eq!(bytes[5], 255);

        assert!(legacy::mouse_click_x10(MouseButton::Left, 223, 0).is_none());
        assert!(legacy::mouse_click_x10(MouseButton::Left, 0, 223).is_none());
    }
}
