use super::*;
use crate::gui::types::{Point, Vector2};
use crate::widgets::interaction::{PointerButton, WidgetInput};

#[test]
fn scrollbar_drag_emits_clamped_offset_changes() {
    let mut scrollbar = ScrollbarWidget::new(
        9,
        ScrollbarAxis::Vertical,
        WidgetSizing::fixed(Vector2::new(12.0, 120.0)),
    );
    scrollbar.props.viewport_fraction = 0.25;
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(12.0, 120.0));
    let thumb = scrollbar.thumb_rect(bounds);
    let grip_y = thumb.min.y + thumb.height() * 0.5;

    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(6.0, grip_y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        None
    );

    let message = scrollbar.handle_input(bounds, WidgetInput::pointer_move(Point::new(6.0, 96.0)));
    assert_eq!(
        message,
        Some(ScrollbarMessage::OffsetChanged {
            offset_fraction: 0.9,
        })
    );
}

#[test]
fn scrollbar_track_click_centers_thumb() {
    let mut scrollbar = ScrollbarWidget::new(
        10,
        ScrollbarAxis::Horizontal,
        WidgetSizing::fixed(Vector2::new(120.0, 12.0)),
    );
    scrollbar.props.viewport_fraction = 0.5;
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 12.0));

    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(90.0, 6.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        Some(ScrollbarMessage::OffsetChanged {
            offset_fraction: 1.0,
        })
    );
}

#[test]
fn pointer_capture_cancellation_clears_scrollbar_drag_without_losing_focus() {
    let mut scrollbar = ScrollbarWidget::new(
        11,
        ScrollbarAxis::Vertical,
        WidgetSizing::fixed(Vector2::new(12.0, 120.0)),
    );
    scrollbar.props.viewport_fraction = 0.25;
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(12.0, 120.0));
    let thumb = scrollbar.thumb_rect(bounds);
    let position = Point::new(6.0, thumb.min.y + thumb.height() * 0.5);

    assert!(
        scrollbar
            .handle_input(bounds, WidgetInput::primary_press(position))
            .is_none()
    );
    assert!(scrollbar.common.state.focused);
    assert!(scrollbar.common.state.pressed);
    assert!(scrollbar.state.drag_grip_fraction.is_some());
    let offset_before_cancel = scrollbar.state.offset_fraction;

    assert!(Widget::handle_pointer_capture_cancelled(&mut scrollbar, bounds).is_none());
    assert!(scrollbar.common.state.focused);
    assert!(!scrollbar.common.state.pressed);
    assert!(scrollbar.state.drag_grip_fraction.is_none());
    assert_eq!(scrollbar.state.offset_fraction, offset_before_cancel);
}
