use crate::gui::types::{Point, Vector2};
use crate::widgets::interaction::{DragHandleMessage, PointerButton, WidgetInput, WidgetKey};
use std::sync::Arc;

use super::*;

#[test]
fn button_releases_inside_bounds_emit_activation() {
    let mut button = ButtonWidget::new(5, "Play", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(80.0, 28.0));

    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(20.0, 30.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert!(button.common.state.pressed);

    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(24.0, 32.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(ButtonMessage::Activate)
    );
    assert!(!button.common.state.pressed);
}

#[test]
fn focused_button_space_emits_activation() {
    let mut button = ButtonWidget::new(6, "Stop", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));

    let _ = button.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

    assert_eq!(
        button.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Space)),
        Some(ButtonMessage::Activate)
    );
}

#[test]
fn secondary_click_only_emits_when_enabled() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut default_button =
        ButtonWidget::new(7, "More", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    let mut context_button =
        ButtonWidget::new(8, "More", WidgetSizing::fixed(Vector2::new(80.0, 28.0)))
            .with_secondary_click();

    let secondary_press = WidgetInput::PointerPress {
        position: Point::new(10.0, 10.0),
        button: PointerButton::Secondary,
        modifiers: Default::default(),
    };

    assert_eq!(
        default_button.handle_input(bounds, secondary_press.clone()),
        None
    );
    assert_eq!(
        context_button.handle_input(bounds, secondary_press),
        Some(ButtonMessage::SecondaryActivate {
            position: Point::new(10.0, 10.0),
        })
    );
}

#[test]
fn draggable_button_emits_drag_lifecycle_instead_of_click_when_moved() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut button =
        ButtonWidget::new(9, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();

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
            WidgetInput::PointerMove {
                position: Point::new(12.0, 14.0),
            },
        ),
        Some(ButtonMessage::Drag(DragHandleMessage::Started {
            origin: Point::new(10.0, 10.0),
            position: Point::new(12.0, 14.0),
        }))
    );
    assert_eq!(
        button.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(20.0, 22.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(ButtonMessage::Drag(DragHandleMessage::Ended {
            position: Point::new(20.0, 22.0)
        }))
    );
}

#[test]
fn draggable_button_ignores_tiny_pointer_jitter_before_click_release() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut button =
        ButtonWidget::new(17, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    let press_point = Point::new(10.0, 10.0);
    let jitter_point = Point::new(12.0, 11.0);

    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_press(press_point)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::pointer_move(jitter_point)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_release(jitter_point)),
        Some(ButtonMessage::Activate)
    );
}

#[test]
fn draggable_button_release_after_capture_state_restore_ends_drag() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut button =
        ButtonWidget::new(15, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    let press_point = Point::new(10.0, 10.0);
    let move_point = Point::new(100.0, 10.0);
    let release_point = Point::new(140.0, 10.0);

    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_press(press_point)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::pointer_move(move_point)),
        Some(ButtonMessage::Drag(DragHandleMessage::Started {
            origin: press_point,
            position: move_point,
        }))
    );

    let restored_common_state = button.common.state;
    let mut refreshed =
        ButtonWidget::new(15, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    refreshed.common.state = restored_common_state;

    assert_eq!(
        refreshed.handle_input(bounds, WidgetInput::primary_release(release_point)),
        Some(ButtonMessage::Drag(DragHandleMessage::Ended {
            position: release_point
        }))
    );
    assert!(!refreshed.common.state.active);
}

#[test]
fn draggable_button_focus_loss_cancels_drag() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut button =
        ButtonWidget::new(16, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0))).with_drag();
    let press_point = Point::new(10.0, 10.0);
    let move_point = Point::new(30.0, 10.0);

    assert_eq!(
        button.handle_input(bounds, WidgetInput::primary_press(press_point)),
        None
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::pointer_move(move_point)),
        Some(ButtonMessage::Drag(DragHandleMessage::Started {
            origin: press_point,
            position: move_point,
        }))
    );
    assert_eq!(
        button.handle_input(bounds, WidgetInput::FocusChanged(false)),
        Some(ButtonMessage::Drag(DragHandleMessage::Cancelled {
            position: press_point
        }))
    );
    assert!(!button.common.state.pressed);
    assert!(!button.common.state.active);
    assert!(!button.state.dragged);
}

#[test]
fn button_message_helpers_classify_common_outputs() {
    let secondary_position = Point::new(10.0, 12.0);
    let drag_position = Point::new(18.0, 20.0);
    let drag = DragHandleMessage::Moved {
        position: drag_position,
    };

    assert!(ButtonMessage::Activate.is_activate());
    assert_eq!(ButtonMessage::Activate.secondary_position(), None);
    assert_eq!(ButtonMessage::Activate.drag_message(), None);

    let secondary = ButtonMessage::SecondaryActivate {
        position: secondary_position,
    };
    assert!(!secondary.is_activate());
    assert_eq!(secondary.secondary_position(), Some(secondary_position));
    assert_eq!(secondary.drag_message(), None);

    let drag_message = ButtonMessage::Drag(drag);
    assert!(!drag_message.is_activate());
    assert_eq!(drag_message.secondary_position(), None);
    assert_eq!(drag_message.drag_message(), Some(drag));
}

#[test]
fn button_chrome_shares_fill_and_stroke_point_storage() {
    let button = ButtonWidget::new(10, "Play", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut primitives = Vec::new();

    button.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let fill_points = primitives.iter().find_map(|primitive| match primitive {
        PaintPrimitive::FillPolygon(fill) => Some(&fill.points),
        _ => None,
    });
    let stroke_points = primitives.iter().find_map(|primitive| match primitive {
        PaintPrimitive::StrokePolygon(stroke) => Some(&stroke.points),
        _ => None,
    });

    assert!(
        fill_points
            .zip(stroke_points)
            .is_some_and(|(fill, stroke)| Arc::ptr_eq(fill, stroke))
    );
}

#[test]
fn input_only_button_does_not_paint_chrome_or_text() {
    let mut button = ButtonWidget::new(12, "", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
    button.common.paint.paints_state_layers = false;
    let mut primitives = Vec::new();

    button.append_paint(
        &mut primitives,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(primitives.is_empty());
}

#[test]
fn hover_chrome_only_button_paints_only_when_hovered() {
    let mut button = ButtonWidget::new(13, "", WidgetSizing::fixed(Vector2::new(80.0, 28.0)))
        .with_hover_chrome_only();
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
    let mut primitives = Vec::new();

    button.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(primitives.is_empty());

    let _ = button.handle_input(bounds, WidgetInput::pointer_move(Point::new(10.0, 10.0)));
    button.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_)))
    );
}

#[test]
fn button_opts_into_state_synchronization() {
    let button = ButtonWidget::new(14, "Drag", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));

    assert!(button.needs_state_synchronization());
}

#[test]
fn button_text_alignment_can_be_overridden() {
    let mut button =
        ButtonWidget::new(11, "Folder", WidgetSizing::fixed(Vector2::new(120.0, 24.0)));

    assert_eq!(button.props.text_align, TextAlign::Center);
    assert!(button.set_text_align(TextAlign::Left));
    assert_eq!(button.props.text_align, TextAlign::Left);
}
