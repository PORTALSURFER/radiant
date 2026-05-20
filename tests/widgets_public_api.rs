//! Public API coverage for `radiant::widgets`.

use radiant::{
    gui::{svg::SvgIcon, types::ImageRgba},
    layout::{
        ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams, Vector2,
        layout_tree,
    },
    widgets::{
        BadgeWidget, BadgeWidgetParts, ButtonWidget, ButtonWidgetParts, CanvasWidget,
        CanvasWidgetParts, CardWidget, CardWidgetParts, DragHandleWidget, DragHandleWidgetParts,
        IconButtonWidget, IconButtonWidgetParts, ImageWidget, ImageWidgetParts,
        InteractiveRowWidget, InteractiveRowWidgetParts, ListItemWidget, ListItemWidgetParts,
        ScrollbarAxis, ScrollbarWidget, ScrollbarWidgetParts, SelectableWidget,
        SelectableWidgetParts, SliderWidget, SliderWidgetParts, TextInputWidget,
        TextInputWidgetParts, TextWidget, TextWidgetParts, ToggleWidget, ToggleWidgetParts, Widget,
        WidgetInput, WidgetKey, WidgetOutput, WidgetSizing, WidgetSizingParts,
    },
};
use std::{fmt::Debug, sync::Arc};

#[path = "widgets_public_api/composition.rs"]
mod composition;
#[path = "widgets_public_api/construction.rs"]
mod construction;
#[path = "widgets_public_api/dispatch.rs"]
mod dispatch;

fn assert_typed_widget_output<T>(output: Option<WidgetOutput>, expected: T)
where
    T: Debug + PartialEq + 'static,
{
    let output = output.expect("widget should emit output");
    assert_eq!(output.typed_ref::<T>(), Some(&expected));
}
