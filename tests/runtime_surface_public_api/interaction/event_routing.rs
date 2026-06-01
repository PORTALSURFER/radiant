use super::*;

#[test]
fn surface_runtime_resolves_host_shortcuts_before_widget_key_routing() {
    let mut runtime = SurfaceRuntime::new(ShortcutDemoBridge::default(), Vector2::new(420.0, 32.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::I),
        None,
        FocusSurface::None
    ));
    assert_eq!(runtime.bridge().state.count, 1);
}

#[test]
fn surface_runtime_routes_backend_neutral_events() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert_eq!(
        runtime.dispatch_event(Event::resize(Vector2::new(360.0, 40.0))),
        None
    );
    assert_eq!(runtime.viewport(), Vector2::new(360.0, 40.0));

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(150.0, 10.0))),
        Some(11)
    );
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), Some(11));
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(Point::new(150.0, 10.0))),
        Some(11)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        runtime.dispatch_event(Event::traverse_focus(FocusTraversal::Forward)),
        Some(12)
    );
    assert_eq!(runtime.dispatch_event(Event::character('R')), Some(12));

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "R (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "R"
    );
}

#[test]
fn surface_runtime_skips_duplicate_viewport_resize_work() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let initial_stats = runtime.layout().stats;

    runtime.dispatch_event(Event::resize(Vector2::new(420.0, 32.0)));

    assert_eq!(runtime.viewport(), Vector2::new(420.0, 32.0));
    assert_eq!(
        runtime.layout().stats,
        initial_stats,
        "duplicate logical resize should not replace the current layout evaluation"
    );
}

#[test]
fn backend_neutral_event_constructors_preserve_payloads() {
    let point = Point::new(20.0, 10.0);
    let delta = Vector2::new(0.0, -32.0);
    let modifiers = PointerModifiers {
        command: true,
        shift: true,
        alt: false,
    };

    assert_eq!(
        Event::pointer_press(point, PointerButton::Auxiliary, modifiers),
        Event::PointerPress {
            position: point,
            button: PointerButton::Auxiliary,
            modifiers,
        }
    );
    assert_eq!(
        Event::secondary_press(point),
        Event::PointerPress {
            position: point,
            button: PointerButton::Secondary,
            modifiers: PointerModifiers::default(),
        }
    );
    assert_eq!(
        Event::primary_release(point),
        Event::PointerRelease {
            position: point,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        }
    );
    assert_eq!(
        Event::primary_double_click(point),
        Event::PointerDoubleClick {
            position: point,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        }
    );
    assert_eq!(
        Event::scroll(point, delta),
        Event::Scroll {
            position: point,
            delta,
        }
    );
}

#[test]
fn surface_runtime_routes_pointer_click_convenience_through_press_and_release_events() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_primary_click(Point::new(150.0, 10.0));

    assert_eq!(outcome.press_target, Some(11));
    assert_eq!(outcome.release_target, Some(11));
    assert_eq!(outcome.completed_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.bridge().state().count, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Untitled (1)"
    );
}

#[test]
fn surface_runtime_routes_secondary_click_convenience() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_secondary_click(Point::new(150.0, 10.0));

    assert_eq!(outcome.press_target, Some(11));
    assert_eq!(outcome.release_target, Some(11));
    assert_eq!(outcome.completed_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), None);
}
