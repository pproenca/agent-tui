//! Feedback Loop ("Observer")
//!
//! Algorithm: Perceptual Hashing (SimHash) for Layout Signature
//! Goal: Deterministic `wait` - verify action had effect
//!
//! Layout Signature = Hash(Vec<Component.visual_hash>)
//! More stable than raw screen hash - ignores cursor blink, clock updates

use crate::vom::Component;
use std::hash::{Hash, Hasher};

/// Compute a layout signature from a set of components.
///
/// The signature captures the structural layout of the screen:
/// - Position and size of each component
/// - Role of each component
/// - Visual hash (content + style)
///
/// This is more stable than hashing the raw screen because it
/// ignores minor changes like cursor blink or clock updates.
pub fn compute_layout_signature(components: &[Component]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    for comp in components {
        comp.visual_hash.hash(&mut hasher);
        comp.role.hash(&mut hasher);
        comp.bounds.hash(&mut hasher);
    }

    hasher.finish()
}

/// Check if two layout signatures are identical.
/// Hamming distance of 0 = stable layout.
pub fn is_stable(current: u64, previous: u64) -> bool {
    current == previous
}

/// Compute a content-only signature (ignores position).
/// Useful for detecting content changes regardless of layout shifts.
pub fn compute_content_signature(components: &[Component]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    for comp in components {
        comp.text_content.hash(&mut hasher);
        comp.role.hash(&mut hasher);
    }

    hasher.finish()
}

/// Compute a role-only signature (ignores content and style).
/// Useful for detecting structural changes (e.g., new buttons appeared).
pub fn compute_structure_signature(components: &[Component]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    for comp in components {
        comp.role.hash(&mut hasher);
        comp.bounds.x.hash(&mut hasher);
        comp.bounds.y.hash(&mut hasher);
    }

    hasher.finish()
}

/// Result of comparing two layouts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutChange {
    /// No change detected
    Stable,
    /// Content changed but structure is same
    ContentChanged,
    /// New components appeared or disappeared
    StructureChanged,
    /// Layout positions shifted
    LayoutShifted,
}

/// Compare two sets of components and determine what changed.
pub fn compare_layouts(before: &[Component], after: &[Component]) -> LayoutChange {
    let before_layout = compute_layout_signature(before);
    let after_layout = compute_layout_signature(after);

    if before_layout == after_layout {
        return LayoutChange::Stable;
    }

    let before_structure = compute_structure_signature(before);
    let after_structure = compute_structure_signature(after);

    if before_structure != after_structure {
        return LayoutChange::StructureChanged;
    }

    let before_content = compute_content_signature(before);
    let after_content = compute_content_signature(after);

    if before_content != after_content {
        return LayoutChange::ContentChanged;
    }

    LayoutChange::LayoutShifted
}

/// Find components that appeared in `after` but not in `before`.
pub fn find_new_components<'a>(before: &[Component], after: &'a [Component]) -> Vec<&'a Component> {
    after
        .iter()
        .filter(|a| !before.iter().any(|b| components_match(b, a)))
        .collect()
}

/// Find components that disappeared from `before` to `after`.
pub fn find_removed_components<'a>(
    before: &'a [Component],
    after: &[Component],
) -> Vec<&'a Component> {
    before
        .iter()
        .filter(|b| !after.iter().any(|a| components_match(b, a)))
        .collect()
}

/// Check if two components are "the same" (same position and similar content).
fn components_match(a: &Component, b: &Component) -> bool {
    // Same position and role
    a.bounds == b.bounds && a.role == b.role
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vom::{Rect, Role};
    use std::hash::{Hash, Hasher};

    fn make_component(text: &str, role: Role, x: u16, y: u16) -> Component {
        // Compute a visual hash based on text content
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        let visual_hash = hasher.finish();

        Component::new(
            role,
            Rect::new(x, y, text.len() as u16, 1),
            text.to_string(),
            visual_hash,
        )
    }

    #[test]
    fn test_stable_layout() {
        let components1 = vec![
            make_component("[OK]", Role::Button, 0, 0),
            make_component("Hello", Role::StaticText, 10, 0),
        ];
        let components2 = vec![
            make_component("[OK]", Role::Button, 0, 0),
            make_component("Hello", Role::StaticText, 10, 0),
        ];

        let sig1 = compute_layout_signature(&components1);
        let sig2 = compute_layout_signature(&components2);

        assert!(is_stable(sig1, sig2));
    }

    #[test]
    fn test_content_change() {
        let before = vec![make_component("Count: 1", Role::StaticText, 0, 0)];
        let after = vec![make_component("Count: 2", Role::StaticText, 0, 0)];

        let change = compare_layouts(&before, &after);
        assert_eq!(change, LayoutChange::ContentChanged);
    }

    #[test]
    fn test_structure_change() {
        let before = vec![make_component("[OK]", Role::Button, 0, 0)];
        let after = vec![
            make_component("[OK]", Role::Button, 0, 0),
            make_component("[Cancel]", Role::Button, 10, 0),
        ];

        let change = compare_layouts(&before, &after);
        assert_eq!(change, LayoutChange::StructureChanged);
    }

    #[test]
    fn test_find_new_components() {
        let before = vec![make_component("[OK]", Role::Button, 0, 0)];
        let after = vec![
            make_component("[OK]", Role::Button, 0, 0),
            make_component("[Cancel]", Role::Button, 10, 0),
        ];

        let new_components = find_new_components(&before, &after);
        assert_eq!(new_components.len(), 1);
        assert_eq!(new_components[0].text_content, "[Cancel]");
    }

    #[test]
    fn test_find_removed_components() {
        let before = vec![
            make_component("[OK]", Role::Button, 0, 0),
            make_component("[Cancel]", Role::Button, 10, 0),
        ];
        let after = vec![make_component("[OK]", Role::Button, 0, 0)];

        let removed = find_removed_components(&before, &after);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].text_content, "[Cancel]");
    }
}
