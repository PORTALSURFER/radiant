//! First-class public widget taxonomy and contracts for `radiant`.
//!
//! `radiant::layout` owns container placement. `radiant::widgets` owns the leaf
//! and tightly-bounded composite vocabulary that applications place inside those
//! containers.
//!
//! This module is intentionally additive and design-focused:
//! - it defines the public widget taxonomy
//! - it documents the shared sizing, focus, paint, and message contracts
//! - it lets the generic runtime project reusable widgets into paint data
//!
//! Native runtime adapters can layer on top of this vocabulary without changing
//! the public widget contracts.
//!
//! # Example
//!
//! ```
//! use radiant::{
//!     layout::{
//!         ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams,
//!         Vector2, layout_tree,
//!     },
//!     widgets::{ButtonWidget, TextWidget, WidgetSizing},
//! };
//!
//! let title = TextWidget::new(
//!     10,
//!     "Items",
//!     WidgetSizing::fixed(Vector2::new(80.0, 20.0)).with_baseline(14.0),
//! );
//! let add_button = ButtonWidget::new(
//!     11,
//!     "Import",
//!     WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
//! );
//!
//! let layout = LayoutNode::container(
//!     1,
//!     ContainerPolicy {
//!         kind: ContainerKind::Row,
//!         spacing: 8.0,
//!         ..ContainerPolicy::default()
//!     },
//!     vec![
//!         SlotChild::new(SlotParams::fill(), title.common.layout_node()),
//!         SlotChild::new(SlotParams::fill(), add_button.common.layout_node()),
//!     ],
//! );
//!
//! let output = layout_tree(
//!     &layout,
//!     Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 32.0)),
//! );
//!
//! assert!(output.rects.contains_key(&title.common.id));
//! assert!(output.rects.contains_key(&add_button.common.id));
//! ```

mod contract;
mod interaction;
mod primitives;
mod theme;

pub use contract::{
    FocusBehavior, PaintBounds, PaintContract, WidgetId, WidgetKind, WidgetMessageKind,
    WidgetProminence, WidgetSizing, WidgetState, WidgetStyle, WidgetTone,
};
pub use interaction::{
    BadgeMessage, ButtonMessage, CanvasMessage, DragHandleMessage, ListItemMessage, PointerButton,
    ScrollbarMessage, SelectableMessage, TextEditCommand, TextInputMessage, ToggleMessage,
    WidgetInput, WidgetKey, WidgetOutput,
};
pub use primitives::{
    BadgeProps, BadgeState, BadgeWidget, ButtonProps, ButtonState, ButtonWidget, CanvasWidget,
    CardWidget, DragHandleWidget, ImageProps, ImageWidget, ListItemWidget,
    RetainedSurfaceDescriptor, ScrollbarAxis, ScrollbarProps, ScrollbarState, ScrollbarWidget,
    SelectableProps, SelectableWidget, TextInputProps, TextInputState, TextInputWidget, TextWidget,
    TextWrap, ToggleProps, ToggleState, ToggleWidget, WidgetCommon, WidgetSpec,
};
pub use theme::{WidgetVisualTokens, resolve_widget_visual_tokens};
