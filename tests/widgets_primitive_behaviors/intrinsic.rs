use super::*;

#[test]
fn button_intrinsic_sizing_and_activation_are_public_and_deterministic() {
    let sizing = WidgetSizing::new(Vector2::new(80.0, 28.0), Vector2::new(96.0, 28.0));
    let mut button = ButtonWidget::new(1, "Import", sizing);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));

    assert_eq!(button.common.sizing, sizing);
    match button.common.layout_node() {
        radiant::layout::LayoutNode::Widget(node) => {
            assert_eq!(node.intrinsic, Vector2::new(96.0, 28.0));
        }
        other => panic!("expected widget leaf, got {other:?}"),
    }
    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(ButtonMessage::Activate)
    );
}

#[test]
fn badge_intrinsic_sizing_and_activation_are_public_and_deterministic() {
    let sizing = WidgetSizing::new(Vector2::new(56.0, 22.0), Vector2::new(72.0, 24.0));
    let mut badge = BadgeWidget::new(7, "Ready", sizing);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(72.0, 24.0));

    assert_eq!(badge.common.sizing, sizing);
    match badge.common.layout_node() {
        radiant::layout::LayoutNode::Widget(node) => {
            assert_eq!(node.intrinsic, Vector2::new(72.0, 24.0));
        }
        other => panic!("expected widget leaf, got {other:?}"),
    }
    assert_eq!(
        badge.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(
        badge.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(BadgeMessage::Activate)
    );
}

#[test]
fn card_intrinsic_sizing_is_public_and_non_interactive() {
    let sizing = WidgetSizing::new(Vector2::new(160.0, 96.0), Vector2::new(240.0, 120.0));
    let card = CardWidget::new(8, sizing);

    assert_eq!(card.common.sizing, sizing);
    assert!(!card.common.paint.paints_focus);
    match card.common.layout_node() {
        radiant::layout::LayoutNode::Widget(node) => {
            assert_eq!(node.intrinsic, Vector2::new(240.0, 120.0));
        }
        other => panic!("expected widget leaf, got {other:?}"),
    }
}

#[test]
fn image_intrinsic_sizing_reuses_shared_rgba_payload() {
    let image = Arc::new(ImageRgba::new(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 255]).unwrap());
    let sizing = WidgetSizing::fixed(Vector2::new(80.0, 40.0));
    let widget = ImageWidget::new(10, Arc::clone(&image), sizing);

    assert_eq!(widget.common.sizing, sizing);
    assert!(Arc::ptr_eq(&widget.props.image, &image));
    match widget.common.layout_node() {
        radiant::layout::LayoutNode::Widget(node) => {
            assert_eq!(node.intrinsic, Vector2::new(80.0, 40.0));
        }
        other => panic!("expected widget leaf, got {other:?}"),
    }
}
