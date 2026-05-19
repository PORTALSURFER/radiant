//! Public API coverage for `radiant::widgets`.

use radiant::{
    gui::types::ImageRgba,
    layout::{
        ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams, Vector2,
        layout_tree,
    },
    widgets::{
        BadgeWidget, BadgeWidgetParts, ButtonWidget, ButtonWidgetParts, CardWidget,
        DragHandleWidget, ImageWidget, ListItemWidget, ListItemWidgetParts, ScrollbarAxis,
        ScrollbarWidget, SelectableWidget, SelectableWidgetParts, TextInputWidget,
        TextInputWidgetParts, TextWidget, TextWidgetParts, ToggleWidget, ToggleWidgetParts, Widget,
        WidgetInput, WidgetKey, WidgetOutput, WidgetSizing, WidgetSizingParts,
    },
};
use std::{fmt::Debug, sync::Arc};

fn assert_typed_widget_output<T>(output: Option<WidgetOutput>, expected: T)
where
    T: Debug + PartialEq + 'static,
{
    let output = output.expect("widget should emit output");
    assert_eq!(output.typed_ref::<T>(), Some(&expected));
}

#[test]
fn public_widgets_compose_with_public_layout_containers() {
    let header = TextWidget::new(
        2,
        "Projects",
        WidgetSizing::fixed(Vector2::new(72.0, 20.0)).with_baseline(14.0),
    );
    let rename = ButtonWidget::new(3, "Rename", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));
    let filter = TextInputWidget::new(
        4,
        "",
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(200.0, 28.0)),
    );
    let snap = ToggleWidget::new(5, "Snap", WidgetSizing::fixed(Vector2::new(84.0, 28.0)))
        .with_checked(true);
    let muted_snap = ToggleWidget::new(17, "Muted", WidgetSizing::fixed(Vector2::new(84.0, 28.0)));
    let scroll = ScrollbarWidget::new(
        6,
        ScrollbarAxis::Vertical,
        WidgetSizing::fixed(Vector2::new(12.0, 28.0)),
    );
    let drag = DragHandleWidget::new(18, WidgetSizing::fixed(Vector2::new(24.0, 24.0)));
    let badge = BadgeWidget::new(7, "Ready", WidgetSizing::fixed(Vector2::new(72.0, 24.0)));
    let card = CardWidget::new(8, WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let item = ListItemWidget::new(
        9,
        "Document",
        WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
    );
    let selectable = SelectableWidget::new(
        16,
        "Selected",
        true,
        WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
    );
    let image_payload =
        Arc::new(ImageRgba::new(1, 1, vec![255, 255, 255, 255]).expect("valid image"));
    let image = ImageWidget::new(
        15,
        image_payload,
        WidgetSizing::fixed(Vector2::new(32.0, 28.0)),
    );

    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 8.0,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(SlotParams::fill(), header.common().layout_node()),
            SlotChild::new(SlotParams::fill(), rename.common().layout_node()),
            SlotChild::new(SlotParams::fill(), filter.common().layout_node()),
            SlotChild::new(SlotParams::fill(), snap.common().layout_node()),
            SlotChild::new(SlotParams::fill(), muted_snap.common().layout_node()),
            SlotChild::new(SlotParams::fill(), scroll.common().layout_node()),
            SlotChild::new(SlotParams::fill(), drag.common().layout_node()),
            SlotChild::new(SlotParams::fill(), badge.common().layout_node()),
            SlotChild::new(SlotParams::fill(), card.common().layout_node()),
            SlotChild::new(SlotParams::fill(), item.common().layout_node()),
            SlotChild::new(SlotParams::fill(), selectable.common().layout_node()),
            SlotChild::new(SlotParams::fill(), image.common().layout_node()),
        ],
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(920.0, 32.0)),
    );

    assert!(output.rects.contains_key(&header.common().id));
    assert!(output.rects.contains_key(&rename.common().id));
    assert!(output.rects.contains_key(&filter.common().id));
    assert!(output.rects.contains_key(&snap.common().id));
    assert!(output.rects.contains_key(&muted_snap.common().id));
    assert!(output.rects.contains_key(&scroll.common().id));
    assert!(output.rects.contains_key(&drag.common().id));
    assert!(output.rects.contains_key(&badge.common().id));
    assert!(output.rects.contains_key(&card.common().id));
    assert!(output.rects.contains_key(&item.common().id));
    assert!(output.rects.contains_key(&selectable.common().id));
    assert!(output.rects.contains_key(&image.common().id));
    assert!(snap.common().state.active);
    assert!(!muted_snap.common().state.active);
    assert!(!rename.common().state.disabled);
    assert!(!filter.common().state.disabled);
    assert!(!scroll.common().state.disabled);
    assert!(!drag.common().state.disabled);
    assert!(!item.common().state.disabled);
    assert!(selectable.common().state.selected);
    assert!(!image.common().paint.paints_focus);
    assert!(!card.common().paint.paints_focus);
    assert_eq!(
        badge.common().style.prominence,
        radiant::widgets::WidgetProminence::Subtle
    );
}

