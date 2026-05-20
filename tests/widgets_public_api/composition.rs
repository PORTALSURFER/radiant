use super::*;

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
