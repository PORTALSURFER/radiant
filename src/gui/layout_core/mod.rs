//! Stable slot-based layout primitives for `radiant`.
//!
//! This module exposes a deterministic two-pass measure/layout engine that is
//! independent from the current native shell. Applications describe a layout
//! tree with [`LayoutNode`] values, configure parent-owned slot behavior with
//! [`SlotParams`], and then run [`layout_tree`] or [`LayoutEngine`] to produce
//! a [`LayoutOutput`](crate::gui::layout_core::LayoutOutput).
//!
//! The layout flow is intentionally explicit:
//! - widgets provide intrinsic size hints through [`WidgetNode`]
//! - containers own child placement through [`ContainerPolicy`]
//! - slots express the contract between a parent and one child
//! - the engine measures bottom-up and assigns final rectangles top-down
//!
//! Baseline container policies:
//! - [`ContainerKind::Row`]
//! - [`ContainerKind::Column`]
//! - [`ContainerKind::Stack`]
//! - [`ContainerKind::PaddingBox`]
//! - [`ContainerKind::AlignBox`]
//! - [`ContainerKind::AspectBox`]
//! - [`ContainerKind::Grid`]
//! - [`ContainerKind::ScrollView`]
//! - [`ContainerKind::Wrap`]
//! - [`ContainerKind::SwitchLayout`]
//!
//! # Example
//!
//! ```
//! use radiant::layout::{
//!     ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams, Vector2,
//!     layout_tree,
//! };
//!
//! let root = LayoutNode::container(
//!     1,
//!     ContainerPolicy {
//!         kind: ContainerKind::Row,
//!         spacing: 8.0,
//!         ..ContainerPolicy::default()
//!     },
//!     vec![
//!         SlotChild::new(SlotParams::fill(), LayoutNode::widget(2, Vector2::new(40.0, 20.0))),
//!         SlotChild::new(SlotParams::fill(), LayoutNode::widget(3, Vector2::new(40.0, 20.0))),
//!     ],
//! );
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
pub use constraints::Constraints;
pub use engine::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive, LayoutDiagnostic,
    LayoutDiagnosticCode, LayoutEngine, LayoutOutput, LayoutState, LayoutStats, OverflowInfo,
    VirtualWindowInfo, layout_tree, layout_tree_with_state,
};
pub use model::{
    ContainerKind, ContainerPolicy, CrossAlign, GridPolicy, Insets, MainAlign, OverflowPolicy,
    SizeModeCross, SizeModeMain, SlotParams, SwitchBreakpoint, VirtualizationAxis,
    VirtualizationPolicy, WrapPolicy,
};
pub use row_helpers::{
    fixed_width_row_rects_end, fixed_width_row_rects_start, visible_suffix_widths,
};
pub use tree::{ContainerNode, LayoutNode, NodeId, SlotChild, WidgetNode};
