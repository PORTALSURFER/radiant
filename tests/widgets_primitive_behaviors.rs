//! Focused public behavior coverage for reusable widget primitives.

use radiant::gui::types::{ImageRgba, Point, Rect};
use radiant::{
    layout::Vector2,
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, CardWidget, DragHandleMessage,
        DragHandleWidget, ImageWidget, InteractiveRowMessage, InteractiveRowWidget,
        ListItemMessage, ListItemWidget, PointerButton, ScrollbarAxis, ScrollbarMessage,
        ScrollbarWidget, SelectableMessage, SelectableWidget, SliderMessage, SliderWidget,
        TextInputMessage, TextInputWidget, ToggleMessage, ToggleWidget, Widget, WidgetInput,
        WidgetKey, WidgetSizing,
    },
};
use std::sync::Arc;

#[path = "widgets_primitive_behaviors/value_controls.rs"]
mod value_controls;

#[path = "widgets_primitive_behaviors/intrinsic.rs"]
mod intrinsic;

#[path = "widgets_primitive_behaviors/interaction.rs"]
mod interaction;
