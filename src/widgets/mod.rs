//! First-class public widget contracts for `radiant`.
//!
//! `radiant::layout` owns container placement. `radiant::widgets` owns the leaf
//! and tightly-bounded composite vocabulary that applications place inside those
//! containers.
//!
//! This module is intentionally additive and design-focused:
//! - it documents the shared sizing, focus, and paint contracts
//! - it lets the generic runtime project reusable widgets into paint data
//! - it keeps widget variation in concrete widget implementations
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
//!     widgets::{ButtonWidget, ButtonWidgetParts, TextWidget, TextWidgetParts, WidgetSizing},
//! };
//!
//! let title = TextWidget::from_parts(TextWidgetParts {
//!     id: 10,
//!     text: "Items".into(),
//!     sizing: WidgetSizing::fixed(Vector2::new(80.0, 20.0)).with_baseline(14.0),
//! });
//! let add_button = ButtonWidget::from_parts(ButtonWidgetParts {
//!     id: 11,
//!     label: "Import".into(),
//!     sizing: WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
//! });
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
    FocusBehavior, PaintBounds, PaintContract, Widget, WidgetId, WidgetProminence, WidgetSizing,
    WidgetSizingParts, WidgetState, WidgetStyle, WidgetTone,
};
pub use interaction::{
    BadgeMessage, ButtonMessage, CanvasGestureEvent, CanvasGestureState, CanvasMessage,
    CanvasPointer, CustomWidgetOutput, DragHandleMessage, GpuSurfaceMessage, InteractiveRowMessage,
    ListItemMessage, PointerButton, PointerModifiers, ScrollbarMessage, SelectableMessage,
    SliderMessage, TextEditCommand, TextInputMessage, ToggleMessage, WidgetInput, WidgetKey,
    WidgetOutput,
};
pub use primitives::{
    BadgeProps, BadgeState, BadgeWidget, BadgeWidgetParts, ButtonProps, ButtonState, ButtonWidget,
    ButtonWidgetParts, CanvasWidget, CanvasWidgetParts, CardWidget, CardWidgetParts,
    DragHandleWidget, DragHandleWidgetParts, GpuSurfaceParts, GpuSurfaceWidget, IconButtonWidget,
    IconButtonWidgetParts, ImageProps, ImageWidget, ImageWidgetParts, InteractiveRowProps,
    InteractiveRowWidget, InteractiveRowWidgetParts, ListItemWidget, ListItemWidgetParts,
    RetainedSurfaceDescriptor, ScrollbarAxis, ScrollbarProps, ScrollbarState, ScrollbarWidget,
    ScrollbarWidgetParts, SelectableProps, SelectableWidget, SelectableWidgetParts, SliderProps,
    SliderState, SliderWidget, SliderWidgetParts, TextAlign, TextInputEditResult, TextInputProps,
    TextInputState, TextInputWidget, TextInputWidgetParts, TextWidget, TextWidgetParts, TextWrap,
    ToggleProps, ToggleState, ToggleWidget, ToggleWidgetParts, WidgetCommon,
};
pub use theme::{WidgetVisualTokens, resolve_widget_visual_tokens};
