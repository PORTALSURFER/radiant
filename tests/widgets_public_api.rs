//! Public API coverage for `radiant::widgets`.

use radiant::{
    layout::{
        ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams, Vector2,
        layout_tree,
    },
    widgets::{
        BadgeWidget, ButtonWidget, CardWidget, ListItemWidget, ScrollbarAxis, ScrollbarWidget,
        TextInputWidget, TextWidget, ToggleWidget, WidgetInput, WidgetKey, WidgetMessageKind,
        WidgetOutput, WidgetSizing, WidgetSpec,
    },
};

#[test]
fn public_widgets_compose_with_public_layout_containers() {
    let header = WidgetSpec::Text(TextWidget::new(
        2,
        "Projects",
        WidgetSizing::fixed(Vector2::new(72.0, 20.0)).with_baseline(14.0),
    ));
    let rename = WidgetSpec::Button(ButtonWidget::new(
        3,
        "Rename",
        WidgetSizing::fixed(Vector2::new(88.0, 28.0)),
    ));
    let filter = WidgetSpec::TextInput(TextInputWidget::new(
        4,
        "",
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(200.0, 28.0)),
    ));
    let snap = WidgetSpec::Toggle(ToggleWidget::new(
        5,
        "Snap",
        WidgetSizing::fixed(Vector2::new(84.0, 28.0)),
    ));
    let scroll = WidgetSpec::Scrollbar(ScrollbarWidget::new(
        6,
        ScrollbarAxis::Vertical,
        WidgetSizing::fixed(Vector2::new(12.0, 28.0)),
    ));
    let badge = WidgetSpec::Badge(BadgeWidget::new(
        7,
        "Ready",
        WidgetSizing::fixed(Vector2::new(72.0, 24.0)),
    ));
    let card = WidgetSpec::Card(CardWidget::new(
        8,
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));
    let item = WidgetSpec::ListItem(ListItemWidget::new(
        9,
        "Document",
        WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
    ));

    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 8.0,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(SlotParams::fill(), header.layout_node()),
            SlotChild::new(SlotParams::fill(), rename.layout_node()),
            SlotChild::new(SlotParams::fill(), filter.layout_node()),
            SlotChild::new(SlotParams::fill(), snap.layout_node()),
            SlotChild::new(SlotParams::fill(), scroll.layout_node()),
            SlotChild::new(SlotParams::fill(), badge.layout_node()),
            SlotChild::new(SlotParams::fill(), card.layout_node()),
            SlotChild::new(SlotParams::fill(), item.layout_node()),
        ],
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 32.0)),
    );

    assert!(output.rects.contains_key(&header.id()));
    assert!(output.rects.contains_key(&rename.id()));
    assert!(output.rects.contains_key(&filter.id()));
    assert!(output.rects.contains_key(&snap.id()));
    assert!(output.rects.contains_key(&scroll.id()));
    assert!(output.rects.contains_key(&badge.id()));
    assert!(output.rects.contains_key(&card.id()));
    assert!(output.rects.contains_key(&item.id()));
    assert_eq!(
        rename.common().emitted_messages,
        vec![WidgetMessageKind::Activate]
    );
    assert_eq!(
        filter.common().emitted_messages,
        vec![WidgetMessageKind::TextEdited]
    );
    assert_eq!(
        snap.common().emitted_messages,
        vec![WidgetMessageKind::ValueChanged]
    );
    assert_eq!(
        scroll.common().emitted_messages,
        vec![WidgetMessageKind::ScrollRequested]
    );
    assert_eq!(
        item.common().emitted_messages,
        vec![WidgetMessageKind::ItemInvoked]
    );
    assert_eq!(
        badge.common().emitted_messages,
        vec![WidgetMessageKind::Activate]
    );
    assert!(card.common().emitted_messages.is_empty());
}

#[test]
fn widget_spec_dispatches_public_messages_for_reusable_controls() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));
    let mut button = WidgetSpec::Button(ButtonWidget::new(
        10,
        "Import",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));
    let mut toggle = WidgetSpec::Toggle(ToggleWidget::new(
        11,
        "Enabled",
        WidgetSizing::fixed(Vector2::new(84.0, 28.0)),
    ));
    let mut input = WidgetSpec::TextInput(TextInputWidget::new(
        12,
        "ab",
        WidgetSizing::new(Vector2::new(96.0, 28.0), Vector2::new(160.0, 28.0)),
    ));
    let mut badge = WidgetSpec::Badge(BadgeWidget::new(
        13,
        "Ready",
        WidgetSizing::fixed(Vector2::new(64.0, 24.0)),
    ));
    let mut item = WidgetSpec::ListItem(ListItemWidget::new(
        14,
        "Document",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    ));

    assert_eq!(
        button.handle_input(bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(WidgetOutput::Button(
            radiant::widgets::ButtonMessage::Activate
        ))
    );

    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Space)),
        Some(WidgetOutput::Toggle(
            radiant::widgets::ToggleMessage::ValueChanged { checked: true }
        ))
    );

    assert_eq!(
        input.handle_input(bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        input.handle_input(bounds, WidgetInput::Character('z')),
        Some(WidgetOutput::TextInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("abz"),
            }
        ))
    );

    assert_eq!(
        badge.handle_input(bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        badge.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(WidgetOutput::Badge(
            radiant::widgets::BadgeMessage::Activate
        ))
    );

    assert_eq!(
        item.handle_input(bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        item.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(WidgetOutput::ListItem(
            radiant::widgets::ListItemMessage::Invoked
        ))
    );
}
