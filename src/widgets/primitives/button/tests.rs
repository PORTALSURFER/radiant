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
            position: Point::new(12.0, 14.0)
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
fn button_text_alignment_can_be_overridden() {
    let mut button =
        ButtonWidget::new(11, "Folder", WidgetSizing::fixed(Vector2::new(120.0, 24.0)));

    assert_eq!(button.props.text_align, TextAlign::Center);
    assert!(button.set_text_align(TextAlign::Left));
    assert_eq!(button.props.text_align, TextAlign::Left);
}
