//! Focused public behavior coverage for reusable widget primitives.

use radiant::gui::types::{Point, Rect};
use radiant::{
    layout::Vector2,
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, PointerButton, ScrollbarAxis,
        ScrollbarMessage, ScrollbarWidget, TextInputMessage, TextInputWidget, ToggleMessage,
        ToggleWidget, WidgetInput, WidgetKey, WidgetSizing,
    },
};

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
            },
        ),
        Some(BadgeMessage::Activate)
    );
}

#[test]
fn toggle_updates_active_state_when_activated() {
    let mut toggle = ToggleWidget::new(2, "Enabled", WidgetSizing::fixed(Vector2::new(84.0, 28.0)));

    assert_eq!(
        toggle.handle_input(Rect::default(), WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        toggle.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Space)),
        Some(ToggleMessage::ValueChanged { checked: true })
    );
    assert!(toggle.common.state.active);
}

#[test]
fn text_input_edits_and_submits_single_line_values() {
    let mut input = TextInputWidget::new(
        3,
        "ab",
        WidgetSizing::new(Vector2::new(96.0, 28.0), Vector2::new(160.0, 28.0)),
    );

    let _ = input.handle_input(Rect::default(), WidgetInput::FocusChanged(true));
    input.state.caret = 1;

    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::Character('z')),
        Some(TextInputMessage::Changed {
            value: String::from("azb"),
        })
    );
    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Backspace)),
        Some(TextInputMessage::Changed {
            value: String::from("ab"),
        })
    );
    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(TextInputMessage::Submitted {
            value: String::from("ab"),
        })
    );
}

#[test]
fn scrollbar_drag_and_track_click_emit_normalized_offsets() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(12.0, 120.0));
    let mut scrollbar = ScrollbarWidget::new(
        4,
        ScrollbarAxis::Vertical,
        WidgetSizing::fixed(Vector2::new(12.0, 120.0)),
    );
    scrollbar.props.viewport_fraction = 0.25;
    let thumb = scrollbar.thumb_rect(bounds);
    let grip_y = thumb.min.y + thumb.height() * 0.5;

    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(6.0, grip_y),
                button: PointerButton::Primary,
            },
        ),
        None
    );
    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(6.0, 96.0),
            },
        ),
        Some(ScrollbarMessage::OffsetChanged {
            offset_fraction: 0.9,
        })
    );
    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(6.0, 96.0),
                button: PointerButton::Primary,
            },
        ),
        None
    );

    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(6.0, 12.0),
                button: PointerButton::Primary,
            },
        ),
        Some(ScrollbarMessage::OffsetChanged {
            offset_fraction: 0.0,
        })
    );
}
