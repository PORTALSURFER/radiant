//! Focused public behavior coverage for reusable widget primitives.

use radiant::gui::{
    paint::BorderSides,
    types::{ImageRgba, Point, Rect},
};
use radiant::{
    layout::Vector2,
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, CardWidget, ColorMarkerWidget,
        DragHandleMessage, DragHandleWidget, FeedbackOverlayWidget, ImageWidget,
        InteractiveRowMessage, InteractiveRowWidget, ListItemMessage, ListItemWidget,
        PointerButton, PointerShieldMessage, PointerShieldWidget, ProgressBarMessage,
        ProgressBarWidget, ScrollbarAxis, ScrollbarMessage, ScrollbarWidget, SelectableMessage,
        SelectableWidget, SliderMessage, SliderWidget, TextBackgroundRole, TextColorRole,
        TextInputMessage, TextInputWidget, TextWidget, ToggleMessage, ToggleWidget, Widget,
        WidgetInput, WidgetKey, WidgetSizing,
    },
};
use std::sync::Arc;

#[path = "widgets_primitive_behaviors/value_controls.rs"]
mod value_controls;

#[path = "widgets_primitive_behaviors/intrinsic.rs"]
mod intrinsic;

#[path = "widgets_primitive_behaviors/interaction.rs"]
mod interaction;
