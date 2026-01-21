//! Core traits for extensible element detection
//!
//! This module defines the trait-based architecture that allows
//! framework-specific detectors to be added without modifying shared code.

use crate::terminal::ScreenBuffer;

use super::pattern::PatternMatch;

/// Context provided to detectors containing screen information
pub struct DetectionContext<'a> {
    /// Raw screen text content
    pub screen_text: &'a str,
    /// Pre-split lines for efficient access
    pub lines: Vec<&'a str>,
}

impl<'a> DetectionContext<'a> {
    /// Create a new detection context from screen content
    pub fn new(screen_text: &'a str, _screen_buffer: Option<&'a ScreenBuffer>) -> Self {
        Self {
            screen_text,
            lines: screen_text.lines().collect(),
        }
    }
}

/// Primary trait for framework-specific element detection
///
/// Implementations detect UI elements using patterns specific to their framework.
/// Each framework can define its own detection logic while sharing the common
/// Element and PatternMatch types.
///
/// Methods beyond `detect_patterns` are used for framework auto-detection
/// and are accessed through the `FrameworkDetector` enum dispatch.
#[allow(dead_code)] // Methods accessed through FrameworkDetector enum delegation
pub trait ElementDetectorImpl: Send + Sync {
    /// Detect pattern matches in the screen content
    ///
    /// Returns a list of pattern matches that will be converted to Elements
    /// by the main ElementDetector.
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch>;

    /// Human-readable framework identifier
    fn framework_name(&self) -> &'static str;

    /// Priority for detection (higher = checked first when auto-detecting framework)
    ///
    /// Default is 0. Framework-specific detectors should return higher values
    /// to be checked before the generic fallback.
    fn priority(&self) -> i32 {
        0
    }

    /// Check if this detector can handle the given screen content
    ///
    /// Used for auto-detection: returns true if this detector's framework
    /// patterns are present in the screen. Default implementation returns true.
    fn can_detect(&self, _ctx: &DetectionContext) -> bool {
        true
    }
}
