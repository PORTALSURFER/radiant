use super::*;
use crate::{
    gui::types::Point,
    widgets::{
        CanvasPointer,
        interaction::{PointerButton, PointerModifiers},
    },
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
    assert!(pointer.is_inside(bounds()));
    assert_eq!(pointer.normalized_x(), 0.25);
    assert_eq!(pointer.normalized_y(), 0.5);
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
fn canvas_gesture_event_exposes_shared_pointer_metadata() {
    let mut state = CanvasGestureState::new();
    let modifiers = PointerModifiers {
        command: true,
        ..PointerModifiers::default()
    };

    state.handle_input(
        bounds(),
        &WidgetInput::PointerPress {
            position: Point::new(20.0, 30.0),
            button: PointerButton::Secondary,
            modifiers,
        },
    );
    let event = state
        .handle_input(
            bounds(),
            &WidgetInput::PointerMove {
                position: Point::new(35.0, 40.0),
            },
        )
        .unwrap();

    assert_eq!(
        event.pointer().map(|pointer| pointer.position),
        Some(Point::new(35.0, 40.0))
    );
    assert_eq!(
        event.origin().map(|pointer| pointer.position),
        Some(Point::new(20.0, 30.0))
    );
    assert_eq!(event.button(), Some(PointerButton::Secondary));
    assert_eq!(event.modifiers(), Some(modifiers));
    assert_eq!(event.delta(), Some(Vector2::new(15.0, 10.0)));
    assert!(event.pointer_is_inside(bounds()));
}

#[test]
fn canvas_gesture_event_extracts_common_event_shapes() {
    let mut state = CanvasGestureState::new();
    let modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };

    let hover = state
        .handle_input(
            bounds(),
            &WidgetInput::PointerMove {
                position: Point::new(15.0, 25.0),
            },
        )
        .unwrap();
    assert_eq!(
        hover.hover_pointer().map(|pointer| pointer.position),
        Some(Point::new(15.0, 25.0))
    );
    assert_eq!(hover.press_pointer(PointerButton::Primary), None);

    let press = state
        .handle_input(
            bounds(),
            &WidgetInput::PointerPress {
                position: Point::new(20.0, 30.0),
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .unwrap();
    assert_eq!(
        press
            .press_pointer(PointerButton::Primary)
            .map(|pointer| pointer.position),
        Some(Point::new(20.0, 30.0))
    );
    assert_eq!(
        press
            .press_pointer_inside(bounds(), PointerButton::Primary)
            .map(|pointer| pointer.position),
        Some(Point::new(20.0, 30.0))
    );
    assert_eq!(press.press_pointer(PointerButton::Secondary), None);

    let release = state
        .handle_input(
            bounds(),
            &WidgetInput::PointerRelease {
                position: Point::new(30.0, 35.0),
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .unwrap();
    assert_eq!(
        release
            .release_pointer(PointerButton::Primary)
            .map(|pointer| pointer.position),
        Some(Point::new(30.0, 35.0))
    );
    assert_eq!(
        release
            .release_pointer_inside(bounds(), PointerButton::Primary)
            .map(|pointer| pointer.position),
        Some(Point::new(30.0, 35.0))
    );

    let double_click = state
        .handle_input(
            bounds(),
            &WidgetInput::PointerDoubleClick {
                position: Point::new(40.0, 45.0),
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .unwrap();
    assert_eq!(
        double_click
            .double_click_pointer(PointerButton::Primary)
            .map(|pointer| pointer.position),
        Some(Point::new(40.0, 45.0))
    );
    assert_eq!(
        double_click
            .double_click_pointer_inside(bounds(), PointerButton::Primary)
            .map(|pointer| pointer.position),
        Some(Point::new(40.0, 45.0))
    );

    let wheel = state
        .handle_input(
            bounds(),
            &WidgetInput::plain_wheel(Point::new(50.0, 55.0), Vector2::new(0.0, -120.0)),
        )
        .unwrap();
    assert_eq!(
        wheel
            .wheel_pointer_delta()
            .map(|(pointer, delta)| (pointer.position, delta)),
        Some((Point::new(50.0, 55.0), Vector2::new(0.0, -120.0)))
    );
    assert_eq!(
        wheel
            .wheel_pointer_delta_inside(bounds())
            .map(|(pointer, delta)| (pointer.position, delta)),
        Some((Point::new(50.0, 55.0), Vector2::new(0.0, -120.0)))
    );

    let outside_press = CanvasGestureEvent::Press {
        pointer: CanvasPointer {
            position: Point::new(500.0, 500.0),
            local: Point::new(490.0, 480.0),
            normalized: Vector2::new(1.0, 1.0),
        },
        button: PointerButton::Primary,
        modifiers,
    };
    assert!(
        outside_press
            .press_pointer(PointerButton::Primary)
            .is_some()
    );
    assert_eq!(
        outside_press.press_pointer_inside(bounds(), PointerButton::Primary),
        None
    );
}

#[test]
fn canvas_gesture_event_accessors_handle_non_pointer_events() {
    let event = CanvasGestureEvent::FocusChanged(false);

    assert_eq!(event.pointer(), None);
    assert_eq!(event.origin(), None);
    assert_eq!(event.button(), None);
    assert_eq!(event.modifiers(), None);
    assert_eq!(event.delta(), None);
    assert_eq!(event.hover_pointer(), None);
    assert_eq!(event.press_pointer(PointerButton::Primary), None);
    assert_eq!(
        event.press_pointer_inside(bounds(), PointerButton::Primary),
        None
    );
    assert_eq!(event.double_click_pointer(PointerButton::Primary), None);
    assert_eq!(
        event.double_click_pointer_inside(bounds(), PointerButton::Primary),
        None
    );
    assert_eq!(event.release_pointer(PointerButton::Primary), None);
    assert_eq!(
        event.release_pointer_inside(bounds(), PointerButton::Primary),
        None
    );
    assert_eq!(event.wheel_pointer_delta(), None);
    assert_eq!(event.wheel_pointer_delta_inside(bounds()), None);
    assert!(!event.pointer_is_inside(bounds()));
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
