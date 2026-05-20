use super::*;

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
fn control_and_media_widgets_support_named_parts_construction() {
    let slider = SliderWidget::from_parts(SliderWidgetParts {
        id: 37,
        value: 1.5,
        sizing: WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    });
    let scrollbar = ScrollbarWidget::from_parts(ScrollbarWidgetParts {
        id: 38,
        axis: ScrollbarAxis::Horizontal,
        sizing: WidgetSizing::fixed(Vector2::new(120.0, 12.0)),
    });
    let image_payload = Arc::new(ImageRgba::new(1, 1, vec![0, 0, 0, 255]).expect("valid image"));
    let image = ImageWidget::from_parts(ImageWidgetParts {
        id: 39,
        image: Arc::clone(&image_payload),
        sizing: WidgetSizing::fixed(Vector2::new(32.0, 32.0)),
    });
    let icon = SvgIcon::from_svg(
        r##"<svg viewBox="0 0 4 4" xmlns="http://www.w3.org/2000/svg"><rect width="4" height="4"/></svg>"##,
    )
    .expect("valid svg icon");
    let icon_button = IconButtonWidget::from_parts(IconButtonWidgetParts {
        id: 40,
        icon,
        sizing: WidgetSizing::fixed(Vector2::new(28.0, 28.0)),
    });

    assert_eq!(slider.common().id, 37);
    assert_eq!(slider.state.value, 1.0);
    assert_eq!(scrollbar.common().id, 38);
    assert_eq!(scrollbar.props.axis, ScrollbarAxis::Horizontal);
    assert_eq!(image.common().id, 39);
    assert!(Arc::ptr_eq(&image.props.image, &image_payload));
    assert_eq!(icon_button.common().id, 40);
    assert!(!icon_button.common().paint.paints_focus);
}

#[test]
fn sizing_only_widgets_support_named_parts_construction() {
    let canvas = CanvasWidget::from_parts(CanvasWidgetParts {
        id: 41,
        sizing: WidgetSizing::fixed(Vector2::new(160.0, 90.0)),
    });
    let card = CardWidget::from_parts(CardWidgetParts {
        id: 42,
        sizing: WidgetSizing::fixed(Vector2::new(96.0, 48.0)),
    });
    let drag = DragHandleWidget::from_parts(DragHandleWidgetParts {
        id: 43,
        sizing: WidgetSizing::fixed(Vector2::new(24.0, 24.0)),
    });
    let row = InteractiveRowWidget::from_parts(InteractiveRowWidgetParts {
        id: 44,
        sizing: WidgetSizing::fixed(Vector2::new(240.0, 28.0)),
    })
    .with_drag()
    .with_drop_target(true);

    assert_eq!(canvas.common().id, 41);
    assert!(!canvas.common().paint.paints_state_layers);
    assert_eq!(card.common().id, 42);
    assert!(card.common().paint.suppresses_container_hover);
    assert_eq!(drag.common().id, 43);
    assert_eq!(row.common().id, 44);
    assert!(row.props.draggable);
    assert!(row.props.droppable);
    assert!(row.props.drag_active);
}