#[test]
fn widget_sizing_supports_named_parts_construction() {
    let sizing = WidgetSizing::from_parts(WidgetSizingParts {
        min: Vector2::new(120.0, 32.0),
        preferred: Vector2::new(96.0, 24.0),
        baseline: Some(-4.0),
    });

    assert_eq!(sizing.min, Vector2::new(120.0, 32.0));
    assert_eq!(sizing.preferred, Vector2::new(120.0, 32.0));
    assert_eq!(sizing.baseline, Some(0.0));
}

#[test]
fn labeled_primitive_widgets_support_named_parts_construction() {
    let text = TextWidget::from_parts(TextWidgetParts {
        id: 30,
        text: "Projects".into(),
        sizing: WidgetSizing::fixed(Vector2::new(72.0, 20.0)).with_baseline(14.0),
    });
    let button = ButtonWidget::from_parts(ButtonWidgetParts {
        id: 31,
        label: "Import".into(),
        sizing: WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    })
    .with_secondary_click();
    let toggle = ToggleWidget::from_parts(ToggleWidgetParts {
        id: 32,
        label: "Snap".into(),
        sizing: WidgetSizing::fixed(Vector2::new(84.0, 28.0)),
    })
    .with_checked(true);

    assert_eq!(text.common().id, 30);
    assert!(!text.common().paint.paints_focus);
    assert_eq!(button.common().id, 31);
    assert!(button.props.secondary_click);
    assert_eq!(toggle.common().id, 32);
    assert!(toggle.common().state.active);
}

#[test]
fn interactive_primitive_widgets_support_named_parts_construction() {
    let badge = BadgeWidget::from_parts(BadgeWidgetParts {
        id: 33,
        label: "Ready".into(),
        sizing: WidgetSizing::fixed(Vector2::new(72.0, 24.0)),
    });
    let item = ListItemWidget::from_parts(ListItemWidgetParts {
        id: 34,
        label: "Document".into(),
        sizing: WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
    });
    let selectable = SelectableWidget::from_parts(SelectableWidgetParts {
        id: 35,
        label: "Selected".into(),
        selected: true,
        sizing: WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
    });
    let input = TextInputWidget::from_parts(TextInputWidgetParts {
        id: 36,
        value: String::from("filter"),
        sizing: WidgetSizing::new(Vector2::new(96.0, 28.0), Vector2::new(160.0, 28.0)),
    });

    assert_eq!(badge.common().id, 33);
    assert_eq!(
        badge.common().style.prominence,
        radiant::widgets::WidgetProminence::Subtle
    );
    assert_eq!(item.common().id, 34);
    assert_eq!(item.detail, None);
    assert_eq!(selectable.common().id, 35);
    assert!(selectable.common().state.selected);
    assert_eq!(input.common().id, 36);
    assert_eq!(input.state.value, "filter");
}

#[test]
fn public_widgets_dispatch_messages_for_reusable_controls() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));
    let mut button = ButtonWidget::new(10, "Import", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let mut toggle =
        ToggleWidget::new(11, "Enabled", WidgetSizing::fixed(Vector2::new(84.0, 28.0)));
    let mut input = TextInputWidget::new(
        12,
        "ab",
        WidgetSizing::new(Vector2::new(96.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    let mut badge = BadgeWidget::new(13, "Ready", WidgetSizing::fixed(Vector2::new(64.0, 24.0)));
    let mut drag = DragHandleWidget::new(17, WidgetSizing::fixed(Vector2::new(24.0, 24.0)));
    let mut item = ListItemWidget::new(
        14,
        "Document",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );
    let mut selectable = SelectableWidget::new(
        16,
        "Selected",
        false,
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );

    assert_eq!(
        Widget::handle_input(&mut button, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut button, bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        radiant::widgets::ButtonMessage::Activate,
    );

    assert_eq!(
        Widget::handle_input(&mut toggle, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut toggle, bounds, WidgetInput::KeyPress(WidgetKey::Space)),
        radiant::widgets::ToggleMessage::ValueChanged { checked: true },
    );

    assert_eq!(
        Widget::handle_input(&mut input, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut input, bounds, WidgetInput::Character('z')),
        radiant::widgets::TextInputMessage::Changed {
            value: String::from("abz"),
        },
    );

    assert_eq!(
        Widget::handle_input(&mut badge, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut badge, bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        radiant::widgets::BadgeMessage::Activate,
    );

    assert_typed_widget_output(
        Widget::handle_input(
            &mut drag,
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(10.0, 10.0),
                button: radiant::widgets::PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        radiant::widgets::DragHandleMessage::Started {
            position: Point::new(10.0, 10.0),
        },
    );
    assert_typed_widget_output(
        Widget::handle_input(
            &mut drag,
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(10.0, 38.0),
            },
        ),
        radiant::widgets::DragHandleMessage::Moved {
            position: Point::new(10.0, 38.0),
        },
    );

    assert_eq!(
        Widget::handle_input(&mut item, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut item, bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        radiant::widgets::ListItemMessage::Invoked,
    );

    assert_eq!(
        Widget::handle_input(&mut selectable, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(
            &mut selectable,
            bounds,
            WidgetInput::KeyPress(WidgetKey::Space),
        ),
        radiant::widgets::SelectableMessage::SelectionChanged { selected: true },
    );
}
