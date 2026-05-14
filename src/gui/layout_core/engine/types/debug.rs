use super::super::super::tree::NodeId;
use crate::gui::types::Rect;

/// Debug primitive kind emitted by the layout debug overlay path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugPrimitiveKind {
    /// Full node rectangle.
    NodeBounds,
    /// Node measured size anchored at the final layout origin.
    MeasuredBounds,
    /// Container content rectangle after padding.
    ContentBounds,
    /// Slot margin rectangle around the child rect.
    SlotMargin,
    /// Node overflow marker.
    OverflowMarker,
    /// Scroll viewport bounds.
    ViewportBounds,
    /// Active virtualization window bounds.
    VirtualWindowBounds,
    /// Culled area outside the active window.
    CulledRegion,
}

/// One debug primitive emitted during layout traversal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutDebugPrimitive {
    /// Node associated with this primitive.
    pub node_id: NodeId,
    /// Primitive kind.
    pub kind: DebugPrimitiveKind,
    /// Primitive rectangle.
    pub rect: Rect,
}

/// Debug primitive emission switches for layout traversal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayoutDebugOptions {
    /// Master switch for debug primitive recording.
    pub enabled: bool,
    /// Emit node bounds.
    pub show_bounds: bool,
    /// Emit measured bounds anchored at final node origins.
    pub show_measured: bool,
    /// Emit padded content bounds.
    pub show_padding: bool,
    /// Emit slot margin bounds.
    pub show_margins: bool,
    /// Emit overflow markers.
    pub show_overflow: bool,
}

impl LayoutDebugOptions {
    /// Enable node-boundary debug primitives only.
    pub fn bounds_only() -> Self {
        Self {
            enabled: true,
            show_bounds: true,
            show_measured: false,
            show_padding: false,
            show_margins: false,
            show_overflow: false,
        }
    }

    /// Enable all debug primitive categories.
    pub fn all_enabled() -> Self {
        Self {
            enabled: true,
            show_bounds: true,
            show_measured: true,
            show_padding: true,
            show_margins: true,
            show_overflow: true,
        }
    }
}
