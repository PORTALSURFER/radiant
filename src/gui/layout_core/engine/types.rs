//! Shared data types for layout output, diagnostics, and debug rendering.

use super::super::model::OverflowPolicy;
use super::super::tree::NodeId;
use crate::gui::types::{Rect, Vector2};
use std::collections::{BTreeMap, BTreeSet};

/// Stable diagnostic category emitted during measure/layout normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDiagnosticCode {
    /// A negative size or coordinate was clamped to a non-negative value.
    NegativeSizeClamped,
    /// A constraint range (`min > max`) was normalized to a valid range.
    ConstraintContradiction,
    /// A requested overflow policy was unsupported and defaulted to a fallback.
    OverflowPolicyDefaulted,
    /// Content overflow was detected for the node.
    OverflowOccurred,
    /// A provided scroll offset was outside legal bounds and clamped.
    InvalidScrollOffsetClamped,
}

/// Layout diagnostic emitted when invalid states are normalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutDiagnostic {
    /// Node that triggered the diagnostic.
    pub node_id: NodeId,
    /// Stable diagnostic category.
    pub code: LayoutDiagnosticCode,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Overflow metadata recorded for one node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverflowInfo {
    /// True when width overflowed.
    pub x: bool,
    /// True when height overflowed.
    pub y: bool,
    /// Policy used when handling overflow.
    pub policy: OverflowPolicy,
}

impl Default for OverflowInfo {
    fn default() -> Self {
        Self {
            x: false,
            y: false,
            policy: OverflowPolicy::Clip,
        }
    }
}

/// Debug primitive kind emitted by the layout debug overlay path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugPrimitiveKind {
    /// Full node rectangle.
    NodeBounds,
    /// Container content rectangle after padding.
    ContentBounds,
    /// Slot margin rectangle around the child rect.
    SlotMargin,
    /// Node overflow marker.
    OverflowMarker,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutDebugOptions {
    /// Master switch for debug primitive recording.
    pub enabled: bool,
    /// Emit node bounds.
    pub show_bounds: bool,
    /// Emit padded content bounds.
    pub show_padding: bool,
    /// Emit slot margin bounds.
    pub show_margins: bool,
    /// Emit overflow markers.
    pub show_overflow: bool,
}

impl Default for LayoutDebugOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            show_bounds: false,
            show_padding: false,
            show_margins: false,
            show_overflow: false,
        }
    }
}

impl LayoutDebugOptions {
    /// Enable all debug primitive categories.
    pub fn all_enabled() -> Self {
        Self {
            enabled: true,
            show_bounds: true,
            show_padding: true,
            show_margins: true,
            show_overflow: true,
        }
    }
}

/// Dynamic layout state supplied by callers for stateful containers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutState {
    /// Per-node scroll offsets used by `ScrollView` containers.
    pub scroll_offsets: BTreeMap<NodeId, Vector2>,
}

impl LayoutState {
    /// Return the configured scroll offset for a node or `(0, 0)` when absent.
    pub fn scroll_offset(&self, node_id: NodeId) -> Vector2 {
        self.scroll_offsets
            .get(&node_id)
            .copied()
            .unwrap_or_default()
    }
}

/// Final layout output from a full measure/layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutOutput {
    /// Final rounded rectangles by node id.
    pub rects: BTreeMap<NodeId, Rect>,
    /// Node ids that overflowed available space.
    pub overflowed: BTreeSet<NodeId>,
    /// Per-node overflow metadata.
    pub overflow_flags: BTreeMap<NodeId, OverflowInfo>,
    /// Diagnostics collected during measure/layout normalization.
    pub diagnostics: Vec<LayoutDiagnostic>,
    /// Optional debug primitives emitted by the traversal.
    pub debug_primitives: Vec<LayoutDebugPrimitive>,
}
