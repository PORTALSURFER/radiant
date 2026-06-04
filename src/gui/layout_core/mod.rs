//! Stable slot-based layout primitives for `radiant`.
//!
//! This module exposes a deterministic two-pass measure/layout engine that is
//! independent from the current native shell. Applications describe a layout
//! tree with [`LayoutNode`](crate::layout::LayoutNode) values, configure parent-owned slot behavior with
//! [`SlotParams`](crate::layout::SlotParams), and then run
//! [`layout_tree`](crate::layout::layout_tree) or [`LayoutEngine`](crate::layout::LayoutEngine) to produce
//! a [`LayoutOutput`](crate::gui::layout_core::LayoutOutput).
//!
//! The layout flow is intentionally explicit:
//! - widgets provide intrinsic size hints through [`WidgetNode`](crate::layout::WidgetNode)
//! - containers own child placement through [`ContainerPolicy`](crate::layout::ContainerPolicy)
//! - slots express the contract between a parent and one child
//! - the engine measures bottom-up and assigns final rectangles top-down
//!
//! Baseline container policies:
//! - [`ContainerKind::Row`](crate::layout::ContainerKind::Row)
//! - [`ContainerKind::Column`](crate::layout::ContainerKind::Column)
//! - [`ContainerKind::Stack`](crate::layout::ContainerKind::Stack)
//! - [`ContainerKind::PaddingBox`](crate::layout::ContainerKind::PaddingBox)
//! - [`ContainerKind::AlignBox`](crate::layout::ContainerKind::AlignBox)
//! - [`ContainerKind::AspectBox`](crate::layout::ContainerKind::AspectBox)
//! - [`ContainerKind::Grid`](crate::layout::ContainerKind::Grid)
//! - [`ContainerKind::ScrollView`](crate::layout::ContainerKind::ScrollView)
//! - [`ContainerKind::Wrap`](crate::layout::ContainerKind::Wrap)
//! - [`ContainerKind::SwitchLayout`](crate::layout::ContainerKind::SwitchLayout)
//! - [`ContainerKind::FloatingLayer`](crate::layout::ContainerKind::FloatingLayer)
//!
//! # Example
//!
//! ```
//! use radiant::layout::{
//!     ContainerKind, ContainerNodeParts, ContainerPolicy, LayoutNode, Point, Rect, SlotChild,
//!     SlotChildParts, SlotParams, Vector2, WidgetNodeParts, layout_tree,
//! };
//!
//! let root = LayoutNode::container_from_parts(ContainerNodeParts {
//!     id: 1,
//!     policy: ContainerPolicy {
//!         kind: ContainerKind::Row,
//!         spacing: 8.0,
//!         ..ContainerPolicy::default()
//!     },
//!     children: vec![
//!         SlotChild::from_parts(SlotChildParts {
//!             slot: SlotParams::fill(),
//!             child: LayoutNode::widget_from_parts(WidgetNodeParts {
//!                 id: 2,
//!                 intrinsic: Vector2::new(40.0, 20.0),
//!             }),
//!         }),
//!         SlotChild::from_parts(SlotChildParts {
//!             slot: SlotParams::fill(),
//!             child: LayoutNode::widget_from_parts(WidgetNodeParts {
//!                 id: 3,
//!                 intrinsic: Vector2::new(40.0, 20.0),
//!             }),
//!         }),
//!     ],
//! });
//!
//! let output = layout_tree(
//!     &root,
//!     Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 40.0)),
//! );
//!
//! assert!(output.rects.contains_key(&2));
//! assert!(output.rects.contains_key(&3));
//! ```

mod constraints;
mod engine;
mod model;
mod row_helpers;
mod tree;

pub use crate::gui::types::{Point, Rect, Vector2};
pub use constraints::{Constraints, ConstraintsParts};
pub use engine::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive, LayoutDiagnostic,
    LayoutDiagnosticCode, LayoutEngine, LayoutOutput, LayoutState, LayoutStats, OverflowInfo,
    VirtualWindowInfo, layout_tree, layout_tree_with_state,
};
pub use model::{
    ContainerKind, ContainerPolicy, CrossAlign, FloatingLayerPolicy, GridPolicy, Insets, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotParams, SwitchBreakpoint, VirtualizationAxis,
    VirtualizationPolicy, WrapPolicy,
};
pub use row_helpers::{
    StackedLayoutCursor, StackedLayoutItem, StackedRowRectsParts, fixed_width_group_width,
    fixed_width_item_extent_for_available_width, fixed_width_row_rects_end,
    fixed_width_row_rects_end_into, fixed_width_row_rects_start, fixed_width_row_rects_start_into,
    grouped_fixed_width_row_width, stacked_row_rects, stacked_row_rects_from_parts,
    stacked_row_rects_into, stacked_row_rects_into_from_parts, visible_suffix_widths,
    visible_suffix_widths_into,
};
pub use tree::{
    ContainerNode, ContainerNodeParts, LayoutNode, NodeId, SlotChild, SlotChildParts, WidgetNode,
    WidgetNodeParts,
};
