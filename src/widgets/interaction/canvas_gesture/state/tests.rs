use super::*;
use crate::{
    gui::types::Point,
    widgets::interaction::{PointerButton, PointerModifiers},
};

fn bounds() -> Rect {
    Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(100.0, 50.0))
}

#[test]
fn canvas_gesture_state_projects_local_and_normalized_positions() {
    let mut state = CanvasGestureState::new();
    let event = state
        .handle_input(
            bounds(),
            &WidgetInput::PointerMove {
                position: Point::new(35.0, 45.0),
            },
        )
        .unwrap();

    let CanvasGestureEvent::Hover(pointer) = event else {
        panic!("expected hover event");
    };
    assert_eq!(pointer.local, Point::new(25.0, 25.0));
    assert_eq!(pointer.normalized, Vector2::new(0.25, 0.5));
}

#[test]
fn canvas_gesture_state_tracks_press_drag_and_release() {
    let mut state = CanvasGestureState::new();
    let modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };

    state.handle_input(
        bounds(),
        &WidgetInput::PointerPress {
            position: Point::new(20.0, 30.0),
            button: PointerButton::Primary,
            modifiers,
        },
    );
    assert!(state.is_dragging());

    let drag = state
        .handle_input(
            bounds(),
            &WidgetInput::PointerMove {
                position: Point::new(25.0, 42.0),
            },
        )
        .unwrap();
    let CanvasGestureEvent::Drag {
        origin,
        delta,
        button,
        modifiers: drag_modifiers,
        ..
    } = drag
    else {
        panic!("expected drag event");
    };
    assert_eq!(origin.position, Point::new(20.0, 30.0));
    assert_eq!(delta, Vector2::new(5.0, 12.0));
    assert_eq!(button, PointerButton::Primary);
    assert_eq!(drag_modifiers, modifiers);

    let release = state
        .handle_input(
            bounds(),
            &WidgetInput::PointerRelease {
                position: Point::new(30.0, 35.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .unwrap();
    let CanvasGestureEvent::Release { delta, .. } = release else {
        panic!("expected release event");
    };
    assert_eq!(delta, Vector2::new(10.0, 5.0));
    assert!(!state.is_dragging());
}

#[test]
fn canvas_gesture_state_clears_drag_on_focus_loss() {
    let mut state = CanvasGestureState::new();
    state.handle_input(
        bounds(),
        &WidgetInput::PointerPress {
            position: Point::new(20.0, 30.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert!(matches!(
        state.handle_input(bounds(), &WidgetInput::FocusChanged(false)),
        Some(CanvasGestureEvent::FocusChanged(false))
    ));
    assert!(!state.is_dragging());
}
