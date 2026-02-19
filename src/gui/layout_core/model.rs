//! Slot and container policy definitions for the slot-based layout engine.

use super::constraints::Constraints;

/// Main-axis sizing mode for a slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SizeModeMain {
    /// Fixed logical pixels.
    Fixed(f32),
    /// Fill remaining space by weight.
    Fill(f32),
    /// Percentage of parent content space.
    Percent(f32),
    /// Resolve from child intrinsic measurement.
    Intrinsic,
}

/// Cross-axis sizing mode for a slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SizeModeCross {
    /// Fixed logical pixels.
    Fixed(f32),
    /// Fill available cross-axis space.
    Fill,
    /// Resolve from child intrinsic measurement.
    Intrinsic,
}

/// Main-axis alignment within a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainAlign {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Cross-axis alignment for children within a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CrossAlign {
    Start,
    Center,
    End,
    Stretch,
}

/// Explicit overflow policy for containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OverflowPolicy {
    Clip,
    Scroll,
    Wrap,
    Shrink,
}

/// Axis-agnostic insets.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Insets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Insets {
    /// Build symmetrical insets for every edge.
    pub(crate) fn all(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Return total horizontal inset.
    pub(crate) fn horizontal(self) -> f32 {
        self.left + self.right
    }

    /// Return total vertical inset.
    pub(crate) fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

/// Parent-owned slot configuration for a single child.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SlotParams {
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
    pub(crate) fn fill() -> Self {
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

/// Container kind used to select a deterministic layout algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContainerKind {
    Row,
    Column,
    Stack,
}

/// Shared policy configuration for container nodes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContainerPolicy {
    /// Layout algorithm kind.
    pub kind: ContainerKind,
    /// Child spacing on the main axis.
    pub spacing: f32,
    /// Container-level content padding.
    pub padding: Insets,
    /// Default main-axis alignment.
    pub align_main: MainAlign,
    /// Default cross-axis alignment.
    pub align_cross: CrossAlign,
    /// Explicit overflow policy.
    pub overflow: OverflowPolicy,
}

impl Default for ContainerPolicy {
    fn default() -> Self {
        Self {
            kind: ContainerKind::Column,
            spacing: 0.0,
            padding: Insets::default(),
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
        }
    }
}
