//! Core types and Visual Object Model (VOM) for agent-tui.
//!
//! This crate provides the element detection system that identifies UI components
//! (buttons, inputs, tabs, etc.) in terminal screens using Connected-Component Labeling.

#![deny(clippy::all)]

mod element;
pub mod vom;

#[cfg(test)]
pub mod test_fixtures;

pub use element::Element;
pub use element::ElementType;
pub use element::Position;
pub use element::component_to_element;
pub use element::detect_checkbox_state;
pub use element::find_element_by_ref;
pub use vom::Cluster;
pub use vom::Component;
pub use vom::Rect;
pub use vom::Role;
pub use vom::analyze;
pub use vom::classify;
pub use vom::hash_cluster;
pub use vom::segment_buffer;
