//! Shared data types for layout output, diagnostics, and debug rendering.

use super::super::model::{MainAlign, OverflowPolicy};
use super::super::tree::NodeId;
use crate::gui::types::{Rect, Vector2};
use std::borrow::Cow;
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
    /// A virtualization policy was ignored because it could not be applied.
    VirtualizationPolicyIgnored,
    /// A computed virtualization window was clamped to legal bounds.
    VirtualizationWindowClamped,
    /// Virtualization fell back because alignment-resolved windows were invalid.
    VirtualizationAlignmentFallback,
    /// Virtualization fell back because span resolution could not be trusted.
    VirtualizationSpanResolutionFallback,
}

/// Layout diagnostic emitted when invalid states are normalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutDiagnostic {
    /// Node that triggered the diagnostic.
    pub node_id: NodeId,
    /// Stable diagnostic category.
    pub code: LayoutDiagnosticCode,
    /// Human-readable diagnostic message. Static engine diagnostics are
    /// borrowed to keep normal layout passes allocation-lean.
    pub message: Cow<'static, str>,
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
    /// Virtualization window metadata keyed by scroll container id.
    pub virtual_windows: BTreeMap<NodeId, VirtualWindowInfo>,
    /// Traversal counters collected during this layout pass.
    pub stats: LayoutStats,
}

impl LayoutOutput {
    /// Return one resolved node rectangle or the caller-provided fallback.
    pub fn rect_for(&self, node_id: NodeId, fallback: Rect) -> Rect {
        self.rects.get(&node_id).copied().unwrap_or(fallback)
    }

    /// Return one resolved node rectangle clamped inside `bounds`.
    pub fn rect_for_clamped(&self, node_id: NodeId, fallback: Rect, bounds: Rect) -> Rect {
        self.rect_for(node_id, fallback).clamp_to(bounds)
    }
}

/// Virtualized window metadata for a scroll container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualWindowInfo {
    /// Total children available in the virtualized content list.
    pub total_children: usize,
    /// First materialized child index.
    pub first_index: usize,
    /// Exclusive end index of materialized children.
    pub last_index_exclusive: usize,
    /// Number of children culled before the window.
    pub culled_before: usize,
    /// Number of children culled after the window.
    pub culled_after: usize,
    /// Viewport start on the virtualization axis.
    pub viewport_main_start: f32,
    /// Viewport end on the virtualization axis.
    pub viewport_main_end: f32,
    /// Window start on the virtualization axis.
    pub window_main_start: f32,
    /// Window end on the virtualization axis.
    pub window_main_end: f32,
    /// Total resolved main-axis extent for the content container.
    pub resolved_total_main: f32,
    /// Resolved main-axis alignment mode.
    pub alignment_mode: MainAlign,
}

impl Default for VirtualWindowInfo {
    fn default() -> Self {
        Self {
            total_children: 0,
            first_index: 0,
            last_index_exclusive: 0,
            culled_before: 0,
            culled_after: 0,
            viewport_main_start: 0.0,
            viewport_main_end: 0.0,
            window_main_start: 0.0,
            window_main_end: 0.0,
            resolved_total_main: 0.0,
            alignment_mode: MainAlign::Start,
        }
    }
}

/// Collected traversal counters for one layout evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayoutStats {
    /// Number of nodes measured with a cache miss.
    pub measured_nodes: usize,
    /// Number of nodes visited by the layout traversal.
    pub laid_out_nodes: usize,
    /// Number of nodes materialized into `LayoutOutput::rects`.
    pub materialized_nodes: usize,
}
