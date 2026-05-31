//! Public primitive widget descriptors for `radiant::widgets`.

mod badge;
mod button;
mod canvas;
mod card;
mod color_marker;
mod drag_handle;
mod feedback_overlay;
mod gpu_surface;
mod icon_button;
mod image;
mod interactive_row;
mod list_item;
mod marker_run;
mod pointer_shield;
mod progress_bar;
mod scrollbar;
mod selectable;
mod slider;
mod support;
mod text;
mod text_input;
mod toggle;

pub use badge::{BadgeProps, BadgeState, BadgeWidget, BadgeWidgetParts};
pub use button::{ButtonProps, ButtonState, ButtonWidget, ButtonWidgetParts};
pub use canvas::{CanvasWidget, CanvasWidgetParts, RetainedSurfaceDescriptor};
pub use card::{CardWidget, CardWidgetParts};
pub use color_marker::{
    ColorMarkerAlign, ColorMarkerProps, ColorMarkerWidget, ColorMarkerWidgetParts,
};
pub use drag_handle::{DragHandleWidget, DragHandleWidgetParts};
pub use feedback_overlay::{
    FeedbackOverlayEdge, FeedbackOverlayProgress, FeedbackOverlayProps, FeedbackOverlayWidget,
    FeedbackOverlayWidgetParts,
};
pub use gpu_surface::{GpuSurfaceParts, GpuSurfaceWidget};
pub use icon_button::{IconButtonWidget, IconButtonWidgetParts};
pub use image::{ImageProps, ImageWidget, ImageWidgetParts};
pub use interactive_row::{
    InteractiveRowPointerMotion, InteractiveRowProps, InteractiveRowVisualStateParts,
    InteractiveRowWidget, InteractiveRowWidgetParts,
};
pub use list_item::{ListItemWidget, ListItemWidgetParts};
pub use marker_run::{MarkerRunAlign, MarkerRunProps, MarkerRunWidget, MarkerRunWidgetParts};
pub use pointer_shield::{PointerShieldProps, PointerShieldWidget, PointerShieldWidgetParts};
pub use progress_bar::{
    ProgressBarMessage, ProgressBarMode, ProgressBarProps, ProgressBarWidget,
    ProgressBarWidgetParts,
};
pub use scrollbar::{
    ScrollbarAxis, ScrollbarProps, ScrollbarState, ScrollbarWidget, ScrollbarWidgetParts,
};
pub use selectable::{SelectableProps, SelectableWidget, SelectableWidgetParts};
pub use slider::{SliderProps, SliderState, SliderWidget, SliderWidgetParts};
pub use support::WidgetCommon;
pub use text::{
    TextAlign, TextBackgroundRole, TextColorRole, TextWidget, TextWidgetParts, TextWrap,
};
pub use text_input::{
    TextInputChrome, TextInputEditResult, TextInputProps, TextInputState, TextInputWidget,
    TextInputWidgetParts,
};
pub use toggle::{ToggleProps, ToggleState, ToggleWidget, ToggleWidgetParts};
