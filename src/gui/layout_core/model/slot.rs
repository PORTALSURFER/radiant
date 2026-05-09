//! Parent-owned child slot policy values.

use super::{CrossAlign, SizeModeCross, SizeModeMain};
use crate::gui::layout_core::constraints::Constraints;

/// Axis-agnostic insets.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    /// Left edge inset in logical pixels.
    pub left: f32,
    /// Right edge inset in logical pixels.
    pub right: f32,
    /// Top edge inset in logical pixels.
    pub top: f32,
    /// Bottom edge inset in logical pixels.
    pub bottom: f32,
}

impl Insets {
    /// Build symmetrical insets for every edge.
    pub fn all(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Return total horizontal inset.
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    /// Return total vertical inset.
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

/// Parent-owned slot configuration for a single child.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotParams {
    /// Main-axis sizing mode.
    pub size_main: SizeModeMain,
    /// Cross-axis sizing mode.
    pub size_cross: SizeModeCross,
    /// Child constraints applied by this slot.
    pub constraints: Constraints,
    /// Slot margins around the child.
    pub margin: Insets,
    /// Optional per-slot cross-axis alignment override.
    pub align_cross_override: Option<CrossAlign>,
    /// Whether fixed-size slots can be compressed under hard overflow.
    pub allow_fixed_compress: bool,
}

impl SlotParams {
    /// Create a fill/fill slot with unconstrained limits and zero margin.
    pub fn fill() -> Self {
        Self {
            size_main: SizeModeMain::Fill(1.0),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::unconstrained(),
            margin: Insets::default(),
            align_cross_override: None,
            allow_fixed_compress: false,
        }
    }
}
