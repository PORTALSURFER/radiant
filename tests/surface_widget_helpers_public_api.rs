//! Public API coverage for surface widget helper construction and mapping.

use radiant::{
    layout::{Point, Rect, Vector2},
    runtime::{SurfaceChild, SurfaceNode, UiSurface},
    widgets::{
        BadgeMessage, ButtonMessage, CanvasMessage, CanvasWidget, InteractionProvenance,
        ListItemMessage, ListItemWidget, PointerButton, ScrollbarAxis, ScrollbarMessage,
        SelectableMessage, SelectableWidget, TextInputMessage, TextWidget, ToggleMessage, Widget,
        WidgetInput, WidgetSizing,
    },
};

#[path = "surface_widget_helpers_public_api/leaf_helpers.rs"]
mod leaf_helpers;
#[path = "surface_widget_helpers_public_api/surface_helpers.rs"]
mod surface_helpers;
#[path = "surface_widget_helpers_public_api/value_helpers.rs"]
mod value_helpers;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Rename(String),
    SetActive(bool),
    ToggleChanged {
        checked: bool,
        provenance: InteractionProvenance,
    },
    CanvasInput(WidgetInput),
}

fn widget_ref<'a, T, Message>(surface: &'a UiSurface<Message>, id: u64, expected: &str) -> &'a T
where
    T: Widget + 'static,
{
    surface
        .find_widget(id)
        .unwrap_or_else(|| panic!("expected {expected} widget {id} to exist"))
        .widget()
        .as_any()
        .downcast_ref::<T>()
        .unwrap_or_else(|| panic!("expected widget {id} to be {expected}"))
}
