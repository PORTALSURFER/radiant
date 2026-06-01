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
    T: Clone + Debug + PartialEq + 'static,
{
    let output = output.expect("widget should emit output");
    assert_eq!(output.typed_ref::<T>(), Some(&expected));
    assert_eq!(output.typed_cloned::<T>(), Some(expected));
}

#[test]
fn widget_output_exposes_typed_and_custom_value_helpers() {
    let copied = WidgetOutput::typed(42_u8);
    assert_eq!(copied.typed_ref::<u8>(), Some(&42));
    assert_eq!(copied.typed_copied::<u8>(), Some(42));
    assert_eq!(copied.custom_copied::<u8>(), Some(42));

    let cloned = WidgetOutput::custom(String::from("activated"));
    assert_eq!(
        cloned.custom_ref::<String>().map(String::as_str),
        Some("activated")
    );
    assert_eq!(
        cloned.typed_cloned::<String>(),
        Some(String::from("activated"))
    );
    assert_eq!(
        cloned.custom_cloned::<String>(),
        Some(String::from("activated"))
    );
}

#[test]
fn widget_paint_primitives_helper_captures_builtin_widget_paint() {
    let widget = ButtonWidget::new(42, "Paint", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));

    let primitives = widget.paint_primitives_with_defaults(bounds);

    assert!(
        primitives
            .iter()
            .any(|primitive| primitive.fill_polygon().is_some()),
        "button chrome should be captured without app-local paint buffer setup"
    );
    assert!(
        primitives
            .iter()
            .any(|primitive| primitive.text_run().is_some()),
        "button label should be captured without app-local paint buffer setup"
    );
}
