//! Public API coverage for `radiant::widgets`.

use radiant::{
    layout::{
        ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams, Vector2,
        layout_tree,
    },
    widgets::{
        ButtonWidget, TextInputWidget, TextWidget, WidgetMessageKind, WidgetSizing, WidgetSpec,
    },
};

#[test]
fn public_widgets_compose_with_public_layout_containers() {
    let header = WidgetSpec::Text(TextWidget::new(
        2,
        "Folders",
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
        ],
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 32.0)),
    );

    assert!(output.rects.contains_key(&header.id()));
    assert!(output.rects.contains_key(&rename.id()));
    assert!(output.rects.contains_key(&filter.id()));
    assert_eq!(
        rename.common().emitted_messages,
        vec![WidgetMessageKind::Activate]
    );
    assert_eq!(
        filter.common().emitted_messages,
        vec![WidgetMessageKind::TextEdited]
    );
}
